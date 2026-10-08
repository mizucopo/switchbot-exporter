"""リリースコントローラーのDocker Hub照会契約の回帰テスト。"""

import importlib.util
from email.message import Message
from io import BytesIO
from pathlib import Path
from types import ModuleType
from urllib import error

import pytest
from pytest_mock import MockerFixture

RELEASE_PATH = Path(__file__).resolve().parents[1] / ".github/scripts/release.py"
REPOSITORY_URL = "https://hub.docker.com/v2/repositories/mizucopo/switchbot-exporter/"
TAG_URL = REPOSITORY_URL + "tags/2.0.2/"
IMAGE = {
    "name": "exporter",
    "repository": "mizucopo/switchbot-exporter",
    "tag": "2.0.2",
}


@pytest.fixture
def release_controller() -> ModuleType:
    """生成済みのリリースコントローラーが読み込まれること。"""
    spec = importlib.util.spec_from_file_location("release_controller", RELEASE_PATH)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def docker_hub_error(status: int) -> error.HTTPError:
    """Docker HubのHTTP照会失敗が再現されること。"""
    return error.HTTPError(REPOSITORY_URL, status, "lookup failed", Message(), None)


def test_public_repository_is_checked_without_credentials(
    release_controller: ModuleType,
    monkeypatch: pytest.MonkeyPatch,
    mocker: MockerFixture,
) -> None:
    """公開リポジトリの未使用タグが認証情報なしで確認されること。

    Arrange: 公開リポジトリと未使用タグを返すDocker Hub APIが用意されること。
    Act: 認証情報なしで公開APIのimage digest照会が実行されること。
    Assert: 匿名の照会によりタグが未使用として報告されること。
    """
    # Arrange
    monkeypatch.delenv("DOCKERHUB_USERNAME", raising=False)
    monkeypatch.delenv("DOCKERHUB_TOKEN", raising=False)
    lookup = mocker.patch(
        "urllib.request.urlopen",
        side_effect=[BytesIO(b"{}"), docker_hub_error(404)],
    )

    # Act
    digest = release_controller.image_digest(IMAGE)

    # Assert
    assert digest is None
    requests = [call.args[0] for call in lookup.call_args_list]
    assert [request.full_url for request in requests] == [REPOSITORY_URL, TAG_URL]
    assert all(request.get_header("Authorization") is None for request in requests)


@pytest.mark.parametrize(
    ("credentials", "responses"),
    [
        pytest.param(False, (docker_hub_error(404),), id="unknown-repository"),
        pytest.param(False, (docker_hub_error(403),), id="repository-forbidden"),
        pytest.param(False, (docker_hub_error(500),), id="repository-unavailable"),
        pytest.param(
            False, (error.URLError("network unavailable"),), id="network-error"
        ),
        pytest.param(False, (b"invalid json",), id="malformed-response"),
        pytest.param(False, (b"{}", docker_hub_error(403)), id="tag-forbidden"),
        pytest.param(False, (b"{}", docker_hub_error(500)), id="tag-unavailable"),
        pytest.param(False, (b"{}", b'{"digest":"unknown"}'), id="unverifiable-digest"),
        pytest.param(True, (docker_hub_error(404),), id="authentication-not-found"),
        pytest.param(True, (docker_hub_error(403),), id="authentication-forbidden"),
        pytest.param(True, (b"{}",), id="missing-access-token"),
    ],
)
def test_registry_authentication_or_lookup_failure_fails_closed(
    release_controller: ModuleType,
    monkeypatch: pytest.MonkeyPatch,
    mocker: MockerFixture,
    credentials: bool,
    responses: tuple[bytes | error.HTTPError | error.URLError, ...],
) -> None:
    """認証または照会に失敗した場合にfail-closedとなること。

    Arrange: 認証またはimage digest照会に失敗するDocker Hub APIが用意されること。
    Act: 公開APIのimage digest照会が実行されること。
    Assert: 照会が失敗し、タグの未使用が報告されないこと。
    """
    # Arrange
    monkeypatch.delenv("DOCKERHUB_USERNAME", raising=False)
    monkeypatch.delenv("DOCKERHUB_TOKEN", raising=False)
    if credentials:
        monkeypatch.setenv("DOCKERHUB_USERNAME", "test-user")
        monkeypatch.setenv("DOCKERHUB_TOKEN", "test-token")
    mocker.patch(
        "urllib.request.urlopen",
        side_effect=[
            BytesIO(response) if isinstance(response, bytes) else response
            for response in responses
        ],
    )

    # Act
    with pytest.raises(release_controller.PreparationError) as failure:
        release_controller.image_digest(IMAGE)

    # Assert
    assert str(failure.value).startswith(
        (
            "Registry lookup failed",
            "Registry returned no verifiable digest",
            "Docker Hub did not return an access token",
        )
    )
