use std::{
    fs,
    net::SocketAddr,
    path::{Path, PathBuf},
    process::Command,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
};

use switchbot_exporter::{config::Config, switchbot::Switchbot};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, copy_bidirectional},
    net::{TcpListener, TcpStream},
    task::JoinHandle,
};
use tokio_rustls::{
    TlsAcceptor,
    rustls::{
        self,
        pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject},
    },
};

static DIRECTORY_SEQUENCE: AtomicU64 = AtomicU64::new(0);

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "switchbot-network-{}-{}",
            std::process::id(),
            DIRECTORY_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        fs::write(path.join(".env"), "").unwrap();
        Self(path)
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[derive(Clone)]
enum ServerMode {
    Http,
    Https(TlsAcceptor),
    HttpProxy,
    ConnectProxy(SocketAddr),
}

struct LocalServer {
    address: SocketAddr,
    requests: Arc<Mutex<Vec<String>>>,
    task: JoinHandle<()>,
}

impl LocalServer {
    async fn new(mode: ServerMode) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let recorded = requests.clone();
        let task = tokio::spawn(async move {
            loop {
                let (stream, _) = listener.accept().await.unwrap();
                let mode = mode.clone();
                let recorded = recorded.clone();
                tokio::spawn(async move {
                    match mode {
                        ServerMode::Https(acceptor) => {
                            if let Ok(stream) = acceptor.accept(stream).await {
                                serve_http(stream, recorded, 88).await;
                            }
                        }
                        ServerMode::Http => serve_http(stream, recorded, 88).await,
                        ServerMode::HttpProxy => serve_http(stream, recorded, 77).await,
                        ServerMode::ConnectProxy(target) => {
                            serve_tunnel(stream, recorded, target).await
                        }
                    }
                });
            }
        });
        Self {
            address,
            requests,
            task,
        }
    }

    fn url(&self, scheme: &str) -> String {
        format!("{scheme}://{}", self.address)
    }

    fn requests(&self) -> Vec<String> {
        self.requests.lock().unwrap().clone()
    }
}

impl Drop for LocalServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn read_request_line(stream: &mut (impl AsyncRead + Unpin)) -> String {
    let mut request = Vec::new();
    let mut buffer = [0_u8; 1024];
    while !request.windows(4).any(|part| part == b"\r\n\r\n") {
        let count = stream.read(&mut buffer).await.unwrap();
        assert!(count > 0, "HTTP ヘッダーが読み込まれること");
        request.extend_from_slice(&buffer[..count]);
        assert!(request.len() <= 16_384, "HTTP ヘッダーが有界であること");
    }
    String::from_utf8(request)
        .unwrap()
        .lines()
        .next()
        .unwrap()
        .to_owned()
}

async fn serve_http(
    mut stream: impl AsyncRead + AsyncWrite + Unpin,
    requests: Arc<Mutex<Vec<String>>>,
    battery: u64,
) {
    let line = read_request_line(&mut stream).await;
    requests.lock().unwrap().push(line);
    let body = format!("{{\"statusCode\":100,\"body\":{{\"battery\":{battery}}}}}");
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(response.as_bytes()).await.unwrap();
    stream.shutdown().await.unwrap();
}

async fn serve_tunnel(
    mut stream: TcpStream,
    requests: Arc<Mutex<Vec<String>>>,
    target: SocketAddr,
) {
    let line = read_request_line(&mut stream).await;
    assert_eq!(line, format!("CONNECT {target} HTTP/1.1"));
    requests.lock().unwrap().push(line);
    let mut upstream = TcpStream::connect(target).await.unwrap();
    stream
        .write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
        .await
        .unwrap();
    let _ = copy_bidirectional(&mut stream, &mut upstream).await;
}

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/tls")
        .join(name)
}

fn tls_acceptor() -> TlsAcceptor {
    let certificates = CertificateDer::pem_file_iter(fixture("server.pem"))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let key = PrivateKeyDer::from_pem_file(fixture("server-key.pem")).unwrap();
    let config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certificates, key)
        .unwrap();
    TlsAcceptor::from(Arc::new(config))
}

