use std::path::Path;

use crate::SoftwareEntry;
use crate::cdk_info;

#[derive(Debug, Clone)]
pub(crate) struct TargetComparisonRow {
    pub target: String,
    pub osd_description: String,
    pub installed_version: String,
    pub osd_version: String,
    pub state: String,
    pub action: String,
    pub download_link: String,
    pub note: String,
}

pub(crate) fn log_startup_summary(log_file_path: &Path, mode: &str, version_source_url: &str) {
    let rows = vec![
        vec!["App".to_string(), "CDK Drive updater".to_string()],
        vec!["Mode".to_string(), mode.to_string()],
        vec!["OSD URL".to_string(), version_source_url.to_string()],
        vec!["Log File".to_string(), log_file_path.display().to_string()],
    ];

    log::info!("{}", build_ascii_table(
        "Runtime Summary",
        &["Setting", "Value"],
        &rows,
    ));
}

pub(crate) fn log_cdk_info_summary(info: &cdk_info::CdkInfo) {
    let mut rows: Vec<Vec<String>> = Vec::new();

    rows.push(vec!["ADP (wsvc 4.5)".to_string(), info.adp_check.to_string()]);
    rows.push(vec!["WebStart URL Protocol".to_string(), info.webstart_url_check.to_string()]);
    rows.push(vec!["WebStart Shell Command".to_string(), info.webstart_shell_var.clone()]);
    rows.push(vec!["UnifyDriveEnabler".to_string(), info.unify_drive_enabler_check.to_string()]);
    rows.push(vec!["Adaptiva Client".to_string(), info.adaptiva_check.to_string()]);
    expand_key_value_rows("Adaptiva CDK Key (Native)", &info.adaptiva_cdk_key_values, &mut rows);
    expand_key_value_rows("Adaptiva CDK Key (WOW6432Node)", &info.adaptiva_cdk_key_wow_values, &mut rows);
    rows.push(vec!["Adaptiva Server Host (Native)".to_string(), info.adaptiva_server_host_name.clone()]);
    rows.push(vec!["Adaptiva Server Host (WOW6432Node)".to_string(), info.adaptiva_server_host_name_wow.clone()]);
    rows.push(vec!["Adaptiva Server Locator (Native)".to_string(), info.adaptiva_server_locator_name.clone()]);
    rows.push(vec!["Adaptiva Server Locator (WOW6432Node)".to_string(), info.adaptiva_server_locator_name_wow.clone()]);
    rows.push(vec!["Adaptiva Setup GUID (Native)".to_string(), info.adaptiva_setup_guid.clone()]);
    rows.push(vec!["Adaptiva Setup GUID (WOW6432Node)".to_string(), info.adaptiva_setup_guid_wow.clone()]);
    rows.push(vec!["Adaptiva Client Data Manager GUID (Native)".to_string(), info.adaptiva_client_data_manager_guid.clone()]);
    rows.push(vec!["Adaptiva Client Data Manager GUID (WOW6432Node)".to_string(), info.adaptiva_client_data_manager_guid_wow.clone()]);
    rows.push(vec!["SIA Directory".to_string(), info.sia_check.to_string()]);
    rows.push(vec!["SIA Win10 XML".to_string(), info.sia_xml_check.to_string()]);
    rows.push(vec!["SIA Fix Script".to_string(), info.sia_fix_check.to_string()]);
    rows.push(vec!["WebStart Version".to_string(), info.webstart_version.clone()]);

    log::info!("{}", build_ascii_table(
        "CDK Installation Info",
        &["Check", "Result"],
        &rows,
    ));
}

//=-- Emits one row per value pair when the key exists, or a single "Not Found" row when absent.
fn expand_key_value_rows(
    label: &str,
    values: &Option<Vec<(String, String)>>,
    rows: &mut Vec<Vec<String>>,
) {
    match values {
        None => rows.push(vec![label.to_string(), "Not Found".to_string()]),
        Some(v) if v.is_empty() => {
            rows.push(vec![label.to_string(), "(key exists, no values)".to_string()]);
        }
        Some(v) => {
            for (name, data) in v {
                rows.push(vec![format!("{label} - {name}"), data.clone()]);
            }
        }
    }
}

pub(crate) fn log_adaptiva_remote_version(url: &str, version: &Option<String>) {
    let rows = match version {
        Some(v) => vec![
            vec!["Source URL".to_string(), url.to_string()],
            vec!["Remote Version".to_string(), v.clone()],
        ],
        None => vec![
            vec!["Source URL".to_string(), url.to_string()],
            vec!["Remote Version".to_string(), "(not found/empty)".to_string()],
        ],
    };

    log::info!("{}", build_ascii_table(
        "Adaptiva Remote Version",
        &["Setting", "Value"],
        &rows,
    ));
}

pub(crate) fn log_osd_catalog(entries: &[SoftwareEntry]) {
    let mut core_rows = Vec::new();
    let mut detail_rows = Vec::new();

    for entry in entries {
        let version = if entry.file_version.is_empty() {
            &entry.version_number
        } else {
            &entry.file_version
        };

        core_rows.push(vec![
            entry.category.clone(),
            entry.description.clone(),
            version.to_string(),
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
    let summary_table = build_ascii_table(
        "OSD Catalog Summary",
        &["Metric", "Value"],
        &footer_rows,
    );

    log::info!("{}", [core_table, detail_table, summary_table].join("\n\n"));
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
        &["Target", "OSD Description", "Download Link", "Note"],
        &detail_rows,
    );

    log::info!("{}", [summary_table, detail_table].join("\n\n"));
}

fn build_ascii_table(title: &str, headers: &[&str], rows: &[Vec<String>]) -> String {
    let widths = compute_widths(headers, rows);
    let mut lines = Vec::new();

    lines.push(title.to_string());
    lines.push(build_separator(&widths));
    lines.push(build_ascii_row(
        &headers.iter().map(|value| (*value).to_string()).collect::<Vec<String>>(),
        &widths,
    ));
    lines.push(build_separator(&widths));

    for row in rows {
        lines.push(build_ascii_row(row, &widths));
        lines.push(build_separator(&widths));
    }

    lines.join("\n")
}

fn build_ascii_row(cells: &[String], widths: &[usize]) -> String {
    let mut line = String::from("|");
    for (index, width) in widths.iter().enumerate() {
        let cell = cells.get(index).map_or("", String::as_str).replace(['\r', '\n'], " ");
        line.push(' ');
        line.push_str(&format!("{cell:<width$}", width = *width));
        line.push(' ');
        line.push('|');
    }
    line
}

fn compute_widths(headers: &[&str], rows: &[Vec<String>]) -> Vec<usize> {
    let mut widths: Vec<usize> = headers.iter().map(|header| header.chars().count()).collect();

    for row in rows {
        if row.len() > widths.len() {
            widths.resize(row.len(), 0);
        }

        for (index, cell) in row.iter().enumerate() {
            let length = cell.replace(['\r', '\n'], " ").chars().count();
            if length > widths[index] {
                widths[index] = length;
            }
        }
    }

    widths
}

fn build_separator(widths: &[usize]) -> String {
    let mut separator = String::from("+");
    for width in widths {
        separator.push_str(&"-".repeat(*width + 2));
        separator.push('+');
    }
    separator
}
