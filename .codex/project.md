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

- Keep one class per file and one test file per class.
- Keep `__init__.py` files empty.
- Create a new file when adding a new class; name its test file `test_<filename>.py`.
- Place imports at the top of the file.
- Fix lint errors in code; never relax configuration or modify `pyproject.toml` to fix lint errors.

## Tests

- Use function-based pytest tests, not test classes.
- Write test comments and docstrings in Japanese.
- Test observable behavior and results, not implementation details or private methods directly.
- Minimize mocks. Use real instances for domain logic and mock only external boundaries such as DB, API, and SMTP.
- Separate domain logic from IO using Humble Object or Hexagonal patterns where needed for testability.
- Mirror the source module structure under `tests/` and use English names for test functions that describe business requirements.
- Use explicit Arrange, Act, Assert sections in each test:
  - In the Japanese docstring, include `Arrange:`, `Act:`, and `Assert:` lines describing each step.
  - In the function body, use `# Arrange`, `# Act`, and `# Assert` section dividers.
- Describe each test in a Japanese docstring using passive `〜こと` phrasing consistently, including the title and each step.

## Temporary template exception

- The flat Python application imports modules directly from `src/`. Omit the template-generated `src/__init__.py` and `tests/test_import.py` until `mizucopo/repo-template#105` supplies a version that passes `uv run task check` for this layout. Refs mizucopo/repo-template#105.
