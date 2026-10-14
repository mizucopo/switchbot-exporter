# Switchbot Exporter project guidance

## Issue-first branch workflow

- Create a GitHub Issue describing the work before making implementation changes.
- Work on a non-`main` branch associated with that Issue.

## Documentation

- Update related documentation when code changes affect users.
- Document usage for new features in README.
- Update relevant docs when interfaces change.
- Split large docs into separate files in `docs/` and link them from README.

## File operations

- Move files with `git mv <old-path> <new-path>`.
- Delete files with `git rm <path>`.

## Code organization

- The application runtime is Rust. Keep modules separated by responsibility and integration tests named after the affected module.
- Place imports at the top of the file.
- Fix lint errors in code; never relax quality configuration to make a failing check pass.

## Tests

- Use function-based Rust integration tests and pytest tests for release-controller tooling.
- Write test comments and docstrings in Japanese.
- Test observable behavior and results, not implementation details or private methods directly.
- Minimize mocks. Use real instances for domain logic and mock only external boundaries such as DB, API, and SMTP.
- Separate domain logic from IO using Humble Object or Hexagonal patterns where needed for testability.
- Mirror the source module structure under `tests/` and use English names for test functions that describe business requirements.
- Use explicit Arrange, Act, Assert sections in each test:
  - In Japanese test documentation, include `Arrange:`, `Act:`, and `Assert:` lines describing each step.
  - In the function body, use language-appropriate Arrange / Act / Assert comment dividers.
- Describe each test in a Japanese docstring using passive `〜こと` phrasing consistently, including the title and each step.

## Python tooling

- Python supports the shared release controller, native-distribution smoke checks, and release-controller regression tests. uv manages development dependencies for the regression tests, lint, and type checks. Neither Python nor uv is required by the distributed executable or Docker runtime image.
- Follow [the Python tooling inventory](../docs/python-tooling.md) for callers, retained files, and migration conditions. Run `uv sync --locked` and `uv run task check` in addition to the Rust gate; native-distribution smoke checks run separately on each target runner.
- Preserve the pinned shared release controller. Rust migration changes its manifest declarations and build validation; unrelated template fixes require a separate Issue/PR.
