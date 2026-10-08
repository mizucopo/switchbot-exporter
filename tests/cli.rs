use std::process::{Command, Output};

fn cli(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_switchbot-exporter"))
        .args(args)
        .env_clear()
        .output()
        .expect("CLI が実行されること")
}

/// 認証情報なしで既存の四コマンドのヘルプが表示されること。
/// Arrange: 空の子プロセス環境が用意されること。
/// Act: トップレベルのヘルプが呼び出されること。
/// Assert: API 通信前に全コマンドが表示されること。
#[test]
fn help_lists_all_existing_command_names_without_credentials() {
    // Arrange
    let args = ["--help"];

    // Act
    let output = cli(&args);

    // Assert
    assert!(output.status.success());
    let help = String::from_utf8(output.stdout).unwrap();
    for command in ["devices", "device-status", "metrics", "exporter"] {
        assert!(help.contains(command));
    }
    assert!(output.stderr.is_empty());
}

/// バイナリの version がパッケージ設定と一致すること。
/// Arrange: 認証値を持たない version 引数が用意されること。
/// Act: version が表示されること。
/// Assert: 手動採番を含まず Cargo の version が返されること。
#[test]
fn version_comes_from_the_package_without_credentials() {
    // Arrange
    let args = ["--version"];

    // Act
    let output = cli(&args);

    // Assert
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap().trim(),
        format!("switchbot-exporter {}", env!("CARGO_PKG_VERSION"))
    );
    assert!(output.stderr.is_empty());
}

/// device-status の device ID 引数と各ヘルプが維持されること。
/// Arrange: 各サブコマンドの help 引数が用意されること。
/// Act: 認証情報なしでサブコマンドが解析されること。
/// Assert: 全ヘルプが成功し device-status に DEVICE_ID が示されること。
#[test]
fn subcommand_help_retains_device_status_argument() {
    // Arrange
    for name in ["devices", "device-status", "metrics", "exporter"] {
        let args = [name, "--help"];

        // Act
        let output = cli(&args);

        // Assert
        assert!(output.status.success(), "{name}");
        if name == "device-status" {
            assert!(
                String::from_utf8(output.stdout)
                    .unwrap()
                    .contains("<DEVICE_ID>")
            );
        }
        assert!(output.stderr.is_empty());
    }
}

/// device-status の不足引数が API 接続前に拒否されること。
/// Arrange: device ID を含まない CLI 引数が用意されること。
/// Act: サブコマンドが解析されること。
/// Assert: 認証エラーではなく不足引数のエラーが返されること。
#[test]
fn missing_device_status_id_is_reported_before_loading_credentials() {
    // Arrange
    let args = ["device-status"];

    // Act
    let output = cli(&args);

    // Assert
    assert!(!output.status.success());
    let error = String::from_utf8(output.stderr).unwrap();
    assert!(error.contains("<DEVICE_ID>"));
    assert!(!error.contains("SWITCHBOT_API_TOKEN"));
}

/// 関係のない非 UTF-8 環境値でも通常の設定エラーが返されること。
/// Arrange: 子プロセスだけに非 UTF-8 値と不正な SERVER_PORT が設定されること。
/// Act: exporter の設定が読み込まれること。
/// Assert: 環境列挙が panic せず、port の通常エラーが返されること。
#[cfg(unix)]
#[test]
fn unrelated_non_utf8_environment_value_does_not_panic() {
    use std::{ffi::OsString, os::unix::ffi::OsStringExt};

    // Arrange
    let mut command = Command::new(env!("CARGO_BIN_EXE_switchbot-exporter"));
    command
        .args(["exporter"])
        .env_clear()
        .env("SERVER_PORT", "invalid-port")
        .env(
            "UNRELATED_BINARY_VALUE",
            OsString::from_vec(vec![0xff, 0xfe]),
        );

    // Act
    let output = command.output().unwrap();

    // Assert
    assert!(!output.status.success());
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("SERVER_PORT"), "{error}");
    assert!(!error.contains("panicked"), "{error}");
    assert!(!error.contains("VarError"), "{error}");
}

/// 関係のない非 UTF-8 環境キーでも通常の認証設定エラーが返されること。
/// Arrange: 子プロセスだけに非 UTF-8 キーと認証値のない環境が設定されること。
/// Act: metrics の設定が読み込まれること。
/// Assert: 環境列挙が panic せず、必須認証値の通常エラーが返されること。
#[cfg(unix)]
#[test]
fn unrelated_non_utf8_environment_key_does_not_panic() {
    use std::{ffi::OsString, os::unix::ffi::OsStringExt};

    // Arrange
    let mut command = Command::new(env!("CARGO_BIN_EXE_switchbot-exporter"));
    command
        .args(["metrics"])
        .env_clear()
        .env(OsString::from_vec(vec![b'U', b'N', 0xff]), "unrelated");

    // Act
    let output = command.output().unwrap();

    // Assert
    assert!(!output.status.success());
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("SWITCHBOT_API_TOKEN"), "{error}");
    assert!(!error.contains("panicked"), "{error}");
    assert!(!error.contains("VarError"), "{error}");
}
