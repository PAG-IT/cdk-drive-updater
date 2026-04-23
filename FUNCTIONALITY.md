# CDK Drive Updater Functionality Reference

This document is the AI-oriented operational map for the repository. Keep it synchronized with `src/` whenever behavior, public types, module responsibilities, configuration, logging, or target software definitions change.

## Data Flow

1. `main()` loads `.env` with `dotenvy::dotenv().ok()`.
2. `main()` calls `init_logging()`, which resolves `LOG_DIR`, creates the log directory, opens a timestamped log file, and configures `fern` to write to stdout and the file.
3. `AppConfig::from_env()` reads `CDK_DRIVE_OSD_URL`, `ADAPTIVA_VERSION_URL`, and `DOWNLOAD_DIR`.
4. `AppMode::from_args(&args[1..])` parses the CLI mode. Query mode is the default.
5. `app_logging::log_app_mode()` and `app_logging::log_startup_summary()` emit the initial runtime tables.
6. `cdk_info::gather()` collects registry, filesystem, Adaptiva, SIA, URL-handler, and WebStart version state.
7. `app_logging::log_cdk_info_summary()` logs the gathered CDK installation snapshot.
8. `write_cdk_info_variables()` writes each CDK Installation Info check result to a lowercased `.txt` file in `VARIABLES_DIR`, a `summary.txt` containing the full table, and a `last-run--<timestamp>--<epoch>.txt` marker file.
9. `fetch_software_catalog()` performs an HTTP GET for the OSD URL and passes the response HTML plus final response URL to `parse_software_catalog()`.
10. `parse_software_catalog()` reads OSD category sections and adjacent tables into `Vec<SoftwareEntry>`.
11. `fetch_adaptiva_version()` performs an HTTP GET for the Adaptiva version text file.
12. If a remote Adaptiva version is available, `main()` updates the existing Adaptiva catalog entry or appends a synthetic one.
13. `app_logging::log_adaptiva_remote_version()` and `app_logging::log_osd_catalog()` log the remote Adaptiva source and full catalog.
14. `main()` iterates `TARGET_SOFTWARES`, calling `process_target()` once per target.
15. `process_target()` calls the target's detection function, compares against the OSD catalog, and either describes (query) or executes (update) the install action via `perform_or_describe_install()`.
16. In update mode for installable targets: `perform_or_describe_install()` calls `actually_install()`, which downloads the file with `download_installer()`, runs it with `run_installer()`, deletes the downloaded file, and returns an outcome string.
17. `app_logging::log_target_comparisons()` logs the installed-vs-OSD summary and details tables.
18. `main()` exits with `Ok(())` on success or returns an `anyhow::Error` on unrecoverable configuration, logging, HTTP, or parsing failures.

## Configuration

| AppConfig field / runtime setting | Environment variable | Required | Default | Used by |
| --- | --- | --- | --- | --- |
| `version_source_url` | `CDK_DRIVE_OSD_URL` | Yes | None | `fetch_software_catalog()` |
| `adaptiva_version_url` | `ADAPTIVA_VERSION_URL` | No | `https://raw.githubusercontent.com/PAG-IT/public-configs/refs/heads/main/cdk--drive--adaptiva-version.txt` | `fetch_adaptiva_version()` |
| Log directory | `LOG_DIR` | No | `<cwd>/cdk-updater-logs` | `init_logging()` |
| `download_dir` | `DOWNLOAD_DIR` | No | `<cwd>/cdk-updater-downloads` | `download_installer()` |
| `variables_dir` | `VARIABLES_DIR` | No | `<exe dir>/cdk-updater-variables` | `write_cdk_info_variables()` |
| Install args override | `CDK_3RD_PARTY_INSTALL_ARGS` | No | OSD silent install arguments | `perform_or_describe_install()` for CDK Drive 3rd Party Managed Assemblies 96.x |
| Install args override | `CDK_WEBSTART_INSTALL_ARGS` | No | OSD silent install arguments | `perform_or_describe_install()` for CDK Drive WebStart |
| Install args override | `CDK_BLUEZONE_INSTALL_ARGS` | No | OSD silent install arguments | `perform_or_describe_install()` for BlueZone |

## `main.rs`

Purpose: entry point, environment configuration, CLI mode parsing, HTTP retrieval, OSD HTML parsing, target orchestration, version comparison, and logger initialization.

