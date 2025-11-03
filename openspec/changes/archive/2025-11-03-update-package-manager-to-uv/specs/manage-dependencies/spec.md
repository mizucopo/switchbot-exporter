## ADDED Requirements
### Requirement: Dependency Management With uv
The project MUST rely on uv for resolving and running Python dependencies throughout local development, CI, and container builds.

#### Scenario: Resolve dependencies
- **WHEN** setting up a development environment or building Docker images
- **THEN** uv installs dependencies using the pinned `uv.lock` (and `uv.dev.lock` for developer tooling)
- **AND** the Poetry CLI is not part of the build chain.

#### Scenario: Run project tooling
- **WHEN** executing lint, type-check, or test commands via documented workflows
- **THEN** the commands run inside an environment provisioned by uv (no `poetry run` wrappers)
- **AND** they do not assume `poetry run` availability.
