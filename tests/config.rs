use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};

use switchbot_exporter::config::{Config, Values, load_values, parse_env};

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