### Types

| Type | Visibility | Fields / variants | Description |
| --- | --- | --- | --- |
| `TargetSoftware` | private | `installed_name: &'static str`; `osd_description: &'static str`; `detect_installed: fn(&cdk_info::CdkInfo) -> Result<Option<installed::InstalledProduct>>`; `install_args_env_var: Option<&'static str>` | Static target definition used by `process_target()`. `install_args_env_var` is `None` for targets this tool does not install (Adaptiva). |
| `AppMode` | private | `Query`; `Update` | Runtime mode parsed from CLI arguments. |
| `AppConfig` | private | `version_source_url: String`; `adaptiva_version_url: String`; `download_dir: PathBuf`; `variables_dir: PathBuf` | Environment-derived application configuration. |
| `SoftwareEntry` | private crate root type | `category: String`; `description: String`; `version_number: String`; `file_version: String`; `silent_install_arguments: String`; `download_link: String` | Parsed OSD catalog row or synthetic Adaptiva row. |
| `VersionState` | private | `NeedsUpdate`; `Newer`; `Same` | Comparison result for an installed version against an OSD version. |
| `SoftwareComparison` | private | `description: String`; `version: String`; `state: VersionState`; `download_link: String`; `silent_install_arguments: String` | Normalized comparison data returned by `compare_software_version()`. |

### Constants

| Constant | Value |
| --- | --- |
| `TARGET_SOFTWARES` | Four `TargetSoftware` entries processed in order. |

### Target Software Table

| `installed_name` | `osd_description` | Detection function | `install_args_env_var` |
| --- | --- | --- | --- |
| `CDK Drive 3rd Party Managed Assemblies 96.x` | `CDK Drive 3rd Party Managed Assemblies 96.x` | `installed::detect_cdk_drive_3rd_party_managed_assemblies_96x` | `Some("CDK_3RD_PARTY_INSTALL_ARGS")` |
| `Adaptiva` | `CDK Software Install Agent ( Adaptiva )` | `installed::detect_adaptiva` | `None` |
| `BlueZone` | `CDK Terminal Emulator` | `installed::detect_bluezone` | `Some("CDK_BLUEZONE_INSTALL_ARGS")` |
| `CDK Drive WebStart` | `CDK Drive WebStart` | `installed::get_webstart_installed_version_from_cdk_info` | `Some("CDK_WEBSTART_INSTALL_ARGS")` |

### Functions

