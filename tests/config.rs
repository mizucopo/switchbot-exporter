use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};

use switchbot_exporter::config::{Config, Values, load_values, parse_env};

#[cfg(unix)]
use std::{ffi::OsString, os::unix::ffi::OsStringExt, path::Path, process::Command};

#[cfg(unix)]
use switchbot_exporter::config::server_port;

static DIRECTORY_SEQUENCE: AtomicU64 = AtomicU64::new(0);

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "switchbot-exporter-config-{}-{}",
            std::process::id(),
            DIRECTORY_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

fn credentials() -> Values {
    Values::from([
        ("SWITCHBOT_API_TOKEN".to_owned(), "test-token".to_owned()),
        ("SWITCHBOT_API_SECRET".to_owned(), "test-secret".to_owned()),
    ])
}

/// 既存の既定設定と追加された API timeout が読み込まれること。
/// Arrange: 必須の認証値だけが用意されること。
/// Act: 設定が生成されること。
/// Assert: cache 600 秒・delay 1 秒・timeout 30 秒が返されること。
#[test]
fn defaults_preserve_cache_and_request_delay() {
    // Arrange
    let values = credentials();

    // Act
    let config = Config::from_values(&values).unwrap();

    // Assert
    assert_eq!(config.api_token, "test-token");
    assert_eq!(config.api_secret, "test-secret");
    assert!(config.cache_enabled);
    assert_eq!(config.cache_ttl, Duration::from_secs(600));
    assert_eq!(config.delay, Duration::from_secs(1));
    assert_eq!(config.api_timeout, Duration::from_secs(30));
}

/// 必須値の欠落が秘密値を含めず説明されること。
/// Arrange: token または secret が欠落した設定が用意されること。
/// Act: 設定の生成が試行されること。
/// Assert: 欠落した環境変数名だけを含むエラーが返されること。
#[test]
fn missing_credentials_identify_the_required_variable() {
    // Arrange
    for name in ["SWITCHBOT_API_TOKEN", "SWITCHBOT_API_SECRET"] {
        let mut values = credentials();
        values.remove(name);

        // Act
        let error = Config::from_values(&values)
            .err()
            .expect("必須設定のエラー")
            .to_string();

        // Assert
        assert!(error.contains(name));
        assert!(!error.contains("test-token"));
        assert!(!error.contains("test-secret"));
    }
}

/// 空の認証値が未定義と区別され、環境変数互換性が保たれること。
/// Arrange: 定義済みの空の token と secret が用意されること。
/// Act: 設定が生成されること。
/// Assert: python-decouple と同様に空の値が保持されること。
#[test]
fn defined_empty_credentials_are_preserved() {
    // Arrange
    let values = Values::from([
        ("SWITCHBOT_API_TOKEN".to_owned(), String::new()),
        ("SWITCHBOT_API_SECRET".to_owned(), String::new()),
    ]);

    // Act
    let config = Config::from_values(&values).unwrap();

    // Assert
    assert!(config.api_token.is_empty());
    assert!(config.api_secret.is_empty());
}

/// 空の cache directory と正でない TTL でキャッシュが無効になること。
/// Arrange: 既存のキャッシュ無効化設定が用意されること。
/// Act: 各設定が読み込まれること。
/// Assert: 永続キャッシュを利用せず無効状態が返されること。
#[test]
fn cache_can_be_disabled_with_empty_directory_or_non_positive_ttl() {
    // Arrange
    for (key, value) in [
        ("CACHE_DIR", ""),
        ("CACHE_EXPIRE_SECOND", "0"),
        ("CACHE_EXPIRE_SECOND", "-1"),
    ] {
        let mut values = credentials();
        values.insert(key.to_owned(), value.to_owned());

        // Act
        let config = Config::from_values(&values).unwrap();

        // Assert
        assert!(!config.cache_enabled);
    }
}

