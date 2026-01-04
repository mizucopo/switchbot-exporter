"""Pytest configuration for VCR cassettes."""

from pathlib import Path

import pytest
from decouple import AutoConfig


# python-decoupleを使用して.envファイルを読み込む
# プロジェクトルートを検索パスとして設定
search_path = Path(__file__).parent.parent
app_config = AutoConfig(search_path=search_path)


@pytest.fixture
def vcr_config():
    """VCRのデフォルト設定を返すフィクスチャ.

    SwitchBot APIは動的な署名（sign, t, nonce）を使用しているため、
    これらのヘッダーをマッチング対象から除外します.
    """

    def before_record_response(response):
        """レスポンス記録前にヘッダーをフィルタリング."""
        # 認証関連のヘッダーをマスク
        if "request" in response and "headers" in response["request"]:
            headers = response["request"]["headers"]
            for key in ["Authorization", "sign", "t", "nonce"]:
                if key in headers:
                    headers[key] = ["DUMMY_" + key.upper()]
        return response

    return {
        "record_mode": "once",  # 既存のカセットがあれば再生、なければ記録
        "match_on": ["method", "scheme", "host", "port", "path", "query"],
        "decode_compressed_response": True,
        "before_record_response": before_record_response,
    }
