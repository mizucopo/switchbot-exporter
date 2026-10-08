use serde_json::{Map, Value, json};
use switchbot_exporter::{metrics::Metrics, switchbot::Device};

fn device(id: &str, name: &str) -> Device {
    Device {
        device_id: id.to_owned(),
        device_type: "Meter".to_owned(),
        device_name: name.to_owned(),
    }
}

fn status(value: Value) -> Map<String, Value> {
    value.as_object().expect("status は JSON object").clone()
}

/// 七種類のメトリクス名・型・ラベル・出力順序が維持されること。
/// Arrange: 全メトリクスと対象外フィールドが用意されること。
/// Act: デバイスのメトリクスが描画されること。
/// Assert: Python 版の公開形式と同じテキストが返されること。
#[test]
fn all_metric_families_keep_names_types_labels_and_order() {
    // Arrange
    let mut metrics = Metrics::default();
    let values = status(json!({
        "electricCurrent": 0.25,
        "weight": 1.75,
        "voltage": 100,
        "CO2": 650,
        "temperature": -5.5,
        "humidity": 45,
        "battery": 88,
        "power": "on"
    }));
    let expected = concat!(
        "# HELP switchbot_device_battery SwitchBot Battery level\n",
        "# TYPE switchbot_device_battery gauge\n",
        "switchbot_device_battery{device_id=\"device-1\",device_name=\"Living Room\"} 88\n",
        "# HELP switchbot_device_humidity SwitchBot Humidity\n",
        "# TYPE switchbot_device_humidity gauge\n",
        "switchbot_device_humidity{device_id=\"device-1\",device_name=\"Living Room\"} 45\n",
        "# HELP switchbot_device_temperature SwitchBot Temperature\n",
        "# TYPE switchbot_device_temperature gauge\n",
        "switchbot_device_temperature{device_id=\"device-1\",device_name=\"Living Room\"} -5.5\n",
        "# HELP switchbot_device_co2 SwitchBot CO2\n",
        "# TYPE switchbot_device_co2 gauge\n",
        "switchbot_device_co2{device_id=\"device-1\",device_name=\"Living Room\"} 650\n",
        "# HELP switchbot_device_voltage SwitchBot Voltage\n",
        "# TYPE switchbot_device_voltage gauge\n",
        "switchbot_device_voltage{device_id=\"device-1\",device_name=\"Living Room\"} 100\n",
        "# HELP switchbot_device_weight SwitchBot Weight\n",
        "# TYPE switchbot_device_weight gauge\n",
        "switchbot_device_weight{device_id=\"device-1\",device_name=\"Living Room\"} 1.75\n",
        "# HELP switchbot_device_electric_current SwitchBot ElectricCurrent\n",
        "# TYPE switchbot_device_electric_current gauge\n",
        "switchbot_device_electric_current{device_id=\"device-1\",device_name=\"Living Room\"} 0.25"
    );

    // Act
    metrics
        .add(&device("device-1", "Living Room"), &values)
        .unwrap();
    let rendered = metrics.render();

    // Assert
    assert_eq!(rendered, expected);
}

/// 値がない場合にも七種類の HELP と TYPE が返されること。
/// Arrange: 空のメトリクスが用意されること。
/// Act: メトリクスが描画されること。
/// Assert: サンプルなしの公開メトリクス定義が返されること。
#[test]
fn empty_metrics_still_expose_every_family() {
    // Arrange
    let metrics = Metrics::default();

    // Act
    let rendered = metrics.render();

    // Assert
    assert_eq!(rendered.lines().count(), 14);
    assert_eq!(rendered.matches("# HELP ").count(), 7);
    assert_eq!(rendered.matches(" gauge").count(), 7);
    assert!(!rendered.ends_with('\n'));
    assert!(!rendered.contains("device_id="));
}

/// 引用符・改行・バックスラッシュが両ラベルでエスケープされること。
/// Arrange: 特殊文字と日本語を含む識別子・名前が用意されること。
/// Act: battery メトリクスが描画されること。
/// Assert: Prometheus の一行のラベル値として安全に返されること。
#[test]
fn label_special_characters_are_escaped_without_changing_unicode() {
    // Arrange
    let mut metrics = Metrics::default();
    let target = device("id\\\"\n", "居間\\\"\n温度計");

    // Act
    metrics
        .add(&target, &status(json!({"battery": 90})))
        .unwrap();
    let rendered = metrics.render();

    // Assert
    assert!(rendered.contains(
        "switchbot_device_battery{device_id=\"id\\\\\\\"\\n\",device_name=\"居間\\\\\\\"\\n温度計\"} 90"
    ));
    assert_eq!(rendered.lines().count(), 15);
}

/// 重複 ID が初回順序を保ち、最後の名前が全メトリクスへ反映されること。
/// Arrange: 二台と一部フィールドだけを更新する重複 ID が用意されること。
/// Act: 各デバイスのメトリクスが追加されること。
/// Assert: 値の更新順序と共有名が Python の辞書と一致すること。
#[test]
fn duplicate_ids_update_values_and_shared_names_without_reordering() {
    // Arrange
    let mut metrics = Metrics::default();
    metrics
        .add(
            &device("first", "旧名"),
            &status(json!({"battery": 10, "humidity": 30})),
        )
        .unwrap();
    metrics
        .add(&device("second", "二台目"), &status(json!({"battery": 20})))
        .unwrap();

    // Act
    metrics
        .add(&device("first", "新名"), &status(json!({"battery": 99})))
        .unwrap();
    let rendered = metrics.render();

    // Assert
    let samples: Vec<_> = rendered
        .lines()
        .filter(|line| !line.starts_with('#'))
        .collect();
    assert_eq!(
        samples,
        [
            "switchbot_device_battery{device_id=\"first\",device_name=\"新名\"} 99",
            "switchbot_device_battery{device_id=\"second\",device_name=\"二台目\"} 20",
            "switchbot_device_humidity{device_id=\"first\",device_name=\"新名\"} 30",
        ]
    );
    assert!(!rendered.contains("旧名"));
}

/// 数値でないメトリクスが公開前に拒否されること。
/// Arrange: 文字列・真偽値・null・配列を含む battery 値が用意されること。
/// Act: 各値がメトリクスへ追加されること。
/// Assert: 無効な Prometheus サンプルが成功として扱われないこと。
#[test]
fn non_numeric_metric_values_are_rejected() {
    // Arrange
    let invalid_values = [
        json!("88"),
        json!(true),
        Value::Null,
        json!([88]),
        json!({"value": 88}),
    ];

    // Act
    let results: Vec<_> = invalid_values
        .into_iter()
        .map(|battery| {
            Metrics::default().add(
                &device("device-1", "Meter"),
                &status(json!({"battery": battery})),
            )
        })
        .collect();

    // Assert
    assert!(results.into_iter().all(|result| result.is_err()));
}
