import os
from pathlib import Path
from typing import cast
from unittest.mock import Mock, patch

import pytest
from decouple import UndefinedValueError

from app import create_app, generate_prometheus_response_text, get_switchbot
from config import get_optional_env_var, get_required_env_var
from switchbot import Switchbot, SwitchbotMetrics


def test_generate_prometheus_response_text() -> None:
    """取得済みメトリクスがPrometheus形式へ変換されること.

    Arrange: SwitchBotメトリクスが用意されること。
    Act: レスポンステキストが生成されること。
    Assert: 対象デバイスのbattery metricが含まれること。
    """
    # Arrange
    metrics = SwitchbotMetrics(
        escape_device_names={"123": "Test Bot"},
        batteries={"123": 100},
    )

    # Act
    response_text = generate_prometheus_response_text(metrics)

    # Assert
    assert 'device_id="123"' in response_text
    assert (
        'switchbot_device_battery{device_id="123",device_name="Test Bot"} 100'
        in response_text
    )


def test_create_app_uses_injected_client() -> None:
    """認証情報なしでも注入済みclientでHTTP応答が生成されること.

    Arrange: SwitchBot clientと取得結果が用意されること。
    Act: application factoryからmetrics endpointが呼び出されること。
    Assert: 正常応答と取得済みmetricが返されること。
    """
    # Arrange
    client = cast(Switchbot, Mock(spec=Switchbot))
    client.fetch_devices.return_value = []  # type: ignore[attr-defined]
    client.fetch_metrics.return_value = SwitchbotMetrics(  # type: ignore[attr-defined]
        escape_device_names={"123": "Test Bot"},
        batteries={"123": 100},
    )
    flask_app = create_app(client)

    # Act
    response = flask_app.test_client().get("/metrics")

    # Assert
    assert response.status_code == 200
    assert 'device_id="123"' in response.get_data(as_text=True)


def test_get_switchbot_reads_environment_when_first_used(tmp_path: Path) -> None:
    """SwitchBot client生成時に環境設定が読み込まれること.

    Arrange: 必須・任意の環境変数が用意されること。
    Act: 共有SwitchBot clientが初回取得されること。
    Assert: 読み込まれた設定でclientが生成されること。
    """
    # Arrange
    environment = {
        "SWITCHBOT_API_TOKEN": "test_token",
        "SWITCHBOT_API_SECRET": "test_secret",
        "CACHE_DIR": str(tmp_path),
        "CACHE_EXPIRE_SECOND": "300",
        "DELAY_SECOND": "0.5",
    }
    get_switchbot.cache_clear()

    # Act
    with patch.dict(os.environ, environment, clear=True):
        client = get_switchbot()

    # Assert
    assert client.api_token == "test_token"
    assert client.api_secret == "test_secret"
    assert client.cache_dir == str(tmp_path)
    assert client.cache_expire_second == 300
    assert client.delay_second == 0.5
    get_switchbot.cache_clear()


def test_required_env_var_missing() -> None:
    """必須環境変数が未設定の場合に例外が送出されること.

    Arrange: 空の環境変数が用意されること。
    Act: 未設定の必須値が取得されること。
    Assert: UndefinedValueErrorが送出されること。
    """
    # Arrange
    environment: dict[str, str] = {}

    # Act
    with (
        patch.dict(os.environ, environment, clear=True),
        pytest.raises(UndefinedValueError) as error,
    ):
        get_required_env_var("MISSING_VAR")

    # Assert
    assert error.type is UndefinedValueError


def test_optional_env_var_defaults() -> None:
    """任意環境変数が未設定の場合に既定値が返されること.

    Arrange: 空の環境変数が用意されること。
    Act: 任意のserver portが取得されること。
    Assert: 既定portが返されること。
    """
    # Arrange
    environment: dict[str, str] = {}

    # Act
    with patch.dict(os.environ, environment, clear=True):
        port = get_optional_env_var("SERVER_PORT", 9171, int)

    # Assert
    assert port == 9171
