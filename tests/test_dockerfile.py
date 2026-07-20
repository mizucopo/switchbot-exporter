"""Docker image build contract tests."""

from pathlib import Path


def test_docker_build_installs_dependencies_without_installing_project() -> None:
    """Flattened application modules are not installed as a Python package."""
    dockerfile = Path("Dockerfile").read_text()

    assert "uv sync --frozen --no-install-project" in dockerfile
