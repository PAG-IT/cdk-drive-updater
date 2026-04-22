use std::cmp::Ordering;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

mod app_logging;
mod installed;
mod cdk_info;

use app_logging::TargetComparisonRow;

use anyhow::{Context, Result};
use chrono::{Local, Timelike};
use reqwest::Url;
use scraper::{ElementRef, Html, Selector};
use version_compare::Cmp;

struct TargetSoftware {
    installed_name: &'static str,
    osd_description: &'static str,
    detect_installed: fn(&cdk_info::CdkInfo) -> Result<Option<installed::InstalledProduct>>,
    //=-- ENV var name that overrides the OSD silent install arguments; None = no automated install.
    install_args_env_var: Option<&'static str>,
}

const TARGET_SOFTWARES: [TargetSoftware; 4] = [
    TargetSoftware {
        installed_name: "CDK Drive 3rd Party Managed Assemblies 96.x",
        osd_description: "CDK Drive 3rd Party Managed Assemblies 96.x",
        detect_installed: installed::detect_cdk_drive_3rd_party_managed_assemblies_96x,
        install_args_env_var: Some("CDK_3RD_PARTY_INSTALL_ARGS"),
    },
    TargetSoftware {
        installed_name: "Adaptiva",
        osd_description: "CDK Software Install Agent ( Adaptiva )",
        detect_installed: installed::detect_adaptiva,
        //=-- Adaptiva is managed externally (CDK SIA); this tool does not install it.
        install_args_env_var: None,
    },
    TargetSoftware {
        installed_name: "BlueZone",
        osd_description: "CDK Terminal Emulator",
        detect_installed: installed::detect_bluezone,
        install_args_env_var: Some("CDK_BLUEZONE_INSTALL_ARGS"),
    },
    TargetSoftware {
        installed_name: "CDK Drive WebStart",
        osd_description: "CDK Drive WebStart",
        detect_installed: installed::get_webstart_installed_version_from_cdk_info,
        install_args_env_var: Some("CDK_WEBSTART_INSTALL_ARGS"),
    },
];

#[derive(Debug, Clone, PartialEq)]
enum AppMode {
    Query,
    Update,
}

impl AppMode {
    /// Parses the run mode from the provided CLI argument list.
    ///
    /// Accepts any casing of `/query`, `--query`, `-query`, `/update`, `--update`, or `-update`.
    /// Defaults to [`AppMode::Query`] when no recognised flag is present.
    fn from_args(args: &[String]) -> Self {
        for arg in args {
            let normalised = arg
                .trim_start_matches('/')
                .trim_start_matches('-')
                .to_ascii_lowercase();
            match normalised.as_str() {
                "update" => return AppMode::Update,
                "query" => return AppMode::Query,
                _ => {}
            }
        }
        AppMode::Query
    }
}

#[derive(Debug)]
struct AppConfig {
    version_source_url: String,
    adaptiva_version_url: String,
    download_dir: PathBuf,
}

#[derive(Debug, Clone)]
struct SoftwareEntry {
    category: String,
    description: String,
    version_number: String,
    file_version: String,
    silent_install_arguments: String,
    download_link: String,
}

#[allow(dead_code)]
#[derive(Debug)]
enum VersionState {
    NeedsUpdate,
    Newer,
    Same,
}

#[allow(dead_code)]
#[derive(Debug)]
struct SoftwareComparison {
    description: String,
    version: String,
    state: VersionState,
    download_link: String,
    silent_install_arguments: String,
}

impl AppConfig {
    fn from_env() -> Result<Self> {
        let version_source_url = env::var("CDK_DRIVE_OSD_URL")
            .context("missing env var CDK_DRIVE_OSD_URL")?;
        let adaptiva_version_url = env::var("ADAPTIVA_VERSION_URL")
            .unwrap_or_else(|_| "https://raw.githubusercontent.com/PAG-IT/public-configs/refs/heads/main/cdk--drive--adaptiva-version.txt".to_string());
        let download_dir = env::var("DOWNLOAD_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                env::current_dir()
                    .unwrap_or_else(|_| PathBuf::from("."))
                    .join("cdk-updater-downloads")
            });

