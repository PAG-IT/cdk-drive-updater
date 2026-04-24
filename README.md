# CDK Drive Updater

A Windows CLI tool written in Rust that compares locally-installed CDK Drive
component versions against the versions published on the CDK Online Software
Distribution (OSD) catalog page and logs a detailed report.  In **update** mode
the tool also triggers silent installs for any installable package that is
out-of-date or missing.

---

## Table of Contents

1. [Overview](#overview)
2. [Prerequisites](#prerequisites)
3. [Building](#building)
4. [Configuration](#configuration)
5. [Usage](#usage)
6. [Output](#output)
7. [Architecture](#architecture)
8. [Modules](#modules)
9. [Tracked Software](#tracked-software)

---

## Overview

CDK Drive is a dealer-management platform that relies on several Windows
software components.  Keeping those components at the correct version is
normally a manual or Kaseya-scripted process.  This tool automates the
detection and update step by:

1. Scraping the OSD HTML catalog to build a software version table.
2. Reading the Windows registry and filesystem to determine what is currently
   installed.
3. Comparing installed vs. available versions using semantic version ordering.
4. Emitting a rich ASCII-table report to stdout **and** a timestamped log file.
5. In **update** mode, downloading and silently installing any out-of-date or
   missing packages (CDK 3rd Party Managed Assemblies, WebStart, BlueZone).

---

## Prerequisites

| Requirement | Detail |
| --- | --- |
| **Operating System** | Windows only (uses `winreg`, `windows-sys` Win32 APIs) |
| **Rust edition** | 2024 (declared in `Cargo.toml`) |
| **Rust toolchain** | stable, any recent version that supports edition 2024 |
| **Network access** | The machine running the tool must be able to reach the OSD URL and the Adaptiva version URL |

---

## Building

```powershell
cargo build --release
```

The executable uses `resources\logo.ico` as its Windows application icon. For a
single-file release artifact, run:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\build-release.ps1
```

The release executable is written to `dist\cdk-drive-updater.exe`. The script
does not copy Cargo intermediates, logs, resources, or local configuration files.

---

## Configuration

The tool is configured entirely through environment variables.  A `.env` file
in the working directory is loaded automatically via `dotenvy`.

| Variable | Required | Default | Description |
| --- | --- | --- | --- |
| `CDK_DRIVE_OSD_URL` | **Yes** | — | Full URL of the CDK OSD HTML page that lists current software versions |
| `ADAPTIVA_VERSION_URL` | No | `https://raw.githubusercontent.com/PAG-IT/public-configs/refs/heads/main/cdk--drive--adaptiva-version.txt` | URL of a plain-text file containing the current Adaptiva version string |
| `LOG_DIR` | No | `./cdk-updater-logs` (relative to cwd) | Directory where timestamped log files are written |
| `DOWNLOAD_DIR` | No | `./cdk-updater-downloads` (relative to cwd) | Directory where installer files are saved during update mode; deleted after each install |
| `VARIABLES_DIR` | No | `<exe dir>/cdk-updater-variables` | Directory where each CDK Installation Info check result is written as an individual `.txt` file |
| `CDK_3RD_PARTY_INSTALL_ARGS` | No | OSD silent install arguments | Silent install arguments for CDK Drive 3rd Party Managed Assemblies 96.x |
| `CDK_WEBSTART_INSTALL_ARGS` | No | OSD silent install arguments | Silent install arguments for CDK Drive WebStart |
| `CDK_BLUEZONE_INSTALL_ARGS` | No | OSD silent install arguments | Silent install arguments for CDK BlueZone (Terminal Emulator) |

### .env example

```env
CDK_DRIVE_OSD_URL=https://your-cdk-server/apps/autoTools/cds/osd/osd.php
DOWNLOAD_DIR=C:\Temp\cdk-downloads
VARIABLES_DIR=C:\Temp\cdk-variables
CDK_3RD_PARTY_INSTALL_ARGS=
CDK_WEBSTART_INSTALL_ARGS=
CDK_BLUEZONE_INSTALL_ARGS=
```

---

## Usage

```powershell
cdk-drive-updater [/query | --query | -query | /update | --update | -update]
```

| Flag | Mode | Behaviour |
| --- | --- | --- |
| *(none)* | query | Default. Detects installed versions and compares against OSD. Logs exactly what update mode *would* do, but makes no changes. |
| `/query`, `--query`, `-query` | query | Explicit query mode. Identical to running with no flag. |
| `/update`, `--update`, `-update` | update | Downloads and silently installs any out-of-date or missing packages (CDK 3rd Party Managed Assemblies, WebStart, BlueZone). Installers are deleted after each install completes. Adaptiva requires external management and is not installed by this tool. |

Flag matching is **case-insensitive** and accepts `/`, `--`, or `-` prefixes.

---

## Output

### Log file

A log file is created at:

```cmd
<LOG_DIR>\cdk-drive-updater--YYYY-MM-DD--H-MM-am|pm.log
```

The file and stdout both receive the same content, prefixed with timestamps and
log levels, e.g.:

```txt
[2026-04-21 09:15:00] [INFO] ...
```

### Report tables

The tool produces the following ASCII tables in order:

| Table | Contents |
| --- | --- |
| **Runtime Summary** | App name, mode, OSD URL, download directory, log file path |
| **CDK Installation Info** | All registry and path checks (ADP, WebStart URL, Adaptiva GUIDs, SIA paths, WebStart version) |
| **CDK Installation Info variable files** | Each check result from the CDK Installation Info table is written to an individual `.txt` file in `VARIABLES_DIR`. Filenames are the check label sanitized to a Windows-safe token and lowercased (e.g. `adp_wsvc_4.5.txt`, `webstart_version_executable.txt`). Each file contains only the result value. Multi-value Adaptiva CDK key entries generate one file per sub-entry. A `summary.txt` containing the full formatted table is also written, along with a `last-run.txt` whose content is the run timestamp in `YYYY-MM-DD--H-MM-am\|pm--EPOCH` format. |
| **Adaptiva Remote Version** | Source URL and fetched Adaptiva version string |
| **OSD Catalog Core** | Category, description, version for every entry on the OSD page |
| **OSD Catalog Details** | Category, description, silent-install arguments, download link |
| **OSD Catalog Summary** | Total entry count |
| **Installed vs OSD Summary** | Per-target: installed version, OSD version, state, action |
| **Installed vs OSD Details** | Per-target: download link, install args used/planned, notes |

#### Version state values

| State | Meaning |
| --- | --- |
| `Older (Web is newer)` | Installed version is behind OSD — update needed |
| `Newer (Installed is newer)` | Installed version is ahead of OSD |
| `Same` | Versions match |
| `Missing` | Software is not installed |
| `Unknown` | Installed but not found on OSD |

---

## Architecture

```md
main.rs
  |  Parses args -> resolves AppConfig -> init logging
  |  cdk_info::gather() ----------------------------> cdk_info.rs
  |                                                    Registry + path checks
  |  fetch_software_catalog() ----> HTTP GET OSD page
  |  parse_software_catalog()  ----> scraper HTML parse
  |  fetch_adaptiva_version()  ----> HTTP GET version URL
  |  process_target() x 4 --------------------------> installed.rs
  |                                                    Registry + exe version reads
  |  app_logging::log_*() --------------------------> app_logging.rs
  |                                                    ASCII table builder + fern logger
  `-> shared helpers -------------------------------> utils.rs
```

### High-level data flow

1. Load `.env`, initialise `fern` logger (stdout + file).
2. Read `AppConfig` from environment.
3. Parse `AppMode` from CLI args.
4. Call `cdk_info::gather()` to snapshot registry and path state.
5. Scrape the OSD HTML page into `Vec<SoftwareEntry>`.
6. Fetch the plain-text Adaptiva version; merge it into the catalog.
7. For each entry in `TARGET_SOFTWARES`, call `process_target()`:
   - Detect installed version via the entry's `detect_installed` fn pointer.
   - Compare against OSD catalog entry.
   - Produce a `TargetComparisonRow`.
8. Log all tables via `app_logging`.

---

## Modules

| Module | File | Responsibility |
| --- | --- | --- |
| `main` | `src/main.rs` | Entry point, config, arg parsing, HTTP fetching, HTML parsing, comparison orchestration |
| `installed` | `src/installed.rs` | Windows registry and executable file-version detection for all tracked packages |
| `cdk_info` | `src/cdk_info.rs` | Snapshot of CDK-specific registry keys and filesystem paths |
| `app_logging` | `src/app_logging.rs` | ASCII table builder and all structured log emission functions |
| `utils` | `src/utils.rs` | Shared helpers for env/path defaults, version comparison, timestamps, safe filenames, missing-value checks, and replace-before-write file output |

---

## Tracked Software

| Friendly Name | OSD Description | Detection Method | Auto-Install |
| --- | --- | --- | --- |
| CDK Drive 3rd Party Managed Assemblies 96.x | CDK Drive 3rd Party Managed Assemblies 96.x | Add/Remove `DisplayVersion`, with MSI product scan fallback | Yes (`CDK_3RD_PARTY_INSTALL_ARGS`) |
| Adaptiva | CDK Software Install Agent ( Adaptiva ) | MSI scan + `OneSiteClient.exe` file version | No (external/CDK SIA) |
| BlueZone | CDK Terminal Emulator | `bzvt.exe` file version under Program Files | Yes (`CDK_BLUEZONE_INSTALL_ARGS`) |
| CDK Drive WebStart | CDK Drive WebStart | Add/Remove Programs MSI scan + `CDK Drive WebStart.exe` file version fallback (cached in `CdkInfo`) | Yes (`CDK_WEBSTART_INSTALL_ARGS`) |