| Function | Signature | Description |
| --- | --- | --- |
| `main` | `fn main() -> Result<()>` | Runs the full program: load config, gather local state, fetch remote state, compare targets, and log reports. |
| `AppMode::from_args` | `fn from_args(args: &[String]) -> Self` | Parses `/query`, `--query`, `-query`, `/update`, `--update`, or `-update`, case-insensitively; defaults to query. |
| `AppConfig::from_env` | `fn from_env() -> Result<Self>` | Reads required and optional environment variables, including `VARIABLES_DIR` defaulting to `<exe dir>/cdk-updater-variables`. |
| `app_mode_as_str` | `fn app_mode_as_str(mode: &AppMode) -> &'static str` | Converts `AppMode` to `query` or `update` for logs. |
| `process_target` | `fn process_target(entries: &[SoftwareEntry], mode: AppMode, target: &TargetSoftware, cdk_info: &cdk_info::CdkInfo, config: &AppConfig) -> Result<TargetComparisonRow>` | Detects a target's installed version, compares it to the OSD catalog, and calls `perform_or_describe_install()` for any target that needs install or update. |
| `perform_or_describe_install` | `fn perform_or_describe_install(target: &TargetSoftware, mode: &AppMode, download_link: &str, osd_args: &str, config: &AppConfig, operation: &str) -> (String, String)` | Returns `(action, install_args)`. In query mode, returns a description of what would happen. In update mode, calls `actually_install()` and returns the outcome. Targets with `install_args_env_var = None` return an external-management message. |
| `actually_install` | `fn actually_install(url: &str, args: &str, download_dir: &Path) -> String` | Downloads the installer, runs it, deletes the file, and returns a human-readable outcome string. |
| `download_installer` | `fn download_installer(url: &str, download_dir: &Path) -> Result<PathBuf>` | GETs the installer URL, checks HTTP status, writes the body to `download_dir/<filename>`, and returns the path. |
| `run_installer` | `fn run_installer(path: &Path, args: &str) -> Result<std::process::ExitStatus>` | Splits `args` with `split_install_args()` and runs the installer via `Command::new`, waiting for completion. |
| `split_install_args` | `fn split_install_args(args: &str) -> Vec<String>` | Tokenises an installer argument string by whitespace, respecting double-quoted substrings as single tokens. |
| `extract_filename_from_url` | `fn extract_filename_from_url(url: &str) -> Option<String>` | Returns the last URL path segment for use as a local filename. |
| `capitalize_first` | `fn capitalize_first(s: &str) -> String` | Uppercases the first character of a string slice. |
| `write_cdk_info_variables` | `fn write_cdk_info_variables(info: &cdk_info::CdkInfo, dir: &Path) -> Result<()>` | Creates `dir`; iterates `app_logging::cdk_info_entries(info)`, calling `delete_if_exists` before writing each lowercased `<to_safe_filename(check)>.txt`; deletes then writes `summary.txt`; scans for and deletes any existing `last-run--*.txt` files before writing the new `last-run--<build_timestamp(now)>--<epoch>.txt` marker. |
| `delete_if_exists` | `fn delete_if_exists(path: &Path)` | Deletes `path` when it exists; logs a warning on deletion or existence-check failure rather than propagating the error. |
| `to_safe_filename` | `fn to_safe_filename(name: &str) -> String` | Replaces non-alphanumeric, non-dot, non-hyphen characters with underscores, collapses consecutive underscores, trims leading/trailing underscores, and lowercases the result. |
| `fetch_adaptiva_version` | `fn fetch_adaptiva_version(url: &str) -> Result<Option<String>>` | Fetches and trims a plain-text Adaptiva version; returns `None` for empty content. |
| `fetch_software_catalog` | `fn fetch_software_catalog(source_url: &str) -> Result<Vec<SoftwareEntry>>` | Fetches OSD HTML and parses it into catalog entries. |
| `parse_software_catalog` | `fn parse_software_catalog(html: &str, base_url: &Url) -> Result<Vec<SoftwareEntry>>` | Parses OSD category/table markup into `SoftwareEntry` values. |
| `next_table_sibling` | `fn next_table_sibling(category_element: ElementRef<'_>) -> Option<ElementRef<'_>>` | Finds the table immediately following a category `div`. |
| `collect_text` | `fn collect_text(element: ElementRef<'_>) -> String` | Normalizes all text within an element by collapsing whitespace. |
| `value_at_index` | `fn value_at_index(cells: &[ElementRef<'_>], index: Option<usize>) -> String` | Returns normalized cell text for an optional column index. |
| `resolve_link` | `fn resolve_link(base_url: &Url, href: &str) -> String` | Resolves relative download links against the final OSD response URL. |
| `get_software_by_description` | `fn get_software_by_description<'a>(entries: &'a [SoftwareEntry], description: &str) -> Option<&'a SoftwareEntry>` | Finds a catalog entry by case-insensitive description. |
| `compare_software_version` | `fn compare_software_version(entries: &[SoftwareEntry], description: &str, provided_version: &str) -> Option<SoftwareComparison>` | Compares an installed version to the matching catalog version. |
| `compare_versions` | `fn compare_versions(provided: &str, target: &str) -> Ordering` | Uses `version_compare::compare`; falls back to trimmed string ordering when parsing fails. |
| `version_state_as_str` | `fn version_state_as_str(state: &VersionState) -> &'static str` | Converts `VersionState` to the human log string. |
| `init_logging` | `fn init_logging() -> Result<PathBuf>` | Creates the log path and configures stdout plus file logging. |
| `build_timestamp` | `fn build_timestamp(now: chrono::DateTime<Local>) -> String` | Formats log filenames as `YYYY-MM-DD--H-MM-am/pm`. |

### Key Algorithms

HTML catalog parsing: `parse_software_catalog()` selects `div.category` elements, finds each adjacent `table`, maps table headers by lowercase text (`description`, `version`, `file version`, `silent install arguments`, `download`/`link`), skips rows without a description, resolves anchor `href` values against the final response URL, and treats a `File Version` header as both `version_number` and `file_version`.

