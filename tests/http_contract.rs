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
use hmac::{Hmac, KeyInit, Mac};
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
    stalled_path: Mutex<Option<String>>,
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
            stalled_path: Mutex::new(None),
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
    let stalled_path = state.stalled_path.lock().unwrap().as_deref() == Some(path.as_str());
    if state.stalled.load(Ordering::Relaxed) || stalled_path {
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

fn device_status_errors(values: &Values) -> Vec<(&'static str, MockResponse)> {
    let private_details = format!(
        "raw-upstream-detail: {} {}",
        values["SWITCHBOT_API_TOKEN"], values["SWITCHBOT_API_SECRET"]
    );
    vec![
        (
            "http-503",
            MockResponse {
                status: StatusCode::SERVICE_UNAVAILABLE,
                body: private_details.clone(),
            },
        ),
        (
            "api-190",
            MockResponse::json(json!({
                "statusCode": 190,
                "body": {"battery": 1},
                "message": private_details,
            })),
        ),
        (
            "invalid-json",
            MockResponse {
                status: StatusCode::OK,
                body: format!("{{broken-json {private_details}"),
            },
        ),
        (
            "missing-body",
            MockResponse::json(json!({"statusCode": 100, "message": private_details})),
        ),
        ("null-body", status_response(Value::Null)),
        (
            "invalid-metric",
            status_response(json!({"battery": 1, "humidity": private_details})),
        ),
    ]
}

fn assert_no_upstream_details(body: &str, values: &Values) {
    assert!(!body.contains(&values["SWITCHBOT_API_TOKEN"]));
    assert!(!body.contains(&values["SWITCHBOT_API_SECRET"]));
    assert!(!body.contains("raw-upstream-detail"));
    assert!(!body.contains("broken-json"));
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
/// Arrange: 固定の token・secret・ミリ秒 timestamp が用意されること。
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

/// 端末の失敗位置と種類に関係なく正常端末だけが公開されること。
/// Arrange: 三台の先頭・中間・末尾のいずれかが各種異常を返す API が用意されること。
/// Act: metrics endpoint が呼び出されること。
/// Assert: 残りの端末まで取得され、異常端末の全サンプルと秘密値を除いた HTTP 200 が返されること。
#[tokio::test]
async fn device_failures_publish_only_healthy_metrics_in_every_list_position() {
    // Arrange
    let values = configuration();
    let ids = ["first", "middle", "last"];
    for (kind, error) in device_status_errors(&values) {
        for (failed_index, failed_id) in ids.iter().enumerate() {
            let devices: Vec<_> = ids
                .iter()
                .map(|id| json!({"deviceId": id, "deviceType": "Meter", "deviceName": id}))
                .collect();
            let mut responses = vec![(
                "/v1.1/devices".to_owned(),
                vec![devices_response(json!(devices))],
            )];
            for (index, id) in ids.iter().enumerate() {
                responses.push((
                    format!("/v1.1/devices/{id}/status"),
                    vec![if index == failed_index {
                        error.clone()
                    } else {
                        status_response(json!({"battery": 80 + index}))
                    }],
                ));
            }
            let api = MockApi::new(responses).await;
            let app = server::router(PathBuf::new(), Some(api.client(&values)));

            // Act
            let response = app.oneshot(request(Method::GET, "/metrics")).await.unwrap();
            let status = response.status();
            let content_type = response.headers()["content-type"].clone();
            let body = response_text(response).await;

            // Assert
            let case = format!("{kind}, failed position {failed_index}");
            assert_eq!(status, StatusCode::OK, "{case}");
            assert_eq!(content_type, "text/plain; charset=utf-8", "{case}");
            let samples: Vec<_> = body.lines().filter(|line| !line.starts_with('#')).collect();
            let expected: Vec<_> = ids
                .iter()
                .enumerate()
                .filter(|(index, _)| *index != failed_index)
                .map(|(index, id)| {
                    format!(
                        "switchbot_device_battery{{device_id=\"{id}\",device_name=\"{id}\"}} {}",
                        80 + index
                    )
                })
                .collect();
            assert_eq!(samples, expected, "{case}");
            assert!(!body.contains(&format!("device_id=\"{failed_id}\"")));
            assert_no_upstream_details(&body, &values);
            let paths: Vec<_> = api
                .requests()
                .into_iter()
                .map(|request| request.path)
                .collect();
            let expected_paths: Vec<_> = std::iter::once("/v1.1/devices".to_owned())
                .chain(ids.iter().map(|id| format!("/v1.1/devices/{id}/status")))
                .collect();
            assert_eq!(paths, expected_paths, "{case}");
        }
    }
}

/// 対象端末が全て失敗した場合にはメトリクスや秘密値が公開されないこと。
/// Arrange: 二台の対象端末が各種異常を返す API が用意されること。
/// Act: metrics endpoint が呼び出されること。
/// Assert: 全端末の取得が試行され HTTP 500 の固定本文だけが返されること。
#[tokio::test]
async fn all_device_failures_return_500_without_metrics_or_credentials() {
    // Arrange
    let values = configuration();
    for (kind, error) in device_status_errors(&values) {
        let api = MockApi::new(vec![
            (
                "/v1.1/devices".to_owned(),
                vec![devices_response(json!([
                    {"deviceId": "first", "deviceType": "Meter", "deviceName": "異常1"},
                    {"deviceId": "second", "deviceType": "Meter", "deviceName": "異常2"}
                ]))],
            ),
            ("/v1.1/devices/first/status".to_owned(), vec![error.clone()]),
            ("/v1.1/devices/second/status".to_owned(), vec![error]),
        ])
        .await;
        let app = server::router(PathBuf::new(), Some(api.client(&values)));

        // Act
        let response = app.oneshot(request(Method::GET, "/metrics")).await.unwrap();
        let status = response.status();
        let body = response_text(response).await;

        // Assert
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR, "{kind}");
        assert_eq!(body, "Internal Server Error", "{kind}");
        assert!(!body.contains("switchbot_device_battery"));
        assert_no_upstream_details(&body, &values);
        assert_eq!(api.requests().len(), 3, "{kind}");
    }
}

/// 認証の HTTP エラーでは正常端末があっても scrape 全体が失敗すること。
/// Arrange: 三台の各位置で HTTP 401 または 403 を返す API が用意されること。
/// Act: metrics endpoint が呼び出されること。
/// Assert: 取得済みの正常サンプルや秘密値を含めず HTTP 500 の固定本文が返されること。
#[tokio::test]
async fn authentication_failures_reject_the_whole_scrape_after_any_partial_success() {
    // Arrange
    let values = configuration();
    let ids = ["first", "middle", "last"];
    for auth_status in [StatusCode::UNAUTHORIZED, StatusCode::FORBIDDEN] {
        for (failed_index, _) in ids.iter().enumerate() {
            let devices: Vec<_> = ids
                .iter()
                .map(|id| json!({"deviceId": id, "deviceType": "Meter", "deviceName": id}))
                .collect();
            let mut responses = vec![(
                "/v1.1/devices".to_owned(),
                vec![devices_response(json!(devices))],
            )];
            for (index, id) in ids.iter().enumerate() {
                responses.push((
                    format!("/v1.1/devices/{id}/status"),
                    vec![if index == failed_index {
                        MockResponse {
                            status: auth_status,
                            body: format!(
                                "raw-upstream-detail: {} {}",
                                values["SWITCHBOT_API_TOKEN"], values["SWITCHBOT_API_SECRET"]
                            ),
                        }
                    } else {
                        status_response(json!({"battery": 88}))
                    }],
                ));
            }
            let api = MockApi::new(responses).await;
            let app = server::router(PathBuf::new(), Some(api.client(&values)));

            // Act
            let response = app.oneshot(request(Method::GET, "/metrics")).await.unwrap();
            let status = response.status();
            let body = response_text(response).await;

            // Assert
            assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
            assert_eq!(body, "Internal Server Error");
            assert_no_upstream_details(&body, &values);
            let paths: Vec<_> = api
                .requests()
                .into_iter()
                .map(|request| request.path)
                .collect();
            let expected_prefix: Vec<_> = std::iter::once("/v1.1/devices".to_owned())
                .chain(
                    ids[..=failed_index]
                        .iter()
                        .map(|id| format!("/v1.1/devices/{id}/status")),
                )
                .collect();
            assert!(paths.starts_with(&expected_prefix));
        }
    }
}

/// 一覧の取得エラーは正常な空一覧として扱われないこと。
/// Arrange: HTTP・業務エラー・不正 JSON を返す一覧 API が用意されること。
/// Act: metrics endpoint が呼び出されること。
/// Assert: status API を呼ばず秘密値を含まない HTTP 500 の固定本文が返されること。
#[tokio::test]
async fn device_list_request_errors_fail_the_scrape_without_fetching_status() {
    // Arrange
    let values = configuration();
    let private_details = format!(
        "raw-upstream-detail: {} {}",
        values["SWITCHBOT_API_TOKEN"], values["SWITCHBOT_API_SECRET"]
    );
    let mut errors = vec![
        MockResponse::json(json!({"statusCode": 190, "message": private_details})),
        MockResponse {
            status: StatusCode::OK,
            body: format!("{{broken-json {private_details}"),
        },
    ];
    errors.extend(
        [
            StatusCode::SERVICE_UNAVAILABLE,
            StatusCode::UNAUTHORIZED,
            StatusCode::FORBIDDEN,
        ]
        .map(|status| MockResponse {
            status,
            body: private_details.clone(),
        }),
    );
    for error in errors {
        let api = MockApi::new(vec![("/v1.1/devices".to_owned(), vec![error])]).await;
        let app = server::router(PathBuf::new(), Some(api.client(&values)));

        // Act
        let response = app.oneshot(request(Method::GET, "/metrics")).await.unwrap();
        let status = response.status();
        let body = response_text(response).await;

        // Assert
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body, "Internal Server Error");
        assert_no_upstream_details(&body, &values);
        assert_eq!(api.requests().len(), 1);
    }
}

