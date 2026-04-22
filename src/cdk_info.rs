//! Gathers current CDK installation state from the local system.
//!
//! Translates the checks from the "Get CDK Install Information" Kaseya
//! procedure into native Rust registry and filesystem queries.

use std::path::Path;

use winreg::RegKey;
use winreg::enums::*;

use crate::installed::{read_executable_file_version, get_webstart_add_remove_installed_version};

/// Status returned by a registry key + named-value presence check.
#[derive(Debug, Clone, PartialEq)]
pub enum RegistryCheckStatus {
    /// The key exists and contains the expected named value.
    Found,
    /// The key exists but does not contain the expected named value.
    PathExists,
    /// The registry key does not exist.
    PathMissing,
}

impl std::fmt::Display for RegistryCheckStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Found => write!(f, "Found"),
            Self::PathExists => write!(f, "PathExists"),
            Self::PathMissing => write!(f, "PathMissing"),
        }
    }
}

/// Status returned by a filesystem path existence check.
#[derive(Debug, Clone, PartialEq)]
pub enum PathCheckStatus {
    /// The path exists on the filesystem.
    Found,
    /// The path does not exist.
    Missing,
    /// An I/O error occurred while checking for the path.
    Error(String),
}

impl std::fmt::Display for PathCheckStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Found => write!(f, "Found"),
            Self::Missing => write!(f, "Missing"),
            Self::Error(e) => write!(f, "Error: {e}"),
        }
    }
}

/// Snapshot of CDK installation state gathered from the local system.
#[derive(Debug, Clone)]
pub struct CdkInfo {
    /// ADP WSVC 4.5 registry key + `version` value presence.
    pub adp_check: RegistryCheckStatus,
    /// CDKDrive URL-protocol registry key + `URL Protocol` value presence.
    pub webstart_url_check: RegistryCheckStatus,
    /// Default shell-open command for the CDKDrive URL handler.
    ///
    /// Contains the actual command string when the key and default value both
    /// exist, `"PathExists"` when the key exists but has no default value, or
    /// `"PathMissing"` when the key is absent.
    pub webstart_shell_var: String,
    /// CDKGlobal `CDKUnifyDriveEnabler` registry value presence.
    pub unify_drive_enabler_check: RegistryCheckStatus,
    /// Adaptiva client `setup.status` registry value presence.
    pub adaptiva_check: RegistryCheckStatus,
    /// All named values from `HKLM\SOFTWARE\CDK\Adaptiva` and its subkeys.
    ///
    /// `None` when the key is absent; `Some(vec)` with `(name, data)` pairs otherwise.
    pub adaptiva_cdk_key_values: Option<Vec<(String, String)>>,
    /// All named values from `HKLM\SOFTWARE\WOW6432Node\CDK\Adaptiva` and its subkeys.
    ///
    /// `None` when the key is absent; `Some(vec)` with `(name, data)` pairs otherwise.
    pub adaptiva_cdk_key_wow_values: Option<Vec<(String, String)>>,
    /// `setup.server_host_name` value in `HKLM\SOFTWARE\Adaptiva\client`.
    pub adaptiva_server_host_name: String,
    /// `setup.server_host_name` value in `HKLM\SOFTWARE\WOW6432Node\Adaptiva\client`.
    pub adaptiva_server_host_name_wow: String,
    /// `server_locator.server_name` value in `HKLM\SOFTWARE\Adaptiva\client`.
    pub adaptiva_server_locator_name: String,
    /// `server_locator.server_name` value in `HKLM\SOFTWARE\WOW6432Node\Adaptiva\client`.
    pub adaptiva_server_locator_name_wow: String,
    /// `setup.server_guid` value in `HKLM\SOFTWARE\Adaptiva\client`.
    pub adaptiva_setup_guid: String,
    /// `client_data_manager.server_guid` value in `HKLM\SOFTWARE\Adaptiva\client`.
    pub adaptiva_client_data_manager_guid: String,
    /// `setup.server_guid` value in `HKLM\SOFTWARE\WOW6432Node\Adaptiva\client`.
    pub adaptiva_setup_guid_wow: String,
    /// `client_data_manager.server_guid` value in `HKLM\SOFTWARE\WOW6432Node\Adaptiva\client`.
    pub adaptiva_client_data_manager_guid_wow: String,
    /// CDK SIA directory presence (`C:\Program Files (x86)\CDK\sia`).
    pub sia_check: PathCheckStatus,
    /// CDK SIA win10 maintenance XML file presence.
    pub sia_xml_check: PathCheckStatus,
    /// CDK SIA w10 fix VBS script presence.
    pub sia_fix_check: PathCheckStatus,
    /// File version of `CDK Drive WebStart.exe`, or `"NotFound"` when the
    /// executable is absent.
    pub webstart_version: String,
    /// Add/Remove Programs (MSI) version for `CDK Drive WebStart`, or `"NotFound"`
    /// when not installed via MSI.
    pub webstart_add_remove_version: String,
}

