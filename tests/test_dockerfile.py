"""Docker image build契約のテスト。"""

from pathlib import Path


def test_docker_build_installs_dependencies_without_installing_project() -> None:
    """flat配置のapplication moduleがPython packageとしてinstallされないこと。

    Arrange: Dockerfileのpathが用意される。
    Act: Dockerfileの内容が読み込まれる。
    Assert: root projectをinstallしないuv sync commandが指定されること。
    """
    # Arrange
    dockerfile_path = Path("Dockerfile")

    # Act
    dockerfile = dockerfile_path.read_text()

    # Assert
    assert "uv sync --frozen --no-install-project" in dockerfile