/// 対象端末がない場合にはメトリクス定義だけが正常に公開されること。
/// Arrange: 空一覧または対象外・種類欠落・赤外線だけの一覧が用意されること。
/// Act: metrics endpoint が呼び出されること。
/// Assert: status API を呼ばず七種類の HELP と TYPE だけを持つ HTTP 200 が返されること。
#[tokio::test]
async fn no_supported_devices_return_200_with_only_metric_definitions() {
    // Arrange
    for devices in [
        json!([]),
        json!([
            {"deviceId": "unknown", "deviceType": "Future Device", "deviceName": "対象外"},
            {"deviceId": "missing", "deviceName": "種類なし"},
        ]),
    ] {
        let list = MockResponse::json(json!({"statusCode": 100, "body": {
            "deviceList": devices,
            "infraredRemoteList": [{"deviceId": "infrared", "deviceType": "Remote", "deviceName": "赤外線"}],
        }}));
        let api = MockApi::new(vec![("/v1.1/devices".to_owned(), vec![list])]).await;
        let app = server::router(PathBuf::new(), Some(api.client(&configuration())));

        // Act
        let response = app.oneshot(request(Method::GET, "/metrics")).await.unwrap();
        let status = response.status();
        let body = response_text(response).await;

        // Assert
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body.lines().count(), 14);
        assert_eq!(
            body.lines()
                .filter(|line| line.starts_with("# HELP "))
                .count(),
            7
        );
        assert_eq!(
            body.lines()
                .filter(|line| line.starts_with("# TYPE "))
                .count(),
            7
        );
        assert!(!body.contains("device_id="));
        assert_eq!(api.requests().len(), 1);
    }
}

