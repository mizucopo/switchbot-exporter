"""Pytest configuration for VCR cassettes."""

from pathlib import Path
from typing import Any

import pytest
from decouple import AutoConfig


# python-decoupleを使用して.envファイルを読み込む
# プロジェクトルートを検索パスとして設定
search_path = Path(__file__).parent.parent
app_config = AutoConfig(search_path=search_path)


@pytest.fixture
def vcr_config() -> dict[str, Any]:
    """VCRのデフォルト設定を返すフィクスチャ.

    SwitchBot APIは動的な署名（sign, t, nonce）を使用しているため、
    これらのヘッダーをマッチング対象から除外します.
    """

    def before_record_request(request: Any) -> Any:
        """カセット記録前にリクエストヘッダーの認証情報を削除."""
        # リクエストヘッダーから認証情報を削除
        if hasattr(request, "headers"):
            request.headers.pop("Authorization", None)
            request.headers.pop("sign", None)
            request.headers.pop("t", None)
            request.headers.pop("nonce", None)
        return request

    def before_record_response(response: dict[str, Any]) -> dict[str, Any]:
        """カセット記録前にレスポンスヘッダーの動的な値を削除."""
        # レスポンスヘッダーから動的な値を削除
        if "headers" in response:
            headers = response["headers"]
            headers.pop("Content-Length", None)
            headers.pop("Date", None)
            headers.pop("switchbot-request-id", None)

        return response

    return {
        "record_mode": "once",  # 既存のカセットがあれば再生、なければ記録
        "match_on": ["method", "scheme", "host", "port", "path", "query"],
        "decode_compressed_response": True,
        "before_record": before_record_request,
        "before_record_response": before_record_response,
    }
