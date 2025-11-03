<!-- OPENSPEC:START -->
# OpenSpec Instructions

These instructions are for AI assistants working in this project.

Always open `@/openspec/AGENTS.md` when the request:
- Mentions planning or proposals (words like proposal, spec, change, plan)
- Introduces new capabilities, breaking changes, architecture shifts, or big performance/security work
- Sounds ambiguous and you need the authoritative spec before coding

Use `@/openspec/AGENTS.md` to learn:
- How to create and apply change proposals
- Spec format and conventions
- Project structure and guidelines

Keep this managed block so 'openspec update' can refresh the instructions.

<!-- OPENSPEC:END -->

# Repository Guidelines

## Project Structure & Module Organization
- `docker/` holds application code, uv metadata, and Dockerfiles; treat this as the working project root.
- `docker/src/` contains the Flask exporter (`app.py`) and the SwitchBot client (`switchbot.py`). Place new modules alongside these files.
- `docker/tests/` mirrors the source layout for pytest suites; create matching `test_*.py` files when adding features.
- Root-level `docker-compose.yml` retains only production-oriented services; keep it in sync with container distribution needs. `secrets.example` provides the template for local secrets.

## Build, Test, and Development Commands
- `uv venv .venv` (run inside `docker/`) — create or refresh the local virtual environment managed by uv.
- `source .venv/bin/activate` and `uv pip sync uv.lock uv.dev.lock` — install pinned runtime and tooling dependencies.
- `pytest tests`, `mypy --pretty src`, `ruff check src`, `black src tests` — execute the validation suite directly inside the uv environment.
- `act -j build-and-push` — rehearse the GitHub Actions workflow before raising a PR.
- Docker images are the production distribution format: use `docker compose build prod` or `docker build -f Dockerfile.prod` when preparing releases or verifying artifacts; they are not required for day-to-day development.

## Coding Style & Naming Conventions
- Target Python 3.13 with strict typing; keep public functions annotated to satisfy `mypy --pretty ./src`.
- Black enforces an 88-character line length; Ruff handles import ordering and docstring rules—prefer auto-fixes.
- Use `snake_case` for modules and functions, `CamelCase` for classes, and Prometheus-style metric identifiers (e.g., `switchbot_device_battery_percent`).

## Testing Guidelines
- Pytest discovers files named `test_*.py`; mirror source structure (e.g., `tests/test_switchbot.py` for `src/switchbot.py`).
- Stub external HTTP calls in tests to avoid hitting the SwitchBot API and keep runs deterministic.
- Run `pytest` before commits; include coverage notes if regressions in metric exposure are possible.

## Commit & Pull Request Guidelines
- Follow Conventional Commits (`fix:`, `docs:`, `chore:`) as shown in recent history.
- Keep commits scoped and add body text when multiple components change.
- Pull requests should describe the change, list validation commands, and link related issues; attach screenshots for metric or endpoint output changes when helpful.

## Security & Configuration Tips
- Copy `secrets.example` to `.secrets` for local runs and never commit real credentials.
- Required env vars: `SWITCHBOT_API_TOKEN` and `SWITCHBOT_API_SECRET`; optional cache settings (`CACHE_DIR`, `CACHE_EXPIRE_SECOND`, `DELAY_SECOND`) are documented in `README.md`.
- Ensure mounted `CACHE_DIR` paths are writable when running inside Docker to prevent exporter crashes.
