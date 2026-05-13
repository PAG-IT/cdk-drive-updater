# CDK Drive Updater Functionality Reference

This document is the AI-oriented operational map for the repository. Keep it synchronized with `src/` whenever behavior, public types, module responsibilities, configuration, logging, or target software definitions change. Also update it when build or release packaging behavior changes in a way future agents need to preserve.

## Data Flow

1. `main()` loads `.env` with `dotenvy::dotenv().ok()`.
2. `main()` calls `init_logging()`, which resolves `LOG_DIR` through `utils::env_path_or_else()`, creates the log directory, opens a timestamped log file, and configures `fern` to write to stdout and the file.
3. `AppConfig::from_env()` reads `CDK_DRIVE_OSD_URL`, `ADAPTIVA_VERSION_URL`, `DOWNLOAD_DIR`, and `VARIABLES_DIR`.
4. `AppMode::from_args(&args[1..])` parses the CLI mode. Query mode is the default.
5. `app_logging::log_app_mode()` and `app_logging::log_startup_summary()` emit the initial runtime tables.
6. `cdk_info::gather()` collects registry, filesystem, Adaptiva, SIA, URL-handler, and WebStart version state.
7. `app_logging::log_cdk_info_summary()` logs the gathered CDK installation snapshot.
8. `write_cdk_info_variables()` writes each CDK Installation Info check result to a lowercased `.txt` file in `VARIABLES_DIR`, a `summary.txt` containing the full table, and a `last-run.txt` marker file whose content is `<timestamp>--<epoch>`.
9. `fetch_software_catalog()` performs an HTTP GET for the OSD URL and passes the response HTML plus final response URL to `parse_software_catalog()`.
10. `parse_software_catalog()` reads OSD category sections and adjacent tables into `Vec<SoftwareEntry>`.
11. `fetch_adaptiva_version()` performs an HTTP GET for the Adaptiva version text file.
12. If a remote Adaptiva version is available, `merge_adaptiva_catalog_entry()` updates the existing Adaptiva catalog entry or appends a synthetic one.
13. `app_logging::log_adaptiva_remote_version()` and `app_logging::log_osd_catalog()` log the remote Adaptiva source and full catalog.
14. `main()` iterates `TARGET_SOFTWARES`, calling `process_target()` once per target.
15. `process_target()` calls the target's detection function, compares against the OSD catalog, and either describes (query) or executes (update) the install action via `perform_or_describe_install()`.
16. In update mode for standard targets: `perform_or_describe_install()` calls `actually_install()`, which downloads the file with `download_installer()`, terminates `wsStart_4.exe` with `kill_process_if_running()`, waits 20 seconds only when that process was killed, terminates `wsStartChrome.exe` with `kill_process_if_running()`, runs the installer with `run_installer()`, deletes the downloaded file, and returns an outcome string.
17. In update mode for Adaptiva: `perform_or_describe_install()` calls `actually_install_adaptiva()`, which rewrites the OSD URL from `index.php` to `download.php`, downloads a zip payload, extracts it with `extract_zip()`, runs `preadaptiva.msi`, runs `AdaptivaClientSetup.exe`, then deletes the zip and extraction directory.
18. `app_logging::log_target_comparisons()` logs the installed-vs-OSD summary and details tables.
19. In update mode, if at least one install was attempted, `main()` runs a post-update verification pass that re-gathers local CDK state via `cdk_info::gather()`, rewrites variables, re-fetches the software catalog and Adaptiva version, and re-processes every target in `AppMode::Query` to produce a second comparison table confirming whether installations succeeded. When all targets are already up-to-date (no install attempted), the verification pass is skipped.
20. `log_post_run_summary()` writes a human-readable Post-Run Summary after all comparison tables. In query mode it reports whether any packages need updating and how to do so. In update mode it lists every install action that was attempted with a SUCCESS/FAILED verdict. Always includes the variables directory path and a pointer to `summary.txt`.
21. `main()` exits with `Ok(())` on success or returns an `anyhow::Error` on unrecoverable configuration, logging, HTTP, or parsing failures.

## Configuration

