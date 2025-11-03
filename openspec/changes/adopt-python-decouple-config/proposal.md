## Why
- Environment variables are currently fetched with `os.getenv`, which scatters default handling and raises generic errors when required secrets are absent.
- Contributors often rely on local `.env` files; adopting `python-decouple` provides first-class support without duplicating parsing logic.
- A single configuration helper improves type casting and keeps runtime configuration aligned across CLI usage and the Flask exporter.

## What Changes
- Add `python-decouple` to the uv-managed dependency set and document it as the canonical runtime configuration helper.
- Replace direct `os.getenv` calls in the exporter with `python-decouple` helpers, ensuring required secrets surface clear exceptions and optional settings preserve typed defaults.
- Update contributor documentation to explain how `.env` files and the new configuration loader should be used during local development and deployment.

## Impact
- Runtime configuration gains consistent casting and `.env` support, reducing misconfiguration friction for contributors.
- Missing secret errors become more actionable, surfacing precise guidance from `python-decouple` instead of generic `ValueError` messages.
- The project introduces a lightweight dependency, managed through existing uv lockfiles without adding container overhead.
