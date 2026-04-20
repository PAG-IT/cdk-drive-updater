use super::*;

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