| AppConfig field / runtime setting | Environment variable | Required | Default | Used by |
| --- | --- | --- | --- | --- |
| `version_source_url` | `CDK_DRIVE_OSD_URL` | Yes | None | `fetch_software_catalog()` |
| `adaptiva_version_url` | `ADAPTIVA_VERSION_URL` | No | `https://raw.githubusercontent.com/PAG-IT/public-configs/refs/heads/main/cdk--drive--adaptiva-version.txt` | `fetch_adaptiva_version()` |
| Log directory | `LOG_DIR` | No | `<cwd>/cdk-updater-logs` | `init_logging()` |
| `download_dir` | `DOWNLOAD_DIR` | No | `<cwd>/cdk-updater-downloads` | `download_installer()` |
| `variables_dir` | `VARIABLES_DIR` | No | `<exe dir>/cdk-updater-variables` | `write_cdk_info_variables()` |
| Install args override | `CDK_3RD_PARTY_INSTALL_ARGS` | No | empty string | `perform_or_describe_install()` for CDK Drive 3rd Party Managed Assemblies 96.x |
| Install args override | `CDK_WEBSTART_INSTALL_ARGS` | No | `/quiet /norestart` | `perform_or_describe_install()` for CDK Drive WebStart |
| Install args override | `CDK_BLUEZONE_INSTALL_ARGS` | No | `/silent` | `perform_or_describe_install()` for BlueZone |
| Adaptiva preinstall args override | `CDK_ADAPTIVA_PREADAPTIVA_ARGS` | No | `CNUMBER=<ADAPTIVA_CNUMBER> HOST=<ADAPTIVA_HOST>` | `resolve_adaptiva_preadaptiva_args()` |
| Adaptiva client args override | `CDK_ADAPTIVA_CLIENT_ARGS` | No | `-installorupgrade -servername <ADAPTIVA_HOST> -cloudrelay -serverguid <ADAPTIVA_SERVER_GUID>` | `resolve_adaptiva_client_args()` |
| Adaptiva default C-number | `ADAPTIVA_CNUMBER` | No | `C000000` | `resolve_adaptiva_preadaptiva_args()` |
| Adaptiva default host | `ADAPTIVA_HOST` | No | `C000000-example.drive.example.com` | `resolve_adaptiva_preadaptiva_args()` / `resolve_adaptiva_client_args()` |
| Adaptiva default server GUID | `ADAPTIVA_SERVER_GUID` | No | `00000000-0000-0000-0000-000000000000` | `resolve_adaptiva_client_args()` |

## Build And Release Packaging

Purpose: build a Windows executable with the project icon embedded and publish a single executable artifact rather than the full Cargo output tree.

| File / setting | Role |
| --- | --- |
| `build.rs` | On Windows, compiles Windows resources with `winresource` and embeds `resources/logo.ico` as the executable icon. |
| `resources/logo.ico` | Application icon used by the Windows executable resource. |
| `[build-dependencies] winresource` | Build-time dependency used only for Windows resource compilation. |
| `[profile.release] opt-level = "z"` | Optimizes release builds for smaller binary size. |
| `[profile.release] lto = "thin"` | Enables thin link-time optimization for size/runtime balance without the slowest full-LTO path. |
| `[profile.release] codegen-units = 1` | Lets the optimizer see more code together for a smaller release binary. |
| `[profile.release] panic = "abort"` | Removes unwind machinery from release builds. |
| `[profile.release] strip = "symbols"` | Strips symbols from the release executable. |
| `scripts/build-release.ps1` | Runs `cargo build --release`, creates `dist/`, and copies only `target/release/cdk-drive-updater.exe` to `dist/cdk-drive-updater.exe`. |
| `.gitignore` `/dist` | Keeps generated release artifacts out of source control. |

Release artifact rule: `dist/cdk-drive-updater.exe` is the intended slim distributable. Do not copy Cargo intermediates, `resources/`, logs, `.env`, or local configuration into the release directory unless a future runtime requirement explicitly needs them.

## `main.rs`

Purpose: entry point, environment configuration, CLI mode parsing, HTTP retrieval, OSD HTML parsing, standard and Adaptiva installer orchestration, version comparison through `utils`, and logger initialization.

### Types