Version comparison ordering: `compare_versions()` delegates to `version_compare::compare()` and maps `Lt`, `Gt`, and `Eq` to `std::cmp::Ordering`; if the crate cannot compare the strings, it compares trimmed strings lexicographically.

Update mode behavior: `process_target()` delegates install/update decisions to `perform_or_describe_install()`. For targets with `install_args_env_var = None` (Adaptiva), both modes return an external-management message and no download occurs. For installable targets in query mode, the action is `"Would download and install/update"` and the resolved args (ENV override or OSD args) are shown in `install_args`. In update mode, `actually_install()` is called: it downloads the installer to `DOWNLOAD_DIR`, runs it via `Command::new` with `split_install_args()` tokens, deletes the file (even on failure), and returns an outcome such as `"Installed (exit code: 0)"` or `"Install failed (exit code: N)"`. Targets that are current or newer in any mode produce `"No update required"`.

## `installed.rs`

Purpose: Windows MSI registry scanning and executable file-version detection for target software.

### Types

| Type | Visibility | Fields | Description |
| --- | --- | --- | --- |
| `InstalledProduct` | public | `product_name: String`; `version: String` | Installed software/version pair returned by detection functions. |

### Constants

| Constant | Value | Description |
| --- | --- | --- |
| `CDK_DRIVE_3RD_PARTY_MANAGED_ASSEMBLIES_96X_PATTERN` | `CDK Drive 3rd Party Managed Assemblies` | Case-insensitive MSI product-name substring. |
| `ADAPTIVA_ADD_REMOVE_PATTERN` | `Adaptiva` | Case-insensitive MSI product-name substring. |
| `WEBSTART_ADD_REMOVE_PATTERN` | `CDKDriveWebStart` | Case-insensitive MSI product-name substring. |
| `BLUEZONE_EXECUTABLE_NAME` | `bzvt.exe` | BlueZone executable filename searched under Program Files roots. |
| `ADAPTIVA_ONESITE_CLIENT_RELATIVE_PATH` | `Adaptiva\AdaptivaClient\bin\OneSiteClient.exe` | Adaptiva executable relative to Program Files roots. |

### Public / Exported Functions

| Function | Signature | Description |
| --- | --- | --- |
| `get_cdk_drive_3rd_party_managed_assemblies_96x_installed_version` | `pub fn get_cdk_drive_3rd_party_managed_assemblies_96x_installed_version() -> Result<Option<InstalledProduct>>` | Returns the newest matching MSI product version for CDK Drive 3rd Party Managed Assemblies. |
| `get_adaptiva_installed_version` | `pub fn get_adaptiva_installed_version() -> Result<Option<InstalledProduct>>` | Returns the highest Adaptiva version found from executable metadata or Add/Remove MSI entries. |
| `get_bluezone_installed_version` | `pub fn get_bluezone_installed_version() -> Result<Option<InstalledProduct>>` | Returns the highest BlueZone version found from `bzvt.exe` metadata. |
| `get_webstart_add_remove_installed_version` | `pub fn get_webstart_add_remove_installed_version() -> Result<Option<InstalledProduct>>` | Returns the newest WebStart version from Add/Remove MSI product registry entries. |
| `detect_cdk_drive_3rd_party_managed_assemblies_96x` | `pub fn detect_cdk_drive_3rd_party_managed_assemblies_96x(_cdk_info: &CdkInfo) -> Result<Option<InstalledProduct>>` | Target adapter that ignores `CdkInfo` and calls the MSI detector. |
| `detect_adaptiva` | `pub fn detect_adaptiva(_cdk_info: &CdkInfo) -> Result<Option<InstalledProduct>>` | Target adapter that ignores `CdkInfo` and calls the Adaptiva detector. |
| `detect_bluezone` | `pub fn detect_bluezone(_cdk_info: &CdkInfo) -> Result<Option<InstalledProduct>>` | Target adapter that ignores `CdkInfo` and calls the BlueZone detector. |
| `get_webstart_installed_version_from_cdk_info` | `pub fn get_webstart_installed_version_from_cdk_info(cdk_info: &CdkInfo) -> Result<Option<InstalledProduct>>` | Returns WebStart version from `CdkInfo`, preferring Add/Remove MSI version over executable file version. |
| `get_installed_version` | `pub fn get_installed_version(name_contains: &str) -> Result<Option<InstalledProduct>>` | Scans `HKCR\Installer\Products` and returns the highest matching MSI version. |
| `read_executable_file_version` | `pub(crate) fn read_executable_file_version(path: &Path) -> Result<Option<String>>` | Reads Windows fixed file-version metadata from an executable. |

