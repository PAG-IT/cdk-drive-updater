use std::cmp::Ordering;
use std::env;
use std::fs;
use std::path::PathBuf;

mod installed;

use anyhow::{Context, Result};
use chrono::{Local, Timelike};
use reqwest::Url;
use scraper::{ElementRef, Html, Selector};
use version_compare::Cmp;

struct TargetSoftware {
    installed_name: &'static str,
    osd_description: &'static str,
    detect_installed: fn() -> Result<Option<installed::InstalledProduct>>,
}

const TARGET_SOFTWARES: [TargetSoftware; 2] = [
    TargetSoftware {
        installed_name: "CDK Drive 3rd Party Managed Assemblies 96.x",
        osd_description: "CDK Drive 3rd Party Managed Assemblies 96.x",
        detect_installed: installed::get_cdk_drive_3rd_party_managed_assemblies_96x_installed_version,
    },
    TargetSoftware {
        installed_name: "BlueZone",
        osd_description: "CDK Terminal Emulator",
        detect_installed: installed::get_bluezone_installed_version,
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
}

impl AppConfig {
    fn from_env() -> Result<Self> {
        let version_source_url = env::var("CDK_DRIVE_OSD_URL")
            .context("missing env var CDK_DRIVE_OSD_URL")?;

        Ok(Self { version_source_url })
    }
}

fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    let log_file_path = init_logging()?;
    let config = AppConfig::from_env()?;

    let args: Vec<String> = env::args().collect();
    let mode = AppMode::from_args(&args[1..]);

    log::info!("CDK Drive updater started");
    log::info!("Log file: {}", log_file_path.display());
    log::info!("Mode: {}", match mode { AppMode::Query => "query", AppMode::Update => "update" });
    log::info!("Version source URL: {}", config.version_source_url);

    let catalog = fetch_software_catalog(&config.version_source_url)?;
    log::info!("Parsed {} software entries from OSD HTML", catalog.len());
    log_catalog_table(&catalog);

    for target in TARGET_SOFTWARES {
        process_target(&catalog, mode.clone(), &target)?;
    }

    Ok(())
}

fn process_target(entries: &[SoftwareEntry], mode: AppMode, target: &TargetSoftware) -> Result<()> {
    let installed = (target.detect_installed)()?;
    match installed {
        Some(ref product) => {
            log::info!(
                "Installed target software version | target={} | description={} | product_name={} | version={}",
                target.installed_name,
                target.osd_description,
                product.product_name,
                product.version
            );

            match compare_software_version(entries, target.osd_description, &product.version) {
                Some(result) => {
                    log::info!(
                        "OSD comparison | target={} | description={} | installed_version={} | osd_version={} | state={} | download_link={}",
                        target.installed_name,
                        result.description,
                        product.version,
                        result.version,
                        version_state_as_str(&result.state),
                        result.download_link
                    );

                    if mode == AppMode::Update {
                        if matches!(result.state, VersionState::NeedsUpdate) {
                            log::info!(
                                "Update mode: update required | target={} | description={} | download_link={}",
                                target.installed_name,
                                result.description,
                                result.download_link
                            );
                            //=-- Placeholder: download and silent-install logic goes here.
                        } else {
                            log::info!(
                                "Update mode: no update required | target={} | description={} | state={}",
                                target.installed_name,
                                result.description,
                                version_state_as_str(&result.state)
                            );
                        }
                    }
                }
                None => log::warn!(
                    "OSD comparison skipped: target software '{}' not found on page",
                    target.osd_description
                ),
            }
        }
        None => {
            if let Some(entry) = get_software_by_description(entries, target.osd_description) {
                log::warn!(
                    "Target software not installed | target={} | description={} | osd_version={} | download_link={}",
                    target.installed_name,
                    target.osd_description,
                    if entry.file_version.is_empty() {
                        &entry.version_number
                    } else {
                        &entry.file_version
                    },
                    entry.download_link
                );

                if mode == AppMode::Update {
                    log::info!(
                        "Update mode: software not installed, install required | target={} | description={} | download_link={}",
                        target.installed_name,
                        target.osd_description,
                        entry.download_link
                    );
                    //=-- Placeholder: download and silent-install logic goes here.
                }
            } else {
                log::warn!(
                    "Target software not installed and not found on OSD page | target={} | description={}",
                    target.installed_name,
                    target.osd_description
                );
            }
        }
    }

    Ok(())
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

fn log_catalog_table(entries: &[SoftwareEntry]) {
    log::info!(
        "| Category | Description | Version Number | File Version | Silent Install Arguments | Download Link |"
    );
    log::info!("| --- | --- | --- | --- | --- | --- |");

    for entry in entries {
        log::info!(
            "| {} | {} | {} | {} | {} | {} |",
            escape_pipes(&entry.category),
            escape_pipes(&entry.description),
            escape_pipes(&entry.version_number),
            escape_pipes(&entry.file_version),
            escape_pipes(&entry.silent_install_arguments),
            escape_pipes(&entry.download_link),
        );
    }
}

fn escape_pipes(value: &str) -> String {
    value.replace('|', "\\|")
}

#[cfg(test)]
#[path = "tests/main_tests.rs"]
mod tests;

fn init_logging() -> Result<PathBuf> {
    let timestamp = build_timestamp(Local::now());
    let log_dir = PathBuf::from("logs");
    fs::create_dir_all(&log_dir).context("failed to create logs directory")?;

    let log_file_path = log_dir.join(format!("{}.log", timestamp));
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
