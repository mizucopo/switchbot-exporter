"""
設定管理モジュール.

python-decouple を使用して環境変数と .env ファイルから設定を読み込みます。
"""

from pathlib import Path
from typing import Type

from decouple import AutoConfig, UndefinedValueError

# プロジェクトルート（docker/ の親ディレクトリ）を検索パスとして設定
search_path = Path(__file__).parent.parent
app_config = AutoConfig(search_path=search_path)


def get_required_env_var(name: str) -> str:
    """
    必須の環境変数を取得します.

    Args:
        name (str): 環境変数名。

    Returns:
        str: 環境変数の値。

    Raises:
        UndefinedValueError: 必須の環境変数が設定されていない場合

    """
    try:
        value = app_config(name)
        # mypy のために型を明示的に指定
        return str(value)
    except UndefinedValueError as e:
        raise UndefinedValueError(
            f"必須の環境変数 '{name}' が設定されていません。"
        ) from e


def get_optional_env_var(
    name: str,
    default: str | int | float | bool,
    cast: Type[str | int | float | bool] = str,
) -> str | int | float | bool:
    """
    任意の環境変数を取得します.

    Args:
        name (str): 環境変数名。
        default (str | int | float | bool): デフォルト値。
        cast (type): 型変換用の関数。

    Returns:
        str | int | float | bool: 環境変数の値、またはデフォルト値。

    """
    result = app_config(name, default=default, cast=cast)
    # mypy のために型を明示的に指定
    return result  # type: ignore
