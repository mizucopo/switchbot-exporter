# repo-guidelines Specification

## Purpose
TBD - created by archiving change localize-agents-instructions. Update Purpose after archive.
## Requirements
### Requirement: Repository Guidelines Localization
The repository guidelines in `AGENTS.md` MUST provide a Japanese translation alongside the managed OpenSpec instructions block.

#### Scenario: Contributor reads AGENTS instructions
- **WHEN** a contributor opens `AGENTS.md` to understand repository workflows
- **THEN** the document is written in Japanese except for the managed block maintained by tooling
- **AND** the guidance covers project structure, development workflow, coding standards, testing, pull request practices, and configuration notes in Japanese.

#### Scenario: Document optional container validation
- **WHEN** the guidelines explain how to validate the Docker image distribution locally
- **THEN** they direct contributors to use plain `docker build` and `docker run` commands instead of Docker Compose services
- **AND** the instructions emphasize that the project maintains a single-image workflow without Compose orchestration.

#### Scenario: Describe flattened project layout
- **WHEN** the documentation explains where source code, tests, and lockfiles are located
- **THEN** it states that `src/`, `tests/`, `pyproject.toml`, and uv lockfiles reside at the repository root rather than inside a nested `docker/` directory
- **AND** setup commands instruct contributors to run tooling from the repository root without requiring `cd docker`.

### Requirement: Agent Instructions Structure
`openspec/AGENTS.md` MUST keep the managed instructions block intact and place the localized quick reference immediately after it.

#### Scenario: Preserve managed block
- **WHEN** contributors edit `openspec/AGENTS.md`
- **THEN** the file retains the `<!-- OPENSPEC:START -->` and `<!-- OPENSPEC:END -->` markers without removal
- **AND** the managed block content remains unchanged except for updates applied by tooling
- **AND** the localized quick reference follows immediately after the managed block.

#### Scenario: Highlight unified localization
- **WHEN** the document explains where to find localized guidance
- **THEN** it states that the Japanese quick reference lives inside `openspec/AGENTS.md`
- **AND** it does not reference a separate `.ja.md` file.

