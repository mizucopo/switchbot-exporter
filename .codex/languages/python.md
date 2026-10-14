# Python guidance

Python is development and CI tooling for the shared release controller, native-distribution smoke checks, and controller regression tests. It is not the exporter runtime. See [the tooling inventory](../../docs/python-tooling.md) before removing files or dependencies.

Run the Python tooling quality gate from the repository root in addition to the Rust gate:

```bash
uv sync --locked
uv run task check
```

Apply automatic Ruff fixes separately with `uv run task fix`, then rerun the non-mutating check.

The gate lints and type-checks the Python test helpers and executes controller regression tests. It does not run the native-distribution smoke checks; those run separately in native PR and Release CI using the packaged binary and local TLS/proxy mocks.