fn probe(directory: &Path, base_url: &str, expected: &str, environment: &[(&str, String)]) {
    let output = Command::new(std::env::current_exe().unwrap())
        .args([
            "--ignored",
            "--exact",
            "isolated_network_probe",
            "--nocapture",
        ])
        .current_dir(directory)
        .env_clear()
        .env("SWITCHBOT_API_TOKEN", "network-test-token")
        .env("SWITCHBOT_API_SECRET", "network-test-secret")
        .env("CACHE_EXPIRE_SECOND", "0")
        .env("DELAY_SECOND", "0")
        .env("API_TIMEOUT_SECOND", "5")
        .env("TEST_BASE_URL", base_url)
        .env("TEST_EXPECTED_RESPONSE", expected)
        .envs(environment.iter().map(|(key, value)| (*key, value)))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "child test failed:\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

/// 子プロセスの環境設定でだけネットワーク client が生成されること。
/// Arrange: 親からローカル URL・期待値・隔離された環境が設定されること。
/// Act: 公開 Config と Switchbot から status が取得されること。
/// Assert: 設定に応じた成功値または検証エラーが返されること。
#[tokio::test]
#[ignore = "隔離された子プロセスからのみ実行"]
async fn isolated_network_probe() {
    // Arrange
    let Ok(base_url) = std::env::var("TEST_BASE_URL") else {
        return;
    };
    let expected = std::env::var("TEST_EXPECTED_RESPONSE").unwrap();
    let directory = std::env::current_dir().unwrap();
    let client = Config::load(&directory).and_then(|config| {
        Switchbot::with_base_url(config, reqwest::Url::parse(&base_url).unwrap())
    });

    // Act
    let result = match client {
        Ok(mut client) => client.fetch_device_status("meter").await,
        Err(error) => Err(error),
    };

    // Assert
    if expected == "error" {
        assert!(result.is_err(), "不正な CA で TLS 検証が成功しないこと");
    } else {
        let response = result.expect("ローカル API 取得が成功すること");
        assert_eq!(
            response["body"]["battery"].as_u64(),
            Some(expected.parse().unwrap())
        );
    }
}

/// HTTP_PROXY と ALL_PROXY の設定でローカル proxy が使用されること。
/// Arrange: 異なる値を返す直接 API と proxy が用意されること。
/// Act: 各 proxy 環境変数を持つ子プロセスで取得されること。
/// Assert: absolute-form の proxy リクエストだけが行われること。
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn http_and_all_proxy_environment_route_requests_through_the_proxy() {
    // Arrange
    for key in ["HTTP_PROXY", "ALL_PROXY"] {
        let directory = TestDirectory::new();
        let target = LocalServer::new(ServerMode::Http).await;
        let proxy = LocalServer::new(ServerMode::HttpProxy).await;

        // Act
        probe(
            &directory.0,
            &target.url("http"),
            "77",
            &[(key, proxy.url("http"))],
        );

        // Assert
        assert!(target.requests().is_empty(), "{key}");
        assert_eq!(
            proxy.requests(),
            [format!(
                "GET {}/v1.1/devices/meter/status HTTP/1.1",
                target.url("http")
            )]
        );
    }
}

/// NO_PROXY の対象では HTTP proxy が迂回されること。
/// Arrange: API のホストと HTTP_PROXY・NO_PROXY が用意されること。
/// Act: 子プロセスから status が取得されること。
/// Assert: API が直接呼び出され proxy には到達しないこと。
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn no_proxy_bypasses_the_configured_http_proxy() {
    // Arrange
    let directory = TestDirectory::new();
    let target = LocalServer::new(ServerMode::Http).await;
    let proxy = LocalServer::new(ServerMode::HttpProxy).await;

    // Act
    probe(
        &directory.0,
        &target.url("http"),
        "88",
        &[
            ("HTTP_PROXY", proxy.url("http")),
            ("NO_PROXY", "127.0.0.1".to_owned()),
        ],
    );

    // Assert
    assert!(proxy.requests().is_empty());
    assert_eq!(
        target.requests(),
        ["GET /v1.1/devices/meter/status HTTP/1.1"]
    );
}

/// HTTPS_PROXY の CONNECT と NO_PROXY の迂回が使用されること。
/// Arrange: 独自 CA の HTTPS API とローカル CONNECT proxy が用意されること。
/// Act: proxy 利用時と bypass 時の子プロセスで取得されること。
/// Assert: TLS 検証を保って指定された経路だけが使われること。
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn https_proxy_uses_connect_and_no_proxy_can_bypass_it() {
    // Arrange
    for bypass in [false, true] {
        let directory = TestDirectory::new();
        let target = LocalServer::new(ServerMode::Https(tls_acceptor())).await;
        let proxy = LocalServer::new(ServerMode::ConnectProxy(target.address)).await;
        let mut environment = vec![
            ("HTTPS_PROXY", proxy.url("http")),
            (
                "REQUESTS_CA_BUNDLE",
                fixture("ca.pem").display().to_string(),
            ),
        ];
        if bypass {
            environment.push(("NO_PROXY", "127.0.0.1".to_owned()));
        }

        // Act
        probe(&directory.0, &target.url("https"), "88", &environment);

        // Assert
        assert_eq!(
            target.requests(),
            ["GET /v1.1/devices/meter/status HTTP/1.1"]
        );
        if bypass {
            assert!(proxy.requests().is_empty());
        } else {
            assert_eq!(
                proxy.requests(),
                [format!("CONNECT {} HTTP/1.1", target.address)]
            );
        }
    }
}

/// REQUESTS_CA_BUNDLE の優先と空値時の CURL_CA_BUNDLE fallback が保たれること。
/// Arrange: 信頼 CA と異なる CA の優先・fallback 設定が用意されること。
/// Act: 各設定を持つ子プロセスでローカル HTTPS が取得されること。
/// Assert: REQUESTS の非空値が優先され、空または未設定なら CURL が使われること。
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn requests_ca_bundle_has_priority_and_empty_values_fall_back_to_curl() {
    // Arrange
    let directory = TestDirectory::new();
    let target = LocalServer::new(ServerMode::Https(tls_acceptor())).await;
    let trusted = fixture("ca.pem").display().to_string();
    let other = fixture("other-ca.pem").display().to_string();
    let cases = [
        (
            "88",
            vec![
                ("REQUESTS_CA_BUNDLE", trusted.clone()),
                ("CURL_CA_BUNDLE", other.clone()),
            ],
        ),
        (
            "88",
            vec![
                ("REQUESTS_CA_BUNDLE", String::new()),
                ("CURL_CA_BUNDLE", trusted.clone()),
            ],
        ),
        ("88", vec![("CURL_CA_BUNDLE", trusted.clone())]),
        (
            "error",
            vec![("REQUESTS_CA_BUNDLE", other), ("CURL_CA_BUNDLE", trusted)],
        ),
    ];

    // Act
    for (expected, environment) in cases {
        probe(&directory.0, &target.url("https"), expected, &environment);
    }

    // Assert
    assert_eq!(target.requests().len(), 3);
}

/// CA が未設定・誤り・不正な場合は HTTPS 検証が拒否されること。
/// Arrange: 信頼されない独自 CA サーバと各不正設定が用意されること。
/// Act: 設定ごとに子プロセスで取得が試行されること。
/// Assert: TLS 検証が無効化されず API の HTTP 応答に到達しないこと。
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn untrusted_missing_and_invalid_ca_bundles_never_disable_verification() {
    // Arrange
    let directory = TestDirectory::new();
    let target = LocalServer::new(ServerMode::Https(tls_acceptor())).await;
    let invalid = directory.0.join("invalid-ca.pem");
    let empty = directory.0.join("empty-ca.pem");
    fs::write(&invalid, "this is not a PEM certificate").unwrap();
    fs::write(&empty, "").unwrap();
    let cases = [
        vec![],
        vec![(
            "REQUESTS_CA_BUNDLE",
            fixture("other-ca.pem").display().to_string(),
        )],
        vec![("REQUESTS_CA_BUNDLE", invalid.display().to_string())],
        vec![("REQUESTS_CA_BUNDLE", empty.display().to_string())],
        vec![(
            "REQUESTS_CA_BUNDLE",
            directory.0.join("missing.pem").display().to_string(),
        )],
    ];

    // Act
    for environment in cases {
        probe(&directory.0, &target.url("https"), "error", &environment);
    }

    // Assert
    assert!(target.requests().is_empty());
}

/// .env で定義された CA bundle が公開設定の境界から利用されること。
/// Arrange: 親階層探索から分離した .env に信頼 CA が指定されること。
/// Act: CA の実環境変数を持たない子プロセスで HTTPS が取得されること。
/// Assert: .env の CA によって検証された応答が返されること。
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dotenv_ca_bundle_is_applied_to_the_https_client() {
    // Arrange
    let directory = TestDirectory::new();
    let target = LocalServer::new(ServerMode::Https(tls_acceptor())).await;
    fs::write(
        directory.0.join(".env"),
        format!("REQUESTS_CA_BUNDLE='{}'\n", fixture("ca.pem").display()),
    )
    .unwrap();

    // Act
    probe(&directory.0, &target.url("https"), "88", &[]);

    // Assert
    assert_eq!(
        target.requests(),
        ["GET /v1.1/devices/meter/status HTTP/1.1"]
    );
}

/// 複数証明書の PEM bundle から全ての CA が読み込まれること。
/// Arrange: 別の CA が先頭で信頼 CA が二番目の bundle が用意されること。
/// Act: bundle を持つ子プロセスで HTTPS が取得されること。
/// Assert: 二番目の信頼 CA によって検証された応答が返されること。
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn every_certificate_in_a_pem_bundle_is_loaded() {
    // Arrange
    let directory = TestDirectory::new();
    let target = LocalServer::new(ServerMode::Https(tls_acceptor())).await;
    let bundle = directory.0.join("multiple-ca.pem");
    fs::write(
        &bundle,
        format!(
            "{}{}",
            fs::read_to_string(fixture("other-ca.pem")).unwrap(),
            fs::read_to_string(fixture("ca.pem")).unwrap()
        ),
    )
    .unwrap();

    // Act
    probe(
        &directory.0,
        &target.url("https"),
        "88",
        &[("REQUESTS_CA_BUNDLE", bundle.display().to_string())],
    );

    // Assert
    assert_eq!(
        target.requests(),
        ["GET /v1.1/devices/meter/status HTTP/1.1"]
    );
}
