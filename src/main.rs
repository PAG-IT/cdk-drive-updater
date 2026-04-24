use std::env;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

mod app_logging;
mod cdk_info;
mod installed;
mod utils;

use app_logging::TargetComparisonRow;
use utils::{
    build_timestamp, capitalize_first, compare_versions, cwd_child, env_path_or_else,
    exe_dir_child, non_empty_env_var, replace_file, safe_filename_token,
};

use anyhow::{Context, Result};
use chrono::Local;
use reqwest::Url;
use scraper::{ElementRef, Html, Selector};

struct TargetSoftware {
    installed_name: &'static str,
    osd_description: &'static str,
    detect_installed: fn(&cdk_info::CdkInfo) -> Result<Option<installed::InstalledProduct>>,
    //=-- ENV var name that overrides the OSD silent install arguments; None = no automated install.
    install_args_env_var: Option<&'static str>,
}

const ADAPTIVA_OSD_DESCRIPTION: &str = "CDK Software Install Agent ( Adaptiva )";
const NOT_INSTALLED: &str = "Not installed";
const NOT_FOUND_ON_OSD: &str = "Not found on OSD";

const TARGET_SOFTWARES: [TargetSoftware; 4] = [
    TargetSoftware {
        installed_name: "CDK Drive 3rd Party Managed Assemblies 96.x",
        osd_description: "CDK Drive 3rd Party Managed Assemblies 96.x",
        detect_installed: installed::detect_cdk_drive_3rd_party_managed_assemblies_96x,
        install_args_env_var: Some("CDK_3RD_PARTY_INSTALL_ARGS"),
    },
    TargetSoftware {
        installed_name: "Adaptiva",
        osd_description: ADAPTIVA_OSD_DESCRIPTION,
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

impl std::fmt::Display for AppMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppMode::Query => write!(f, "query"),
            AppMode::Update => write!(f, "update"),
        }
    }
}

#[derive(Debug)]
struct AppConfig {
    version_source_url: String,
    adaptiva_version_url: String,
    download_dir: PathBuf,
    variables_dir: PathBuf,
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

impl SoftwareEntry {
    pub(crate) fn preferred_version(&self) -> &str {
        if self.file_version.is_empty() {
            &self.version_number
        } else {
            &self.file_version
        }
    }

    fn adaptiva(version: String) -> Self {
        Self {
            category: "Adaptiva".to_string(),
            description: ADAPTIVA_OSD_DESCRIPTION.to_string(),
            version_number: version.clone(),
            file_version: version,
            silent_install_arguments: String::new(),
            download_link: String::new(),
        }
    }
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

struct TargetRowDetails {
    osd_description: String,
    installed_version: String,
    osd_version: String,
    state: String,
    action: String,
    download_link: String,
    note: String,
    install_args: String,
}

impl AppConfig {
    fn from_env() -> Result<Self> {
        let version_source_url =
            env::var("CDK_DRIVE_OSD_URL").context("missing env var CDK_DRIVE_OSD_URL")?;
        let adaptiva_version_url = env::var("ADAPTIVA_VERSION_URL")
            .unwrap_or_else(|_| "https://raw.githubusercontent.com/PAG-IT/public-configs/refs/heads/main/cdk--drive--adaptiva-version.txt".to_string());
        let download_dir = env_path_or_else("DOWNLOAD_DIR", || cwd_child("cdk-updater-downloads"));
        let variables_dir =
            env_path_or_else("VARIABLES_DIR", || exe_dir_child("cdk-updater-variables"));

        Ok(Self {
            version_source_url,
            adaptiva_version_url,
            download_dir,
            variables_dir,
        })
    }
}

fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    let log_file_path = init_logging()?;
    let config = AppConfig::from_env()?;

    let args: Vec<String> = env::args().collect();
    let mode = AppMode::from_args(&args[1..]);
    let mode_str = mode.to_string();

    //=-- Log the app mode prominently at the very start.
    app_logging::log_app_mode(&mode_str);

    app_logging::log_startup_summary(
        &log_file_path,
        &mode_str,
        &config.version_source_url,
        config.download_dir.to_string_lossy().as_ref(),
        config.variables_dir.to_string_lossy().as_ref(),
    );

    //=-- Gather and display current CDK installation state before processing targets.
    let cdk_info = cdk_info::gather();
    app_logging::log_cdk_info_summary(&cdk_info);
    write_cdk_info_variables(&cdk_info, &config.variables_dir)?;

