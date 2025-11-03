# Project Context

## Purpose
Expose SwitchBot device state as Prometheus metrics so that home-automation and monitoring stacks can track device health and environment data.

## Tech Stack
- Python 3.13 with Flask for the HTTP exporter.
- uv-managed dependencies (`pyproject.toml` + `uv.lock`) covering `requests`, `prometheus_client`, `inflect/inflection`, `click`, and tooling.
- Docker images as the primary distribution vehicle to keep host environments clean.
- Prometheus server scraping the exporter endpoint.

## Project Conventions

### Code Style
- Enforce Black (88 chars), Ruff, and strict Mypy before merge; keep modules type-annotated.
- Follow repo conventions for module placement (`docker/src` mirrored by `docker/tests`) and snake_case naming.
- Keep Prometheus metric names descriptive and stable (`switchbot_device_*` style).

### Architecture Patterns
- `docker/src/app.py` hosts the Flask exporter; `docker/src/switchbot.py` wraps the SwitchBot public API.
- Ship functionality via Docker images to avoid polluting contributor environments; local work happens inside the dev container.
- Keep caching/delay concerns close to the SwitchBot client so API usage remains centralized.

### Testing Strategy
- Use pytest for unit tests under `docker/tests`, stubbing external SwitchBot calls.
- Run `mypy --pretty ./src`, `ruff check ./src`, and `black ./src ./tests` (after installing dependencies via uv) as part of pre-merge checks.
- End-to-end validation relies on Prometheus scrape tests in staging rather than automated integration tests today.

### Git Workflow
- Conventional Commits (`fix:`, `feat:`, etc.) for history clarity.
- Develop changes on feature branches; avoid direct pushes to default branches.
- Validate with GitHub Actions workflow (`act -j build-and-push`) before opening PRs when practical.

## Domain Context
- Targets SwitchBot device families (bots, sensors, humidifiers) exposed through the official API.
- Metrics prioritize availability (online/offline), battery percentage, and environment readings (temperature, humidity, light).
- Secrets (`SWITCHBOT_API_TOKEN`, `SWITCHBOT_API_SECRET`) remain out of version control; example configs live in `secrets.example`.

## Important Constraints
- SwitchBot API enforces rate limits; default delay (`DELAY_SECOND`) and caching (`CACHE_EXPIRE_SECOND`) settings must remain configurable.
- Exporter requires write access to `CACHE_DIR` when caching is enabled; mounts must be writable inside containers.
- Treat Docker images as the supported deployment path; bare-metal installs are out of scope.

## External Dependencies
- SwitchBot Public API for device state polling.
- Prometheus (or compatible scrapers) consuming the `/metrics` endpoint.
- GitHub Actions workflow for build-and-push automation.
