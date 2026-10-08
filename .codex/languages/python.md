# Python guidance

Python is used only by release-controller regression tooling. Run its quality gate from the repository root in addition to the Rust gate:

```bash
uv run task check
```

Apply automatic Ruff fixes separately with `uv run task fix`, then rerun the non-mutating check.