### Key Internal Functions

| Function | Signature | Description |
| --- | --- | --- |
| `find_bluezone_executables` | `fn find_bluezone_executables() -> Vec<PathBuf>` | Walks `BlueZone` directories under Program Files roots looking for `bzvt.exe`. |
| `find_adaptiva_executables` | `fn find_adaptiva_executables() -> Vec<PathBuf>` | Checks expected Adaptiva `OneSiteClient.exe` paths under Program Files roots. |
| `candidate_program_files_roots` | `fn candidate_program_files_roots() -> Vec<PathBuf>` | Combines Program Files environment variables with fixed fallback paths, sorted and deduplicated. |
| `to_wide` | `fn to_wide(value: &OsStr) -> Vec<u16>` | Converts a path/string to a null-terminated UTF-16 buffer for Win32 APIs. |
| `format_fixed_file_version` | `fn format_fixed_file_version(version_info: &VS_FIXEDFILEINFO) -> String` | Formats Win32 fixed file-version fields as `major.minor.build.revision`. |
| `select_highest_version` | `fn select_highest_version(mut matches: Vec<InstalledProduct>) -> Option<InstalledProduct>` | Sorts descending by version and returns the first product. |
| `decode_msi_version` | `fn decode_msi_version(product_name: &str, version_int: u32) -> String` | Extracts `V-` versions from product names before falling back to MSI DWORD decoding. |
| `extract_version_from_name` | `fn extract_version_from_name(name: &str) -> Option<String>` | Extracts dot-separated digits after `V-` when the result contains at least one dot. |
| `compare_version_strings` | `fn compare_version_strings(a: &str, b: &str) -> Ordering` | Uses `version_compare`; falls back to string comparison. |

### Key Algorithms

MSI version decoding: `get_installed_version()` scans `HKEY_CLASSES_ROOT\Installer\Products`, opens each product subkey, filters `ProductName` by case-insensitive substring, reads the `Version` DWORD, and passes `(ProductName, Version)` to `decode_msi_version()`. `decode_msi_version()` first tries to extract a product-name version such as `V-104.21.517.125`; if absent, it unpacks the DWORD as `major = top byte`, `minor = next byte`, and `build = low word`.

`V-` prefix extraction: `extract_version_from_name()` finds `v-` case-insensitively, takes following ASCII digits and dots, trims trailing dots, and accepts the result only when it contains at least one dot.

Executable file version reading: `read_executable_file_version()` converts the path to UTF-16, calls `GetFileVersionInfoSizeW`, loads the version resource with `GetFileVersionInfoW`, queries the root block with `VerQueryValueW`, casts to `VS_FIXEDFILEINFO`, and formats `dwFileVersionMS` / `dwFileVersionLS` as `major.minor.build.revision`.

Highest-version selection: detected products are sorted descending with `compare_version_strings()`, then the first entry is used. Adaptiva prefers executable metadata over Add/Remove MSI data because `get_adaptiva_installed_version()` returns `executable_match.or(add_remove_match)`.

## `cdk_info.rs`

Purpose: gather a single local snapshot of CDK-specific registry keys, Adaptiva registry values, SIA filesystem paths, CDKDrive URL-handler state, and WebStart executable version.

### Types

| Type | Visibility | Fields / variants | Description |
| --- | --- | --- | --- |
| `RegistryCheckStatus` | public | `Found`; `PathExists`; `PathMissing` | Result of checking for a registry key and named value. Implements `Display`. |
| `PathCheckStatus` | public | `Found`; `Missing`; `Error(String)` | Result of checking a filesystem path. Implements `Display`. |
| `CdkInfo` | public | See field table below. | Snapshot consumed by logging and WebStart target detection. |

### `CdkInfo` Fields

