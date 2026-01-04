"""Pytest configuration for VCR cassettes."""

from pathlib import Path

import pytest
from decouple import AutoConfig


# python-decoupleを使用して.envファイルを読み込む
# プロジェクトルートを検索パスとして設定
search_path = Path(__file__).parent.parent
app_config = AutoConfig(search_path=search_path)


@pytest.fixture(scope="module")
def vcr_config() -> dict[str, str]:
    """VCR configuration for recording HTTP interactions."""
    return {
        "record_mode": "once",  # Record on first run, replay on subsequent runs
    }
