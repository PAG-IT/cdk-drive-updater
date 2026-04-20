//! Registry queries for installed MSI software versions.
//!
//! Translates the HKCR\Installer\Products scan and MSI version-decoding
//! logic from the PowerShell reference script.

use std::cmp::Ordering;

use anyhow::Result;
use winreg::RegKey;
use winreg::enums::*;

/// A single installed MSI product entry returned from the registry.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct InstalledProduct {
    pub product_name: String,
    pub version: String,
}

pub const CDK_DRIVE_3RD_PARTY_MANAGED_ASSEMBLIES_96X_PATTERN: &str =
    "CDK Drive 3rd Party Managed Assemblies";

/// Returns the newest installed version for
/// `CDK Drive 3rd Party Managed Assemblies 96.x` from MSI product registry
/// entries.
pub fn get_cdk_drive_3rd_party_managed_assemblies_96x_installed_version(
) -> Result<Option<InstalledProduct>> {
    get_installed_version(CDK_DRIVE_3RD_PARTY_MANAGED_ASSEMBLIES_96X_PATTERN)
}

/// Searches `HKEY_CLASSES_ROOT\Installer\Products` for all installed MSI
/// products whose `ProductName` value contains `name_contains`
/// (case-insensitive). Returns the entry with the highest version, or
/// `None` if no matching product is found.
#[allow(dead_code)]
pub fn get_installed_version(name_contains: &str) -> Result<Option<InstalledProduct>> {
    let hkcr = RegKey::predef(HKEY_CLASSES_ROOT);
    let products = hkcr.open_subkey("Installer\\Products")?;

    let pattern = name_contains.to_ascii_lowercase();
    let mut matches: Vec<InstalledProduct> = Vec::new();

    for key_name in products.enum_keys().flatten() {
        let Ok(subkey) = products.open_subkey(&key_name) else {
            continue;
        };

        let Ok(product_name) = subkey.get_value::<String, _>("ProductName") else {
            continue;
        };

        if !product_name.to_ascii_lowercase().contains(&pattern) {
            continue;
        }

        let Ok(version_int) = subkey.get_value::<u32, _>("Version") else {
            continue;
        };

        let version = decode_msi_version(&product_name, version_int);
        matches.push(InstalledProduct {
            product_name,
            version,
        });
    }

    //=-- Sort descending so the highest version comes first.
    matches.sort_by(|a, b| compare_version_strings(&b.version, &a.version));

    Ok(matches.into_iter().next())
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

fn compare_version_strings(a: &str, b: &str) -> Ordering {
    match version_compare::compare(a, b) {
        Ok(version_compare::Cmp::Lt) => Ordering::Less,
        Ok(version_compare::Cmp::Gt) => Ordering::Greater,
        _ => a.cmp(b),
    }
}

#[cfg(test)]
#[path = "tests/installed_tests.rs"]
mod tests;