        Ok(Self {
            version_source_url,
            adaptiva_version_url,
            download_dir,
        })
    }
}

fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    let log_file_path = init_logging()?;
    let config = AppConfig::from_env()?;

    let args: Vec<String> = env::args().collect();
    let mode = AppMode::from_args(&args[1..]);

    //=-- Log the app mode prominently at the very start.
    app_logging::log_app_mode(app_mode_as_str(&mode));

    app_logging::log_startup_summary(
        &log_file_path,
        app_mode_as_str(&mode),
        &config.version_source_url,
        config.download_dir.to_string_lossy().as_ref(),
    );

    //=-- Gather and display current CDK installation state before processing targets.
    let cdk_info = cdk_info::gather();
    app_logging::log_cdk_info_summary(&cdk_info);

    let catalog = fetch_software_catalog(&config.version_source_url)?;
    let mut catalog = catalog;
    let adaptiva_remote_version = fetch_adaptiva_version(&config.adaptiva_version_url)
        .unwrap_or_else(|e| {
            log::warn!("Failed to fetch remote Adaptiva version: {}", e);
            None
        });
    app_logging::log_adaptiva_remote_version(&config.adaptiva_version_url, &adaptiva_remote_version);
    if let Some(adaptiva_version) = adaptiva_remote_version {
        let adaptiva_description = "CDK Software Install Agent ( Adaptiva )";
        //=-- Search for existing Adaptiva entry in catalog and update it; if not found, append.
        if let Some(entry) = catalog.iter_mut().find(|e| e.description == adaptiva_description) {
            entry.version_number = adaptiva_version.clone();
            entry.file_version = adaptiva_version;
        } else {
            catalog.push(SoftwareEntry {
                category: "Adaptiva".to_string(),
                description: adaptiva_description.to_string(),
                version_number: adaptiva_version.clone(),
                file_version: adaptiva_version,
                silent_install_arguments: String::new(),
                download_link: String::new(),
            });
        }
    }
    app_logging::log_osd_catalog(&catalog);

    let mut comparison_rows = Vec::new();
    for target in TARGET_SOFTWARES {
        comparison_rows.push(process_target(&catalog, mode.clone(), &target, &cdk_info, &config)?);
    }
    app_logging::log_target_comparisons(&comparison_rows);

    Ok(())
}

fn app_mode_as_str(mode: &AppMode) -> &'static str {
    match mode {
        AppMode::Query => "query",
        AppMode::Update => "update",
    }
}