| Field | Type | Meaning |
| --- | --- | --- |
| `adp_check` | `RegistryCheckStatus` | `HKLM\SOFTWARE\WOW6432Node\ADP\wsvc\4.5` with `version`. |
| `webstart_url_check` | `RegistryCheckStatus` | `HKLM\SOFTWARE\Classes\CDKDrive` with `URL Protocol`. |
| `webstart_shell_var` | `String` | Default command from `HKCR\CDKDrive\shell\open\command`, or `PathExists` / `PathMissing`. |
| `unify_drive_enabler_check` | `RegistryCheckStatus` | `HKLM\SOFTWARE\CDKGlobal` with `CDKUnifyDriveEnabler`. |
| `adaptiva_check` | `RegistryCheckStatus` | `HKLM\SOFTWARE\Adaptiva\client` with `setup.status`. |
| `adaptiva_cdk_key_values` | `Option<Vec<(String, String)>>` | Recursive values under `HKLM\SOFTWARE\CDK\Adaptiva`. |
| `adaptiva_cdk_key_wow_values` | `Option<Vec<(String, String)>>` | Recursive values under `HKLM\SOFTWARE\WOW6432Node\CDK\Adaptiva`. |
| `adaptiva_server_host_name` | `String` | Native `setup.server_host_name`. |
| `adaptiva_server_host_name_wow` | `String` | WOW6432Node `setup.server_host_name`. |
| `adaptiva_server_locator_name` | `String` | Native `server_locator.server_name`. |
| `adaptiva_server_locator_name_wow` | `String` | WOW6432Node `server_locator.server_name`. |
| `adaptiva_setup_guid` | `String` | Native `setup.server_guid`. |
| `adaptiva_client_data_manager_guid` | `String` | Native `client_data_manager.server_guid`. |
| `adaptiva_setup_guid_wow` | `String` | WOW6432Node `setup.server_guid`. |
| `adaptiva_client_data_manager_guid_wow` | `String` | WOW6432Node `client_data_manager.server_guid`. |
| `sia_check` | `PathCheckStatus` | `C:\Program Files (x86)\CDK\sia`. |
| `sia_xml_check` | `PathCheckStatus` | `C:\Program Files (x86)\CDK\sia\cdk_sia_win10_maint.xml`. |
| `sia_fix_check` | `PathCheckStatus` | `C:\Program Files (x86)\CDK\sia\w10_fix.vbs`. |
| `webstart_version` | `String` | File version of `CDK Drive WebStart.exe`, or `NotFound`. |
| `webstart_add_remove_version` | `String` | Add/Remove Programs (MSI registry) version for `CDK Drive WebStart`, or `NotFound`. |

### Public Functions

| Function | Signature | Description |
| --- | --- | --- |
| `gather` | `pub fn gather() -> CdkInfo` | Performs all CDK registry, path, Adaptiva, and WebStart checks and maps failures to status values. |

### Key Internal Functions

| Function | Signature | Description |
| --- | --- | --- |
| `registry_value_check` | `fn registry_value_check(hive: &RegKey, subkey: &str, value_name: &str) -> RegistryCheckStatus` | Distinguishes missing key, existing key with named value, and existing key without named value. |
| `read_key_values_recursive` | `fn read_key_values_recursive(hive: &RegKey, subkey: &str) -> Option<Vec<(String, String)>>` | Opens a key and collects all named values from it and descendant subkeys. |
| `collect_key_values` | `fn collect_key_values(key: &RegKey, prefix: &str, out: &mut Vec<(String, String)>)` | Recursive worker for registry value enumeration. |
| `format_reg_value` | `fn format_reg_value(value: &winreg::RegValue) -> String` | Converts common registry value types to display strings. |
| `read_registry_string` | `fn read_registry_string(hive: &RegKey, subkey: &str, value_name: &str) -> String` | Reads a named string value or returns `Not Found`. |
| `read_shell_command` | `fn read_shell_command(hkcr: &RegKey) -> String` | Reads the default CDKDrive URL-handler command or returns `PathExists` / `PathMissing`. |
| `path_check` | `fn path_check(path: &str) -> PathCheckStatus` | Uses `try_exists()` to distinguish found, missing, and I/O error. |

### Key Algorithms

Registry value checking: `registry_value_check()` opens the key, enumerates value names case-insensitively, returns `Found` for a matching value, `PathExists` for an existing key without that value, and `PathMissing` when the key cannot be opened.

Recursive Adaptiva key capture: `read_key_values_recursive()` returns `None` when the root key is absent. When present, `collect_key_values()` includes root values and subkey values, labeling subkey entries as `SubKey\ValueName`.

