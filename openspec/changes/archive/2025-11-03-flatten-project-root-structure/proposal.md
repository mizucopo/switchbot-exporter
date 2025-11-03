## Why
- Source code, lockfiles, and packaging assets currently live under the nested `docker/` directory, forcing contributors to `cd docker` before running every command.
- Documentation and tooling references must constantly reminder contributors about the subdirectory, which sparks confusion when scripts or IDEs expect a conventional project root layout.
- Flattening the tree simplifies local development, CI configuration, and Docker builds by making the repository root the single source of truth.

## What Changes
- Relocate all tracked files and folders under `docker/` to the repository root, excluding artifacts ignored by `.gitignore`.
- Update project metadata, documentation, and tooling references to match the new root-level layout.
- Ensure Docker build commands and uv workflows operate from the repository root without requiring a subdirectory change.

## Impact
- Contributors execute uv, pytest, mypy, and Docker commands directly from the repository root, reducing onboarding friction.
- CI and future automation inherit the simplified path structure, decreasing maintenance overhead.
- Repository history retains clarity by eliminating redundant nesting without affecting runtime behavior of the exporter.
