use std::{
    collections::{HashMap, VecDeque},
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use axum::{
    Router,
    body::{Body, to_bytes},
    extract::{Request, State},
    http::{HeaderMap, Method, StatusCode},
    response::{IntoResponse, Response},
};
use base64::{Engine, engine::general_purpose::STANDARD};
use hmac::{Hmac, Mac};
use serde_json::{Value, json};
use sha2::Sha256;
use switchbot_exporter::{
    config::{Config, Values},
    server,
    switchbot::{Device, Switchbot, signature},
};
use tokio::{net::TcpListener, sync::Notify, task::JoinHandle};
use tower::ServiceExt;

#[derive(Clone)]
struct MockResponse {
    status: StatusCode,
    body: String,
}

impl MockResponse {
    fn json(value: Value) -> Self {
        Self {
            status: StatusCode::OK,
            body: value.to_string(),
        }
    }
}

#[derive(Clone, Debug)]
struct RecordedRequest {
    method: Method,
    path: String,
    headers: HeaderMap,
}

struct MockState {
    responses: Mutex<HashMap<String, VecDeque<MockResponse>>>,
    requests: Mutex<Vec<RecordedRequest>>,
    changed: Notify,
    stalled: AtomicBool,
}

struct MockApi {
    state: Arc<MockState>,
    url: reqwest::Url,
    server: JoinHandle<()>,
}

impl MockApi {
    async fn new(responses: Vec<(String, Vec<MockResponse>)>) -> Self {
        let state = Arc::new(MockState {
            responses: Mutex::new(
                responses
                    .into_iter()
                    .map(|(path, values)| (path, values.into()))
                    .collect(),
            ),
            requests: Mutex::new(Vec::new()),
            changed: Notify::new(),
            stalled: AtomicBool::new(false),
        });
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url =
            reqwest::Url::parse(&format!("http://{}", listener.local_addr().unwrap())).unwrap();
        let app = Router::new()
            .fallback(mock_response)
            .with_state(state.clone());
        let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        Self { state, url, server }
    }

    fn requests(&self) -> Vec<RecordedRequest> {
        self.state.requests.lock().unwrap().clone()
    }

    async fn wait_for_requests(&self, count: usize) {
        loop {
            let changed = self.state.changed.notified();
            if self.state.requests.lock().unwrap().len() >= count {
                return;
            }
            changed.await;
        }
    }

    fn client(&self, values: &Values) -> Switchbot {
        Switchbot::with_base_url(Config::from_values(values).unwrap(), self.url.clone()).unwrap()
    }
}

impl Drop for MockApi {
    fn drop(&mut self) {
        self.server.abort();
    }
}

async fn mock_response(State(state): State<Arc<MockState>>, request: Request) -> Response {
    let path = request.uri().path().to_owned();
    state.requests.lock().unwrap().push(RecordedRequest {
        method: request.method().clone(),
        path: path.clone(),
        headers: request.headers().clone(),
    });
    state.changed.notify_one();
    if state.stalled.load(Ordering::Relaxed) {
        std::future::pending::<()>().await;
    }
    let response = state
        .responses
        .lock()
        .unwrap()
        .get_mut(&path)
        .and_then(|queue| {
            if queue.len() > 1 {
                queue.pop_front()
            } else {
                queue.front().cloned()
            }
        })
        .unwrap_or(MockResponse {
            status: StatusCode::NOT_FOUND,
            body: "unexpected mock request".to_owned(),
        });
    (
        response.status,
        [("content-type", "application/json")],
        response.body,
    )
        .into_response()
}

fn configuration() -> Values {
    Values::from([
        (
            "SWITCHBOT_API_TOKEN".to_owned(),
            "mock-token-not-a-real-credential".to_owned(),
        ),
        (
            "SWITCHBOT_API_SECRET".to_owned(),
            "mock-secret-not-a-real-credential".to_owned(),
        ),
        ("DELAY_SECOND".to_owned(), "0".to_owned()),
    ])
}

fn devices_response(devices: Value) -> MockResponse {
    MockResponse::json(json!({"statusCode": 100, "body": {"deviceList": devices}}))
}

fn meter_devices() -> MockResponse {
    devices_response(json!([{"deviceId": "meter", "deviceType": "Meter", "deviceName": "居間"}]))
}

fn status_response(body: Value) -> MockResponse {
    MockResponse::json(json!({"statusCode": 100, "body": body}))
}

fn request(method: Method, path: &str) -> Request {
    Request::builder()
        .method(method)
        .uri(path)
        .body(Body::empty())
        .unwrap()
}

async fn response_text(response: Response) -> String {
    String::from_utf8(
        to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap()
}

// 仮想時刻の自動進行を止め、ネットワーク I/O と明示的な advance を分離する。
// 実時間の計測や性能比較は行わない。
struct ControlledClock(JoinHandle<()>);

impl ControlledClock {
    fn new() -> Self {
        Self(tokio::spawn(async {
            loop {
                tokio::task::yield_now().await;
            }
        }))
    }
}

impl Drop for ControlledClock {
    fn drop(&mut self) {
        self.0.abort();
    }
}

/// 固定の timestamp に対して既存 API の HMAC-SHA256 署名が生成されること。
/// Arrange: Python と同じ token・secret・ミリ秒 timestamp が用意されること。
/// Act: 署名が生成されること。
/// Assert: 空 nonce の既知ベクトルと一致すること。
#[test]
fn signature_matches_the_existing_empty_nonce_protocol() {
    // Arrange
    let (token, secret, timestamp) = ("token", "secret", "1700000000123");

    // Act
    let sign = signature(token, secret, timestamp);

    // Assert
    assert_eq!(sign, "b8vjmGGymLPqhpJCvYILnsZKooc81rsyjzi2kE+Nxw8=");
}

/// HTTP 境界へ GET と正しい認証ヘッダーが送信されること。
/// Arrange: 空の一覧を返すローカル API が用意されること。
/// Act: デバイス一覧が取得されること。
/// Assert: token・ミリ秒 timestamp・空 nonce・対応する署名が送信されること。
#[tokio::test]
async fn api_requests_use_exact_authentication_headers() {
    // Arrange
    let values = configuration();
    let api = MockApi::new(vec![(
        "/v1.1/devices".to_owned(),
        vec![devices_response(json!([]))],
    )])
    .await;
    let mut client = api.client(&values);

    // Act
    let devices = client.fetch_devices().await.unwrap();

    // Assert
    assert!(devices.is_empty());
    let requests = api.requests();
    assert_eq!(requests.len(), 1);
    let sent = &requests[0];
    assert_eq!(sent.method, Method::GET);
    assert_eq!(sent.path, "/v1.1/devices");
    assert_eq!(sent.headers["authorization"], values["SWITCHBOT_API_TOKEN"]);
    assert_eq!(sent.headers["nonce"], "");
    let timestamp = sent.headers["t"].to_str().unwrap();
    assert!(timestamp.len() >= 13);
    assert!(timestamp.parse::<u128>().is_ok());
    let mut hmac =
        Hmac::<Sha256>::new_from_slice(values["SWITCHBOT_API_SECRET"].as_bytes()).unwrap();
    hmac.update(format!("{}{timestamp}", values["SWITCHBOT_API_TOKEN"]).as_bytes());
    assert_eq!(
        sent.headers["sign"],
        STANDARD.encode(hmac.finalize().into_bytes())
    );
}

/// deviceType がないデバイスも一覧の順序を保って取得されること。
/// Arrange: 通常デバイス・種類欠落・未知の種類を含む一覧が用意されること。
/// Act: デバイス一覧が取得されること。
/// Assert: 名前・識別子と種類の既定値が保持されること。
#[tokio::test]
async fn device_list_preserves_order_unknown_types_and_missing_type_default() {
    // Arrange
    let api = MockApi::new(vec![(
        "/v1.1/devices".to_owned(),
        vec![devices_response(json!([
            {"deviceId": "first", "deviceType": "Meter", "deviceName": "温度計"},
            {"deviceId": "missing", "deviceName": "種類なし"},
            {"deviceId": "unknown", "deviceType": "Future Device", "deviceName": "未知"}
        ]))],
    )])
    .await;
    let mut client = api.client(&configuration());

    // Act
    let devices = client.fetch_devices().await.unwrap();

    // Assert
    assert_eq!(
        devices,
        [
            Device {
                device_id: "first".to_owned(),
                device_type: "Meter".to_owned(),
                device_name: "温度計".to_owned()
            },
            Device {
                device_id: "missing".to_owned(),
                device_type: "-".to_owned(),
                device_name: "種類なし".to_owned()
            },
            Device {
                device_id: "unknown".to_owned(),
                device_type: "Future Device".to_owned(),
                device_name: "未知".to_owned()
            },
        ]
    );
}

/// 十二種類の対応デバイスだけが取得され、未知・欠落・赤外線一覧が除外されること。
/// Arrange: 全対応種類と対象外のデバイスが用意されること。
/// Act: 一覧からメトリクスが取得されること。
/// Assert: 対応種類の順序でのみ status API が呼び出されること。
#[tokio::test]
async fn metrics_fetches_every_supported_device_and_skips_unsupported_lists() {
    // Arrange
    let supported = [
        "Bot",
        "Ceiling Light",
        "Color Bulb",
        "Contact Sensor",
        "Curtain",
        "Hub Mini",
        "Indoor Cam",
        "Meter",
        "MeterPro(CO2)",
        "Motion Sensor",
        "Plug Mini (JP)",
        "Remote",
    ];
    let mut devices: Vec<_> = supported.iter().enumerate().map(|(index, kind)| {
        json!({"deviceId": format!("supported-{index}"), "deviceType": kind, "deviceName": kind})
    }).collect();
    devices.extend([
        json!({"deviceId": "unknown", "deviceType": "TV", "deviceName": "未知"}),
        json!({"deviceId": "missing", "deviceName": "種類なし"}),
    ]);
    let list = MockResponse::json(json!({"body": {
        "deviceList": devices,
        "infraredRemoteList": [{"deviceId": "infrared", "deviceType": "Remote", "deviceName": "赤外線"}]
    }}));
    let mut responses = vec![("/v1.1/devices".to_owned(), vec![list])];
    for index in 0..supported.len() {
        responses.push((
            format!("/v1.1/devices/supported-{index}/status"),
            vec![status_response(json!({"battery": 80}))],
        ));
    }
    let api = MockApi::new(responses).await;
    let mut client = api.client(&configuration());

    // Act
    let metrics = client.fetch_metrics().await.unwrap().render();

    // Assert
    let paths: Vec<_> = api
        .requests()
        .into_iter()
        .map(|request| request.path)
        .collect();
    let expected: Vec<_> = std::iter::once("/v1.1/devices".to_owned())
        .chain((0..supported.len()).map(|index| format!("/v1.1/devices/supported-{index}/status")))
        .collect();
    assert_eq!(paths, expected);
    assert_eq!(
        metrics
            .lines()
            .filter(|line| !line.starts_with('#'))
            .count(),
        12
    );
    assert!(!metrics.contains("device_id=\"unknown\""));
    assert!(!metrics.contains("device_id=\"missing\""));
    assert!(!metrics.contains("device_id=\"infrared\""));
}

/// device ID が一つの path segment として送信され、status 全体が保持されること。
/// Arrange: path の予約文字を含む ID と追加フィールドを持つ応答が用意されること。
/// Act: 指定 ID の status が取得されること。
/// Assert: 別の URL へ変形せず完全な API 応答が返されること。
#[tokio::test]
async fn device_status_encodes_the_id_and_preserves_the_full_response() {
    // Arrange
    let expected =
        json!({"statusCode": 100, "body": {"battery": 88, "power": "on"}, "message": "success"});
    let path = "/v1.1/devices/device%2Fid%3Fx=1%20%23%22/status";
    let api = MockApi::new(vec![(
        path.to_owned(),
        vec![MockResponse::json(expected.clone())],
    )])
    .await;
    let mut client = api.client(&configuration());

    // Act
    let status = client
        .fetch_device_status("device/id?x=1 #\"")
        .await
        .unwrap();

    // Assert
    assert_eq!(status, expected);
    assert_eq!(api.requests()[0].path, path);
}

/// cache TTL の境界までは再取得されず、期限後に更新されること。
/// Arrange: 値が変化する API と 10 秒 TTL の cache が用意されること。
/// Act: 仮想時刻を TTL の境界と直後へ進めて取得されること。
/// Assert: 境界では旧値が返り、期限後だけ二回目の API が呼ばれること。
#[tokio::test(start_paused = true)]
async fn cache_reuses_values_through_ttl_and_refreshes_after_expiry() {
    // Arrange
    let _clock = ControlledClock::new();
    let mut values = configuration();
    values.insert("CACHE_EXPIRE_SECOND".to_owned(), "10".to_owned());
    let api = MockApi::new(vec![(
        "/v1.1/devices/meter/status".to_owned(),
        vec![
            status_response(json!({"battery": 80})),
            status_response(json!({"battery": 70})),
        ],
    )])
    .await;
    let mut client = api.client(&values);
    let initial = client.fetch_device_status("meter").await.unwrap();

    // Act
    tokio::time::advance(Duration::from_secs(10)).await;
    let boundary = client.fetch_device_status("meter").await.unwrap();
    let boundary_requests = api.requests().len();
    tokio::time::advance(Duration::from_millis(1)).await;
    let refreshed = client.fetch_device_status("meter").await.unwrap();

    // Assert
    assert_eq!(initial["body"]["battery"], 80);
    assert_eq!(boundary, initial);
    assert_eq!(boundary_requests, 1);
    assert_eq!(refreshed["body"]["battery"], 70);
    assert_eq!(api.requests().len(), 2);
}

/// キャッシュ無効時には毎回 API が取得されること。
/// Arrange: 空の CACHE_DIR と正でない TTL の設定が用意されること。
/// Act: 同じ status が続けて二回取得されること。
/// Assert: 両方の既存無効化設定で二回の HTTP 呼び出しが行われること。
#[tokio::test]
async fn disabled_cache_never_reuses_a_response() {
    // Arrange
    for (key, value) in [
        ("CACHE_DIR", ""),
        ("CACHE_EXPIRE_SECOND", "0"),
        ("CACHE_EXPIRE_SECOND", "-1"),
    ] {
        let mut values = configuration();
        values.insert(key.to_owned(), value.to_owned());
        let api = MockApi::new(vec![(
            "/v1.1/devices/meter/status".to_owned(),
            vec![status_response(json!({"battery": 80}))],
        )])
        .await;
        let mut client = api.client(&values);

        // Act
        client.fetch_device_status("meter").await.unwrap();
        client.fetch_device_status("meter").await.unwrap();

        // Assert
        assert_eq!(api.requests().len(), 2, "{key}={value}");
    }
}

/// 実リクエストの成功後だけ指定 delay が適用されること。
/// Arrange: 5 秒 delay と cache を備えたローカル API が用意されること。
/// Act: 仮想時刻を進めて初回取得と cache hit が実行されること。
/// Assert: delay 前に初回が完了せず、cache hit は時刻進行なしで返されること。
#[tokio::test(start_paused = true)]
async fn request_delay_applies_after_success_but_not_to_cache_hits() {
    // Arrange
    let _clock = ControlledClock::new();
    let mut values = configuration();
    values.insert("DELAY_SECOND".to_owned(), "5".to_owned());
    let api = MockApi::new(vec![("/v1.1/devices".to_owned(), vec![meter_devices()])]).await;
    let mut client = api.client(&values);
    let pending = tokio::spawn(async move {
        let first = client.fetch_devices().await.unwrap();
        (client, first)
    });
    api.wait_for_requests(1).await;
    // 応答の I/O を処理する。仮想時刻は進めない。
    for _ in 0..100 {
        tokio::task::yield_now().await;
    }

    // Act
    let initially_pending = !pending.is_finished();
    tokio::time::advance(Duration::from_secs(4)).await;
    let before_delay = !pending.is_finished();
    tokio::time::advance(Duration::from_secs(1)).await;
    let (mut client, first) = pending.await.unwrap();
    let cached = client.fetch_devices().await.unwrap();

    // Assert
    assert!(initially_pending);
    assert!(before_delay);
    assert_eq!(cached, first);
    assert_eq!(api.requests().len(), 1);
}

/// 同時 scrape で client と cache が共有されること。
/// Arrange: 一台の Meter を返す API と共有 router が用意されること。
/// Act: 三つの metrics リクエストが同時に処理されること。
/// Assert: 全応答が同一で一覧と status は一回ずつしか取得されないこと。
#[tokio::test]
async fn concurrent_scrapes_share_the_client_and_cache() {
    // Arrange
    let api = MockApi::new(vec![
        ("/v1.1/devices".to_owned(), vec![meter_devices()]),
        (
            "/v1.1/devices/meter/status".to_owned(),
            vec![status_response(json!({"battery": 88}))],
        ),
    ])
    .await;
    let app = server::router(PathBuf::new(), Some(api.client(&configuration())));

    // Act
    let (first, second, third) = tokio::join!(
        app.clone().oneshot(request(Method::GET, "/metrics")),
        app.clone().oneshot(request(Method::GET, "/metrics")),
        app.oneshot(request(Method::GET, "/metrics")),
    );
    let responses = [first.unwrap(), second.unwrap(), third.unwrap()];
    let mut bodies = Vec::new();
    for response in responses {
        assert_eq!(response.status(), StatusCode::OK);
        bodies.push(response_text(response).await);
    }

    // Assert
    assert_eq!(bodies[0], bodies[1]);
    assert_eq!(bodies[1], bodies[2]);
    assert!(
        bodies[0].contains("switchbot_device_battery{device_id=\"meter\",device_name=\"居間\"} 88")
    );
    assert_eq!(api.requests().len(), 2);
}

/// API の HTTP・業務エラーと不正 JSON が cache に保存されないこと。
/// Arrange: 初回エラーと次回成功を返す status API が用意されること。
/// Act: 同一 status が二回取得されること。
/// Assert: 初回は失敗し、再取得された次回だけが成功すること。
#[tokio::test]
async fn unsuccessful_responses_are_not_cached() {
    // Arrange
    let errors = [
        MockResponse {
            status: StatusCode::SERVICE_UNAVAILABLE,
            body: "upstream failure".to_owned(),
        },
        MockResponse::json(json!({"statusCode": 190, "body": {"battery": 1}})),
        MockResponse {
            status: StatusCode::OK,
            body: "{broken-json".to_owned(),
        },
    ];
    for error in errors {
        let api = MockApi::new(vec![(
            "/v1.1/devices/meter/status".to_owned(),
            vec![error, status_response(json!({"battery": 88}))],
        )])
        .await;
        let mut client = api.client(&configuration());

        // Act
        let first = client.fetch_device_status("meter").await;
        let second = client.fetch_device_status("meter").await.unwrap();

        // Assert
        assert!(first.is_err());
        assert_eq!(second["body"]["battery"], 88);
        assert_eq!(api.requests().len(), 2);
    }
}

/// 一部デバイス取得後のエラーで部分メトリクスや秘密値が公開されないこと。
/// Arrange: 一台目が成功し二台目が各種異常を返す API が用意されること。
/// Act: metrics endpoint が呼び出されること。
/// Assert: HTTP 500 の固定本文だけが返されること。
#[tokio::test]
async fn scrape_errors_return_500_without_partial_metrics_or_credentials() {
    // Arrange
    let values = configuration();
    let secret_body = format!(
        "{} {}",
        values["SWITCHBOT_API_TOKEN"], values["SWITCHBOT_API_SECRET"]
    );
    let errors = [
        MockResponse {
            status: StatusCode::SERVICE_UNAVAILABLE,
            body: secret_body.clone(),
        },
        MockResponse::json(json!({"statusCode": 190, "message": secret_body})),
        MockResponse {
            status: StatusCode::OK,
            body: "{broken-json".to_owned(),
        },
        MockResponse::json(json!({"statusCode": 100})),
        status_response(Value::Null),
        status_response(json!({"battery": "not-numeric"})),
    ];
    for error in errors {
        let api = MockApi::new(vec![
            (
                "/v1.1/devices".to_owned(),
                vec![devices_response(json!([
                    {"deviceId": "good", "deviceType": "Meter", "deviceName": "成功"},
                    {"deviceId": "broken", "deviceType": "Meter", "deviceName": "異常"}
                ]))],
            ),
            (
                "/v1.1/devices/good/status".to_owned(),
                vec![status_response(json!({"battery": 88}))],
            ),
            ("/v1.1/devices/broken/status".to_owned(), vec![error]),
        ])
        .await;
        let app = server::router(PathBuf::new(), Some(api.client(&values)));

        // Act
        let response = app.oneshot(request(Method::GET, "/metrics")).await.unwrap();
        let status = response.status();
        let body = response_text(response).await;

        // Assert
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body, "Internal Server Error");
        assert!(!body.contains("switchbot_device_battery"));
        assert!(!body.contains(&values["SWITCHBOT_API_TOKEN"]));
        assert!(!body.contains(&values["SWITCHBOT_API_SECRET"]));
        assert_eq!(api.requests().len(), 3);
    }
}

/// 不正な一覧形式が空の成功応答として扱われないこと。
/// Arrange: 必須 body・deviceList・デバイス必須項目が欠落した応答が用意されること。
/// Act: metrics endpoint が呼び出されること。
/// Assert: status API を呼ばず HTTP 500 が返されること。
#[tokio::test]
async fn malformed_device_lists_fail_the_scrape() {
    // Arrange
    for value in [
        json!({}),
        json!({"body": {}}),
        json!({"body": {"deviceList": null}}),
        json!({"body": {"deviceList": [{"deviceId": "no-name"}]}}),
    ] {
        let api = MockApi::new(vec![(
            "/v1.1/devices".to_owned(),
            vec![MockResponse::json(value)],
        )])
        .await;
        let app = server::router(PathBuf::new(), Some(api.client(&configuration())));

        // Act
        let response = app.oneshot(request(Method::GET, "/metrics")).await.unwrap();

        // Assert
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(response_text(response).await, "Internal Server Error");
        assert_eq!(api.requests().len(), 1);
    }
}

/// GET・HEAD のメトリクスと OPTIONS・未知 path・拒否 method が維持されること。
/// Arrange: 一台の Meter を返す router が用意されること。
/// Act: 各 HTTP method と path が呼び出されること。
/// Assert: 公開ルートの status・content type・Allow・HEAD 本文が保たれること。
#[tokio::test]
async fn http_routes_preserve_methods_headers_and_head_semantics() {
    // Arrange
    let api = MockApi::new(vec![
        ("/v1.1/devices".to_owned(), vec![meter_devices()]),
        (
            "/v1.1/devices/meter/status".to_owned(),
            vec![status_response(json!({"battery": 88}))],
        ),
    ])
    .await;
    let app = server::router(PathBuf::new(), Some(api.client(&configuration())));

    // Act
    let options = app
        .clone()
        .oneshot(request(Method::OPTIONS, "/metrics"))
        .await
        .unwrap();
    let missing = app
        .clone()
        .oneshot(request(Method::GET, "/missing"))
        .await
        .unwrap();
    let method = app
        .clone()
        .oneshot(request(Method::POST, "/metrics"))
        .await
        .unwrap();
    let before_scrape = api.requests().len();
    let get = app
        .clone()
        .oneshot(request(Method::GET, "/metrics"))
        .await
        .unwrap();
    let head = app
        .oneshot(request(Method::HEAD, "/metrics"))
        .await
        .unwrap();

    // Assert
    assert_eq!(options.status(), StatusCode::OK);
    assert_eq!(options.headers()["allow"], "GET, HEAD, OPTIONS");
    assert_eq!(response_text(options).await, "");
    assert_eq!(missing.status(), StatusCode::NOT_FOUND);
    assert_eq!(method.status(), StatusCode::METHOD_NOT_ALLOWED);
    let allow = method.headers()["allow"].to_str().unwrap();
    assert!(allow.contains("GET") && allow.contains("HEAD") && allow.contains("OPTIONS"));
    assert_eq!(before_scrape, 0);
    assert_eq!(get.status(), StatusCode::OK);
    assert_eq!(get.headers()["content-type"], "text/plain; charset=utf-8");
    assert!(
        response_text(get)
            .await
            .contains("switchbot_device_battery")
    );
    assert_eq!(head.status(), StatusCode::OK);
    assert_eq!(head.headers()["content-type"], "text/plain; charset=utf-8");
    assert_eq!(response_text(head).await, "");
    assert_eq!(api.requests().len(), 2);
}

/// 停止した API 応答が設定 timeout で中断されること。
/// Arrange: 応答しないローカル API と 2 秒 timeout が用意されること。
/// Act: metrics 取得中に仮想時刻が timeout まで進められること。
/// Assert: HTTP 500 の固定本文が返り秘密値が公開されないこと。
#[tokio::test(start_paused = true)]
async fn stalled_api_response_is_aborted_at_the_configured_timeout() {
    // Arrange
    let _clock = ControlledClock::new();
    let mut values = configuration();
    values.insert("API_TIMEOUT_SECOND".to_owned(), "2".to_owned());
    let api = MockApi::new(vec![]).await;
    api.state.stalled.store(true, Ordering::Relaxed);
    let app = server::router(PathBuf::new(), Some(api.client(&values)));
    let pending = tokio::spawn(app.oneshot(request(Method::GET, "/metrics")));
    api.wait_for_requests(1).await;

    // Act
    let before_timeout = !pending.is_finished();
    tokio::time::advance(Duration::from_secs(2)).await;
    let response = pending.await.unwrap().unwrap();
    let status = response.status();
    let body = response_text(response).await;

    // Assert
    assert!(before_timeout);
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(body, "Internal Server Error");
    assert!(!body.contains(&values["SWITCHBOT_API_TOKEN"]));
    assert!(!body.contains(&values["SWITCHBOT_API_SECRET"]));
    assert_eq!(api.requests().len(), 1);
}

/// cache 期限後の upstream エラーで古いメトリクスが公開されないこと。
/// Arrange: 初回正常で次回失敗する status と 10 秒 TTL が用意されること。
/// Act: 初回取得後に仮想時刻が期限を超え、再 scrape されること。
/// Assert: 再取得が試行され HTTP 500 が返り古い値が含まれないこと。
#[tokio::test(start_paused = true)]
async fn expired_cache_does_not_mask_an_upstream_failure() {
    // Arrange
    let _clock = ControlledClock::new();
    let mut values = configuration();
    values.insert("CACHE_EXPIRE_SECOND".to_owned(), "10".to_owned());
    let api = MockApi::new(vec![
        ("/v1.1/devices".to_owned(), vec![meter_devices()]),
        (
            "/v1.1/devices/meter/status".to_owned(),
            vec![
                status_response(json!({"battery": 88})),
                MockResponse {
                    status: StatusCode::SERVICE_UNAVAILABLE,
                    body: "unavailable".to_owned(),
                },
            ],
        ),
    ])
    .await;
    let mut client = api.client(&values);
    let initial = client.fetch_metrics().await.unwrap().render();
    let app = server::router(PathBuf::new(), Some(client));

    // Act
    tokio::time::advance(Duration::from_secs(11)).await;
    let response = app.oneshot(request(Method::GET, "/metrics")).await.unwrap();
    let status = response.status();
    let body = response_text(response).await;

    // Assert
    assert!(initial.contains("device_name=\"居間\"} 88"));
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(body, "Internal Server Error");
    assert!(!body.contains("switchbot_device_battery"));
    assert_eq!(api.requests().len(), 4);
}
