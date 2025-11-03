"""
設定管理モジュールのテスト.
"""

import os
import tempfile
from pathlib import Path
from unittest.mock import patch

import pytest
from decouple import UndefinedValueError

from config import get_optional_env_var, get_required_env_var


class TestConfig:
    """設定管理モジュールのテストクラス."""

    def test_get_required_env_var_success(self) -> None:
        """必須環境変数が正常に取得できることを確認."""
        with patch.dict(os.environ, {"TEST_VAR": "test_value"}):
            assert get_required_env_var("TEST_VAR") == "test_value"

    def test_get_required_env_var_failure(self) -> None:
        """必須環境変数が未設定の場合に例外が発生することを確認."""
        with pytest.raises(UndefinedValueError) as exc_info:
            get_required_env_var("NON_EXISTENT_VAR")
        assert "必須の環境変数 'NON_EXISTENT_VAR' が設定されていません。" in str(
            exc_info.value
        )

    def test_get_optional_env_var_with_default(self) -> None:
        """任意環境変数が未設定の場合にデフォルト値が返されることを確認."""
        assert (
            get_optional_env_var("NON_EXISTENT_VAR", "default_value") == "default_value"
        )

    def test_get_optional_env_var_with_env(self) -> None:
        """任意環境変数が設定されている場合にその値が返されることを確認."""
        with patch.dict(os.environ, {"TEST_VAR": "test_value"}):
            assert get_optional_env_var("TEST_VAR", "default_value") == "test_value"

    def test_get_optional_env_var_with_cast(self) -> None:
        """任意環境変数が型変換されることを確認."""
        with patch.dict(os.environ, {"TEST_VAR": "123"}):
            assert get_optional_env_var("TEST_VAR", "0", int) == 123

    def test_get_optional_env_var_with_cast_default(self) -> None:
        """任意環境変数が未設定の場合にデフォルト値が型変換されずに返されることを確認."""
        assert get_optional_env_var("NON_EXISTENT_VAR", "456", int) == 456

    def test_env_file_loading(self) -> None:
        """.env ファイルから環境変数が読み込まれることを確認."""
        with tempfile.TemporaryDirectory() as temp_dir:
            env_file = Path(temp_dir) / ".env"
            env_file.write_text("TEST_VAR=env_file_value\n")

            # 新しい AutoConfig インスタンスを作成して一時ディレクトリを指定
            from decouple import AutoConfig

            temp_config = AutoConfig(search_path=temp_dir)

            # 一時ディレクトリをプロジェクトルートとして設定
            with patch("config.app_config", temp_config):
                assert get_optional_env_var("TEST_VAR", "default") == "env_file_value"
