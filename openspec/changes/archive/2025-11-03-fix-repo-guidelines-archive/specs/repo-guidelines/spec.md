## ADDED Requirements
### Requirement: Agent Instructions Structure
`openspec/AGENTS.md` MUST keep the managed instructions block intact and place the localized quick reference immediately after it.

#### Scenario: Preserve managed block
- **WHEN** contributors edit `openspec/AGENTS.md`
- **THEN** the file retains the `<!-- OPENSPEC:START -->` と `<!-- OPENSPEC:END -->` マーカーを削除せずに保持する
- **AND** the managed block content remains unchanged except for updates made by the tooling
- **AND** the localized quick reference continues right after the managed block.

#### Scenario: Highlight unified localization
- **WHEN** the document explains where to find Japanese guidance
- **THEN** it states that the localized instructions are part of `openspec/AGENTS.md` itself
- **AND** it does not point contributors to a separate `.ja.md` file.
