# dev-environment Specification

## Purpose
Switchbot Exporter のローカル開発を uv 管理の Python 仮想環境に統一し、コンテナに依存しない開発手順と検証方法をドキュメント化する。
## Requirements
### Requirement: uv-Based Local Development Environment
The canonical local development workflow MUST use a uv-managed Python virtual environment instead of a Docker-only setup.

#### Scenario: Set up local environment
- **WHEN** a contributor prepares a fresh workstation
- **THEN** the documented steps instruct them to install uv, create or reuse a uv-managed virtual environment, and sync dependencies from the committed lockfiles without building a Docker image
- **AND** Docker usage is presented as an optional fallback rather than the primary workflow.

#### Scenario: Run project tooling locally
- **WHEN** the contributor executes linting, typing, tests, or CLI utilities during development
- **THEN** the commands run directly inside the uv virtual environment using the same lockfiles as CI
- **AND** no container-only assumptions (volume mounts, container paths) are required for the commands to succeed.