/// delay と timeout の小数秒、および整数 TTL が読み込まれること。
/// Arrange: 既存の環境変数名による上書きが用意されること。
/// Act: 設定が生成されること。
/// Assert: 指定された時間がそのまま保持されること。
#[test]
fn fractional_delays_and_custom_ttl_are_supported() {
    // Arrange
    let mut values = credentials();
    values.extend([
        ("CACHE_EXPIRE_SECOND".to_owned(), "300".to_owned()),
        ("DELAY_SECOND".to_owned(), "0.5".to_owned()),
        ("API_TIMEOUT_SECOND".to_owned(), "2.5".to_owned()),
    ]);

    // Act
    let config = Config::from_values(&values).unwrap();

    // Assert
    assert_eq!(config.cache_ttl, Duration::from_secs(300));
    assert_eq!(config.delay, Duration::from_millis(500));
    assert_eq!(config.api_timeout, Duration::from_millis(2500));
}

/// 無効な数値設定が起動前に拒否されること。
/// Arrange: 非数値・負数・非有限数・ゼロ timeout が用意されること。
/// Act: 各設定が読み込まれること。
/// Assert: 該当環境変数名のエラーが返されること。
#[test]
fn invalid_numeric_settings_are_rejected() {
    // Arrange
    let invalid = [
        ("CACHE_EXPIRE_SECOND", "1.5"),
        ("CACHE_EXPIRE_SECOND", "invalid"),
        ("DELAY_SECOND", "invalid"),
        ("DELAY_SECOND", "-1"),
        ("DELAY_SECOND", "NaN"),
        ("DELAY_SECOND", "inf"),
        ("API_TIMEOUT_SECOND", "0"),
        ("API_TIMEOUT_SECOND", "-1"),
        ("API_TIMEOUT_SECOND", "NaN"),
        ("API_TIMEOUT_SECOND", "inf"),
    ];
    for (key, value) in invalid {
        let mut values = credentials();
        values.insert(key.to_owned(), value.to_owned());

        // Act
        let error = Config::from_values(&values)
            .err()
            .expect("数値設定のエラー");

        // Assert
        assert!(error.to_string().contains(key), "{key}={value}");
    }
}

/// .env の引用符・空白・コメント・重複キーの互換性が保たれること。
/// Arrange: python-decouple で読み込める .env テキストが用意されること。
/// Act: .env の値が解析されること。
/// Assert: 展開を行わず値と最終定義が保持されること。
#[test]
fn dotenv_parser_preserves_python_decouple_values() {
    // Arrange
    let contents = "\n # comment\n TOKEN = \"quoted = # value\"\n SINGLE='日本語'\n RAW=literal # text\n EMPTY=\n DUP=old\n DUP=new\n REF=${TOKEN}\n not_an_assignment\n";

    // Act
    let values = parse_env(contents);

    // Assert
    assert_eq!(
        values,
        Values::from([
            ("TOKEN".to_owned(), "quoted = # value".to_owned()),
            ("SINGLE".to_owned(), "日本語".to_owned()),
            ("RAW".to_owned(), "literal # text".to_owned()),
            ("EMPTY".to_owned(), String::new()),
            ("DUP".to_owned(), "new".to_owned()),
            ("REF".to_owned(), "${TOKEN}".to_owned()),
        ])
    );
}

/// 最寄りの .env 一つだけが読み込まれ、実環境変数が優先されること。
/// Arrange: 親子の .env と既存の PATH 環境変数が用意されること。
/// Act: 深い作業ディレクトリから設定が読み込まれること。
/// Assert: 親の値が混入せず PATH の実値が保持されること。
#[test]
fn nearest_dotenv_is_used_and_environment_has_priority() {
    // Arrange
    let directory = TestDirectory::new();
    let child = directory.0.join("child");
    let nested = child.join("nested");
    fs::create_dir_all(&nested).unwrap();
    let key = format!("SWITCHBOT_TEST_{}_LOCAL", std::process::id());
    let parent_key = format!("SWITCHBOT_TEST_{}_PARENT", std::process::id());
    fs::write(
        directory.0.join(".env"),
        format!("{parent_key}=parent\n{key}=parent\n"),
    )
    .unwrap();
    fs::write(
        child.join(".env"),
        format!("{key}=child\nPATH=dotenv-path\n"),
    )
    .unwrap();
    let environment_path = std::env::var("PATH").expect("PATH が定義されること");

    // Act
    let values = load_values(&nested).unwrap();

    // Assert
    assert_eq!(values.get(&key).map(String::as_str), Some("child"));
    assert!(!values.contains_key(&parent_key));
    assert_eq!(values.get("PATH"), Some(&environment_path));
}

