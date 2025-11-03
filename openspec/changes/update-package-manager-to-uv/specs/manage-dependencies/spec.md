## ADDED Requirements
### Requirement: Dependency Management With uv
The project MUST rely on uv for resolving and running Python dependencies throughout local development, CI, and container builds.

#### Scenario: Resolve dependencies
- **WHEN** setting up a development environment or building Docker images
- **THEN** uv installs dependencies from `pyproject.toml` and `uv.lock`
- **AND** the Poetry CLI is not part of the build chain.

#### Scenario: Run project tooling
- **WHEN** executing lint, type-check, or test commands via documented workflows
- **THEN** the commands use `uv run` (or equivalent uv invocation)
- **AND** they do not assume `poetry run` availability.
