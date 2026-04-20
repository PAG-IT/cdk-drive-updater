# Spec: Query and Update Run Modes

## Overview

The program supports two explicit run modes, selected via a command-line flag. Query mode is the
default and reports version status without taking any action. Update mode performs the same check
and additionally triggers download and silent-install when the software needs updating.

## CLI Flags

A mode flag may be passed as its first recognised argument in any of the following forms:

| Mode   | Accepted flags                    |
|--------|-----------------------------------|
| query  | `/query`, `--query`, `-query`     |
| update | `/update`, `--update`, `-update`  |

Flag matching is **case-insensitive** and any leading `-` or `/` characters are stripped before
comparison. If no recognised mode flag is present the program defaults to **query** mode.

## Behaviour by Mode

### Query Mode (default)

1. Load configuration from environment.
2. Fetch the OSD software catalog.
3. Detect the installed version of the target software.
4. Compare the installed version against the catalog version.
5. Log the comparison result (`Same`, `Older (Web is newer)`, `Newer (Installed is newer)`).
6. Exit without modifying anything on the system.

### Update Mode (`/update` | `--update` | `-update`)

Steps 1–5 are identical to query mode. After the comparison:

- If the state is `NeedsUpdate` **or** the software is not installed at all:
  - Log that an update/install is required, including the download link.
  - Download the installer from the catalog's `download_link`.
  - Execute the installer using the catalog's `silent_install_arguments`.
  - Log the outcome of the installation.
- If the state is `Same` or `Newer`:
  - Log that no update is required and exit cleanly.

## Implementation Location

- Mode enum and parsing logic: `src/main.rs` — `AppMode` enum, `AppMode::from_args()`
- Mode is resolved once in `main()` and threaded through as a value; no global state.
- Download/install logic belongs in a dedicated helper (e.g., `run_installer`) to keep `main()`
  readable.

## Error Handling

- Unknown flags are silently ignored; the mode defaults to query.
- A missing or unreachable download URL in update mode is a hard error logged at `ERROR` level;
  the program exits with a non-zero code.
- A non-zero installer exit code is logged and the program exits with a non-zero code.
