//! Registry queries for installed MSI software versions.
//!
//! Translates Add/Remove Programs and MSI product registry data into installed
//! software versions.

use std::ffi::{OsStr, c_void};
use std::io;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use walkdir::WalkDir;
use windows_sys::Win32::Storage::FileSystem::{
    GetFileVersionInfoSizeW, GetFileVersionInfoW, VS_FIXEDFILEINFO, VerQueryValueW,
};
use winreg::RegKey;
use winreg::enums::*;

use crate::cdk_info::CdkInfo;
use crate::utils::{compare_versions, is_missing_value, non_empty_env_var};

/// A single installed MSI product entry returned from the registry.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct InstalledProduct {
    pub product_name: String,
    pub version: String,
}

impl InstalledProduct {
    fn new(product_name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            product_name: product_name.into(),
            version: version.into(),
        }
    }
}

pub const CDK_DRIVE_3RD_PARTY_MANAGED_ASSEMBLIES_96X_PATTERN: &str =
    "CDK Drive 3rd Party Managed Assemblies";
pub const ADAPTIVA_ADD_REMOVE_PATTERN: &str = "Adaptiva";
pub const WEBSTART_ADD_REMOVE_PATTERN: &str = "CDKDriveWebStart";
pub const BLUEZONE_EXECUTABLE_NAME: &str = "bzvt.exe";
pub const ADAPTIVA_ONESITE_CLIENT_RELATIVE_PATH: &str =
    r"Adaptiva\AdaptivaClient\bin\OneSiteClient.exe";
const UNINSTALL_NATIVE: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall";
const UNINSTALL_WOW64: &str = r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall";

/// Returns the newest installed version for
/// `CDK Drive 3rd Party Managed Assemblies 96.x` from MSI product registry
/// entries.
pub fn get_cdk_drive_3rd_party_managed_assemblies_96x_installed_version() -> Result<Option<InstalledProduct>> {
    get_installed_version(CDK_DRIVE_3RD_PARTY_MANAGED_ASSEMBLIES_96X_PATTERN)
}

/// Returns the newest installed Adaptiva version found in Add/Remove MSI
/// product entries.
pub fn get_adaptiva_installed_version() -> Result<Option<InstalledProduct>> {
    let add_remove_match = get_installed_version(ADAPTIVA_ADD_REMOVE_PATTERN)?;
    let executable_match = select_highest_version(installed_products_from_executables(
        "Adaptiva",
        find_adaptiva_executables(),
    ));
    Ok(executable_match.or(add_remove_match))
}

/// Returns the newest installed BlueZone terminal emulator version found under
/// the available Program Files roots.
pub fn get_bluezone_installed_version() -> Result<Option<InstalledProduct>> {
    Ok(select_highest_version(installed_products_from_executables(
        "BlueZone",
        find_bluezone_executables(),
    )))
}

/// Returns the newest installed WebStart version from Add/Remove MSI product
/// registry entries.
pub fn get_webstart_add_remove_installed_version() -> Result<Option<InstalledProduct>> {
    get_installed_version(WEBSTART_ADD_REMOVE_PATTERN)
}

/// Adapter used by target processing: ignores `cdk_info` and resolves version
/// directly from installed MSI products.
pub fn detect_cdk_drive_3rd_party_managed_assemblies_96x(
    _cdk_info: &CdkInfo,
) -> Result<Option<InstalledProduct>> {
    get_cdk_drive_3rd_party_managed_assemblies_96x_installed_version()
}

/// Adapter used by target processing: ignores `cdk_info` and resolves version
/// directly from Add/Remove MSI products.
pub fn detect_adaptiva(_cdk_info: &CdkInfo) -> Result<Option<InstalledProduct>> {
    get_adaptiva_installed_version()
}

/// Adapter used by target processing: ignores `cdk_info` and resolves version
/// directly from BlueZone executables.
pub fn detect_bluezone(_cdk_info: &CdkInfo) -> Result<Option<InstalledProduct>> {
    get_bluezone_installed_version()
}

/// Returns WebStart version using the cached CDK info snapshot, preferring the
/// Add/Remove MSI version over the executable file version when available.
pub fn get_webstart_installed_version_from_cdk_info(
    cdk_info: &CdkInfo,
) -> Result<Option<InstalledProduct>> {
    let product_name = "CDK Drive WebStart";
    Ok(
        product_from_reported_version(product_name, &cdk_info.webstart_add_remove_version)
            .or_else(|| product_from_reported_version(product_name, &cdk_info.webstart_version)),
    )
}