/// 取得に成功した空の status は全端末失敗として扱われないこと。
/// Arrange: 一台が失敗し別の一台が有効な空オブジェクトを返す API が用意されること。
/// Act: metrics endpoint が呼び出されること。
/// Assert: サンプルがなくてもメトリクス定義だけを持つ HTTP 200 が返されること。
#[tokio::test]
async fn a_valid_status_without_metric_fields_counts_as_a_successful_device() {
    // Arrange
    let api = MockApi::new(vec![
        (
            "/v1.1/devices".to_owned(),
            vec![devices_response(json!([
                {"deviceId": "broken", "deviceType": "Meter", "deviceName": "異常"},
                {"deviceId": "empty", "deviceType": "Meter", "deviceName": "欠測"},
            ]))],
        ),
        (
            "/v1.1/devices/broken/status".to_owned(),
            vec![MockResponse {
                status: StatusCode::SERVICE_UNAVAILABLE,
                body: "upstream failure".to_owned(),
            }],
        ),
        (
            "/v1.1/devices/empty/status".to_owned(),
            vec![status_response(json!({}))],
        ),
    ])
    .await;
    let app = server::router(PathBuf::new(), Some(api.client(&configuration())));

    // Act
    let response = app.oneshot(request(Method::GET, "/metrics")).await.unwrap();
    let status = response.status();
    let body = response_text(response).await;

    // Assert
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.lines().count(), 14);
    assert_eq!(
        body.lines()
            .filter(|line| line.starts_with("# HELP "))
            .count(),
        7
    );
    assert_eq!(
        body.lines()
            .filter(|line| line.starts_with("# TYPE "))
            .count(),
        7
    );
    assert!(!body.contains("device_id="));
    assert_eq!(api.requests().len(), 3);
}

