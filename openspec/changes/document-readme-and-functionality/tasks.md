# Task List for README.md and FUNCTIONALITY.md Creation

## 1. Write README.md

- [x] 1.1 Replace existing README.md stub with full project documentation (purpose, problem context, CDK Drive ecosystem)
- [x] 1.2 Add Prerequisites section (Windows OS, Rust edition 2024, required tooling)
- [x] 1.3 Add Configuration section with env var table (CDK_DRIVE_OSD_URL required; ADAPTIVA_VERSION_URL optional; LOG_DIR optional)
- [x] 1.4 Add Usage section with CLI invocation syntax and flag descriptions (/query, --query, /update, --update, default mode)
- [x] 1.5 Add Output section describing ASCII-table log format, log file location pattern, and stdout behavior
- [x] 1.6 Add Architecture section naming the four modules and the high-level data flow

## 2. Write FUNCTIONALITY.md

- [x] 2.1 Create FUNCTIONALITY.md with a Data Flow section describing end-to-end execution sequence
- [x] 2.2 Add `main` module section with all types (TargetSoftware, AppMode, AppConfig, SoftwareEntry, VersionState, SoftwareComparison), public functions, and TARGET_SOFTWARES table
- [x] 2.3 Add `installed` module section with InstalledProduct type, all public detection functions, and key algorithms (MSI version decoding, V- prefix extraction, executable file version reading via Win32 API)
- [x] 2.4 Add `cdk_info` module section with CdkInfo struct (all fields), RegistryCheckStatus, PathCheckStatus enums, and gather() function description
- [x] 2.5 Add `app_logging` module section with TargetComparisonRow type and all log_* functions plus build_ascii_table description
- [x] 2.6 Add Configuration section listing AppConfig fields with their env var sources

## 3. Add Documentation Maintenance Rules

- [x] 3.1 Update openspec/AGENTS.md with rules requiring README.md and FUNCTIONALITY.md to be kept in sync with src/ changes
- [x] 3.2 Specify in AGENTS.md the required structure of both documents so agents know what to update
- [x] 3.3 Create .copilot/instructions/documentation.instructions.md mirroring the maintenance rules for VS Code Copilot
