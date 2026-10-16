"""リリースコントローラーのDocker Hub照会契約の回帰テスト。"""

import importlib.util
import json
import tomllib
from email.message import Message
from io import BytesIO
from pathlib import Path
from types import ModuleType
from urllib import error
from urllib.response import addinfourl

import pytest
from pytest_mock import MockerFixture

RELEASE_PATH = Path(__file__).resolve().parents[1] / ".github/scripts/release.py"
REPOSITORY_URL = "https://hub.docker.com/v2/repositories/mizucopo/switchbot-exporter/"
REGISTRY_TOKEN_URL = (
    "https://auth.docker.io/token?service=registry.docker.io"
    "&scope=repository%3Amizucopo%2Fswitchbot-exporter%3Apull"
)
MANIFEST_URL = (
    "https://registry-1.docker.io/v2/mizucopo/switchbot-exporter/manifests/2.0.2"
)
IMAGE = {
    "name": "exporter",
    "repository": "mizucopo/switchbot-exporter",
    "tag": "2.0.2",
}


def test_cargo_release_sources_agree_with_manifest_version(
    release_controller: ModuleType,
) -> None:
    """Rust移行後の採番対象から同じversionが読まれること。

    Arrange: 実際のrelease宣言とCargo manifest/lockが用意されること。
    Act: 宣言されたversion sourceとlockがcontrollerで読み込まれること。
    Assert: manifestに宣言されたversionが両方から取得されること。
    """
    # Arrange
    policy = json.loads(Path(".github/release.json").read_text())
    specs = policy["version"]["sources"] + policy["version"]["locks"]

    # Act
    versions = [
        release_controller.read_field(Path(spec["path"]).read_bytes(), spec)
        for spec in specs
    ]

    # Assert
    assert {spec["path"] for spec in specs} == {"Cargo.toml", "Cargo.lock"}
    expected = tomllib.loads(Path("Cargo.toml").read_text())["package"]["version"]
    assert versions == [expected, expected]


def test_cargo_numbering_changes_only_root_package_version(
    release_controller: ModuleType,
) -> None:
    """Cargo採番でroot package以外の設定と依存versionが維持されること。

    Arrange: 実際のCargo採番宣言と元blobが用意されること。
    Act: ファイルを変更せずメモリ内で異なるversionへ更新されること。
    Assert: root versionだけが変更され、同versionではbyte一致となること。
    """
    # Arrange
    policy = json.loads(Path(".github/release.json").read_text())
    specs = policy["version"]["sources"] + policy["version"]["locks"]
    for spec in specs:
        original = Path(spec["path"]).read_bytes()
        old = release_controller.read_field(original, spec)

        # Act
        target = old + "-test.1"
        updated = release_controller.write_field(original, spec, target)
        before = tomllib.loads(original.decode())
        after = tomllib.loads(updated.decode())

        # Assert
        assert release_controller.read_field(updated, spec) == target
        assert release_controller.write_field(original, spec, old) == original
        if spec["format"] == "toml-lock":
            package = next(
                item
                for item in after["package"]
                if item["name"] == "switchbot-exporter"
            )
            package["version"] = old
        else:
            after["package"]["version"] = old
        assert after == before


@pytest.fixture
def release_controller() -> ModuleType:
    """生成済みのリリースコントローラーが読み込まれること。"""
    spec = importlib.util.spec_from_file_location("release_controller", RELEASE_PATH)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def registry_error(status: int, url: str = REPOSITORY_URL) -> error.HTTPError:
    """公開先のHTTP照会失敗が再現されること。"""
    return error.HTTPError(url, status, "lookup failed", Message(), None)


def manifest_response(digest: str) -> addinfourl:
    """Registry manifestのdigestを含むHEAD応答が用意されること。"""
    headers = Message()
    headers["Docker-Content-Digest"] = digest
    return addinfourl(BytesIO(b""), headers, MANIFEST_URL, code=200)