/// 端末の失敗だけが再試行され TTL 内の正常 cache が保持されること。
/// Arrange: 正常端末と各種失敗から次回回復する端末、および 10 秒 TTL が用意されること。
/// Act: 部分成功の一秒後に同じ endpoint が再 scrape されること。
/// Assert: 一覧と正常値を再取得せず、失敗端末だけが回復した HTTP 200 が返されること。
#[tokio::test(start_paused = true)]
async fn failed_device_statuses_are_retried_without_discarding_healthy_cache() {
    // Arrange
    let _clock = ControlledClock::new();
    let mut values = configuration();
    values.insert("CACHE_EXPIRE_SECOND".to_owned(), "10".to_owned());
    for (kind, error) in device_status_errors(&values) {
        let api = MockApi::new(vec![
            (
                "/v1.1/devices".to_owned(),
                vec![devices_response(json!([
                    {"deviceId": "good", "deviceType": "Meter", "deviceName": "正常"},
                    {"deviceId": "broken", "deviceType": "Meter", "deviceName": "回復"},
                ]))],
            ),
            (
                "/v1.1/devices/good/status".to_owned(),
                vec![
                    status_response(json!({"battery": 88})),
                    status_response(json!({"battery": 11})),
                ],
            ),
            (
                "/v1.1/devices/broken/status".to_owned(),
                vec![error, status_response(json!({"battery": 77}))],
            ),
        ])
        .await;
        let app = server::router(PathBuf::new(), Some(api.client(&values)));
        let initial = app
            .clone()
            .oneshot(request(Method::GET, "/metrics"))
            .await
            .unwrap();
        let initial_status = initial.status();
        let initial_body = response_text(initial).await;

        // Act
        tokio::time::advance(Duration::from_secs(1)).await;
        let recovered = app.oneshot(request(Method::GET, "/metrics")).await.unwrap();
        let recovered_status = recovered.status();
        let recovered_body = response_text(recovered).await;

        // Assert
        assert_eq!(initial_status, StatusCode::OK, "{kind}");
        assert!(initial_body.contains("device_id=\"good\",device_name=\"正常\"} 88"));
        assert!(!initial_body.contains("device_id=\"broken\""));
        assert_eq!(recovered_status, StatusCode::OK, "{kind}");
        assert!(recovered_body.contains("device_id=\"good\",device_name=\"正常\"} 88"));
        assert!(recovered_body.contains("device_id=\"broken\",device_name=\"回復\"} 77"));
        assert_no_upstream_details(&initial_body, &values);
        assert_no_upstream_details(&recovered_body, &values);
        let paths: Vec<_> = api
            .requests()
            .into_iter()
            .map(|request| request.path)
            .collect();
        assert_eq!(
            paths,
            [
                "/v1.1/devices",
                "/v1.1/devices/good/status",
                "/v1.1/devices/broken/status",
                "/v1.1/devices/broken/status",
            ],
            "{kind}"
        );
    }
}