fn process_target(
    entries: &[SoftwareEntry],
    mode: AppMode,
    target: &TargetSoftware,
    cdk_info: &cdk_info::CdkInfo,
    config: &AppConfig,
) -> Result<TargetComparisonRow> {
    let installed = (target.detect_installed)(cdk_info)?;
    match installed {
        Some(product) => {
            if let Some(result) = compare_software_version(entries, target.osd_description, &product.version) {
                let (action, install_args) = if matches!(result.state, VersionState::NeedsUpdate) {
                    perform_or_describe_install(
                        target, &mode,
                        &result.download_link,
                        &result.silent_install_arguments,
                        config, "update",
                    )
                } else {
                    //=-- In query mode, still resolve install args so the table shows exactly
                    //=-- what would be used if this target ever needed updating.
                    let args = if mode == AppMode::Query {
                        resolve_target_install_args(target, &result.silent_install_arguments)
                    } else {
                        String::new()
                    };
                    ("No update required".to_string(), args)
                };

                Ok(TargetComparisonRow {
                    target: target.installed_name.to_string(),
                    osd_description: result.description,
                    installed_version: product.version,
                    osd_version: result.version,
                    state: version_state_as_str(&result.state).to_string(),
                    action,
                    download_link: result.download_link,
                    note: String::new(),
                    install_args,
                })
            } else {
                Ok(TargetComparisonRow {
                    target: target.installed_name.to_string(),
                    osd_description: target.osd_description.to_string(),
                    installed_version: product.version,
                    osd_version: "Not found on OSD".to_string(),
                    state: "Unknown".to_string(),
                    action: "Cannot compare".to_string(),
                    download_link: String::new(),
                    note: "OSD comparison skipped: target software not found on page".to_string(),
                    install_args: String::new(),
                })
            }
        }
        None => {
            if let Some(entry) = get_software_by_description(entries, target.osd_description) {
                let osd_version = if entry.file_version.is_empty() {
                    entry.version_number.clone()
                } else {
                    entry.file_version.clone()
                };

                let (action, install_args) = perform_or_describe_install(
                    target, &mode,
                    &entry.download_link,
                    &entry.silent_install_arguments,
                    config, "install",
                );

                Ok(TargetComparisonRow {
                    target: target.installed_name.to_string(),
                    osd_description: target.osd_description.to_string(),
                    installed_version: "Not installed".to_string(),
                    osd_version,
                    state: "Missing".to_string(),
                    action,
                    download_link: entry.download_link.clone(),
                    note: String::new(),
                    install_args,
                })
            } else {
                Ok(TargetComparisonRow {
                    target: target.installed_name.to_string(),
                    osd_description: target.osd_description.to_string(),
                    installed_version: "Not installed".to_string(),
                    osd_version: "Not found on OSD".to_string(),
                    state: "Missing".to_string(),
                    action: "Unavailable".to_string(),
                    download_link: String::new(),
                    note: "Target software not installed and not found on OSD page".to_string(),
                    install_args: String::new(),
                })
            }
        }
    }
}

fn fetch_adaptiva_version(url: &str) -> Result<Option<String>> {
    let response = reqwest::blocking::get(url)
        .with_context(|| format!("failed to fetch Adaptiva version URL: {}", url))?;
    let text = response
        .text()
        .context("failed to read Adaptiva version response")?;
    let version = text.trim().to_string();
    if version.is_empty() {
        Ok(None)
    } else {
        Ok(Some(version))
    }
}

fn fetch_software_catalog(source_url: &str) -> Result<Vec<SoftwareEntry>> {
    let response = reqwest::blocking::get(source_url)
        .with_context(|| format!("failed to request OSD URL: {}", source_url))?;
    let final_url = response.url().clone();
    let html = response
        .text()
        .context("failed to read OSD HTML response")?;

    parse_software_catalog(&html, &final_url)
}

fn parse_software_catalog(html: &str, base_url: &Url) -> Result<Vec<SoftwareEntry>> {
    let document = Html::parse_document(html);
    let category_selector = Selector::parse("div.category")
        .map_err(|_| anyhow::anyhow!("failed to create category selector"))?;
    let row_selector =
        Selector::parse("tr").map_err(|_| anyhow::anyhow!("failed to create row selector"))?;
    let header_selector =
        Selector::parse("th").map_err(|_| anyhow::anyhow!("failed to create header selector"))?;
    let cell_selector =
        Selector::parse("td").map_err(|_| anyhow::anyhow!("failed to create cell selector"))?;
    let link_selector =
        Selector::parse("a").map_err(|_| anyhow::anyhow!("failed to create link selector"))?;

    let mut entries = Vec::new();

    for category_element in document.select(&category_selector) {
        let category = collect_text(category_element);
        let Some(table) = next_table_sibling(category_element) else {
            continue;
        };

        let mut header_index_description = None;
        let mut header_index_version = None;
        let mut header_index_args = None;
        let mut header_index_download = None;
        let mut version_header_is_file_version = false;

        if let Some(header_row) = table.select(&row_selector).next() {
            let headers: Vec<String> = header_row
                .select(&header_selector)
                .map(collect_text)
                .collect();

            for (index, header) in headers.iter().enumerate() {
                let normalized = header.to_ascii_lowercase();
                if normalized.contains("description") {
                    header_index_description = Some(index);
                }
                if normalized.contains("version") {
                    header_index_version = Some(index);
                    if normalized.contains("file version") {
                        version_header_is_file_version = true;
                    }
                }
                if normalized.contains("silent install arguments") {
                    header_index_args = Some(index);
                }
                if normalized.contains("download") || normalized.contains("link") {
                    header_index_download = Some(index);
                }
            }
        }

        for row in table.select(&row_selector).skip(1) {
            let cells: Vec<ElementRef<'_>> = row.select(&cell_selector).collect();
            if cells.is_empty() {
                continue;
            }

            let description = value_at_index(&cells, header_index_description);
            if description.is_empty() {
                continue;
            }

            let version = value_at_index(&cells, header_index_version);
            let silent_install_arguments = value_at_index(&cells, header_index_args);

            let download_link = if let Some(index) = header_index_download {
                if let Some(cell) = cells.get(index) {
                    if let Some(anchor) = cell.select(&link_selector).next() {
                        let href = anchor.value().attr("href").unwrap_or_default();
                        resolve_link(base_url, href)
                    } else {
                        String::new()
                    }
                } else {
                    String::new()
                }
            } else {
                String::new()
            };

            let (version_number, file_version) = if version_header_is_file_version {
                (version.clone(), version)
            } else {
                (version, String::new())
            };

            entries.push(SoftwareEntry {
                category: category.clone(),
                description,
                version_number,
                file_version,
                silent_install_arguments,
                download_link,
            });
        }
    }

    Ok(entries)
}