/// Searches Add/Remove Programs and `HKEY_CLASSES_ROOT\Installer\Products` for
/// all installed products whose display/product name contains `name_contains`
/// (case-insensitive). Returns the entry with the highest version, or `None` if
/// no matching product is found.
#[allow(dead_code)]
pub fn get_installed_version(name_contains: &str) -> Result<Option<InstalledProduct>> {
    let mut matches = get_add_remove_display_versions(name_contains);
    matches.extend(get_installer_product_versions(name_contains)?);

    Ok(select_highest_version(matches))
}

fn get_add_remove_display_versions(name_contains: &str) -> Vec<InstalledProduct> {
    let mut matches = Vec::new();
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);

    for (hive, subkey_path) in [
        (&hklm, UNINSTALL_NATIVE),
        (&hklm, UNINSTALL_WOW64),
        (&hkcu, UNINSTALL_NATIVE),
    ] {
        let Ok(uninstall_key) = hive.open_subkey(subkey_path) else {
            continue;
        };

        collect_add_remove_display_versions(&uninstall_key, name_contains, &mut matches);
    }

    matches
}

fn collect_add_remove_display_versions(
    uninstall_key: &RegKey,
    name_contains: &str,
    matches: &mut Vec<InstalledProduct>,
) {
    for key_name in uninstall_key.enum_keys().flatten() {
        let Ok(subkey) = uninstall_key.open_subkey(&key_name) else {
            continue;
        };

        let Ok(display_name) = subkey.get_value::<String, _>("DisplayName") else {
            continue;
        };

        if !product_name_matches(&display_name, name_contains) {
            continue;
        }

        let Ok(display_version) = subkey.get_value::<String, _>("DisplayVersion") else {
            continue;
        };

        let version = display_version.trim();
        if version.is_empty() {
            continue;
        }

        matches.push(InstalledProduct::new(display_name, version));
    }
}

fn get_installer_product_versions(name_contains: &str) -> Result<Vec<InstalledProduct>> {
    let hkcr = RegKey::predef(HKEY_CLASSES_ROOT);
    let Ok(products) = hkcr.open_subkey("Installer\\Products") else {
        return Ok(Vec::new());
    };

    let mut matches: Vec<InstalledProduct> = Vec::new();

    for key_name in products.enum_keys().flatten() {
        let Ok(subkey) = products.open_subkey(&key_name) else {
            continue;
        };

        let Ok(product_name) = subkey.get_value::<String, _>("ProductName") else {
            continue;
        };

        if !product_name_matches(&product_name, name_contains) {
            continue;
        }

        let Ok(version_int) = subkey.get_value::<u32, _>("Version") else {
            continue;
        };

        let version = decode_msi_version(&product_name, version_int);
        matches.push(InstalledProduct::new(product_name, version));
    }

    Ok(matches)
}

fn installed_products_from_executables(
    component_name: &str,
    executable_paths: Vec<PathBuf>,
) -> Vec<InstalledProduct> {
    executable_paths
        .into_iter()
        .filter_map(|executable_path| {
            let version = match read_executable_file_version(&executable_path) {
                Ok(Some(version)) => version,
                Ok(None) => return None,
                Err(error) => {
                    log::warn!(
                        "Skipping {} executable without readable version | path={} | error={}",
                        component_name,
                        executable_path.display(),
                        error
                    );
                    return None;
                }
            };

            Some(InstalledProduct::new(
                executable_path.display().to_string(),
                version,
            ))
        })
        .collect()
}

fn product_from_reported_version(product_name: &str, version: &str) -> Option<InstalledProduct> {
    if is_missing_value(version) {
        None
    } else {
        Some(InstalledProduct::new(product_name, version.trim()))
    }
}

fn find_bluezone_executables() -> Vec<PathBuf> {
    let mut matches = Vec::new();

    for root in candidate_program_files_roots() {
        let bluezone_root = root.join("BlueZone");
        if !bluezone_root.is_dir() {
            continue;
        }

        for entry in WalkDir::new(&bluezone_root)
            .follow_links(false)
            .into_iter()
            .filter_map(|entry| entry.ok())
        {
            if !entry.file_type().is_file() {
                continue;
            }

            if entry
                .file_name()
                .to_string_lossy()
                .eq_ignore_ascii_case(BLUEZONE_EXECUTABLE_NAME)
            {
                matches.push(entry.into_path());
            }
        }
    }

    matches.sort();
    matches.dedup();
    matches
}

