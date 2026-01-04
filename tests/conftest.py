"""Pytest configuration for VCR cassettes."""

from pathlib import Path

from decouple import AutoConfig


# python-decoupleを使用して.envファイルを読み込む
# プロジェクトルートを検索パスとして設定
search_path = Path(__file__).parent.parent
app_config = AutoConfig(search_path=search_path)
