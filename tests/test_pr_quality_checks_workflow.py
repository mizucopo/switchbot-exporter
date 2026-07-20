"""PR quality checks workflowの回帰テスト。"""

import os
import subprocess
from pathlib import Path

import pytest

WORKFLOW_PATH = Path(".github/workflows/pr-quality-checks.yml")


def load_pytest_script() -> str:
    """Run pytest stepのshell scriptを読み込む。"""
    workflow = WORKFLOW_PATH.read_text()
    step = workflow.split("      - name: Run pytest\n", 1)[1].split(
        "\n      - name: Run mypy", 1
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


@pytest.mark.parametrize(
    ("api_token", "api_secret", "expected_arguments"),
    [
        ("", "", ["run", "pytest", "--tb=short", "-v", "-m", "not vcr"]),
        ("token", "secret", ["run", "pytest", "--tb=short", "-v"]),
    ],
)
def test_pytest_command_depends_on_switchbot_credentials(
    tmp_path: Path,
    api_token: str,
    api_secret: str,
    expected_arguments: list[str],
) -> None:
    """認証情報の有無に応じたpytest対象が選択されること。

    Arrange: pytest stepと引数を記録するfake uvが用意される。
    Act: SwitchBot認証情報を指定してpytest stepが実行される。
    Assert: secretなしではVCRテストが除外され、secretありでは全件実行される。
    """
    # Arrange
    arguments_path = tmp_path / "uv-arguments.txt"
    fake_uv_path = tmp_path / "uv"
    fake_uv_path.write_text('#!/bin/sh\nprintf \'%s\\n\' "$@" > "$UV_ARGS_FILE"\n')
    fake_uv_path.chmod(0o755)
    environment = os.environ.copy()
    environment.update(
        {
            "PATH": f"{tmp_path}:{environment['PATH']}",
            "SWITCHBOT_API_TOKEN": api_token,
            "SWITCHBOT_API_SECRET": api_secret,
            "UV_ARGS_FILE": str(arguments_path),
        }
    )

    # Act
    subprocess.run(
        ["bash", "-c", load_pytest_script()],
        check=True,
        env=environment,
    )

    # Assert
    assert arguments_path.read_text().splitlines() == expected_arguments
