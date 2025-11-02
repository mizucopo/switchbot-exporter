# Repository Guidelines

## Project Structure & Module Organization
- `docker/` holds application code, Poetry config, and Dockerfiles; treat this as the working project root.
- `docker/src/` contains the Flask exporter (`app.py`) and the SwitchBot client (`switchbot.py`). Place new modules alongside these files.
- `docker/tests/` mirrors the source layout for pytest suites; create matching `test_*.py` files when adding features.
- Root-level `docker-compose.yml` and `secrets.example` document how to boot the containerized dev environment.

## Build, Test, and Development Commands
- `docker compose build` — build the development image from `Dockerfile.dev`.
- `docker run --rm -it -v $(pwd)/docker:/app ... mizucopo/switchbot-exporter:develop /bin/sh` — start an interactive shell inside the dev container (copy full command from `README.md`).
- `poetry install` — install dependencies inside the container or a local virtualenv.
- `poetry run pytest` — execute the unit suite under `docker/tests`.
- `poetry run ruff check ./src` and `poetry run black ./src ./tests` — lint and format Python modules.
- `act -j build-and-push` — rehearse the GitHub Actions workflow before raising a PR.

## Coding Style & Naming Conventions
- Target Python 3.13 with strict typing; keep public functions annotated to satisfy `poetry run mypy --pretty ./src`.
- Black enforces an 88-character line length; Ruff handles import ordering and docstring rules—prefer auto-fixes.
- Use `snake_case` for modules and functions, `CamelCase` for classes, and Prometheus-style metric identifiers (e.g., `switchbot_device_battery_percent`).

## Testing Guidelines
- Pytest discovers files named `test_*.py`; mirror source structure (e.g., `tests/test_switchbot.py` for `src/switchbot.py`).
- Stub external HTTP calls in tests to avoid hitting the SwitchBot API and keep runs deterministic.
- Run `poetry run pytest` before commits; include coverage notes if regressions in metric exposure are possible.

## Commit & Pull Request Guidelines
- Follow Conventional Commits (`fix:`, `docs:`, `chore:`) as shown in recent history.
- Keep commits scoped and add body text when multiple components change.
- Pull requests should describe the change, list validation commands, and link related issues; attach screenshots for metric or endpoint output changes when helpful.

## Security & Configuration Tips
- Copy `secrets.example` to `.secrets` for local runs and never commit real credentials.
- Required env vars: `SWITCHBOT_API_TOKEN` and `SWITCHBOT_API_SECRET`; optional cache settings (`CACHE_DIR`, `CACHE_EXPIRE_SECOND`, `DELAY_SECOND`) are documented in `README.md`.
- Ensure mounted `CACHE_DIR` paths are writable when running inside Docker to prevent exporter crashes.
