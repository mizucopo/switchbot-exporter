import pytest

from switchbot import Switchbot


class TestSwitchbot:
    @pytest.fixture(autouse=True)
    def setup(self) -> None:
        from tests.conftest import app_config

        self.api_token = app_config("SWITCHBOT_API_TOKEN", default="test_token")
        self.api_secret = app_config("SWITCHBOT_API_SECRET", default="test_secret")
        self.cache_expire_second = 0

        self.switchbot = Switchbot(
            self.api_token,
            self.api_secret,
            cache_expire_second=self.cache_expire_second,
        )

    @pytest.mark.vcr(cassette_name="test_fetch_devices")
    def test_fetch_devices(self) -> None:
        devices = self.switchbot.fetch_devices()
        # 実際のAPIレスポンスに基づいて検証
        assert len(devices) > 0
        # 最初のデバイスが正しく取得できているか確認
        first_device = devices[0]
        assert first_device.device_id is not None
        assert first_device.device_type is not None
        assert first_device.device_name is not None

    @pytest.mark.vcr(cassette_name="test_fetch_device_status")
    def test_fetch_device_status(self) -> None:
        # fetch_devices() で取得したデバイスを使用
        devices = self.switchbot.fetch_devices()
        assert len(devices) > 0

        # 最初のデバイスのステータスを取得
        first_device = devices[0]
        status = self.switchbot.fetch_device_status(first_device.device_id)
        assert "body" in status

    @pytest.mark.vcr(cassette_name="test_fetch_metrics")
    def test_fetch_metrics(self) -> None:
        # fetch_devices() で取得したデバイスを使用
        devices = self.switchbot.fetch_devices()
        assert len(devices) > 0

        metrics = self.switchbot.fetch_metrics(devices)
        # 最初のデバイスIDがメトリクスに含まれているか確認
        first_device = devices[0]
        assert first_device.device_id in metrics.escape_device_names
