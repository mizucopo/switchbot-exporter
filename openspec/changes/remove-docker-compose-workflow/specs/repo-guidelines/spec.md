## MODIFIED Requirements
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
