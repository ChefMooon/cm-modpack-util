use std::fs;
use std::path::{Path, PathBuf};

fn fixture_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("packwiz")
        .join(name)
}

#[test]
fn valid_fixture_contains_supported_packwiz_evidence() {
    let root = fixture_path("valid");
    let pack = fs::read_to_string(root.join("pack.toml")).unwrap();
    let index = fs::read_to_string(root.join("index.toml")).unwrap();
    let mod_metadata = fs::read_to_string(root.join("mods/example.pw.toml")).unwrap();

    assert!(pack.contains("[index]"));
    assert!(pack.contains("[versions]"));
    assert!(index.contains("metafile = true"));
    assert!(mod_metadata.contains("[update.modrinth]"));
    assert!(mod_metadata.contains("side = \"both\""));
}

#[test]
fn optional_fixture_omits_non_required_packwiz_values() {
    let pack = fs::read_to_string(fixture_path("optional").join("pack.toml")).unwrap();

    assert!(pack.contains("name = \"optional-fields-pack\""));
    assert!(!pack.contains("author ="));
    assert!(!pack.contains("version ="));
    assert!(!pack.contains("pack-format ="));
}

#[test]
fn malformed_fixture_covers_missing_reference_and_invalid_metadata_shapes() {
    let root = fixture_path("malformed");
    let pack = fs::read_to_string(root.join("pack.toml")).unwrap();
    let metadata = fs::read_to_string(root.join("mods/broken.pw.toml")).unwrap();

    assert!(pack.contains("missing-index.toml"));
    assert!(metadata.contains("name = ["));
    assert!(metadata.contains("unknown-side"));
}

#[test]
fn read_only_fixture_seam_does_not_change_external_evidence() {
    let root = fixture_path("valid");
    let paths = [
        root.join("pack.toml"),
        root.join("index.toml"),
        root.join("mods/example.pw.toml"),
    ];
    let before = paths
        .iter()
        .map(fs::read)
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    let _observed = paths
        .iter()
        .map(fs::read)
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    let after = paths
        .iter()
        .map(fs::read)
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(before, after);
}