/// 不正な一覧形式が成功や cache として扱われず次回に再取得されること。
/// Arrange: 必須項目が欠落した一覧と次回の正常な空一覧が用意されること。
/// Act: metrics endpoint が二回呼び出されること。
/// Assert: 初回は HTTP 500、次回は一覧が再取得されメトリクス定義が返されること。
#[tokio::test]
async fn malformed_device_lists_fail_the_scrape_and_are_retried() {
    // Arrange
    for value in [
        json!({}),
        json!({"body": {}}),
        json!({"body": {"deviceList": null}}),
        json!({"body": {"deviceList": [{"deviceId": "no-name"}]}}),
    ] {
        let api = MockApi::new(vec![(
            "/v1.1/devices".to_owned(),
            vec![MockResponse::json(value), devices_response(json!([]))],
        )])
        .await;
        let app = server::router(PathBuf::new(), Some(api.client(&configuration())));

        // Act
        let response = app
            .clone()
            .oneshot(request(Method::GET, "/metrics"))
            .await
            .unwrap();
        let recovered = app.oneshot(request(Method::GET, "/metrics")).await.unwrap();

        // Assert
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(response_text(response).await, "Internal Server Error");
        assert_eq!(recovered.status(), StatusCode::OK);
        assert_eq!(response_text(recovered).await.lines().count(), 14);
        assert_eq!(api.requests().len(), 2);
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

/// 先頭端末の timeout 後にも次の正常端末のメトリクスが公開されること。
/// Arrange: 応答しない先頭端末と正常な次端末、および 2 秒 timeout が用意されること。
/// Act: status の取得中に仮想時刻が timeout まで進められること。
/// Assert: 次端末まで取得され、停止した端末と秘密値を除いた HTTP 200 が返されること。
#[tokio::test(start_paused = true)]
async fn a_timed_out_device_does_not_prevent_later_device_metrics() {
    // Arrange
    let _clock = ControlledClock::new();
    let mut values = configuration();
    values.insert("API_TIMEOUT_SECOND".to_owned(), "2".to_owned());
    let api = MockApi::new(vec![
        (
            "/v1.1/devices".to_owned(),
            vec![devices_response(json!([
                {"deviceId": "stalled", "deviceType": "Meter", "deviceName": "停止"},
                {"deviceId": "good", "deviceType": "Meter", "deviceName": "正常"},
            ]))],
        ),
        (
            "/v1.1/devices/good/status".to_owned(),
            vec![status_response(json!({"battery": 88}))],
        ),
    ])
    .await;
    *api.state.stalled_path.lock().unwrap() = Some("/v1.1/devices/stalled/status".to_owned());
    let app = server::router(PathBuf::new(), Some(api.client(&values)));
    let pending = tokio::spawn(app.oneshot(request(Method::GET, "/metrics")));
    api.wait_for_requests(2).await;

    // Act
    let before_timeout = !pending.is_finished();
    tokio::time::advance(Duration::from_secs(2)).await;
    let response = pending.await.unwrap().unwrap();
    let status = response.status();
    let body = response_text(response).await;

    // Assert
    assert!(before_timeout);
    assert_eq!(status, StatusCode::OK);
    let samples: Vec<_> = body.lines().filter(|line| !line.starts_with('#')).collect();
    assert_eq!(
        samples,
        ["switchbot_device_battery{device_id=\"good\",device_name=\"正常\"} 88",]
    );
    assert!(!body.contains("device_id=\"stalled\""));
    assert_no_upstream_details(&body, &values);
    let paths: Vec<_> = api
        .requests()
        .into_iter()
        .map(|request| request.path)
        .collect();
    assert_eq!(
        paths,
        [
            "/v1.1/devices",
            "/v1.1/devices/stalled/status",
            "/v1.1/devices/good/status",
        ]
    );
}

/// cache 期限後の全端末失敗で古いメトリクスが公開されず再試行できること。
/// Arrange: 初回正常・期限後失敗・次回回復の status と 10 秒 TTL が用意されること。
/// Act: 仮想時刻が期限を超えてから失敗時と回復時に再 scrape されること。
/// Assert: 全端末失敗では固定本文の HTTP 500 が返り、再試行時に新しい値だけが返されること。
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
                status_response(json!({"battery": 70})),
            ],
        ),
    ])
    .await;
    let mut client = api.client(&values);
    let initial = client.fetch_metrics().await.unwrap().render();
    let app = server::router(PathBuf::new(), Some(client));

    // Act
    tokio::time::advance(Duration::from_secs(11)).await;
    let response = app
        .clone()
        .oneshot(request(Method::GET, "/metrics"))
        .await
        .unwrap();
    let status = response.status();
    let body = response_text(response).await;
    let requests_after_failure = api.requests().len();
    let recovered = app.oneshot(request(Method::GET, "/metrics")).await.unwrap();
    let recovered_status = recovered.status();
    let recovered_body = response_text(recovered).await;

    // Assert
    assert!(initial.contains("device_name=\"居間\"} 88"));
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(body, "Internal Server Error");
    assert!(!body.contains("switchbot_device_battery"));
    assert_eq!(requests_after_failure, 4);
    assert_eq!(recovered_status, StatusCode::OK);
    assert!(recovered_body.contains("device_name=\"居間\"} 70"));
    assert!(!recovered_body.contains("device_name=\"居間\"} 88"));
    assert_eq!(api.requests().len(), 5);
}

