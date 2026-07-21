"""PR tag check workflowの回帰テスト。"""

import os
import subprocess
from pathlib import Path

WORKFLOW_PATH = Path(".github/workflows/pr-tag-check.yml")


def load_docker_hub_script() -> str:
    """Docker Hub image tag確認stepのshell scriptを読み込む。"""
    workflow = WORKFLOW_PATH.read_text()
    step = workflow.split("      - name: Check Docker Hub image tag\n", 1)[1].split(
        "\n      - name: Build release version availability summary", 1
    )[0]
    run_section = step.split("        run:", 1)[1]
    first_line, *remaining_lines = run_section.splitlines()

    if first_line.strip() != "|":
        return first_line.strip()

    script_lines = [
        line.removeprefix("          ")
        for line in remaining_lines
        if not line or line.startswith("          ")
    ]
    return "\n".join(script_lines)


def write_fake_curl(tmp_path: Path) -> Path:
    """Docker Hub応答を再現するfake curlを作成する。"""
    fake_curl_path = tmp_path / "curl"
    fake_curl_path.write_text(
        """#!/bin/sh
output=/dev/null
url=
while [ "$#" -gt 0 ]; do
  case "$1" in
    --output)
      output="$2"
      shift 2
      ;;
    http*)
      url="$1"
      shift
      ;;
    *)
      shift
      ;;
  esac
done

case "$url" in
  */repositories/*/)
    printf '{"is_private": %s}' "$REPOSITORY_IS_PRIVATE" > "$output"
    printf '%s' "$REPOSITORY_HTTP_STATUS"
    ;;
  */tags/*)
    printf '%s' "$IMAGE_HTTP_STATUS"
    ;;
  *)
    exit 1
    ;;
esac
"""
    )
    fake_curl_path.chmod(0o755)
    return fake_curl_path


def docker_hub_environment(tmp_path: Path) -> dict[str, str]:
    """Docker Hub確認step用の環境変数を作成する。"""
    environment = os.environ.copy()
    environment.update(
        {
            "PATH": f"{tmp_path}:{environment['PATH']}",
            "DOCKERHUB_TOKEN": "",
            "DOCKERHUB_USERNAME": "mizucopo",
            "DOCKERHUB_NAMESPACE": "mizucopo",
            "DOCKERHUB_REPOSITORY": "switchbot-exporter",
            "VERSION": "2.0.2",
            "GITHUB_OUTPUT": str(tmp_path / "github-output.txt"),
            "REPOSITORY_IS_PRIVATE": "false",
            "REPOSITORY_HTTP_STATUS": "200",
            "IMAGE_HTTP_STATUS": "404",
        }
    )
    return environment


def test_public_repository_is_checked_without_credentials(tmp_path: Path) -> None:
    """公開リポジトリが認証情報なしで確認されること。

    Arrange: 公開リポジトリと未使用タグを返すDocker Hub APIが用意される。
    Act: 認証情報なしでDocker Hub image tag確認stepが実行される。
    Assert: image tagが未使用として報告されること。
    """
    # Arrange
    write_fake_curl(tmp_path)
    environment = docker_hub_environment(tmp_path)

    # Act
    subprocess.run(
        ["bash", "-c", load_docker_hub_script()],
        check=True,
        env=environment,
    )

    # Assert
    github_output = Path(environment["GITHUB_OUTPUT"])
    assert github_output.read_text() == "exists=false\n"


def test_unconfirmed_public_repository_fails_closed(tmp_path: Path) -> None:
    """公開状態を確認できないリポジトリがfail-closedにされること。

    Arrange: リポジトリ情報を取得できないDocker Hub APIが用意される。
    Act: 認証情報なしでDocker Hub image tag確認stepが実行される。
    Assert: stepが失敗し、タグの未使用が報告されないこと。
    """
    # Arrange
    write_fake_curl(tmp_path)
    environment = docker_hub_environment(tmp_path)
    environment["REPOSITORY_HTTP_STATUS"] = "404"

    # Act
    result = subprocess.run(
        ["bash", "-c", load_docker_hub_script()],
        check=False,
        env=environment,
    )

    # Assert
    github_output = Path(environment["GITHUB_OUTPUT"])
    assert result.returncode != 0
    assert not github_output.exists()
