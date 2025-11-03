## Why
- Poetry currently drives dependency management, but contributors report slow install times and extra runtime dependencies in Docker images.
- uv offers faster lock resolution, universal support for PEP 621 `pyproject.toml`, and can act as both installer and runner, reducing tooling surface area.
- Aligning with uv keeps the exporter lightweight and simplifies container builds by avoiding the Poetry runtime.

## What Changes
- Replace Poetry with uv as the canonical dependency manager for local development, CI, and Docker images.
- Convert the existing `pyproject.toml`/`poetry.lock` workflow to uv-compatible metadata and generate the new `uv.lock`.
- Update Dockerfiles, Make/CI scripts, and documentation (`README.md`, developer notes) to reference uv commands.
- Adjust lint/test invocation guidance to use `uv run` wrappers instead of `poetry run`.

## Impact
- Contributors will install uv (single binary) instead of Poetry; onboarding steps must reflect this.
- Docker build layers shrink because uv does not require a virtualenv during image build, but caching behavior needs validation.
- Downstream scripts that assume `poetry` CLI will fail until updated; coordinate rollout with CI changes to avoid breaking builds.
- uv's resolver may surface dependency version conflicts that Poetry previously tolerated; expect at least one pass through dependency pinning.
