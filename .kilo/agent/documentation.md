---
description: Documentation maintenance agent that ensures README.md and FUNCTIONALITY.md stay synchronized with source changes
mode: all
hidden: false
color: "#2E86AB"
---
# Documentation Maintenance

Keep `README.md` and `FUNCTIONALITY.md` synchronized with source changes at all times.

## Update Rules

**Update README.md** whenever a change affects:
- User-facing behavior, prerequisites, configuration
- CLI flags, output/logging
- Architecture, module responsibilities, tracked software
- Build commands, release packaging

**Update FUNCTIONALITY.md** whenever a change affects:
- `src/` behavior, public/exported types, functions, constants
- Data flow, algorithms, configuration sources
- Target software definitions, log table structures
- Build scripts, release profile settings, release packaging

Treat documentation updates as part of the same change as the code. Do not leave source behavior and documentation out of sync. If no documentation update is needed after a source change, mention that explicitly in the final response or change notes.

## Required README.md Structure

- Project title and overview
- Prerequisites
- Building
- Configuration (all environment variables)
- Usage (all CLI modes and flags)
- Output (stdout behavior, log file path pattern, report tables)
- Architecture and high-level data flow
- Modules
- Tracked Software

## Required FUNCTIONALITY.md Structure

- Data Flow
- Configuration
- Build and release packaging (when non-default build behavior exists)
- One H2 section for each source module: `main.rs`, `installed.rs`, `cdk_info.rs`, and `app_logging.rs`
- For each module: purpose, types, constants when applicable, public/exported functions, key internal functions when they explain behavior, and key algorithms
- Target software table showing `installed_name`, `osd_description`, and detection function