#[cfg(unix)]
fn configuration_directory(include_port: bool) -> TestDirectory {
    let directory = TestDirectory::new();
    let port = if include_port {
        "SERVER_PORT=19271\n"
    } else {
        ""
    };
    fs::write(directory.0.join(".env"), format!(
        "{port}SWITCHBOT_API_TOKEN=dotenv-token-sensitive\nSWITCHBOT_API_SECRET=dotenv-secret-sensitive\nREQUESTS_CA_BUNDLE=/tmp/ca-sensitive-fallback.pem\n"
    )).unwrap();
    directory
}

#[cfg(unix)]
fn non_unicode_setting_value() -> OsString {
    let mut value = b"nonunicode-private-marker".to_vec();
    value.push(0xff);
    OsString::from_vec(value)
}

#[cfg(unix)]
fn remove_dotenv_requests_bundle(directory: &Path) {
    let file = directory.join(".env");
    let contents = fs::read_to_string(&file).unwrap();
    let remaining = contents
        .lines()
        .filter(|line| !line.starts_with("REQUESTS_CA_BUNDLE="))
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(file, remaining).unwrap();
}

#[cfg(unix)]
fn configuration_probe(
    directory: &Path,
    consumer: &str,
    expected_error: Option<&str>,
    environment: &[(&str, OsString)],
) -> std::process::Output {
    Command::new(std::env::current_exe().unwrap())
        .args([
            "--ignored",
            "--exact",
            "isolated_configuration_probe",
            "--nocapture",
        ])
        .current_dir(directory)
        .env_clear()
        .env("CONFIG_PROBE_CONSUMER", consumer)
        .env("CONFIG_PROBE_ERROR_VARIABLE", expected_error.unwrap_or(""))
        .envs(environment.iter().map(|(key, value)| (*key, value)))
        .output()
        .unwrap()
}