| Type | Visibility | Fields / variants | Description |
| --- | --- | --- | --- |
| `TargetSoftware` | private | `installed_name: &'static str`; `osd_description: &'static str`; `detect_installed: fn(&cdk_info::CdkInfo) -> Result<Option<installed::InstalledProduct>>`; `installer: InstallerKind` | Static target definition used by `process_target()`. The `installer` field decides whether a target uses the standard single-installer path or the specialized Adaptiva flow. |
| `InstallerKind` | private | `Standard { args_env_var, default_args }`; `Adaptiva { preadaptiva_override_env_var, client_override_env_var }` | Encodes per-target install behavior and argument resolution. |
| `AppMode` | private | `Query`; `Update` | Runtime mode parsed from CLI arguments. Implements `Display` as `query` or `update` for logs. |
| `AppConfig` | private | `version_source_url: String`; `adaptiva_version_url: String`; `download_dir: PathBuf`; `variables_dir: PathBuf` | Environment-derived application configuration. |
| `SoftwareEntry` | private crate root type | `category: String`; `description: String`; `version_number: String`; `file_version: String`; `silent_install_arguments: String`; `download_link: String` | Parsed OSD catalog row or synthetic Adaptiva row. `preferred_version()` returns `file_version` when present, otherwise `version_number`; `adaptiva()` builds the synthetic Adaptiva entry. |
| `CatalogColumns` | private | Optional indexes for description, version, silent-install arguments, and download columns; `version_is_file_version: bool` | Header map used by `parse_software_catalog()` so table parsing does not duplicate column-index logic. |
| `VersionState` | private | `NeedsUpdate`; `Newer`; `Same` | Comparison result for an installed version against an OSD version. |
| `SoftwareComparison` | private | `description: String`; `version: String`; `state: VersionState`; `download_link: String`; `silent_install_arguments: String` | Normalized comparison data returned by `compare_software_version()`. |
| `TargetRowDetails` | private | `osd_description`; `installed_version`; `osd_version`; `state`; `action`; `download_link`; `note`; `install_args` | Detail payload consumed by `target_row()` so target identity is filled in one place without a many-argument helper. |

### Constants

| Constant | Value |
| --- | --- |
| `ADAPTIVA_OSD_DESCRIPTION` | `CDK Software Install Agent ( Adaptiva )`. |
| `NOT_INSTALLED` | `Not installed`. |
| `NOT_FOUND_ON_OSD` | `Not found on OSD`. |
| `TARGET_SOFTWARES` | Four `TargetSoftware` entries processed in order. |

### Target Software Table

| `installed_name` | `osd_description` | Detection function | Installer |
| --- | --- | --- | --- |
| `CDK Drive 3rd Party Managed Assemblies 96.x` | `CDK Drive 3rd Party Managed Assemblies 96.x` | `installed::detect_cdk_drive_3rd_party_managed_assemblies_96x` | `InstallerKind::Standard { args_env_var: "CDK_3RD_PARTY_INSTALL_ARGS", default_args: "" }` |
| `Adaptiva` | `CDK Software Install Agent ( Adaptiva )` | `installed::detect_adaptiva` | `InstallerKind::Adaptiva { preadaptiva_override_env_var: "CDK_ADAPTIVA_PREADAPTIVA_ARGS", client_override_env_var: "CDK_ADAPTIVA_CLIENT_ARGS" }` |
| `BlueZone` | `CDK Terminal Emulator` | `installed::detect_bluezone` | `InstallerKind::Standard { args_env_var: "CDK_BLUEZONE_INSTALL_ARGS", default_args: "/silent" }` |
| `CDK Drive WebStart` | `CDK Drive WebStart` | `installed::get_webstart_installed_version_from_cdk_info` | `InstallerKind::Standard { args_env_var: "CDK_WEBSTART_INSTALL_ARGS", default_args: "/quiet /norestart" }` |

### Functions

