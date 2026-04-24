## ADDED Requirements

### Requirement: AGENTS.md instructs agents to update README.md on relevant changes

The project's `openspec/AGENTS.md` SHALL contain a rule stating that any change to source files (`src/**`) that affects public-facing behavior, CLI interface, environment variables, output format, or architecture MUST be accompanied by an update to `README.md`. Build command and release packaging changes MUST also be accompanied by an update to `README.md`.

#### Scenario: Agent working on a source change is reminded to update README

- **WHEN** an AI agent reads AGENTS.md before modifying source files
- **THEN** it SHALL find a clear rule specifying which types of changes require README.md to be updated

#### Scenario: Agent changing release packaging is reminded to update README

- **WHEN** an AI agent reads AGENTS.md before changing build scripts or release packaging
- **THEN** it SHALL find a clear rule requiring README.md to document the new build or release command

### Requirement: AGENTS.md instructs agents to update FUNCTIONALITY.md on any source change

The project's `openspec/AGENTS.md` SHALL contain a rule stating that any change to any file under `src/` MUST be accompanied by an update to `FUNCTIONALITY.md` to reflect the changed types, functions, algorithms, or behavior. Build scripts, release profile settings, and release packaging changes MUST also be accompanied by an update to `FUNCTIONALITY.md`.

#### Scenario: Agent adding a new function updates FUNCTIONALITY.md

- **WHEN** an AI agent reads AGENTS.md before adding a new function
- **THEN** it SHALL find an explicit rule requiring FUNCTIONALITY.md to be updated with the new function's signature and description

#### Scenario: Agent changing release profile updates FUNCTIONALITY.md

- **WHEN** an AI agent reads AGENTS.md before changing Cargo release profile settings
- **THEN** it SHALL find an explicit rule requiring FUNCTIONALITY.md to describe the release profile or packaging behavior

### Requirement: AGENTS.md specifies what FUNCTIONALITY.md must contain

The project's `openspec/AGENTS.md` SHALL describe the required structure of FUNCTIONALITY.md so agents know exactly what to update: Data Flow section, one section per module, types, function signatures, algorithms, target software table, config section, and build/release packaging section when non-default build behavior exists.

#### Scenario: Agent knows FUNCTIONALITY.md structure without reading the file first

- **WHEN** an AI agent reads AGENTS.md
- **THEN** it SHALL find the documented required sections of FUNCTIONALITY.md and know which section to update for a given type of change

### Requirement: AGENTS.md specifies what README.md must contain

The project's `openspec/AGENTS.md` SHALL describe the required structure of README.md so agents know what sections exist and which to update for a given change type.

#### Scenario: Agent updating CLI flags knows to update the Usage section

- **WHEN** an AI agent reads AGENTS.md
- **THEN** it SHALL find that CLI-flag changes require updating the "Usage" section of README.md

### Requirement: A Copilot instructions file mirrors the documentation maintenance rules

A `.copilot/instructions/documentation.instructions.md` file (or equivalent VS Code Copilot-compatible location) SHALL exist and contain the same documentation maintenance rules so that VS Code GitHub Copilot enforces them automatically.

#### Scenario: Copilot agent in VS Code reminds user to update docs

- **WHEN** a developer is working on source changes via VS Code Copilot
- **THEN** the documentation maintenance rules SHALL be active in Copilot's context and it SHALL remind the developer to keep README.md and FUNCTIONALITY.md in sync