/// Gathers all CDK installation info from the local system.
///
/// All checks are infallible: registry and filesystem failures are mapped to
/// the appropriate status variants rather than propagated as errors.
pub fn gather() -> CdkInfo {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let hkcr = RegKey::predef(HKEY_CLASSES_ROOT);

    let adp_check = registry_value_check(
        &hklm,
        r"SOFTWARE\WOW6432Node\ADP\wsvc\4.5",
        "version",
    );

    let webstart_url_check = registry_value_check(
        &hklm,
        r"SOFTWARE\Classes\CDKDrive",
        "URL Protocol",
    );

    let webstart_shell_var = read_shell_command(&hkcr);

    let unify_drive_enabler_check = registry_value_check(
        &hklm,
        r"SOFTWARE\CDKGlobal",
        "CDKUnifyDriveEnabler",
    );

    let adaptiva_check = registry_value_check(
        &hklm,
        r"SOFTWARE\Adaptiva\client",
        "setup.status",
    );

    let adaptiva_cdk_key_values = read_key_values_recursive(&hklm, r"SOFTWARE\CDK\Adaptiva");
    let adaptiva_cdk_key_wow_values =
        read_key_values_recursive(&hklm, r"SOFTWARE\WOW6432Node\CDK\Adaptiva");

    let adaptiva_server_host_name = read_registry_string(
        &hklm,
        r"SOFTWARE\Adaptiva\client",
        "setup.server_host_name",
    );
    let adaptiva_server_host_name_wow = read_registry_string(
        &hklm,
        r"SOFTWARE\WOW6432Node\Adaptiva\client",
        "setup.server_host_name",
    );
    let adaptiva_server_locator_name = read_registry_string(
        &hklm,
        r"SOFTWARE\Adaptiva\client",
        "server_locator.server_name",
    );
    let adaptiva_server_locator_name_wow = read_registry_string(
        &hklm,
        r"SOFTWARE\WOW6432Node\Adaptiva\client",
        "server_locator.server_name",
    );

    let adaptiva_setup_guid = read_registry_string(
        &hklm,
        r"SOFTWARE\Adaptiva\client",
        "setup.server_guid",
    );
    let adaptiva_client_data_manager_guid = read_registry_string(
        &hklm,
        r"SOFTWARE\Adaptiva\client",
        "client_data_manager.server_guid",
    );
    let adaptiva_setup_guid_wow = read_registry_string(
        &hklm,
        r"SOFTWARE\WOW6432Node\Adaptiva\client",
        "setup.server_guid",
    );
    let adaptiva_client_data_manager_guid_wow = read_registry_string(
        &hklm,
        r"SOFTWARE\WOW6432Node\Adaptiva\client",
        "client_data_manager.server_guid",
    );

    let sia_check = path_check(r"C:\Program Files (x86)\CDK\sia");
    let sia_xml_check = path_check(r"C:\Program Files (x86)\CDK\sia\cdk_sia_win10_maint.xml");
    let sia_fix_check = path_check(r"C:\Program Files (x86)\CDK\sia\w10_fix.vbs");

    let webstart_version = {
        let exe = Path::new(
            r"C:\Program Files (x86)\CDK\CDKDriveWebStart\CDK Drive WebStart.exe",
        );
        match read_executable_file_version(exe) {
            Ok(Some(v)) => v,
            _ => "NotFound".to_string(),
        }
    };

    let webstart_add_remove_version = {
        match get_webstart_add_remove_installed_version() {
            Ok(Some(product)) => product.version,
            _ => "NotFound".to_string(),
        }
    };

    CdkInfo {
        adp_check,
        webstart_url_check,
        webstart_shell_var,
        unify_drive_enabler_check,
        adaptiva_check,
        adaptiva_cdk_key_values,
        adaptiva_cdk_key_wow_values,
        adaptiva_server_host_name,
        adaptiva_server_host_name_wow,
        adaptiva_server_locator_name,
        adaptiva_server_locator_name_wow,
        adaptiva_setup_guid,
        adaptiva_client_data_manager_guid,
        adaptiva_setup_guid_wow,
        adaptiva_client_data_manager_guid_wow,
        sia_check,
        sia_xml_check,
        sia_fix_check,
        webstart_version,
        webstart_add_remove_version,
    }
}

/// Checks whether `value_name` exists as a named value under `subkey` within
/// `hive`.
///
/// - Returns [`RegistryCheckStatus::PathMissing`] when the key cannot be opened.
/// - Returns [`RegistryCheckStatus::Found`] when the named value is present.
/// - Returns [`RegistryCheckStatus::PathExists`] when the key exists but the
///   named value is absent.
fn registry_value_check(hive: &RegKey, subkey: &str, value_name: &str) -> RegistryCheckStatus {
    let key = match hive.open_subkey(subkey) {
        Ok(k) => k,
        Err(_) => return RegistryCheckStatus::PathMissing,
    };

    let name_lower = value_name.to_ascii_lowercase();
    let exists = key
        .enum_values()
        .filter_map(|r| r.ok())
        .any(|(n, _)| n.to_ascii_lowercase() == name_lower);

    if exists {
        RegistryCheckStatus::Found
    } else {
        RegistryCheckStatus::PathExists
    }
}