/// cache 期限後の失敗端末には古い値で補完せず、正常値と回復した値だけが公開されること。
/// Arrange: 二台の初回正常値と、一台の期限後の各種失敗・回復が用意されること。
/// Act: 10 秒 TTL を超えて部分成功が取得され、直後に同じ endpoint が再 scrape されること。
/// Assert: 失敗端末の古い全サンプルが省かれ、正常 cache を保った再試行で新しい値が返されること。
#[tokio::test(start_paused = true)]
async fn expired_device_failures_omit_stale_samples_and_retry_without_refetching_healthy_cache() {
    // Arrange
    let _clock = ControlledClock::new();
    let mut values = configuration();
    values.insert("CACHE_EXPIRE_SECOND".to_owned(), "10".to_owned());
    for (kind, error) in device_status_errors(&values) {
        let api = MockApi::new(vec![
            (
                "/v1.1/devices".to_owned(),
                vec![devices_response(json!([
                    {"deviceId": "good", "deviceType": "Meter", "deviceName": "正常"},
                    {"deviceId": "broken", "deviceType": "Meter", "deviceName": "回復"},
                ]))],
            ),
            (
                "/v1.1/devices/good/status".to_owned(),
                vec![
                    status_response(json!({"battery": 88})),
                    status_response(json!({"battery": 90})),
                    status_response(json!({"battery": 11})),
                ],
            ),
            (
                "/v1.1/devices/broken/status".to_owned(),
                vec![
                    status_response(json!({"battery": 66, "humidity": 50})),
                    error,
                    status_response(json!({"battery": 77})),
                ],
            ),
        ])
        .await;
        let app = server::router(PathBuf::new(), Some(api.client(&values)));
        let initial = app
            .clone()
            .oneshot(request(Method::GET, "/metrics"))
            .await
            .unwrap();
        let initial_status = initial.status();
        let initial_body = response_text(initial).await;

        // Act
        tokio::time::advance(Duration::from_secs(11)).await;
        let partial = app
            .clone()
            .oneshot(request(Method::GET, "/metrics"))
            .await
            .unwrap();
        let partial_status = partial.status();
        let partial_body = response_text(partial).await;
        let recovered = app.oneshot(request(Method::GET, "/metrics")).await.unwrap();
        let recovered_status = recovered.status();
        let recovered_body = response_text(recovered).await;

        // Assert
        assert_eq!(initial_status, StatusCode::OK, "{kind}");
        assert!(initial_body.contains("device_id=\"good\",device_name=\"正常\"} 88"));
        assert!(initial_body.contains("device_id=\"broken\",device_name=\"回復\"} 66"));
        assert_eq!(partial_status, StatusCode::OK, "{kind}");
        let partial_samples: Vec<_> = partial_body
            .lines()
            .filter(|line| !line.starts_with('#'))
            .collect();
        assert_eq!(
            partial_samples,
            ["switchbot_device_battery{device_id=\"good\",device_name=\"正常\"} 90",],
            "{kind}"
        );
        assert_eq!(recovered_status, StatusCode::OK, "{kind}");
        let recovered_samples: Vec<_> = recovered_body
            .lines()
            .filter(|line| !line.starts_with('#'))
            .collect();
        assert_eq!(
            recovered_samples,
            [
                "switchbot_device_battery{device_id=\"good\",device_name=\"正常\"} 90",
                "switchbot_device_battery{device_id=\"broken\",device_name=\"回復\"} 77",
            ],
            "{kind}"
        );
        assert_no_upstream_details(&partial_body, &values);
        assert_no_upstream_details(&recovered_body, &values);
        let paths: Vec<_> = api
            .requests()
            .into_iter()
            .map(|request| request.path)
            .collect();
        assert_eq!(
            paths,
            [
                "/v1.1/devices",
                "/v1.1/devices/good/status",
                "/v1.1/devices/broken/status",
                "/v1.1/devices",
                "/v1.1/devices/good/status",
                "/v1.1/devices/broken/status",
                "/v1.1/devices/broken/status",
            ],
            "{kind}"
        );
    }
}