fn next_table_sibling(category_element: ElementRef<'_>) -> Option<ElementRef<'_>> {
    let mut next_node = category_element.next_sibling();
    while let Some(node) = next_node {
        if let Some(element) = ElementRef::wrap(node) {
            if element.value().name() == "table" {
                return Some(element);
            }
        }
        next_node = node.next_sibling();
    }
    None
}

fn collect_text(element: ElementRef<'_>) -> String {
    element
        .text()
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<&str>>()
        .join(" ")
}

fn value_at_index(cells: &[ElementRef<'_>], index: Option<usize>) -> String {
    if let Some(index) = index {
        if let Some(cell) = cells.get(index) {
            return collect_text(*cell);
        }
    }
    String::new()
}

fn resolve_link(base_url: &Url, href: &str) -> String {
    if href.trim().is_empty() {
        return String::new();
    }

    match base_url.join(href) {
        Ok(url) => url.to_string(),
        Err(_) => href.to_string(),
    }
}

fn get_software_by_description<'a>(
    entries: &'a [SoftwareEntry],
    description: &str,
) -> Option<&'a SoftwareEntry> {
    entries
        .iter()
        .find(|entry| entry.description.eq_ignore_ascii_case(description))
}

fn compare_software_version(
    entries: &[SoftwareEntry],
    description: &str,
    provided_version: &str,
) -> Option<SoftwareComparison> {
    let entry = get_software_by_description(entries, description)?;
    let target_version = if !entry.file_version.is_empty() {
        entry.file_version.clone()
    } else {
        entry.version_number.clone()
    };

    let ordering = compare_versions(provided_version, &target_version);
    let state = match ordering {
        Ordering::Less => VersionState::NeedsUpdate,
        Ordering::Greater => VersionState::Newer,
        Ordering::Equal => VersionState::Same,
    };

    Some(SoftwareComparison {
        description: entry.description.clone(),
        version: target_version,
        state,
        download_link: entry.download_link.clone(),
        silent_install_arguments: entry.silent_install_arguments.clone(),
    })
}

#[allow(dead_code)]
fn compare_versions(provided: &str, target: &str) -> Ordering {
    match version_compare::compare(provided, target) {
        Ok(Cmp::Lt) => Ordering::Less,
        Ok(Cmp::Gt) => Ordering::Greater,
        Ok(Cmp::Eq) => Ordering::Equal,
        _ => provided.trim().cmp(target.trim()),
    }
}

fn version_state_as_str(state: &VersionState) -> &'static str {
    match state {
        VersionState::NeedsUpdate => "Older (Web is newer)",
        VersionState::Newer => "Newer (Installed is newer)",
        VersionState::Same => "Same",
    }
}

