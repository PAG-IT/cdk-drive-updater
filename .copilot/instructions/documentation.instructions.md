# Documentation Maintenance Instructions

Keep `README.md` and `FUNCTIONALITY.md` synchronized with source changes.

Update `README.md` when a change affects user-facing behavior, prerequisites, configuration, CLI flags, output/logging, architecture, module responsibilities, or tracked software.

Update `FUNCTIONALITY.md` when a change affects `src/` behavior, public/exported types, functions, constants, data flow, algorithms, configuration sources, target software definitions, or log table structures.

Required `README.md` structure:

- Project title and overview
- Prerequisites
- Building
- Configuration, including all environment variables
- Usage, including all CLI modes and flags
- Output, including stdout behavior, log file path pattern, and report tables
- Architecture and high-level data flow
- Modules
- Tracked Software

Required `FUNCTIONALITY.md` structure:

- Data Flow
- Configuration
- One H2 section for each source module: `main.rs`, `installed.rs`, `cdk_info.rs`, and `app_logging.rs`
- For each module: purpose, types, constants when applicable, public/exported functions, key internal functions when they explain behavior, and key algorithms
- Target software table showing `installed_name`, `osd_description`, and detection function

When editing Rust source, include documentation updates in the same change. If no documentation update is needed, note why in the final response or change notes.
