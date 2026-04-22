# Project Instructions

This file contains instructions for AI agents working on this project.

## Documentation Maintenance

Keep the root documentation synchronized with source changes at all times:

- Update `README.md` whenever a change affects user-facing behavior, prerequisites, configuration, CLI flags, output/logging, architecture, module responsibilities, or tracked software.
- Update `FUNCTIONALITY.md` whenever a change affects `src/` behavior, public/exported types, functions, constants, data flow, algorithms, configuration sources, target software definitions, or log table structures.
- Treat documentation updates as part of the same change as the code. Do not leave source behavior and documentation out of sync.
- If no documentation update is needed after a source change, mention that explicitly in the final response or change notes.

## Required README.md Structure

`README.md` is the human-facing project guide. Preserve these sections and update them when relevant:

- Project title and overview
- Prerequisites
- Building
- Configuration, including all environment variables
- Usage, including all CLI modes and flags
- Output, including stdout behavior, log file path pattern, and report tables
- Architecture and high-level data flow
- Modules
- Tracked Software

## Required FUNCTIONALITY.md Structure

`FUNCTIONALITY.md` is the AI-facing implementation reference. Preserve these sections and update them when relevant:

- Data Flow
- Configuration
- One H2 section for each source module: `main.rs`, `installed.rs`, `cdk_info.rs`, and `app_logging.rs`
- For each module: purpose, types, constants when applicable, public/exported functions, key internal functions when they explain behavior, and key algorithms
- Target software table showing `installed_name`, `osd_description`, and detection function

When adding, removing, or renaming source modules, update this required structure in both `FUNCTIONALITY.md` and this instruction file.