/// Resolves the effective install arguments for a target without performing any action.
///
/// Returns the ENV override when set, the OSD args otherwise, or an empty string for
/// targets with no automated install support.
fn resolve_target_install_args(target: &TargetSoftware, osd_args: &str) -> String {
    let Some(env_var) = target.install_args_env_var else {
        return String::new();
    };
    env::var(env_var)
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| osd_args.to_string())
}

/// Determines the action string and effective install args for a target that needs to be
/// installed or updated.
///
/// In query mode, returns a description of what *would* happen without performing any action.
/// In update mode, downloads the installer, runs it, cleans up, and returns the outcome.
fn perform_or_describe_install(
    target: &TargetSoftware,
    mode: &AppMode,
    download_link: &str,
    osd_args: &str,
    config: &AppConfig,
    operation: &str,
) -> (String, String) {
    let Some(env_var) = target.install_args_env_var else {
        //=-- Target has no automated install support; report manual requirement.
        return (format!("{} required (external)", capitalize_first(operation)), String::new());
    };

    //=-- ENV override takes priority over OSD-provided silent install arguments.
    let resolved_args = env::var(env_var)
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| osd_args.to_string());

    if download_link.is_empty() {
        return (format!("Cannot {}: no download link", operation), resolved_args);
    }

    match mode {
        AppMode::Query => (format!("Would download and {}", operation), resolved_args),
        AppMode::Update => {
            let action = actually_install(download_link, &resolved_args, &config.download_dir);
            (action, resolved_args)
        }
    }
}

/// Downloads the installer from `url`, runs it with `args`, deletes the local file, and
/// returns a human-readable outcome string.
fn actually_install(url: &str, args: &str, download_dir: &Path) -> String {
    let installer_path = match download_installer(url, download_dir) {
        Ok(path) => path,
        Err(e) => {
            log::error!("Installer download failed | url={} | error={}", url, e);
            return format!("Download failed: {}", e);
        }
    };

    let run_result = run_installer(&installer_path, args);

    //=-- Delete the installer file regardless of the run outcome.
    if let Err(e) = fs::remove_file(&installer_path) {
        log::warn!(
            "Failed to delete installer file | path={} | error={}",
            installer_path.display(),
            e
        );
    }

    match run_result {
        Ok(status) => {
            let code = status.code().unwrap_or(-1);
            if status.success() || code == 3010 {
                let label = if code == 3010 {
                    "Installed — reboot required".to_string()
                } else {
                    "Installed".to_string()
                };
                log::info!(
                    "Installer completed | path={} | code={}",
                    installer_path.display(),
                    code
                );
                format!("{} (exit code: {})", label, code)
            } else {
                let description = msiexec_exit_code_description(code);
                log::error!(
                    "Installer exited with non-zero status | path={} | code={} | reason={}",
                    installer_path.display(),
                    code,
                    description
                );
                format!("Install failed (exit code: {} — {})", code, description)
            }
        }
        Err(e) => {
            log::error!(
                "Failed to run installer | path={} | error={}",
                installer_path.display(),
                e
            );
            format!("Install failed: {}", e)
        }
    }
}

/// Translates common msiexec / Windows Installer exit codes into a short human-readable reason.
fn msiexec_exit_code_description(code: i32) -> &'static str {
    match code {
        1602 => "user cancelled the installation",
        1603 => "fatal error during installation",
        1605 => "product not currently installed (uninstall attempted)",
        1618 => "another installation is already in progress",
        1619 => "installation package not found or could not be opened",
        1620 => "installation package invalid or corrupt",
        1622 => "error opening installation log file",
        1623 => "installation language not supported",
        1625 => "installation forbidden by system policy — run as Administrator",
        1638 => "another version of this product is already installed",
        1639 => "invalid command-line argument to msiexec",
        1641 => "installer initiated a restart (success)",
        _ => "see https://learn.microsoft.com/en-us/windows/win32/msi/error-codes for details",
    }
}