| Function | Signature | Description |
| --- | --- | --- |
| `main` | `fn main() -> Result<()>` | Runs the full program: load config, gather local state, fetch remote state, compare targets, and log reports. |
| `AppMode::from_args` | `fn from_args(args: &[String]) -> Self` | Parses `/query`, `--query`, `-query`, `/update`, `--update`, or `-update`, case-insensitively; defaults to query. |
| `AppConfig::from_env` | `fn from_env() -> Result<Self>` | Reads required and optional environment variables, including `VARIABLES_DIR` defaulting to `<exe dir>/cdk-updater-variables`. |
| `merge_adaptiva_catalog_entry` | `fn merge_adaptiva_catalog_entry(catalog: &mut Vec<SoftwareEntry>, adaptiva_version: String)` | Updates the OSD Adaptiva entry when present or appends a synthetic Adaptiva entry. |
| `process_target` | `fn process_target(entries: &[SoftwareEntry], mode: &AppMode, target: &TargetSoftware, cdk_info: &cdk_info::CdkInfo, config: &AppConfig) -> Result<TargetComparisonRow>` | Detects a target's installed version, then delegates row creation to `installed_target_row()` or `missing_target_row()`. |
| `installed_target_row` | `fn installed_target_row(entries: &[SoftwareEntry], mode: &AppMode, target: &TargetSoftware, product: installed::InstalledProduct, config: &AppConfig) -> TargetComparisonRow` | Builds the comparison row for an installed target. Adaptiva never re-installs over an existing install; it reports `Install skipped: already installed` while still surfacing the resolved args in query mode. |
| `missing_target_row` | `fn missing_target_row(entries: &[SoftwareEntry], mode: &AppMode, target: &TargetSoftware, config: &AppConfig) -> TargetComparisonRow` | Builds the comparison row for a missing target, including install action when the OSD entry exists. |
| `is_adaptiva_target` | `fn is_adaptiva_target(target: &TargetSoftware) -> bool` | Identifies targets that use the specialized Adaptiva installer path. |
| `current_target_action` | `fn current_target_action(mode: &AppMode, target: &TargetSoftware, osd_args: &str) -> (String, String)` | Returns `"No update required"` and, in query mode, the install args that would be used later. |
| `current_target_install_args` | `fn current_target_install_args(mode: &AppMode, target: &TargetSoftware, osd_args: &str) -> String` | Returns query-mode install args for display and an empty string in update mode. |
| `target_row` | `fn target_row(target: &TargetSoftware, details: TargetRowDetails) -> TargetComparisonRow` | Shared row constructor that fills the repeated target identity fields consistently. |
| `resolve_target_install_args` | `fn resolve_target_install_args(target: &TargetSoftware, osd_args: &str) -> String` | Resolves standard installer args from an env override or app-owned defaults. For Adaptiva, builds a combined display string containing the effective `preadaptiva.msi` and `AdaptivaClientSetup.exe` arguments. |
| `perform_or_describe_install` | `fn perform_or_describe_install(target: &TargetSoftware, mode: &AppMode, download_link: &str, osd_args: &str, config: &AppConfig, operation: &str) -> (String, String)` | Returns `(action, install_args)`. In query mode, returns a description of what would happen. In update mode, dispatches to `actually_install()` for standard targets or `actually_install_adaptiva()` for Adaptiva. |
| `resolve_adaptiva_preadaptiva_args` | `fn resolve_adaptiva_preadaptiva_args(override_env_var: &str) -> String` | Builds `preadaptiva.msi` arguments from an override or from `ADAPTIVA_CNUMBER` plus `ADAPTIVA_HOST`. |
| `resolve_adaptiva_client_args` | `fn resolve_adaptiva_client_args(override_env_var: &str) -> String` | Builds `AdaptivaClientSetup.exe` arguments from an override or from `ADAPTIVA_HOST` plus `ADAPTIVA_SERVER_GUID`. |
| `adaptiva_cnumber` | `fn adaptiva_cnumber() -> String` | Returns the effective Adaptiva C-number with a fallback placeholder. |
| `adaptiva_host` | `fn adaptiva_host() -> String` | Returns the effective Adaptiva host/server name with a fallback placeholder. |
| `adaptiva_server_guid` | `fn adaptiva_server_guid() -> String` | Returns the effective Adaptiva server GUID with a fallback placeholder. |
| `actually_install_adaptiva` | `fn actually_install_adaptiva(osd_url: &str, download_dir: &Path, preadaptiva_override_env_var: &str, client_override_env_var: &str) -> String` | Rewrites the OSD URL to the Adaptiva zip download URL, downloads the zip, extracts it, runs `preadaptiva.msi`, runs `AdaptivaClientSetup.exe`, cleans up, and returns a human-readable outcome string. |
| `adaptiva_zip_download_url` | `fn adaptiva_zip_download_url(osd_url: &str) -> String` | Replaces `/index.php` with `/download.php` for the Adaptiva package download path. |
| `evaluate_install_status` | `fn evaluate_install_status(label: &str, path: &Path, status: std::process::ExitStatus) -> Result<()>` | Treats exit code `3010` as success with reboot required and converts failing installer statuses into contextual errors. |
| `actually_install` | `fn actually_install(url: &str, args: &str, download_dir: &Path) -> String` | Downloads the installer; kills `wsStart_4.exe` and waits 20 s only if it was killed; kills `wsStartChrome.exe`; runs the installer; deletes the file; returns a human-readable outcome string. |
| `kill_process_if_running` | `fn kill_process_if_running(process_name: &str)` | Runs `taskkill /F /IM <name>`; logs success, not-running (exit 128), or failure. |
| `download_installer` | `fn download_installer(url: &str, download_dir: &Path) -> Result<PathBuf>` | GETs the installer URL, checks HTTP status, writes the body to `download_dir/<filename>`, logs progress, guards zero-byte `Content-Length`, and returns the path. |
| `run_installer` | `fn run_installer(path: &Path, args: &str) -> Result<std::process::ExitStatus>` | Splits `args` with `split_install_args()` and runs the installer via `Command::new`, waiting for completion. |
| `split_install_args` | `fn split_install_args(args: &str) -> Vec<String>` | Tokenises an installer argument string by whitespace, respecting double-quoted substrings as single tokens. |
| `extract_filename_from_url` | `fn extract_filename_from_url(url: &str) -> Option<String>` | Returns the last URL path segment for use as a local filename, except `download.php`, which is rewritten to `<parent-segment>.zip` so Adaptiva downloads land with a stable zip filename. |
| `extract_zip` | `fn extract_zip(zip_path: &Path, destination: &Path) -> Result<()>` | Extracts a zip archive into `destination`, skipping unsafe entries whose names escape the archive root. |
| `write_cdk_info_variables` | `fn write_cdk_info_variables(info: &cdk_info::CdkInfo, dir: &Path) -> Result<()>` | Creates `dir`; iterates `app_logging::cdk_info_entries(info)`, writing each lowercased `<utils::safe_filename_token(check)>.txt` via `utils::replace_file()`; writes `summary.txt`; writes `last-run.txt` whose content is `<utils::build_timestamp(now)>--<epoch>`. |
| `fetch_adaptiva_version` | `fn fetch_adaptiva_version(url: &str) -> Result<Option<String>>` | Fetches and trims a plain-text Adaptiva version; returns `None` for empty content. |
| `fetch_software_catalog` | `fn fetch_software_catalog(source_url: &str) -> Result<Vec<SoftwareEntry>>` | Fetches OSD HTML and parses it into catalog entries. |
| `parse_software_catalog` | `fn parse_software_catalog(html: &str, base_url: &Url) -> Result<Vec<SoftwareEntry>>` | Parses OSD category/table markup into `SoftwareEntry` values. |
| `CatalogColumns::from_header_row` | `fn from_header_row(header_row: ElementRef<'_>, header_selector: &Selector) -> Self` | Maps OSD table headers to reusable column indexes for catalog row parsing. |
| `next_table_sibling` | `fn next_table_sibling(category_element: ElementRef<'_>) -> Option<ElementRef<'_>>` | Finds the table immediately following a category `div`. |
| `collect_text` | `fn collect_text(element: ElementRef<'_>) -> String` | Normalizes all text within an element by collapsing whitespace. |
| `value_at_index` | `fn value_at_index(cells: &[ElementRef<'_>], index: Option<usize>) -> String` | Returns normalized cell text for an optional column index. |
| `resolve_link` | `fn resolve_link(base_url: &Url, href: &str) -> String` | Resolves relative download links against the final OSD response URL. |
| `get_software_by_description` | `fn get_software_by_description<'a>(entries: &'a [SoftwareEntry], description: &str) -> Option<&'a SoftwareEntry>` | Finds a catalog entry by case-insensitive description. |
| `compare_software_version` | `fn compare_software_version(entries: &[SoftwareEntry], description: &str, provided_version: &str) -> Option<SoftwareComparison>` | Compares an installed version to the matching catalog version. |
| `version_state_as_str` | `fn version_state_as_str(state: &VersionState) -> &'static str` | Converts `VersionState` to the human log string. |
| `log_post_run_summary` | `fn log_post_run_summary(mode: &AppMode, pre_rows: &[TargetComparisonRow], variables_dir: &Path)` | Writes a human-readable Post-Run Summary after all comparison tables. Lists install actions with SUCCESS/FAILED verdicts in update mode; advises how to update in query mode; always points to the variables directory. |
| `init_logging` | `fn init_logging() -> Result<PathBuf>` | Creates the log path and configures stdout plus file logging. |