fn find_adaptiva_executables() -> Vec<PathBuf> {
    let mut matches = Vec::new();

    for root in candidate_program_files_roots() {
        let executable_path = root.join(ADAPTIVA_ONESITE_CLIENT_RELATIVE_PATH);
        if executable_path.is_file() {
            matches.push(executable_path);
        }
    }

    matches.sort();
    matches.dedup();
    matches
}

fn candidate_program_files_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();

    for variable in ["ProgramFiles", "ProgramFiles(x86)", "ProgramW6432"] {
        if let Some(value) = non_empty_env_var(variable) {
            roots.push(PathBuf::from(value));
        }
    }

    for fallback in [r"C:\Program Files", r"C:\Program Files (x86)"] {
        roots.push(PathBuf::from(fallback));
    }

    roots.sort();
    roots.dedup();
    roots
}

pub(crate) fn read_executable_file_version(path: &Path) -> Result<Option<String>> {
    let wide_path = to_wide(path.as_os_str());

    let size = unsafe { GetFileVersionInfoSizeW(wide_path.as_ptr(), std::ptr::null_mut()) };
    if size == 0 {
        return Ok(None);
    }

    let mut buffer = vec![0u8; size as usize];
    let ok = unsafe {
        GetFileVersionInfoW(
            wide_path.as_ptr(),
            0,
            size,
            buffer.as_mut_ptr() as *mut c_void,
        )
    };
    if ok == 0 {
        return Err(io::Error::last_os_error())
            .with_context(|| format!("failed to read version resource from {}", path.display()));
    }

    let mut value_ptr = std::ptr::null_mut();
    let mut value_len = 0u32;
    let root_block = to_wide(OsStr::new("\\"));
    let ok = unsafe {
        VerQueryValueW(
            buffer.as_ptr() as *const c_void,
            root_block.as_ptr(),
            &mut value_ptr,
            &mut value_len,
        )
    };
    if ok == 0 || value_len == 0 {
        return Ok(None);
    }

    let version_info = unsafe { &*(value_ptr as *const VS_FIXEDFILEINFO) };
    Ok(Some(format_fixed_file_version(version_info)))
}

fn to_wide(value: &OsStr) -> Vec<u16> {
    value.to_string_lossy().encode_utf16().chain([0]).collect()
}

fn format_fixed_file_version(version_info: &VS_FIXEDFILEINFO) -> String {
    let major = version_info.dwFileVersionMS >> 16;
    let minor = version_info.dwFileVersionMS & 0xFFFF;
    let build = version_info.dwFileVersionLS >> 16;
    let revision = version_info.dwFileVersionLS & 0xFFFF;

    format!("{}.{}.{}.{}", major, minor, build, revision)
}

fn select_highest_version(mut matches: Vec<InstalledProduct>) -> Option<InstalledProduct> {
    //=-- Sort descending so the highest version comes first.
    matches.sort_by(|a, b| compare_versions(&b.version, &a.version));
    matches.into_iter().next()
}

fn product_name_matches(product_name: &str, pattern: &str) -> bool {
    let product_name_lower = product_name.to_ascii_lowercase();
    let pattern_lower = pattern.to_ascii_lowercase();

    product_name_lower.contains(&pattern_lower)
        || compact_ascii_alphanumeric(&product_name_lower)
            .contains(&compact_ascii_alphanumeric(&pattern_lower))
}

fn compact_ascii_alphanumeric(value: &str) -> String {
    value
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect()
}

/// Decodes an MSI product version, preferring a version string embedded in
/// the product name (pattern `V-X.Y.Z[.W]`) before falling back to
/// unpacking the standard MSI version DWORD
/// (top byte = major, next byte = minor, low word = build).
fn decode_msi_version(product_name: &str, version_int: u32) -> String {
    if let Some(v) = extract_version_from_name(product_name) {
        return v;
    }

    let major = (version_int >> 24) & 0xFF;
    let minor = (version_int >> 16) & 0xFF;
    let build = version_int & 0xFFFF;

    format!("{}.{}.{}", major, minor, build)
}

/// Scans `name` for a `V-` prefix followed by dot-separated digits
/// (e.g. `"CDK Drive 3rd Party Software V-104.21.517.125"`).
/// Returns the version substring if found, otherwise `None`.
fn extract_version_from_name(name: &str) -> Option<String> {
    let lower = name.to_ascii_lowercase();
    let pos = lower.find("v-")?;

    let rest = &name[pos + 2..];
    let version: String = rest
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();

    //=-- A bare "v-1" with no dots is not a valid version; require at least one dot.
    if version.contains('.') {
        Some(version.trim_end_matches('.').to_string())
    } else {
        None
    }
}

#[cfg(test)]
#[path = "tests/installed_tests.rs"]
mod tests;