/// Downloads the file at `url` into `download_dir` and returns the local path.
/// Creates `download_dir` if it does not already exist.
fn download_installer(url: &str, download_dir: &Path) -> Result<PathBuf> {
    let response = reqwest::blocking::get(url)
        .with_context(|| format!("failed to request installer URL: {}", url))?;

    if !response.status().is_success() {
        anyhow::bail!(
            "installer download returned HTTP {}: {}",
            response.status().as_u16(),
            url
        );
    }

    let bytes = response
        .bytes()
        .context("failed to read installer response body")?;

    let filename = extract_filename_from_url(url).unwrap_or_else(|| "installer.exe".to_string());

    fs::create_dir_all(download_dir)
        .with_context(|| format!("failed to create download directory: {}", download_dir.display()))?;

    let dest_path = download_dir.join(&filename);
    fs::write(&dest_path, &bytes)
        .with_context(|| format!("failed to write installer to: {}", dest_path.display()))?;

    log::info!(
        "Installer downloaded | url={} | dest={}",
        url,
        dest_path.display()
    );

    Ok(dest_path)
}

/// Runs the installer at `path` with the provided argument string and waits for completion.
///
/// `.msi` files are launched via `msiexec.exe /i <path> <args>`.
/// All other extensions are executed directly.
fn run_installer(path: &Path, args: &str) -> Result<std::process::ExitStatus> {
    let split_args = split_install_args(args);
    let is_msi = path
        .extension()
        .map_or(false, |ext| ext.eq_ignore_ascii_case("msi"));

    if is_msi {
        log::info!(
            "Running msiexec installer | path={} | args={}",
            path.display(),
            args
        );
        Command::new("msiexec.exe")
            .arg("/i")
            .arg(path)
            .args(&split_args)
            .status()
            .with_context(|| format!("failed to launch msiexec for: {}", path.display()))
    } else {
        log::info!("Running installer | path={} | args={}", path.display(), args);
        Command::new(path)
            .args(&split_args)
            .status()
            .with_context(|| format!("failed to launch installer: {}", path.display()))
    }
}

/// Splits an installer argument string into tokens, treating double-quoted substrings as
/// single tokens and stripping the quotes.
fn split_install_args(args: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;

    for c in args.chars() {
        match c {
            '"' => in_quotes = !in_quotes,
            ' ' if !in_quotes => {
                if !current.is_empty() {
                    result.push(std::mem::take(&mut current));
                }
            }
            _ => current.push(c),
        }
    }
    if !current.is_empty() {
        result.push(current);
    }
    result
}

/// Extracts the last path segment of a URL for use as a local filename.
fn extract_filename_from_url(url: &str) -> Option<String> {
    let parsed = Url::parse(url).ok()?;
    let segment = parsed.path_segments()?.next_back()?.to_string();
    if segment.is_empty() { None } else { Some(segment) }
}

fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

#[cfg(test)]
#[path = "tests/main_tests.rs"]
mod tests;

fn init_logging() -> Result<PathBuf> {
    let timestamp = build_timestamp(Local::now());
    let log_dir = env::var("LOG_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join("cdk-updater-logs")
        });
    fs::create_dir_all(&log_dir).context("failed to create logs directory")?;

    let log_file_path = log_dir.join(format!("cdk-drive-updater--{}.log", timestamp));
    let log_file = fern::log_file(&log_file_path).context("failed to create log file")?;

    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{}] [{}] {}",
                Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.level(),
                message
            ))
        })
        .level(log::LevelFilter::Info)
        .chain(std::io::stdout())
        .chain(log_file)
        .apply()
        .context("failed to initialize logger")?;

    Ok(log_file_path)
}

fn build_timestamp(now: chrono::DateTime<Local>) -> String {
    let hour_24 = now.hour();
    let hour_12 = match hour_24 % 12 {
        0 => 12,
        hour => hour,
    };
    let meridiem = if hour_24 < 12 { "am" } else { "pm" };

    format!(
        "{}--{}-{:02}-{}",
        now.format("%Y-%m-%d"),
        hour_12,
        now.minute(),
        meridiem
    )
}
