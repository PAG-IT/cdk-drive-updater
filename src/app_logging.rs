use std::path::Path;

use crate::SoftwareEntry;
use crate::cdk_info;
use crate::utils::NOT_FOUND_DISPLAY;

#[derive(Debug, Clone)]
pub(crate) struct TargetComparisonRow {
    pub target: String,
    pub osd_description: String,
    pub installed_version: String,
    pub osd_version: String,
    pub state: String,
    pub action: String,
    pub download_link: String,
    pub install_args: String,
    pub note: String,
}

//=-- Logs the application mode prominently at startup.
pub(crate) fn log_app_mode(mode: &str) {
    log::info!("\n");
    log::info!("========================================");
    log::info!("  Application Mode: {}", mode.to_uppercase());
    log::info!("========================================");
}

pub(crate) fn log_startup_summary(
    log_file_path: &Path,
    mode: &str,
    version_source_url: &str,
    download_dir: &str,
    variables_dir: &str,
) {
    let rows = vec![
        vec!["App".to_string(), "CDK Drive updater".to_string()],
        vec!["Mode".to_string(), mode.to_string()],
        vec!["OSD URL".to_string(), version_source_url.to_string()],
        vec!["Download Dir".to_string(), download_dir.to_string()],
        vec!["Variables Dir".to_string(), variables_dir.to_string()],
        vec!["Log File".to_string(), log_file_path.display().to_string()],
    ];

    log::info!(
        "{}",
        build_ascii_table("Runtime Summary", &["Setting", "Value"], &rows,)
    );
}

/// Builds the ordered `(check, result)` pairs for the CDK Installation Info table.
///
/// Used by both [`log_cdk_info_summary`] and [`crate::write_cdk_info_variables`].
pub(crate) fn cdk_info_entries(info: &cdk_info::CdkInfo) -> Vec<(String, String)> {
    let mut entries = vec![
        check_row("ADP (wsvc 4.5)", info.adp_check.to_string()),
        check_row("WebStart URL Protocol", info.webstart_url_check.to_string()),
        check_row("WebStart Shell Command", info.webstart_shell_var.clone()),
        check_row(
            "UnifyDriveEnabler",
            info.unify_drive_enabler_check.to_string(),
        ),
        check_row("Adaptiva Client", info.adaptiva_check.to_string()),
    ];

    expand_key_value_rows(
        "Adaptiva CDK Key (Native)",
        &info.adaptiva_cdk_key_values,
        &mut entries,
    );
    expand_key_value_rows(
        "Adaptiva CDK Key (WOW6432Node)",
        &info.adaptiva_cdk_key_wow_values,
        &mut entries,
    );

    entries.extend([
        check_row(
            "Adaptiva Server Host (Native)",
            info.adaptiva_server_host_name.clone(),
        ),
        check_row(
            "Adaptiva Server Host (WOW6432Node)",
            info.adaptiva_server_host_name_wow.clone(),
        ),
        check_row(
            "Adaptiva Server Locator (Native)",
            info.adaptiva_server_locator_name.clone(),
        ),
        check_row(
            "Adaptiva Server Locator (WOW6432Node)",
            info.adaptiva_server_locator_name_wow.clone(),
        ),
        check_row(
            "Adaptiva Setup GUID (Native)",
            info.adaptiva_setup_guid.clone(),
        ),
        check_row(
            "Adaptiva Setup GUID (WOW6432Node)",
            info.adaptiva_setup_guid_wow.clone(),
        ),
        check_row(
            "Adaptiva Client Data Manager GUID (Native)",
            info.adaptiva_client_data_manager_guid.clone(),
        ),
        check_row(
            "Adaptiva Client Data Manager GUID (WOW6432Node)",
            info.adaptiva_client_data_manager_guid_wow.clone(),
        ),
        check_row("SIA Directory", info.sia_check.to_string()),
        check_row("SIA Win10 XML", info.sia_xml_check.to_string()),
        check_row("SIA Fix Script", info.sia_fix_check.to_string()),
        check_row(
            "WebStart Version (Executable)",
            info.webstart_version.clone(),
        ),
        check_row(
            "WebStart Version (Add/Remove)",
            info.webstart_add_remove_version.clone(),
        ),
        check_row(
            "CDK 3rd Party Assemblies Version",
            info.cdk_3rd_party_assemblies_version.clone(),
        ),
        check_row(
            "Adaptiva Installed Version",
            info.adaptiva_installed_version.clone(),
        ),
        check_row("BlueZone Version", info.bluezone_version.clone()),
    ]);

    entries
}

fn check_row(label: &str, value: impl Into<String>) -> (String, String) {
    (label.to_string(), value.into())
}

pub(crate) fn log_cdk_info_summary(info: &cdk_info::CdkInfo) {
    log::info!("{}", cdk_info_table_string(info));
}

/// Returns the CDK Installation Info ASCII table as a formatted string.
///
/// Used by both [`log_cdk_info_summary`] and [`crate::write_cdk_info_variables`].
pub(crate) fn cdk_info_table_string(info: &cdk_info::CdkInfo) -> String {
    let rows: Vec<Vec<String>> = cdk_info_entries(info)
        .into_iter()
        .map(|(check, result)| vec![check, result])
        .collect();

    build_ascii_table("CDK Installation Info", &["Check", "Result"], &rows)
}

