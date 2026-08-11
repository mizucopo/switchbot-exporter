from unittest.mock import Mock, patch

from switchbot import Switchbot, SwitchbotDevice


def test_fetch_devices_maps_api_response() -> None:
    """APIのdevice responseがdomain objectへ変換されること.

    Arrange: SwitchBot API responseを返すHTTP boundaryが用意されること。
    Act: device一覧が取得されること。
    Assert: deviceの識別子・種類・名前が変換されること。
    """
    # Arrange
    response = Mock()
    response.text = (
        '{"body":{"deviceList":[{"deviceId":"device-1",'
        '"deviceType":"Meter","deviceName":"Living Room"}]}}'
    )
    client = Switchbot("token", "secret", cache_expire_second=0, delay_second=0)

    # Act
    with patch("switchbot.requests.get", return_value=response) as request:
        devices = client.fetch_devices()

    # Assert
    assert devices == [
        SwitchbotDevice(
            device_id="device-1",
            device_type="Meter",
            device_name="Living Room",
        )
    ]
    response.raise_for_status.assert_called_once_with()
    request.assert_called_once()


def test_fetch_device_status_maps_api_response() -> None:
    """APIのstatus responseが返されること.

    Arrange: battery statusを返すHTTP boundaryが用意されること。
    Act: 指定deviceのstatusが取得されること。
    Assert: response bodyが保持されること。
    """
    # Arrange
    response = Mock()
    response.text = '{"body":{"battery":88}}'
    client = Switchbot("token", "secret", cache_expire_second=0, delay_second=0)

    # Act
    with patch("switchbot.requests.get", return_value=response):
        status = client.fetch_device_status("device-1")

    # Assert
    assert status == {"body": {"battery": 88}}
    response.raise_for_status.assert_called_once_with()


def test_fetch_metrics_uses_device_status() -> None:
    """対応deviceのstatusがPrometheus metricへ集約されること.

    Arrange: Meter deviceと取得済みstatusが用意されること。
    Act: device一覧からmetricが生成されること。
    Assert: battery・humidity・temperatureがdevice IDへ対応すること。
    """
    # Arrange
    client = Switchbot("token", "secret", cache_expire_second=0, delay_second=0)
    device = SwitchbotDevice(
        device_id="device-1",
        device_type="Meter",
        device_name='Living "Room"',
    )
    status = {"body": {"battery": 88, "humidity": 45, "temperature": 23.5}}

    # Act
    with patch.object(client, "fetch_device_status", return_value=status):
        metrics = client.fetch_metrics([device])

    # Assert
    assert metrics.escape_device_names == {"device-1": 'Living \\"Room\\"'}
    assert metrics.batteries == {"device-1": 88}
    assert metrics.humidities == {"device-1": 45}
    assert metrics.temperatures == {"device-1": 23.5}
