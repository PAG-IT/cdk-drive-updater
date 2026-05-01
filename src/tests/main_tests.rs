use super::*;

const SAMPLE_HTML: &str = r#"
        <html>
            <body>
                <div class="category">Express Installers</div>
                <table id="express">
                    <tr>
                        <th class="desc">Description</th>
                        <th class="fversion">Installer Version</th>
                        <th class="dlink">Link</th>
                    </tr>
                    <tr>
                        <td>CDK Drive Express Installer</td>
                        <td>2.20.0</td>
                        <td class="dlink"><a href="express/WSPCP_EXP_INS/index.php">Link</a></td>
                    </tr>
                </table>

                <div class="category">CDK Drive Core Software</div>
                <table class="osdTable">
                    <tr>
                        <th class="desc">Description</th>
                        <th class="fversion">File Version</th>
                        <th class="fsize">File Size</th>
                        <th class="args">Silent Install Arguments</th>
                        <th class="dlink">Download</th>
                    </tr>
                    <tr>
                        <td>CDK Init</td>
                        <td>1.7.0.0</td>
                        <td>1818624</td>
                        <td>ALLUSERS=1 /quiet /norestart</td>
                        <td class="dlink"><a href="https://example.com/CDKInitSetup_x64.msi">Download</a></td>
                    </tr>
                    <tr>
                        <td>CDK Terminal Emulator</td>
                        <td>6.2.1.23</td>
                        <td>1818624</td>
                        <td>/silent</td>
                        <td class="dlink"><a href="terminal/bluezone/index.php">Download</a></td>
                    </tr>
                    <tr>
                        <td>CDK Drive WebStart</td>
                        <td>7.4.2.19</td>
                        <td>1818624</td>
                        <td>/quiet /norestart</td>
                        <td class="dlink"><a href="webstart/CDKDriveWebStartSetup.msi">Download</a></td>
                    </tr>
                </table>
            </body>
        </html>
        "#;

#[test]
fn parses_catalog_entries_from_category_tables() {
    let base_url = Url::parse("https://servdemo.cdk.com/apps/autoTools/cds/osd/osd.php")
        .expect("valid base url");
    let entries = parse_software_catalog(SAMPLE_HTML, &base_url).expect("catalog should parse");

    assert_eq!(entries.len(), 4);

    let express = get_software_by_description(&entries, "CDK Drive Express Installer")
        .expect("express installer entry exists");
    assert_eq!(express.category, "Express Installers");
    assert_eq!(express.version_number, "2.20.0");
    assert_eq!(express.file_version, "");
    assert_eq!(
        express.download_link,
        "https://servdemo.cdk.com/apps/autoTools/cds/osd/express/WSPCP_EXP_INS/index.php"
    );

    let core = get_software_by_description(&entries, "CDK Init").expect("core entry exists");
    assert_eq!(core.category, "CDK Drive Core Software");
    assert_eq!(core.version_number, "1.7.0.0");
    assert_eq!(core.file_version, "1.7.0.0");
    assert_eq!(
        core.silent_install_arguments,
        "ALLUSERS=1 /quiet /norestart"
    );

    let terminal = get_software_by_description(&entries, "CDK Terminal Emulator")
        .expect("terminal emulator entry exists");
    assert_eq!(terminal.file_version, "6.2.1.23");
    assert_eq!(
        terminal.download_link,
        "https://servdemo.cdk.com/apps/autoTools/cds/osd/terminal/bluezone/index.php"
    );

    let webstart =
        get_software_by_description(&entries, "CDK Drive WebStart").expect("webstart entry exists");
    assert_eq!(webstart.file_version, "7.4.2.19");
    assert_eq!(
        webstart.download_link,
        "https://servdemo.cdk.com/apps/autoTools/cds/osd/webstart/CDKDriveWebStartSetup.msi"
    );
}

#[test]
fn compares_software_version_states() {
    let base_url = Url::parse("https://servdemo.cdk.com/apps/autoTools/cds/osd/osd.php")
        .expect("valid base url");
    let entries = parse_software_catalog(SAMPLE_HTML, &base_url).expect("catalog should parse");

    let needs_update = compare_software_version(&entries, "CDK Init", "1.6.0.0")
        .expect("comparison result exists");
    assert!(matches!(needs_update.state, VersionState::NeedsUpdate));

    let same = compare_software_version(&entries, "CDK Init", "1.7.0.0")
        .expect("comparison result exists");
    assert!(matches!(same.state, VersionState::Same));

    let newer = compare_software_version(&entries, "CDK Init", "1.8.0.0")
        .expect("comparison result exists");
    assert!(matches!(newer.state, VersionState::Newer));
}

#[test]
fn compares_bluezone_using_osd_alias_description() {
    let base_url = Url::parse("https://servdemo.cdk.com/apps/autoTools/cds/osd/osd.php")
        .expect("valid base url");
    let entries = parse_software_catalog(SAMPLE_HTML, &base_url).expect("catalog should parse");

    let needs_update = compare_software_version(&entries, "CDK Terminal Emulator", "6.2.1.10")
        .expect("comparison result exists");
    assert!(matches!(needs_update.state, VersionState::NeedsUpdate));

    let same = compare_software_version(&entries, "CDK Terminal Emulator", "6.2.1.23")
        .expect("comparison result exists");
    assert!(matches!(same.state, VersionState::Same));
}

#[test]
fn compares_webstart_using_osd_description() {
    let base_url = Url::parse("https://servdemo.cdk.com/apps/autoTools/cds/osd/osd.php")
        .expect("valid base url");
    let entries = parse_software_catalog(SAMPLE_HTML, &base_url).expect("catalog should parse");

    let needs_update = compare_software_version(&entries, "CDK Drive WebStart", "7.4.2.10")
        .expect("comparison result exists");
    assert!(matches!(needs_update.state, VersionState::NeedsUpdate));

    let same = compare_software_version(&entries, "CDK Drive WebStart", "7.4.2.19")
        .expect("comparison result exists");
    assert!(matches!(same.state, VersionState::Same));
}

#[test]
fn adapts_adaptiva_index_link_to_download_link() {
    assert_eq!(
        adaptiva_zip_download_url(
            "https://servdemo.cdk.com/apps/autoTools/cds/osd/express/CDK_ADAPTIVA_EXP_INS/index.php"
        ),
        "https://servdemo.cdk.com/apps/autoTools/cds/osd/express/CDK_ADAPTIVA_EXP_INS/download.php"
    );
}

#[test]
fn names_php_downloads_after_parent_folder_zip() {
    assert_eq!(
        extract_filename_from_url(
            "https://servdemo.cdk.com/apps/autoTools/cds/osd/express/CDK_ADAPTIVA_EXP_INS/download.php"
        ),
        Some("CDK_ADAPTIVA_EXP_INS.zip".to_string())
    );
}