### Key Algorithms

HTML catalog parsing: `parse_software_catalog()` selects `div.category` elements, finds each adjacent `table`, uses `CatalogColumns` to map table headers by lowercase text (`description`, `version`, `file version`, `silent install arguments`, `download`/`link`), skips rows without a description, resolves anchor `href` values against the final response URL, and treats a `File Version` header as both `version_number` and `file_version`.

Version comparison ordering: `compare_software_version()` delegates ordering to `utils::compare_versions()`, which maps `version_compare` results to `std::cmp::Ordering` and falls back to trimmed string ordering when parsing fails.

Update mode behavior: `process_target()` delegates installed/missing cases to row helpers, which delegate install/update decisions to `perform_or_describe_install()`. For standard targets in query mode, the action is `"Would download and install/update"` and `install_args` shows the effective args resolved from env overrides or app-owned defaults. In update mode, `actually_install()` is called: it downloads the installer to `DOWNLOAD_DIR`; kills `wsStart_4.exe` via `taskkill /F /IM` and waits 20 seconds only when that process was killed; kills `wsStartChrome.exe` via `taskkill /F /IM`; runs the installer via `Command::new` with `split_install_args()` tokens; deletes the file (even on failure); and returns an outcome such as `"Installed"`, `"Installed - reboot required"`, or `"Install failed: ..."`.

