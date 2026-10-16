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

- Use function-based Rust integration tests.
- Write test comments and docstrings in Japanese.
- Test observable behavior and results, not implementation details or private methods directly.
- Minimize mocks. Use real instances for domain logic and mock only external boundaries such as DB, API, and SMTP.
- Separate domain logic from IO using Humble Object or Hexagonal patterns where needed for testability.
- Mirror Rust source modules under `tests/`. Use English test function names that describe business requirements.
- Use explicit Arrange, Act, Assert sections in each test:
  - In Japanese test documentation, include `Arrange:`, `Act:`, and `Assert:` lines describing each step.
  - In the function body, use language-appropriate Arrange / Act / Assert comment dividers.
- Describe each test in a Japanese docstring using passive `〜こと` phrasing consistently, including the title and each step.

## Quality and distribution checks

- Run the Rust quality gate with `--locked` for Clippy and tests, then run `cargo build --release --locked`.
- Keep configuration, metrics, API access, HTTP handling, and CLI in separate Rust modules. Use local HTTP mocks and virtual-time checks for TTL and delay.
- Preserve the template-managed release controller. Its regression tests belong in `repo-template`.
- Keep packaged exporter checks in `.github/scripts/release_smoke.py`. It uses only the standard library to verify the CLI, configuration, mock API, TLS/proxy, HTTP, and shutdown on each native distribution target.
