## Why
- `openspec validate --strict` failed because the `manage-dependencies` specification did not use required RFC 2119 keywords.
- Contributors could not rely on `openspec validate` to gate spec changes, which blocks the documented workflow.

## What Changes
- Introduce a normative `MUST` statement in the `manage-dependencies` requirement so the spec satisfies the validation rules.
- Capture the wording update in the capability spec delta for traceability.

## Impact
- Validation succeeds again with `openspec validate --all --strict`.
- No runtime behavior changes; documentation and tooling remain aligned.