    let catalog = fetch_software_catalog(&config.version_source_url)?;
    let mut catalog = catalog;
    let adaptiva_remote_version = fetch_adaptiva_version(&config.adaptiva_version_url)
        .unwrap_or_else(|e| {
            log::warn!("Failed to fetch remote Adaptiva version: {}", e);
            None
        });
    app_logging::log_adaptiva_remote_version(
        &config.adaptiva_version_url,
        &adaptiva_remote_version,
    );
    if let Some(adaptiva_version) = adaptiva_remote_version {
        merge_adaptiva_catalog_entry(&mut catalog, adaptiva_version);
    }
    app_logging::log_osd_catalog(&catalog);

    let mut comparison_rows = Vec::new();
    for target in &TARGET_SOFTWARES {
        comparison_rows.push(process_target(&catalog, &mode, target, &cdk_info, &config)?);
    }
    app_logging::log_target_comparisons(&comparison_rows);

    Ok(())
}

fn merge_adaptiva_catalog_entry(catalog: &mut Vec<SoftwareEntry>, adaptiva_version: String) {
    if let Some(entry) = catalog.iter_mut().find(|entry| {
        entry
            .description
            .eq_ignore_ascii_case(ADAPTIVA_OSD_DESCRIPTION)
    }) {
        entry.version_number = adaptiva_version.clone();
        entry.file_version = adaptiva_version;
    } else {
        catalog.push(SoftwareEntry::adaptiva(adaptiva_version));
    }
}

fn process_target(
    entries: &[SoftwareEntry],
    mode: &AppMode,
    target: &TargetSoftware,
    cdk_info: &cdk_info::CdkInfo,
    config: &AppConfig,
) -> Result<TargetComparisonRow> {
    let installed = (target.detect_installed)(cdk_info)?;
    let row = match installed {
        Some(product) => installed_target_row(entries, mode, target, product, config),
        None => missing_target_row(entries, mode, target, config),
    };
    Ok(row)
}

fn installed_target_row(
    entries: &[SoftwareEntry],
    mode: &AppMode,
    target: &TargetSoftware,
    product: installed::InstalledProduct,
    config: &AppConfig,
) -> TargetComparisonRow {
    if let Some(result) =
        compare_software_version(entries, target.osd_description, &product.version)
    {
        let (action, install_args) = if matches!(result.state, VersionState::NeedsUpdate) {
            perform_or_describe_install(
                target,
                mode,
                &result.download_link,
                &result.silent_install_arguments,
                config,
                "update",
            )
        } else {
            current_target_action(mode, target, &result.silent_install_arguments)
        };

        return target_row(
            target,
            TargetRowDetails {
                osd_description: result.description,
                installed_version: product.version,
                osd_version: result.version,
                state: version_state_as_str(&result.state).to_string(),
                action,
                download_link: result.download_link,
                note: String::new(),
                install_args,
            },
        );
    }

    target_row(
        target,
        TargetRowDetails {
            osd_description: target.osd_description.to_string(),
            installed_version: product.version,
            osd_version: NOT_FOUND_ON_OSD.to_string(),
            state: "Unknown".to_string(),
            action: "Cannot compare".to_string(),
            download_link: String::new(),
            note: "OSD comparison skipped: target software not found on page".to_string(),
            install_args: String::new(),
        },
    )
}

fn missing_target_row(
    entries: &[SoftwareEntry],
    mode: &AppMode,
    target: &TargetSoftware,
    config: &AppConfig,
) -> TargetComparisonRow {
    if let Some(entry) = get_software_by_description(entries, target.osd_description) {
        let (action, install_args) = perform_or_describe_install(
            target,
            mode,
            &entry.download_link,
            &entry.silent_install_arguments,
            config,
            "install",
        );

        return target_row(
            target,
            TargetRowDetails {
                osd_description: target.osd_description.to_string(),
                installed_version: NOT_INSTALLED.to_string(),
                osd_version: entry.preferred_version().to_string(),
                state: "Missing".to_string(),
                action,
                download_link: entry.download_link.clone(),
                note: String::new(),
                install_args,
            },
        );
    }

    target_row(
        target,
        TargetRowDetails {
            osd_description: target.osd_description.to_string(),
            installed_version: NOT_INSTALLED.to_string(),
            osd_version: NOT_FOUND_ON_OSD.to_string(),
            state: "Missing".to_string(),
            action: "Unavailable".to_string(),
            download_link: String::new(),
            note: "Target software not installed and not found on OSD page".to_string(),
            install_args: String::new(),
        },
    )
}

fn current_target_action(
    mode: &AppMode,
    target: &TargetSoftware,
    osd_args: &str,
) -> (String, String) {
    let install_args = if mode == &AppMode::Query {
        resolve_target_install_args(target, osd_args)
    } else {
        String::new()
    };
    ("No update required".to_string(), install_args)
}

fn target_row(target: &TargetSoftware, details: TargetRowDetails) -> TargetComparisonRow {
    TargetComparisonRow {
        target: target.installed_name.to_string(),
        osd_description: details.osd_description,
        installed_version: details.installed_version,
        osd_version: details.osd_version,
        state: details.state,
        action: details.action,
        download_link: details.download_link,
        note: details.note,
        install_args: details.install_args,
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

#[derive(Default)]
struct CatalogColumns {
    description: Option<usize>,
    version: Option<usize>,
    silent_install_arguments: Option<usize>,
    download: Option<usize>,
    version_is_file_version: bool,
}

impl CatalogColumns {
    fn from_header_row(header_row: ElementRef<'_>, header_selector: &Selector) -> Self {
        let mut columns = Self::default();

        for (index, header) in header_row
            .select(header_selector)
            .map(collect_text)
            .enumerate()
        {
            let normalized = header.to_ascii_lowercase();
            if normalized.contains("description") {
                columns.description = Some(index);
            }
            if normalized.contains("version") {
                columns.version = Some(index);
                columns.version_is_file_version |= normalized.contains("file version");
            }
            if normalized.contains("silent install arguments") {
                columns.silent_install_arguments = Some(index);
            }
            if normalized.contains("download") || normalized.contains("link") {
                columns.download = Some(index);
            }
        }

        columns
    }

    fn description(&self, cells: &[ElementRef<'_>]) -> String {
        value_at_index(cells, self.description)
    }

    fn version(&self, cells: &[ElementRef<'_>]) -> String {
        value_at_index(cells, self.version)
    }

    fn silent_install_arguments(&self, cells: &[ElementRef<'_>]) -> String {
        value_at_index(cells, self.silent_install_arguments)
    }

    fn download_link(
        &self,
        cells: &[ElementRef<'_>],
        link_selector: &Selector,
        base_url: &Url,
    ) -> String {
        let Some(cell) = self.download.and_then(|index| cells.get(index)) else {
            return String::new();
        };
        let Some(anchor) = cell.select(link_selector).next() else {
            return String::new();
        };

        resolve_link(base_url, anchor.value().attr("href").unwrap_or_default())
    }
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

        let columns = table
            .select(&row_selector)
            .next()
            .map(|header_row| CatalogColumns::from_header_row(header_row, &header_selector))
            .unwrap_or_default();

        for row in table.select(&row_selector).skip(1) {
            let cells: Vec<ElementRef<'_>> = row.select(&cell_selector).collect();
            if cells.is_empty() {
                continue;
            }

            let description = columns.description(&cells);
            if description.is_empty() {
                continue;
            }

            let version = columns.version(&cells);
            let silent_install_arguments = columns.silent_install_arguments(&cells);
            let download_link = columns.download_link(&cells, &link_selector, base_url);

            let (version_number, file_version) = if columns.version_is_file_version {
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
        if let Some(element) = ElementRef::wrap(node)
            && element.value().name() == "table" {
            return Some(element);
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
    if let Some(index) = index
        && let Some(cell) = cells.get(index)
    {
        return collect_text(*cell);
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
    let target_version = entry.preferred_version().to_string();

    let ordering = compare_versions(provided_version, &target_version);
    let state = match ordering {
        std::cmp::Ordering::Less => VersionState::NeedsUpdate,
        std::cmp::Ordering::Greater => VersionState::Newer,
        std::cmp::Ordering::Equal => VersionState::Same,
    };

    Some(SoftwareComparison {
        description: entry.description.clone(),
        version: target_version,
        state,
        download_link: entry.download_link.clone(),
        silent_install_arguments: entry.silent_install_arguments.clone(),
    })
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
    non_empty_env_var(env_var).unwrap_or_else(|| osd_args.to_string())
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
    if target.install_args_env_var.is_none() {
        //=-- Target has no automated install support; report manual requirement.
        return (
            format!("{} required (external)", capitalize_first(operation)),
            String::new(),
        );
    }

    //=-- ENV override takes priority over OSD-provided silent install arguments.
    let resolved_args = resolve_target_install_args(target, osd_args);

    if download_link.is_empty() {
        return (
            format!("Cannot {}: no download link", operation),
            resolved_args,
        );
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

    //=-- Kill prerequisite processes before running the installer.
    //=-- Only wait if wsStart_4.exe was actually running and killed.
    if kill_process_if_running("wsStart_4.exe") {
        log::info!("Waiting 20 seconds after killing wsStart_4.exe before proceeding with install");
        std::thread::sleep(std::time::Duration::from_secs(20));
    }
    kill_process_if_running("wsStartChrome.exe");

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
                    "Installed - reboot required".to_string()
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
                format!("Install failed (exit code: {} - {})", code, description)
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
        1625 => "installation forbidden by system policy - run as Administrator",
        1638 => "another version of this product is already installed",
        1639 => "invalid command-line argument to msiexec",
        1641 => "installer initiated a restart (success)",
        _ => "see https://learn.microsoft.com/en-us/windows/win32/msi/error-codes for details",
    }
}

/// Downloads the file at `url` into `download_dir` and returns the local path.
/// Creates `download_dir` if it does not already exist.
/// Streams the body in chunks and logs progress. Retries up to 10 attempts with
/// a 10-second delay between each on any network or body-read failure.
/// Uses a browser-compatible User-Agent to avoid server-side rejection.
fn download_installer(url: &str, download_dir: &Path) -> Result<PathBuf> {
    const MAX_ATTEMPTS: u32 = 10;
    const RETRY_DELAY: std::time::Duration = std::time::Duration::from_secs(10);
    const CHUNK_SIZE: usize = 256 * 1024; // 256 KB per chunk

    let client = reqwest::blocking::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36")
        .build()
        .context("failed to build HTTP client")?;

    let filename = extract_filename_from_url(url).unwrap_or_else(|| "installer.exe".to_string());

    fs::create_dir_all(download_dir).with_context(|| {
        format!(
            "failed to create download directory: {}",
            download_dir.display()
        )
    })?;

    let dest_path = download_dir.join(&filename);

    let mut last_error: anyhow::Error = anyhow::anyhow!("no attempts made");

    for attempt in 1..=MAX_ATTEMPTS {
        log::info!(
            "Starting installer download | url={} | attempt={}/{}",
            url,
            attempt,
            MAX_ATTEMPTS
        );

        let attempt_result = (|| -> Result<()> {
            let response = client
                .get(url)
                .send()
                .with_context(|| format!("failed to request installer URL: {}", url))?;

            if !response.status().is_success() {
                anyhow::bail!(
                    "installer download returned HTTP {}: {}",
                    response.status().as_u16(),
                    url
                );
            }

            let content_length = response.content_length();
            if let Some(total) = content_length {
                log::info!(
                    "Installer download started | url={} | size={} bytes ({:.1} MB)",
                    url,
                    total,
                    total as f64 / 1_048_576.0
                );
            } else {
                log::info!("Installer download started | url={} | size=unknown", url);
            }

            let mut reader = response;
            let mut file = fs::File::create(&dest_path).with_context(|| {
                format!("failed to create installer file: {}", dest_path.display())
            })?;

            let mut buf = vec![0u8; CHUNK_SIZE];
            let mut bytes_written: u64 = 0;
            let mut last_pct_milestone: u64 = 0;
            let mut next_mb_milestone: u64 = 10;

            loop {
                let n = reader
                    .read(&mut buf)
                    .context("failed to read installer response body")?;
                if n == 0 {
                    break;
                }
                file.write_all(&buf[..n]).with_context(|| {
                    format!(
                        "failed to write installer chunk to: {}",
                        dest_path.display()
                    )
                })?;
                bytes_written += n as u64;

                if let Some(total) = content_length {
                    let pct = if total > 0 {
                        bytes_written * 100 / total
                    } else {
                        100
                    };
                    let milestone = pct / 10;
                    if milestone > last_pct_milestone {
                        last_pct_milestone = milestone;
                        log::info!(
                            "Installer download progress | url={} | bytes={}/{} ({}%)",
                            url,
                            bytes_written,
                            total,
                            pct
                        );
                    }
                } else {
                    let mb_done = bytes_written / 1_048_576;
                    if mb_done >= next_mb_milestone {
                        log::info!(
                            "Installer download progress | url={} | bytes={} ({:.1} MB)",
                            url,
                            bytes_written,
                            bytes_written as f64 / 1_048_576.0
                        );
                        next_mb_milestone = mb_done + 10;
                    }
                }
            }

            log::info!(
                "Installer downloaded | url={} | dest={} | bytes={}",
                url,
                dest_path.display(),
                bytes_written
            );
            Ok(())
        })();

        match attempt_result {
            Ok(()) => return Ok(dest_path.clone()),
            Err(e) => {
                //=-- Remove any partially-written file so the next attempt starts clean.
                let _ = fs::remove_file(&dest_path);
                log::warn!(
                    "Installer download attempt {}/{} failed | url={} | error={}",
                    attempt,
                    MAX_ATTEMPTS,
                    url,
                    e
                );
                last_error = e;
                if attempt < MAX_ATTEMPTS {
                    log::info!(
                        "Retrying installer download | url={} | next_attempt={}/{} | delay={}s",
                        url,
                        attempt + 1,
                        MAX_ATTEMPTS,
                        RETRY_DELAY.as_secs()
                    );
                    std::thread::sleep(RETRY_DELAY);
                }
            }
        }
    }

    Err(last_error)
}

/// Terminates all running instances of the named process using `taskkill /F /IM`.
/// Logs whether the process was found and killed, was not running, or if the kill failed.
/// Returns `true` if the process was running and successfully killed, `false` otherwise.
fn kill_process_if_running(process_name: &str) -> bool {
    let result = Command::new("taskkill")
        .args(["/F", "/IM", process_name])
        .output();

    match result {
        Ok(output) => {
            if output.status.success() {
                log::info!("Killed process | name={}", process_name);
                true
            } else {
                //=-- Exit code 128 means the process was not found; treat as non-error.
                let stderr = String::from_utf8_lossy(&output.stderr);
                let code = output.status.code().unwrap_or(-1);
                if code == 128 {
                    log::info!(
                        "Process not running, nothing to kill | name={}",
                        process_name
                    );
                } else {
                    log::warn!(
                        "taskkill exited with code {} for process | name={} | stderr={}",
                        code,
                        process_name,
                        stderr.trim()
                    );
                }
                false
            }
        }
        Err(e) => {
            log::warn!(
                "Failed to invoke taskkill | name={} | error={}",
                process_name,
                e
            );
            false
        }
    }
}

/// Runs the installer at `path` with the provided argument string and waits for completion.
///
/// `.msi` files are launched via `msiexec.exe /i <path> <args>`.
/// All other extensions are executed directly.
fn run_installer(path: &Path, args: &str) -> Result<std::process::ExitStatus> {
    let split_args = split_install_args(args);
    let is_msi = path
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("msi"));

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
        log::info!(
            "Running installer | path={} | args={}",
            path.display(),
            args
        );
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
    if segment.is_empty() {
        None
    } else {
        Some(segment)
    }
}

/// Writes each CDK Installation Info check result to an individual `.txt` file in `dir`,
/// a `summary.txt` containing the full CDK Installation Info table, and a
/// `last-run.txt` marker file recording when the run occurred.
///
/// All per-check filenames are lowercased. Non-alphanumeric, non-dot, non-hyphen
/// characters in check labels are replaced with underscores (consecutive collapsed).
fn write_cdk_info_variables(info: &cdk_info::CdkInfo, dir: &Path) -> Result<()> {
    fs::create_dir_all(dir)
        .with_context(|| format!("failed to create variables directory: {}", dir.display()))?;

    for (check, result) in &app_logging::cdk_info_entries(info) {
        let filename = format!("{}.txt", safe_filename_token(check));
        let file_path = dir.join(&filename);
        replace_file(&file_path, result)?;
    }

    let summary_path = dir.join("summary.txt");
    replace_file(&summary_path, app_logging::cdk_info_table_string(info))?;

    let last_run_path = dir.join("last-run.txt");
    let now = Local::now();
    let last_run_content = format!("{}--{}", build_timestamp(now), now.timestamp());
    replace_file(&last_run_path, last_run_content)?;

    Ok(())
}

#[cfg(test)]
#[path = "tests/main_tests.rs"]
mod tests;

fn init_logging() -> Result<PathBuf> {
    let timestamp = build_timestamp(Local::now());
    let log_dir = env_path_or_else("LOG_DIR", || cwd_child("cdk-updater-logs"));
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
