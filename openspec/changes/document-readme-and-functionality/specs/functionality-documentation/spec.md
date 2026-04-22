## ADDED Requirements

### Requirement: FUNCTIONALITY.md has a top-level data flow section

FUNCTIONALITY.md SHALL begin with a "Data Flow" section that describes the complete end-to-end execution path from `main()` entry to process exit, naming every major step and the module responsible.

#### Scenario: AI agent understands execution order without reading source

- **WHEN** an AI agent reads the Data Flow section
- **THEN** it SHALL know the sequence: dotenv load → logging init → config read → mode parse → CDK info gather → OSD catalog fetch → Adaptiva version fetch → catalog merge → per-target comparison → summary log

### Requirement: FUNCTIONALITY.md documents each module with types and functions

FUNCTIONALITY.md SHALL have one H2 section per source file (`main.rs`, `installed.rs`, `cdk_info.rs`, `app_logging.rs`). Each section SHALL list: the module's purpose, all public/exported types with their fields, and all public/exported functions with their signatures and a one-sentence description.

#### Scenario: AI agent locates the correct function without searching source

- **WHEN** an AI agent needs to understand how BlueZone is detected
- **THEN** reading the `installed` module section SHALL show `get_bluezone_installed_version() -> Result<Option<InstalledProduct>>` and its description

### Requirement: FUNCTIONALITY.md documents key algorithms inline

FUNCTIONALITY.md SHALL describe non-obvious algorithms within their module section, including: MSI version decoding (version DWORD to major.minor.build), the `V-X.Y.Z` version extraction from product names, version comparison ordering (using version-compare crate with string fallback), and HTML catalog parsing (category div → adjacent table → header index mapping).

#### Scenario: AI agent can explain MSI version decoding

- **WHEN** an AI agent reads the `installed` module section
- **THEN** it SHALL find a description of how `decode_msi_version` unpacks the `Version` DWORD and falls back to scanning the product name for a `V-` prefix

### Requirement: FUNCTIONALITY.md documents the four target software entries

FUNCTIONALITY.md SHALL document the `TARGET_SOFTWARES` static array: each entry's `installed_name`, `osd_description`, and which detection function it uses.

#### Scenario: AI agent knows all tracked software without reading source

- **WHEN** an AI agent reads the Target Software section
- **THEN** it SHALL find all four entries: CDK Drive 3rd Party Managed Assemblies 96.x, Adaptiva, BlueZone, and CDK Drive WebStart

### Requirement: FUNCTIONALITY.md documents environment variables and config

FUNCTIONALITY.md SHALL include a section listing `AppConfig` fields, the env var each reads, and whether required or optional (matching README but formatted for AI parsing).

#### Scenario: AI agent knows all config sources without reading main.rs

- **WHEN** an AI agent reads the Configuration section in FUNCTIONALITY.md
- **THEN** it SHALL find `CDK_DRIVE_OSD_URL` (required), `ADAPTIVA_VERSION_URL` (optional with default URL), and `LOG_DIR` (optional, defaults to `cdk-updater-logs/` in cwd)
