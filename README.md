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
6. After update mode completes, automatically running a **post-update
   verification query** that re-gathers local state, re-fetches the OSD catalog,
   and re-compars all targets to confirm whether installations succeeded.

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
| `CDK_3RD_PARTY_INSTALL_ARGS` | No | empty string | Silent install arguments for CDK Drive 3rd Party Managed Assemblies 96.x |
| `CDK_WEBSTART_INSTALL_ARGS` | No | `/quiet /norestart` | Silent install arguments for CDK Drive WebStart |
| `CDK_BLUEZONE_INSTALL_ARGS` | No | `/silent` | Silent install arguments for CDK BlueZone (Terminal Emulator) |
| `CDK_ADAPTIVA_PREADAPTIVA_ARGS` | No | `CNUMBER=<ADAPTIVA_CNUMBER> HOST=<ADAPTIVA_HOST>` | Override arguments passed to `preadaptiva.msi` during Adaptiva installs |
| `CDK_ADAPTIVA_CLIENT_ARGS` | No | `-installorupgrade -servername <ADAPTIVA_HOST> -cloudrelay -serverguid <ADAPTIVA_SERVER_GUID>` | Override arguments passed to `AdaptivaClientSetup.exe` during Adaptiva installs |
| `ADAPTIVA_CNUMBER` | No | `C000000` | Default `CNUMBER` value used when building `preadaptiva.msi` arguments |
| `ADAPTIVA_HOST` | No | `C000000-example.drive.example.com` | Default host/server name used when building Adaptiva installer arguments |
| `ADAPTIVA_SERVER_GUID` | No | `00000000-0000-0000-0000-000000000000` | Default server GUID used when building `AdaptivaClientSetup.exe` arguments |

### .env example

```env
CDK_DRIVE_OSD_URL=https://your-cdk-server/apps/autoTools/cds/osd/osd.php
DOWNLOAD_DIR=C:\Temp\cdk-downloads
VARIABLES_DIR=C:\Temp\cdk-variables
CDK_3RD_PARTY_INSTALL_ARGS=
CDK_WEBSTART_INSTALL_ARGS=/quiet /norestart
CDK_BLUEZONE_INSTALL_ARGS=/silent
CDK_ADAPTIVA_PREADAPTIVA_ARGS=
CDK_ADAPTIVA_CLIENT_ARGS=
ADAPTIVA_CNUMBER=C000000
ADAPTIVA_HOST=C000000-example.drive.example.com
ADAPTIVA_SERVER_GUID=00000000-0000-0000-0000-000000000000
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
| `/update`, `--update`, `-update` | update | Downloads and installs any out-of-date or missing tracked package. Standard installers are run directly with app-owned default args or environment overrides. Adaptiva is handled as a two-step zip-based install: the tool rewrites the OSD `index.php` URL to `download.php`, downloads the zip, extracts it, runs `preadaptiva.msi`, then runs `AdaptivaClientSetup.exe`, and deletes the zip and extracted files afterward. If Adaptiva is already installed, the tool reports `Install skipped: already installed`. **After all installs complete, the tool automatically runs a query pass to re-gather local state and verify the updates, producing a second set of comparison tables.** |

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
| **CDK Installation Info** | All registry and path checks (ADP, WebStart URL, Adaptiva GUIDs, SIA paths, WebStart version, CDK 3rd Party Assemblies version, Adaptiva installed version, BlueZone version) |
| **CDK Installation Info variable files** | Each check result from the CDK Installation Info table is written to an individual `.txt` file in `VARIABLES_DIR`. Filenames are the check label sanitized to a Windows-safe token and lowercased (e.g. `adp_wsvc_4.5.txt`, `webstart_version_executable.txt`). Each file contains only the result value. Multi-value Adaptiva CDK key entries generate one file per sub-entry. A `summary.txt` containing the full formatted table is also written, along with a `last-run.txt` whose content is the run timestamp in `YYYY-MM-DD--H-MM-am\|pm--EPOCH` format. |
| **Adaptiva Remote Version** | Source URL and fetched Adaptiva version string |
| **OSD Catalog Core** | Category, description, version for every entry on the OSD page |
| **OSD Catalog Details** | Category, description, silent-install arguments, download link |
| **OSD Catalog Summary** | Total entry count |
| **Installed vs OSD Details** | Per-target: download link, install args used/planned, notes |
| **Installed vs OSD Summary** | Per-target: installed version, OSD version, state, action |
| **Post-Run Summary** | Human-readable summary after all tables. In query mode: whether any packages need updating and how to do so. In update mode: a list of every install action attempted with a SUCCESS/FAILED verdict. Always includes the variables directory path and a pointer to `summary.txt`. |

**In update mode**, the CDK Installation Info, OSD Catalog, and Installed vs OSD
tables appear **twice**: once before the install pass (showing pre-update state)
and once after the install pass as a **Post-Update Verification Query** (showing
post-update state).

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
   - For standard targets, resolve install args from app defaults or env overrides.
   - For Adaptiva in update mode, download a zip payload, extract it, and run `preadaptiva.msi` plus `AdaptivaClientSetup.exe`.
   - Produce a `TargetComparisonRow`.
8. Log all tables via `app_logging`.
9. In update mode only, repeat steps 4-8 in `AppMode::Query` to verify the
   post-update state, producing a second full set of CDK Installation Info and
   Installed vs OSD comparison tables.

---

## Modules

| Module | File | Responsibility |
| --- | --- | --- |
| `main` | `src/main.rs` | Entry point, config, arg parsing, HTTP fetching, HTML parsing, installer orchestration, zip extraction for Adaptiva, and comparison orchestration |
| `installed` | `src/installed.rs` | Windows registry and executable file-version detection for all tracked packages |
| `cdk_info` | `src/cdk_info.rs` | Snapshot of CDK-specific registry keys and filesystem paths |
| `app_logging` | `src/app_logging.rs` | ASCII table builder and all structured log emission functions |
| `utils` | `src/utils.rs` | Shared helpers for env/path defaults, version comparison, timestamps, safe filenames, missing-value checks, and replace-before-write file output |

---

## Tracked Software

| Friendly Name | OSD Description | Detection Method | Auto-Install |
| --- | --- | --- | --- |
| CDK Drive 3rd Party Managed Assemblies 96.x | CDK Drive 3rd Party Managed Assemblies 96.x | Add/Remove `DisplayVersion`, with MSI product scan fallback | Yes (`CDK_3RD_PARTY_INSTALL_ARGS`) |
| Adaptiva | CDK Software Install Agent ( Adaptiva ) | MSI scan + `OneSiteClient.exe` file version | Yes (`CDK_ADAPTIVA_PREADAPTIVA_ARGS`, `CDK_ADAPTIVA_CLIENT_ARGS`) |
| BlueZone | CDK Terminal Emulator | `bzvt.exe` file version under Program Files | Yes (`CDK_BLUEZONE_INSTALL_ARGS`) |
| CDK Drive WebStart | CDK Drive WebStart | Add/Remove Programs MSI scan + `CDK Drive WebStart.exe` file version fallback (cached in `CdkInfo`) | Yes (`CDK_WEBSTART_INSTALL_ARGS`) |
