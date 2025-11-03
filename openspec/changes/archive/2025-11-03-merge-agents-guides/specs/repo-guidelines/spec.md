## MODIFIED Requirements
### Requirement: Agent Instructions Structure
The project SHALL maintain a single source of truth for agent instructions in `openspec/AGENTS.md`.

#### Scenario: Documentation consolidation
- **WHEN** agent instructions are updated
- **THEN** changes are made only to `openspec/AGENTS.md`
- **AND** `openspec/AGENTS.ja.md` is not referenced

#### Scenario: Reference verification
- **WHEN** documentation references are checked
- **THEN** all references point to `openspec/AGENTS.md`
- **AND** no references exist for `openspec/AGENTS.ja.md`