Registry value formatting: `format_reg_value()` decodes `REG_SZ` and `REG_EXPAND_SZ` as UTF-16 strings, `REG_DWORD` and `REG_QWORD` as numbers, `REG_MULTI_SZ` as strings joined with ` | `, and unknown types as hex bytes.

## `app_logging.rs`

Purpose: structured logging and ASCII table rendering for runtime, CDK snapshot, OSD catalog, Adaptiva remote version, and target comparison output.

### Types

| Type | Visibility | Fields | Description |
| --- | --- | --- | --- |
| `TargetComparisonRow` | `pub(crate)` | `target: String`; `osd_description: String`; `installed_version: String`; `osd_version: String`; `state: String`; `action: String`; `download_link: String`; `install_args: String`; `note: String` | Row emitted in installed-vs-OSD summary and details tables. `install_args` holds the resolved install arguments (query: what would be used; update: what was used). |

### Public / Exported Functions

| Function | Signature | Description |
| --- | --- | --- |
| `log_app_mode` | `pub(crate) fn log_app_mode(mode: &str)` | Logs a prominent startup banner for query/update mode. |
| `log_startup_summary` | `pub(crate) fn log_startup_summary(log_file_path: &Path, mode: &str, version_source_url: &str, download_dir: &str, variables_dir: &str)` | Logs the Runtime Summary table including the download and variables directories. |
| `cdk_info_entries` | `pub(crate) fn cdk_info_entries(info: &cdk_info::CdkInfo) -> Vec<(String, String)>` | Builds the ordered `(check, result)` pairs for the CDK Installation Info table; shared by `log_cdk_info_summary` and `write_cdk_info_variables`. |
| `log_cdk_info_summary` | `pub(crate) fn log_cdk_info_summary(info: &cdk_info::CdkInfo)` | Calls `cdk_info_table_string` and logs it. |
| `cdk_info_table_string` | `pub(crate) fn cdk_info_table_string(info: &cdk_info::CdkInfo) -> String` | Returns the CDK Installation Info ASCII table as a string; used by `log_cdk_info_summary` and `write_cdk_info_variables`. |
| `log_adaptiva_remote_version` | `pub(crate) fn log_adaptiva_remote_version(url: &str, version: &Option<String>)` | Logs the Adaptiva Remote Version table. |
| `log_osd_catalog` | `pub(crate) fn log_osd_catalog(entries: &[SoftwareEntry])` | Logs OSD Catalog Core, Details, and Summary tables. |
| `log_target_comparisons` | `pub(crate) fn log_target_comparisons(rows: &[TargetComparisonRow])` | Logs Installed vs OSD Summary and Details tables; Details includes the `install_args` column. |

### Key Internal Functions

| Function | Signature | Description |
| --- | --- | --- |
| `expand_key_value_rows` | `fn expand_key_value_rows(label: &str, values: &Option<Vec<(String, String)>>, rows: &mut Vec<(String, String)>)` | Expands optional registry value vectors into `(check, result)` pairs for `cdk_info_entries`. |
| `build_ascii_table` | `fn build_ascii_table(title: &str, headers: &[&str], rows: &[Vec<String>]) -> String` | Builds a titled ASCII table with separators, header row, and data rows. |
| `build_ascii_row` | `fn build_ascii_row(cells: &[String], widths: &[usize]) -> String` | Renders one padded pipe-delimited table row. |
| `compute_widths` | `fn compute_widths(headers: &[&str], rows: &[Vec<String>]) -> Vec<usize>` | Computes per-column widths from headers and row content. |
| `build_separator` | `fn build_separator(widths: &[usize]) -> String` | Builds a `+---+` separator row. |

### Key Algorithms

ASCII table rendering: `build_ascii_table()` computes max widths across headers and rows, emits the title and blank line, renders separators around the header and every row, normalizes embedded CR/LF characters to spaces, and returns a string ending with two newlines.

Catalog logging: `log_osd_catalog()` emits a compact Core table using `file_version` when present and `version_number` otherwise, a Details table containing silent install arguments and download links, and a Summary table containing total entry count.

Target comparison logging: `log_target_comparisons()` splits each `TargetComparisonRow` into a Summary table for target/version/state/action and a Details table for download links and notes.