Adaptiva update behavior: missing Adaptiva installs also go through `perform_or_describe_install()`, but the target uses `InstallerKind::Adaptiva`. Query mode reports that the tool would download and install, and `install_args` shows a combined summary for `preadaptiva.msi` and `AdaptivaClientSetup.exe`. Update mode rewrites the OSD URL to `download.php`, downloads the zip, extracts it into `DOWNLOAD_DIR`, runs `preadaptiva.msi`, then `AdaptivaClientSetup.exe`, treats exit code `3010` as success, and removes both the zip file and the extraction directory afterward. If Adaptiva is already installed, `installed_target_row()` reports `Install skipped: already installed` instead of reinstalling over it.

Pre-install process termination: `kill_process_if_running()` calls `taskkill /F /IM <name>`. A zero exit from `taskkill` is logged as a successful kill and returns `true`. Exit code 128 means the process was not running and is logged at info level without warning. Any other non-zero exit or launch failure is logged as a warning. The 20-second sleep runs only after `wsStart_4.exe` was found and killed.

## `installed.rs`

Purpose: Windows Add/Remove Programs, MSI registry, and executable file-version detection for target software.

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
| `detect_cdk_drive_3rd_party_managed_assemblies_96x` | `pub fn detect_cdk_drive_3rd_party_managed_assemblies_96x(cdk_info: &CdkInfo) -> Result<Option<InstalledProduct>>` | Target adapter that returns the version cached in `cdk_info.cdk_3rd_party_assemblies_version` via `product_from_reported_version`. |
| `detect_adaptiva` | `pub fn detect_adaptiva(cdk_info: &CdkInfo) -> Result<Option<InstalledProduct>>` | Target adapter that returns the version cached in `cdk_info.adaptiva_installed_version` via `product_from_reported_version`. |
| `detect_bluezone` | `pub fn detect_bluezone(cdk_info: &CdkInfo) -> Result<Option<InstalledProduct>>` | Target adapter that returns the version cached in `cdk_info.bluezone_version` via `product_from_reported_version`. |
| `get_webstart_installed_version_from_cdk_info` | `pub fn get_webstart_installed_version_from_cdk_info(cdk_info: &CdkInfo) -> Result<Option<InstalledProduct>>` | Returns WebStart version from `CdkInfo`, preferring Add/Remove MSI version over executable file version. |
| `get_installed_version` | `pub fn get_installed_version(name_contains: &str) -> Result<Option<InstalledProduct>>` | Scans Add/Remove uninstall keys for `DisplayVersion`, scans `HKCR\Installer\Products` as a fallback source, and returns the highest matching version. |
| `read_executable_file_version` | `pub(crate) fn read_executable_file_version(path: &Path) -> Result<Option<String>>` | Reads Windows fixed file-version metadata from an executable. |

### Key Internal Functions

| Function | Signature | Description |
| --- | --- | --- |
| `InstalledProduct::new` | `fn new(product_name: impl Into<String>, version: impl Into<String>) -> Self` | Internal constructor used by registry, executable, and cached-report detectors. |
| `installed_products_from_executables` | `fn installed_products_from_executables(component_name: &str, executable_paths: Vec<PathBuf>) -> Vec<InstalledProduct>` | Reads file versions from executable paths, logs unreadable files, and returns installed-product rows. |
| `product_from_reported_version` | `fn product_from_reported_version(product_name: &str, version: &str) -> Option<InstalledProduct>` | Converts cached CDK info version strings into installed products unless the value is empty or a known not-found token. |
| `find_bluezone_executables` | `fn find_bluezone_executables() -> Vec<PathBuf>` | Walks `BlueZone` directories under Program Files roots looking for `bzvt.exe`. |
| `find_adaptiva_executables` | `fn find_adaptiva_executables() -> Vec<PathBuf>` | Checks expected Adaptiva `OneSiteClient.exe` paths under Program Files roots. |
| `candidate_program_files_roots` | `fn candidate_program_files_roots() -> Vec<PathBuf>` | Combines Program Files environment variables with fixed fallback paths, sorted and deduplicated. |
| `to_wide` | `fn to_wide(value: &OsStr) -> Vec<u16>` | Converts a path/string to a null-terminated UTF-16 buffer for Win32 APIs. |
| `format_fixed_file_version` | `fn format_fixed_file_version(version_info: &VS_FIXEDFILEINFO) -> String` | Formats Win32 fixed file-version fields as `major.minor.build.revision`. |
| `select_highest_version` | `fn select_highest_version(mut matches: Vec<InstalledProduct>) -> Option<InstalledProduct>` | Sorts descending by version and returns the first product. |
| `decode_msi_version` | `fn decode_msi_version(product_name: &str, version_int: u32) -> String` | Extracts `V-` versions from product names before falling back to MSI DWORD decoding. |
| `extract_version_from_name` | `fn extract_version_from_name(name: &str) -> Option<String>` | Extracts dot-separated digits after `V-` when the result contains at least one dot. |