/// Enumerates all named values in `subkey` and its descendant subkeys within
/// `hive`, returning them as `(label, data)` pairs.
///
/// Returns `None` when the key cannot be opened.  The label for root values is
/// the value name; for subkey values it is `"SubKey\\ValueName"`.
fn read_key_values_recursive(hive: &RegKey, subkey: &str) -> Option<Vec<(String, String)>> {
    let key = hive.open_subkey(subkey).ok()?;
    let mut results = Vec::new();
    collect_key_values(&key, "", &mut results);
    Some(results)
}

fn collect_key_values(key: &RegKey, prefix: &str, out: &mut Vec<(String, String)>) {
    //=-- Collect all named values on this key level.
    for (name, value) in key.enum_values().filter_map(|r| r.ok()) {
        let display_name = if name.is_empty() {
            "(Default)".to_string()
        } else {
            name.clone()
        };
        let label = if prefix.is_empty() {
            display_name
        } else {
            format!("{prefix}\\{display_name}")
        };
        out.push((label, format_reg_value(&value)));
    }
    //=-- Recurse into any subkeys.
    for sub_name in key.enum_keys().filter_map(|r| r.ok()) {
        if let Ok(sub_key) = key.open_subkey(&sub_name) {
            let sub_prefix = if prefix.is_empty() {
                sub_name.clone()
            } else {
                format!("{prefix}\\{sub_name}")
            };
            collect_key_values(&sub_key, &sub_prefix, out);
        }
    }
}

fn format_reg_value(value: &winreg::RegValue) -> String {
    use winreg::enums::RegType;
    match value.vtype {
        RegType::REG_SZ | RegType::REG_EXPAND_SZ => {
            let words: Vec<u16> = value
                .bytes
                .chunks_exact(2)
                .map(|c| u16::from_le_bytes([c[0], c[1]]))
                .take_while(|&w| w != 0)
                .collect();
            String::from_utf16_lossy(&words).to_string()
        }
        RegType::REG_DWORD => {
            if value.bytes.len() >= 4 {
                let arr: [u8; 4] = value.bytes[..4].try_into().unwrap_or([0; 4]);
                u32::from_le_bytes(arr).to_string()
            } else {
                "(invalid DWORD)".to_string()
            }
        }
        RegType::REG_QWORD => {
            if value.bytes.len() >= 8 {
                let arr: [u8; 8] = value.bytes[..8].try_into().unwrap_or([0; 8]);
                u64::from_le_bytes(arr).to_string()
            } else {
                "(invalid QWORD)".to_string()
            }
        }
        RegType::REG_MULTI_SZ => {
            let words: Vec<u16> = value
                .bytes
                .chunks_exact(2)
                .map(|c| u16::from_le_bytes([c[0], c[1]]))
                .collect();
            words
                .split(|&w| w == 0)
                .filter(|s| !s.is_empty())
                .map(|s| String::from_utf16_lossy(s).to_string())
                .collect::<Vec<_>>()
                .join(" | ")
        }
        _ => value
            .bytes
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect::<Vec<_>>()
            .join(" "),
    }
}

/// Reads a named string value from `subkey` in `hive`.
///
/// Returns `"Not Found"` when the key or named value is absent.
fn read_registry_string(hive: &RegKey, subkey: &str, value_name: &str) -> String {
    let key = match hive.open_subkey(subkey) {
        Ok(k) => k,
        Err(_) => return "Not Found".to_string(),
    };
    match key.get_value::<String, _>(value_name) {
        Ok(v) => v,
        Err(_) => "Not Found".to_string(),
    }
}

/// Reads the default shell-open command for the `CDKDrive` URL handler from
/// `HKEY_CLASSES_ROOT\CDKDrive\shell\open\command`.
///
/// Returns the command string, `"PathExists"`, or `"PathMissing"` to mirror
/// the values produced by the Kaseya reference procedure.
fn read_shell_command(hkcr: &RegKey) -> String {
    let key = match hkcr.open_subkey(r"CDKDrive\shell\open\command") {
        Ok(k) => k,
        Err(_) => return "PathMissing".to_string(),
    };

    match key.get_value::<String, _>("") {
        Ok(cmd) if !cmd.is_empty() => cmd,
        _ => "PathExists".to_string(),
    }
}

/// Returns a [`PathCheckStatus`] for `path`, distinguishing missing from
/// I/O errors.
fn path_check(path: &str) -> PathCheckStatus {
    match Path::new(path).try_exists() {
        Ok(true) => PathCheckStatus::Found,
        Ok(false) => PathCheckStatus::Missing,
        Err(e) => PathCheckStatus::Error(e.to_string()),
    }
}

