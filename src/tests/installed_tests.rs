use super::*;
use windows_sys::Win32::Storage::FileSystem::VS_FIXEDFILEINFO;

#[test]
fn decodes_msi_version_from_product_name() {
    //=-- Product name embeds the version after "V-"; should prefer that.
    let result = decode_msi_version(
        "CDK Drive 3rd Party Managed Assemblies 96.x V-104.21.517.125",
        0x68150000,
    );
    assert_eq!(result, "104.21.517.125");
}

#[test]
fn decodes_msi_version_from_dword_when_no_name_version() {
    //=-- 0x01020304 → major=1, minor=2, build=0x0304=772
    let result = decode_msi_version("CDK Drive WebStart", 0x01020304);
    assert_eq!(result, "1.2.772");
}

#[test]
fn ignores_bare_v_prefix_without_dots() {
    let result = decode_msi_version("Some Product V-5", 0x01020304);
    assert_eq!(result, "1.2.772");
}

#[test]
fn extract_version_from_name_finds_embedded_version() {
    assert_eq!(
        extract_version_from_name("CDK Software V-2.3.4.5"),
        Some("2.3.4.5".to_string())
    );
}

#[test]
fn extract_version_from_name_returns_none_when_absent() {
    assert_eq!(extract_version_from_name("CDK Drive WebStart"), None);
}

#[test]
fn format_fixed_file_version_expands_all_four_parts() {
    let version_info = VS_FIXEDFILEINFO {
        dwSignature: 0,
        dwStrucVersion: 0,
        dwFileVersionMS: (6 << 16) | 2,
        dwFileVersionLS: (14 << 16) | 321,
        dwProductVersionMS: 0,
        dwProductVersionLS: 0,
        dwFileFlagsMask: 0,
        dwFileFlags: 0,
        dwFileOS: 0,
        dwFileType: 0,
        dwFileSubtype: 0,
        dwFileDateMS: 0,
        dwFileDateLS: 0,
    };

    assert_eq!(format_fixed_file_version(&version_info), "6.2.14.321");
}

#[test]
fn select_highest_version_returns_newest_match() {
    let matches = vec![
        InstalledProduct {
            product_name: "BlueZone 6.1".to_string(),
            version: "6.1.99.1".to_string(),
        },
        InstalledProduct {
            product_name: "BlueZone 6.2".to_string(),
            version: "6.2.1.15".to_string(),
        },
        InstalledProduct {
            product_name: "BlueZone 6.2 Hotfix".to_string(),
            version: "6.2.1.23".to_string(),
        },
    ];

    let selected = select_highest_version(matches).expect("highest version selected");
    assert_eq!(selected.product_name, "BlueZone 6.2 Hotfix");
    assert_eq!(selected.version, "6.2.1.23");
}

#[test]
fn product_name_match_ignores_spacing_and_punctuation() {
    assert!(product_name_matches(
        "CDK Drive WebStart",
        WEBSTART_ADD_REMOVE_PATTERN
    ));
    assert!(product_name_matches(
        "CDK Drive 3rd Party Managed Assemblies 96.x",
        "cdk drive 3rd party managed assemblies"
    ));
    assert!(!product_name_matches(
        "CDK Drive WebStart",
        ADAPTIVA_ADD_REMOVE_PATTERN
    ));
}