### Key Algorithms

Installed app version detection: `get_installed_version()` first scans Add/Remove uninstall keys under HKLM native, HKLM WOW6432Node, and HKCU, filters `DisplayName` by tolerant case-insensitive matching, and uses `DisplayVersion` so four-part versions such as `104.21.517.125` are preserved. It also scans `HKEY_CLASSES_ROOT\Installer\Products`, filters `ProductName`, reads the MSI `Version` DWORD, and passes `(ProductName, Version)` to `decode_msi_version()` as a fallback source. `decode_msi_version()` first tries to extract a product-name version such as `V-104.21.517.125`; if absent, it unpacks the DWORD as `major = top byte`, `minor = next byte`, and `build = low word`.

`V-` prefix extraction: `extract_version_from_name()` finds `v-` case-insensitively, takes following ASCII digits and dots, trims trailing dots, and accepts the result only when it contains at least one dot.

Executable file version reading: `read_executable_file_version()` converts the path to UTF-16, calls `GetFileVersionInfoSizeW`, loads the version resource with `GetFileVersionInfoW`, queries the root block with `VerQueryValueW`, casts to `VS_FIXEDFILEINFO`, and formats `dwFileVersionMS` / `dwFileVersionLS` as `major.minor.build.revision`.

Highest-version selection: detected products are sorted descending with `utils::compare_versions()`, then the first entry is used. Adaptiva prefers executable metadata over Add/Remove MSI data because `get_adaptiva_installed_version()` returns `executable_match.or(add_remove_match)`.

Adapter caching: `detect_cdk_drive_3rd_party_managed_assemblies_96x`, `detect_adaptiva`, and `detect_bluezone` read from the pre-populated `CdkInfo` snapshot (populated by `cdk_info::gather()`) instead of re-running independent registry and filesystem scans. This avoids redundant work for every call to `TARGET_SOFTWARES` processing and aligns with the pattern used by `get_webstart_installed_version_from_cdk_info`.

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
| `cdk_3rd_party_assemblies_version` | `String` | Detected installed version for CDK Drive 3rd Party Managed Assemblies 96.x, or `NotFound`. |
| `adaptiva_installed_version` | `String` | Detected installed version for Adaptiva (executable or Add/Remove MSI), or `NotFound`. |
| `bluezone_version` | `String` | Detected installed version for BlueZone (`bzvt.exe`), or `NotFound`. |

### Constants

| Constant group | Description |
| --- | --- |
| `ADAPTIVA_CLIENT_NATIVE`, `ADAPTIVA_CLIENT_WOW` | Native and WOW6432Node Adaptiva client registry paths reused by all Adaptiva value reads. |
| `CDK_ADAPTIVA_NATIVE`, `CDK_ADAPTIVA_WOW` | Native and WOW6432Node CDK Adaptiva registry paths used for recursive value capture. |
| `WEBSTART_EXE_PATH` | Fixed path for `CDK Drive WebStart.exe` executable version reading. |
| `SIA_DIR`, `SIA_XML_PATH`, `SIA_FIX_PATH` | Fixed SIA filesystem paths checked by `gather()`. |

### Public Functions

| Function | Signature | Description |
| --- | --- | --- |
| `gather` | `pub fn gather() -> CdkInfo` | Performs all CDK registry, path, Adaptiva, and WebStart checks and maps failures to status values. |

### Key Internal Functions

| Function | Signature | Description |
| --- | --- | --- |
| `read_adaptiva_client_value_pair` | `fn read_adaptiva_client_value_pair(hive: &RegKey, value_name: &str) -> (String, String)` | Reads a named Adaptiva client value from both native and WOW6432Node registry paths. |
| `read_webstart_executable_version` | `fn read_webstart_executable_version() -> String` | Reads the WebStart executable file version or returns `utils::NOT_FOUND_COMPACT`. |
| `read_webstart_add_remove_version` | `fn read_webstart_add_remove_version() -> String` | Reads the WebStart Add/Remove MSI version or returns `utils::NOT_FOUND_COMPACT`. |
| `detect_installed_version` | `fn detect_installed_version<F>(detector: F) -> String` | Calls a zero-argument `installed` detector, returns the version string on success or `utils::NOT_FOUND_COMPACT` on any failure or absent result. |
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