#[cfg(unix)]
fn assert_configuration_probe(output: std::process::Output) {
    assert!(
        output.status.success(),
        "child test failed:\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

/// 子プロセスで消費者ごとの非 UTF-8 設定の検証が行われること。
/// Arrange: 公開設定 API・期待エラー・隔離された環境が用意されること。
/// Act: 指定された消費者の設定が読み込まれること。
/// Assert: 指定変数名だけの通常エラーまたは .env の正常設定が返されること。
#[cfg(unix)]
#[test]
#[ignore = "隔離された子プロセスからのみ実行"]
fn isolated_configuration_probe() {
    // Arrange
    let consumer = std::env::var("CONFIG_PROBE_CONSUMER").unwrap();
    let expected_error = std::env::var("CONFIG_PROBE_ERROR_VARIABLE").unwrap();
    let directory = std::env::current_dir().unwrap();

    // Act
    let result: anyhow::Result<()> = match consumer.as_str() {
        "port" => server_port(&directory).map(|port| assert_eq!(port, 19271)),
        "api" | "api-ca" => Config::load(&directory).map(|config| {
            assert_eq!(config.api_token, "dotenv-token-sensitive");
            assert_eq!(config.api_secret, "dotenv-secret-sensitive");
            if consumer == "api-ca" {
                let expected_ca = std::env::var("CONFIG_PROBE_EXPECTED_CA").unwrap();
                assert_eq!(config.ca_bundle.as_deref(), Some(Path::new(&expected_ca)));
            }
        }),
        "values" => load_values(&directory).map(|values| {
            assert_eq!(
                values.get("SWITCHBOT_API_TOKEN").map(String::as_str),
                Some("dotenv-token-sensitive")
            );
        }),
        _ => panic!("公開設定の消費者が指定されること"),
    };

    // Assert
    if expected_error.is_empty() {
        result.expect("消費しない設定の非 UTF-8 値は検証されないこと");
    } else {
        let error = result.expect_err("既知設定が .env や既定値に fallback されないこと");
        let message = format!("{error:#}");
        assert!(message.contains(&expected_error), "{message}");
        assert!(message.contains("UTF-8"), "{message}");
        for private_value in [
            "nonunicode-private-marker",
            "dotenv-token-sensitive",
            "dotenv-secret-sensitive",
            "ca-sensitive-fallback",
        ] {
            assert!(
                !message.contains(private_value),
                "秘密値がエラーに含まれないこと"
            );
        }
        assert!(!message.contains("panicked"));
    }
}

/// SERVER_PORT の非 UTF-8 override が .env や既定値へ戻らないこと。
/// Arrange: port の .env 有無それぞれに非 UTF-8 の実環境値が設定されること。
/// Act: 子プロセスの公開 server_port から設定が読み込まれること。
/// Assert: port 設定の通常エラーが秘密値を含めず返されること。
#[cfg(unix)]
#[test]
fn non_unicode_port_override_never_falls_back_to_dotenv_or_default() {
    // Arrange
    for include_port in [true, false] {
        let directory = configuration_directory(include_port);
        let environment = [("SERVER_PORT", non_unicode_setting_value())];

        // Act
        let output = configuration_probe(&directory.0, "port", Some("SERVER_PORT"), &environment);

        // Assert
        assert_configuration_probe(output);
    }
}

/// API が消費する非 UTF-8 設定の override が .env や既定値へ戻らないこと。
/// Arrange: 認証・cache・delay・timeout の各非 UTF-8 値が設定されること。
/// Act: 子プロセスの公開 Config::load から設定が読み込まれること。
/// Assert: 指定変数名だけの通常エラーが返されること。
#[cfg(unix)]
#[test]
fn non_unicode_api_overrides_never_fall_back_to_dotenv_or_default() {
    // Arrange
    let directory = configuration_directory(true);
    for key in [
        "SWITCHBOT_API_TOKEN",
        "SWITCHBOT_API_SECRET",
        "CACHE_DIR",
        "CACHE_EXPIRE_SECOND",
        "DELAY_SECOND",
        "API_TIMEOUT_SECOND",
    ] {
        let environment = [(key, non_unicode_setting_value())];

        // Act
        let output = configuration_probe(&directory.0, "api", Some(key), &environment);

        // Assert
        assert_configuration_probe(output);
    }
}

/// 選択される CA bundle の非 UTF-8 override が別の設定へ戻らないこと。
/// Arrange: REQUESTS と選択対象の CURL の各設定に非 UTF-8 値が設定されること。
/// Act: 子プロセスの公開 Config::load から設定が読み込まれること。
/// Assert: 指定変数名だけの通常エラーが秘密値を含めず返されること。
#[cfg(unix)]
#[test]
fn non_unicode_ca_override_never_falls_back_to_another_bundle() {
    // Arrange
    let directory = configuration_directory(true);
    let cases = [
        (
            "REQUESTS_CA_BUNDLE",
            vec![("REQUESTS_CA_BUNDLE", non_unicode_setting_value())],
        ),
        (
            "CURL_CA_BUNDLE",
            vec![
                ("REQUESTS_CA_BUNDLE", OsString::new()),
                ("CURL_CA_BUNDLE", non_unicode_setting_value()),
            ],
        ),
    ];
    for (key, environment) in cases {
        // Act
        let output = configuration_probe(&directory.0, "api", Some(key), &environment);

        // Assert
        assert_configuration_probe(output);
    }
}

/// REQUESTS が選択済みなら未使用 CURL の非 UTF-8 値が無視されること。
/// Arrange: .env または実環境の REQUESTS と非 UTF-8 の CURL が設定されること。
/// Act: 子プロセスの公開 Config::load から設定が読み込まれること。
/// Assert: 優先順位どおりの REQUESTS の CA path が正常に返されること。
#[cfg(unix)]
#[test]
fn selected_requests_bundle_ignores_unused_non_unicode_curl_override() {
    // Arrange
    for from_environment in [false, true] {
        let directory = configuration_directory(true);
        let selected_path = if from_environment {
            "/tmp/process-selected-ca.pem"
        } else {
            "/tmp/ca-sensitive-fallback.pem"
        };
        let mut environment = vec![
            ("CURL_CA_BUNDLE", non_unicode_setting_value()),
            ("CONFIG_PROBE_EXPECTED_CA", OsString::from(selected_path)),
        ];
        if from_environment {
            remove_dotenv_requests_bundle(&directory.0);
            environment.push(("REQUESTS_CA_BUNDLE", OsString::from(selected_path)));
        }

        // Act
        let output = configuration_probe(&directory.0, "api-ca", None, &environment);

        // Assert
        assert_configuration_probe(output);
    }
}

/// REQUESTS 未設定時に選択される CURL の非 UTF-8 値が拒否されること。
/// Arrange: REQUESTS の定義がなく非 UTF-8 の CURL が設定されること。
/// Act: 子プロセスの公開 Config::load から設定が読み込まれること。
/// Assert: CURL の通常エラーが秘密値を含めず返されること。
#[cfg(unix)]
#[test]
fn selected_non_unicode_curl_override_is_rejected_when_requests_is_unset() {
    // Arrange
    let directory = configuration_directory(true);
    remove_dotenv_requests_bundle(&directory.0);
    let environment = [("CURL_CA_BUNDLE", non_unicode_setting_value())];

    // Act
    let output = configuration_probe(&directory.0, "api", Some("CURL_CA_BUNDLE"), &environment);

    // Assert
    assert_configuration_probe(output);
}

/// 公開 load_values でも既知の非 UTF-8 override が拒否されること。
/// Arrange: port・token・優先順位によらない全 CA 設定に非 UTF-8 値が設定されること。
/// Act: 子プロセスの公開 load_values から設定が読み込まれること。
/// Assert: 型変換前に該当変数名だけの通常エラーが返されること。
#[cfg(unix)]
#[test]
fn public_values_loader_rejects_known_non_unicode_overrides() {
    // Arrange
    let directory = configuration_directory(true);
    for key in [
        "SERVER_PORT",
        "SWITCHBOT_API_TOKEN",
        "REQUESTS_CA_BUNDLE",
        "CURL_CA_BUNDLE",
    ] {
        let environment = [(key, non_unicode_setting_value())];

        // Act
        let output = configuration_probe(&directory.0, "values", Some(key), &environment);

        // Assert
        assert_configuration_probe(output);
    }
}

/// port 読み込み時には API 設定の非 UTF-8 値が遅延検証されること。
/// Arrange: 正常な port と非 UTF-8 の token・secret・CA が設定されること。
/// Act: 子プロセスから公開 server_port が読み込まれること。
/// Assert: API 設定の検証前に正しい port が返されること。
#[cfg(unix)]
#[test]
fn server_port_defers_validation_of_api_configuration() {
    // Arrange
    let directory = configuration_directory(true);
    let environment = [
        "SWITCHBOT_API_TOKEN",
        "SWITCHBOT_API_SECRET",
        "REQUESTS_CA_BUNDLE",
        "CURL_CA_BUNDLE",
    ]
    .map(|key| (key, non_unicode_setting_value()));

    // Act
    let output = configuration_probe(&directory.0, "port", None, &environment);

    // Assert
    assert_configuration_probe(output);
}

/// API 設定読み込み時には未使用 port の非 UTF-8 値が無視されること。
/// Arrange: 正常な API 設定と非 UTF-8 の SERVER_PORT が設定されること。
/// Act: 子プロセスから公開 Config::load が読み込まれること。
/// Assert: server 起動を必要としない API 設定が正常に返されること。
#[cfg(unix)]
#[test]
fn api_configuration_does_not_validate_an_unused_server_port() {
    // Arrange
    let directory = configuration_directory(true);
    let environment = [("SERVER_PORT", non_unicode_setting_value())];

    // Act
    let output = configuration_probe(&directory.0, "api", None, &environment);

    // Assert
    assert_configuration_probe(output);
}
