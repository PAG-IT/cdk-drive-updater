//! Gathers current CDK installation state from the local system.
//!
//! Translates the checks from the "Get CDK Install Information" Kaseya
//! procedure into native Rust registry and filesystem queries.

use std::path::Path;

use winreg::RegKey;
use winreg::enums::*;

use crate::installed::read_executable_file_version;

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
    /// `HKLM\SOFTWARE\CDK\Adaptiva` key presence.
    pub adaptiva_cdk_key_check: RegistryCheckStatus,
    /// `HKLM\SOFTWARE\WOW6432Node\CDK\Adaptiva` key presence.
    pub adaptiva_cdk_key_wow_check: RegistryCheckStatus,
    /// `setup.server_host_name` value in `HKLM\SOFTWARE\Adaptiva\client`.
    pub adaptiva_server_host_name: String,
    /// `setup.server_host_name` value in `HKLM\SOFTWARE\WOW6432Node\Adaptiva\client`.
    pub adaptiva_server_host_name_wow: String,
    /// `server_locator.server_name` value in `HKLM\SOFTWARE\Adaptiva\client`.
    pub adaptiva_server_locator_name: String,
    /// `server_locator.server_name` value in `HKLM\SOFTWARE\WOW6432Node\Adaptiva\client`.
    pub adaptiva_server_locator_name_wow: String,
    /// CDK SIA directory presence (`C:\Program Files (x86)\CDK\sia`).
    pub sia_check: PathCheckStatus,
    /// CDK SIA win10 maintenance XML file presence.
    pub sia_xml_check: PathCheckStatus,
    /// CDK SIA w10 fix VBS script presence.
    pub sia_fix_check: PathCheckStatus,
    /// File version of `CDK Drive WebStart.exe`, or `"NotFound"` when the
    /// executable is absent.
    pub webstart_version: String,
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

    let adaptiva_cdk_key_check = registry_key_check(&hklm, r"SOFTWARE\CDK\Adaptiva");
    let adaptiva_cdk_key_wow_check =
        registry_key_check(&hklm, r"SOFTWARE\WOW6432Node\CDK\Adaptiva");

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

    CdkInfo {
        adp_check,
        webstart_url_check,
        webstart_shell_var,
        unify_drive_enabler_check,
        adaptiva_check,
        adaptiva_cdk_key_check,
        adaptiva_cdk_key_wow_check,
        adaptiva_server_host_name,
        adaptiva_server_host_name_wow,
        adaptiva_server_locator_name,
        adaptiva_server_locator_name_wow,
        sia_check,
        sia_xml_check,
        sia_fix_check,
        webstart_version,
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

/// Checks whether the registry key at `subkey` exists in `hive`.
///
/// Returns [`RegistryCheckStatus::Found`] when the key opens successfully, or
/// [`RegistryCheckStatus::PathMissing`] when it cannot be opened.
fn registry_key_check(hive: &RegKey, subkey: &str) -> RegistryCheckStatus {
    match hive.open_subkey(subkey) {
        Ok(_) => RegistryCheckStatus::Found,
        Err(_) => RegistryCheckStatus::PathMissing,
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