## `utils.rs`

Purpose: shared helpers for behavior that is needed by multiple modules or was previously repeated inside `main.rs`, `installed.rs`, and `cdk_info.rs`.

### Constants

| Constant | Value | Description |
| --- | --- | --- |
| `NOT_FOUND_COMPACT` | `NotFound` | Compact not-found token used by cached version fields. |
| `NOT_FOUND_DISPLAY` | `Not Found` | Human-readable not-found token used in tables and registry string reads. |

### Functions

| Function | Signature | Description |
| --- | --- | --- |
| `cwd_child` | `fn cwd_child(name: &str) -> PathBuf` | Returns `<cwd>/<name>`, falling back to `.` when cwd cannot be read. |
| `exe_dir_child` | `fn exe_dir_child(name: &str) -> PathBuf` | Returns `<exe dir>/<name>`, falling back to `.` when the executable directory cannot be read. |
| `env_path_or_else` | `fn env_path_or_else(var_name: &str, default: impl FnOnce() -> PathBuf) -> PathBuf` | Reads a non-empty path environment variable or returns the supplied default path. |
| `non_empty_env_var` | `fn non_empty_env_var(var_name: &str) -> Option<String>` | Reads and trims an environment variable, returning `None` for missing or empty values. |
| `compare_versions` | `fn compare_versions(left: &str, right: &str) -> Ordering` | Uses `version_compare`; falls back to trimmed string ordering when parsing fails. |
| `is_missing_value` | `fn is_missing_value(value: &str) -> bool` | Returns true for empty strings, `NotFound`, or `Not Found`, case-insensitively. |
| `replace_file` | `fn replace_file(path: &Path, contents: impl AsRef<[u8]>) -> Result<()>` | Deletes an existing file via `delete_if_exists()` and writes new contents with path context. |
| `delete_if_exists` | `fn delete_if_exists(path: &Path)` | Deletes `path` when it exists; logs warnings on deletion or existence-check failure. |
| `safe_filename_token` | `fn safe_filename_token(name: &str) -> String` | Replaces non-alphanumeric, non-dot, non-hyphen characters with collapsed underscores, trims underscores, and lowercases the result. |
| `build_timestamp` | `fn build_timestamp(now: chrono::DateTime<Local>) -> String` | Formats timestamps as `YYYY-MM-DD--H-MM-am/pm` for log filenames and marker content. |

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
| `check_row` | `fn check_row(label: &str, value: impl Into<String>) -> (String, String)` | Shared constructor for CDK Installation Info rows. |
| `expand_key_value_rows` | `fn expand_key_value_rows(label: &str, values: &Option<Vec<(String, String)>>, rows: &mut Vec<(String, String)>)` | Expands optional registry value vectors into `(check, result)` pairs for `cdk_info_entries`. |
| `build_ascii_table` | `fn build_ascii_table(title: &str, headers: &[&str], rows: &[Vec<String>]) -> String` | Builds a titled ASCII table with separators, header row, and data rows. |
| `build_ascii_row` | `fn build_ascii_row(cells: &[String], widths: &[usize]) -> String` | Renders one padded pipe-delimited table row. |
| `compute_widths` | `fn compute_widths(headers: &[&str], rows: &[Vec<String>]) -> Vec<usize>` | Computes per-column widths from headers and row content. |
| `clean_table_cell` | `fn clean_table_cell(cell: &str) -> String` | Normalizes embedded CR/LF characters to spaces before measuring or rendering table cells. |
| `build_separator` | `fn build_separator(widths: &[usize]) -> String` | Builds a `+---+` separator row. |

### Key Algorithms

ASCII table rendering: `build_ascii_table()` computes max widths across headers and rows, emits the title and blank line, renders separators around the header and every row, normalizes embedded CR/LF characters to spaces, and returns a string ending with two newlines.

Catalog logging: `log_osd_catalog()` emits a compact Core table using `SoftwareEntry::preferred_version()`, a Details table containing silent install arguments and download links, and a Summary table containing total entry count.

Target comparison logging: `log_target_comparisons()` splits each `TargetComparisonRow` into a Summary table for target/version/state/action and a Details table for download links and notes.