//=-- Emits one row per value pair when the key exists, or a single "Not Found" row when absent.
fn expand_key_value_rows(
    label: &str,
    values: &Option<Vec<(String, String)>>,
    rows: &mut Vec<(String, String)>,
) {
    match values {
        None => rows.push(check_row(label, NOT_FOUND_DISPLAY)),
        Some(v) if v.is_empty() => {
            rows.push(check_row(label, "(key exists, no values)"));
        }
        Some(v) => {
            for (name, data) in v {
                rows.push(check_row(&format!("{label} - {name}"), data.clone()));
            }
        }
    }
}

pub(crate) fn log_adaptiva_remote_version(url: &str, version: &Option<String>) {
    let remote_version = version.as_deref().unwrap_or("(not found/empty)");
    let rows = vec![
        vec!["Source URL".to_string(), url.to_string()],
        vec!["Remote Version".to_string(), remote_version.to_string()],
    ];

    log::info!(
        "{}",
        build_ascii_table("Adaptiva Remote Version", &["Setting", "Value"], &rows,)
    );
}

pub(crate) fn log_osd_catalog(entries: &[SoftwareEntry]) {
    let mut core_rows = Vec::new();
    let mut detail_rows = Vec::new();

    for entry in entries {
        core_rows.push(vec![
            entry.category.clone(),
            entry.description.clone(),
            entry.preferred_version().to_string(),
        ]);

        detail_rows.push(vec![
            entry.category.clone(),
            entry.description.clone(),
            entry.silent_install_arguments.clone(),
            entry.download_link.clone(),
        ]);
    }

    let core_table = build_ascii_table(
        "OSD Catalog Core",
        &["Category", "Description", "Version"],
        &core_rows,
    );

    let detail_table = build_ascii_table(
        "OSD Catalog Details",
        &[
            "Category",
            "Description",
            "Silent Install Args",
            "Download Link",
        ],
        &detail_rows,
    );

    let footer_rows = vec![vec!["Total Entries".to_string(), entries.len().to_string()]];
    let summary_table =
        build_ascii_table("OSD Catalog Summary", &["Metric", "Value"], &footer_rows);

    log::info!("{}", core_table);
    log::info!("{}", detail_table);
    log::info!("{}", summary_table);
}

pub(crate) fn log_target_comparisons(rows: &[TargetComparisonRow]) {
    let mut summary_rows = Vec::new();
    let mut detail_rows = Vec::new();

    for row in rows {
        summary_rows.push(vec![
            row.target.clone(),
            row.osd_description.clone(),
            row.installed_version.clone(),
            row.osd_version.clone(),
            row.state.clone(),
            row.action.clone(),
        ]);

        detail_rows.push(vec![
            row.target.clone(),
            row.osd_description.clone(),
            row.download_link.clone(),
            row.install_args.clone(),
            row.note.clone(),
        ]);
    }

    let summary_table = build_ascii_table(
        "Installed vs OSD Summary",
        &[
            "Target",
            "OSD Description",
            "Installed Version",
            "OSD Version",
            "State",
            "Action",
        ],
        &summary_rows,
    );

    let detail_table = build_ascii_table(
        "Installed vs OSD Details",
        &[
            "Target",
            "OSD Description",
            "Download Link",
            "Install Args",
            "Note",
        ],
        &detail_rows,
    );

    log::info!("{}", summary_table);
    log::info!("{}", detail_table);
}

fn build_ascii_table(title: &str, headers: &[&str], rows: &[Vec<String>]) -> String {
    let widths = compute_widths(headers, rows);
    let header_cells = headers
        .iter()
        .map(|value| (*value).to_string())
        .collect::<Vec<String>>();
    let mut lines = vec![
        title.to_string(),
        String::new(),
        build_separator(&widths),
        build_ascii_row(&header_cells, &widths),
        build_separator(&widths),
    ];

    for row in rows {
        lines.push(build_ascii_row(row, &widths));
        lines.push(build_separator(&widths));
    }

    lines.join("\n") + "\n\n"
}

fn build_ascii_row(cells: &[String], widths: &[usize]) -> String {
    let mut line = String::from("|");
    for (index, width) in widths.iter().enumerate() {
        let cell = clean_table_cell(cells.get(index).map_or("", String::as_str));
        line.push(' ');
        line.push_str(&format!("{cell:<width$}", width = *width));
        line.push(' ');
        line.push('|');
    }
    line
}

fn compute_widths(headers: &[&str], rows: &[Vec<String>]) -> Vec<usize> {
    let mut widths: Vec<usize> = headers
        .iter()
        .map(|header| header.chars().count())
        .collect();

    for row in rows {
        if row.len() > widths.len() {
            widths.resize(row.len(), 0);
        }

        for (index, cell) in row.iter().enumerate() {
            let length = clean_table_cell(cell).chars().count();
            if length > widths[index] {
                widths[index] = length;
            }
        }
    }

    widths
}

fn clean_table_cell(cell: &str) -> String {
    cell.replace(['\r', '\n'], " ")
}

fn build_separator(widths: &[usize]) -> String {
    let mut separator = String::from("+");
    for width in widths {
        separator.push_str(&"-".repeat(*width + 2));
        separator.push('+');
    }
    separator
}