def test_public_repository_is_checked_without_credentials(
    release_controller: ModuleType,
    monkeypatch: pytest.MonkeyPatch,
    mocker: MockerFixture,
) -> None:
    """公開リポジトリの未使用タグが認証情報なしで確認されること。

    Arrange: 公開リポジトリと匿名token、未使用タグのRegistry応答が用意されること。
    Act: 利用者の認証情報なしでimage digest照会が実行されること。
    Assert: 匿名tokenのmanifest HEAD照会によりタグが未使用と報告されること。
    """
    # Arrange
    monkeypatch.delenv("DOCKERHUB_USERNAME", raising=False)
    monkeypatch.delenv("DOCKERHUB_TOKEN", raising=False)
    lookup = mocker.patch(
        "urllib.request.OpenerDirector.open",
        side_effect=[
            BytesIO(b"{}"),
            BytesIO(b'{"token":"anonymous-registry-token"}'),
            registry_error(404, MANIFEST_URL),
        ],
    )

    # Act
    digest = release_controller.image_digest(IMAGE)

    # Assert
    assert digest is None
    requests = [call.args[0] for call in lookup.call_args_list]
    assert [request.full_url for request in requests] == [
        REPOSITORY_URL,
        REGISTRY_TOKEN_URL,
        MANIFEST_URL,
    ]
    assert [request.get_method() for request in requests] == ["GET", "GET", "HEAD"]
    assert all(request.get_header("Authorization") is None for request in requests[:2])
    assert requests[2].get_header("Authorization") == "Bearer anonymous-registry-token"


@pytest.mark.parametrize(
    ("credentials", "responses"),
    [
        pytest.param(False, (registry_error(404),), id="unknown-repository"),
        pytest.param(False, (registry_error(403),), id="repository-forbidden"),
        pytest.param(False, (registry_error(500),), id="repository-unavailable"),
        pytest.param(
            False, (error.URLError("network unavailable"),), id="network-error"
        ),
        pytest.param(False, (b"invalid json",), id="malformed-response"),
        pytest.param(
            False,
            (b"{}", b'{"token":"registry-token"}', registry_error(403, MANIFEST_URL)),
            id="tag-forbidden",
        ),
        pytest.param(
            False,
            (b"{}", b'{"token":"registry-token"}', registry_error(500, MANIFEST_URL)),
            id="tag-unavailable",
        ),
        pytest.param(
            False,
            (b"{}", b'{"token":"registry-token"}', manifest_response("unknown")),
            id="unverifiable-digest",
        ),
        pytest.param(True, (registry_error(404),), id="authentication-not-found"),
        pytest.param(True, (registry_error(403),), id="authentication-forbidden"),
        pytest.param(True, (b"{}",), id="missing-access-token"),
    ],
)
def test_registry_authentication_or_lookup_failure_fails_closed(
    release_controller: ModuleType,
    monkeypatch: pytest.MonkeyPatch,
    mocker: MockerFixture,
    credentials: bool,
    responses: tuple[bytes | addinfourl | error.HTTPError | error.URLError, ...],
) -> None:
    """認証または照会に失敗した場合にfail-closedとなること。

    Arrange: 認証またはmanifest HEAD照会に失敗する公開先の応答が用意されること。
    Act: Hubの可視性確認とRegistryのimage digest照会が実行されること。
    Assert: 照会が失敗し、タグの未使用が報告されないこと。
    """
    # Arrange
    monkeypatch.delenv("DOCKERHUB_USERNAME", raising=False)
    monkeypatch.delenv("DOCKERHUB_TOKEN", raising=False)
    if credentials:
        monkeypatch.setenv("DOCKERHUB_USERNAME", "test-user")
        monkeypatch.setenv("DOCKERHUB_TOKEN", "test-token")
    mocker.patch(
        "urllib.request.OpenerDirector.open",
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
