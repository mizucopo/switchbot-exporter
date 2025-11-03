import os
import unittest
from unittest.mock import MagicMock, patch

from app import generate_prometheus_response_text
from config import get_optional_env_var, get_required_env_var
from decouple import UndefinedValueError
from switchbot import SwitchbotMetrics


class TestApp(unittest.TestCase):

    def setUp(self):
        pass

    def test_generate_prometheus_response_text(self):
        metrics = SwitchbotMetrics(
            escape_device_names={
                "123": "Test Bot",
            },
            batteries={
                "123": 100,
            },
        )

        response_text = generate_prometheus_response_text(metrics)
        self.assertTrue(isinstance(response_text, str))

    @patch.dict(
        os.environ,
        {
            "SWITCHBOT_API_TOKEN": "test_token",
            "SWITCHBOT_API_SECRET": "test_secret",
            "SERVER_PORT": "8080",
            "CACHE_DIR": "/tmp/test",
            "CACHE_EXPIRE_SECOND": "300",
            "DELAY_SECOND": "0.5",
        },
    )
    def test_app_config_from_env(self):
        """環境変数から設定が正しく読み込まれることを確認."""
        # モジュールを再インポートして環境変数を反映
        import importlib
        import app

        importlib.reload(app)

        self.assertEqual(app.SWITCHBOT_API_TOKEN, "test_token")
        self.assertEqual(app.SWITCHBOT_API_SECRET, "test_secret")
        self.assertEqual(app.SERVER_PORT, 8080)
        self.assertEqual(app.CACHE_DIR, "/tmp/test")
        self.assertEqual(app.CACHE_EXPIRE_SECOND, 300)
        self.assertEqual(app.DELAY_SECOND, 0.5)

    def test_required_env_var_missing(self):
        """必須環境変数が未設定の場合に例外が発生することを確認."""
        with patch.dict(os.environ, {}, clear=True):
            with self.assertRaises(UndefinedValueError):
                get_required_env_var("MISSING_VAR")

    def test_optional_env_var_defaults(self):
        """任意環境変数が未設定の場合にデフォルト値が使用されることを確認."""
        with patch.dict(os.environ, {}, clear=True):
            port = get_optional_env_var("SERVER_PORT", 9171, int)
            self.assertEqual(port, 9171)
