## MODIFIED Requirements
### Requirement: Agent Instructions Structure
`openspec/AGENTS.md` MUST keep the managed instructions block intact and place the localized quick reference immediately after it.

#### Scenario: Preserve managed block
- **WHEN** contributors edit `openspec/AGENTS.md`
- **THEN** the file retains the `<!-- OPENSPEC:START -->` and `<!-- OPENSPEC:END -->` markers without removal
- **AND** the managed block content remains unchanged except for updates applied by tooling
- **AND** the localized quick reference follows immediately after the managed block.

#### Scenario: Highlight unified localization
- **WHEN** the document explains where to find localized guidance
- **THEN** it states that the Japanese quick reference lives inside `openspec/AGENTS.md`
- **AND** it does not reference a separate `.ja.md` file.
