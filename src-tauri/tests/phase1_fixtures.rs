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

#[test]
fn mixed_fixture_covers_independent_provider_side_and_pin_evidence() {
    let root = fixture_path("mixed");
    let modrinth = fs::read_to_string(root.join("mods/modrinth.pw.toml")).unwrap();
    let curseforge = fs::read_to_string(root.join("mods/curseforge.pw.toml")).unwrap();
    let server = fs::read_to_string(root.join("mods/server.pw.toml")).unwrap();

    assert!(modrinth.contains("[update.modrinth]"));
    assert!(modrinth.contains("pin = true"));
    assert!(curseforge.contains("[update.curseforge]"));
    assert!(curseforge.contains("side = \"client\""));
    assert!(server.contains("side = \"server\""));
    assert!(!server.contains("[update.modrinth]"));
}

#[test]
fn edge_case_fixture_preserves_explicit_unknown_and_malformed_inputs() {
    let root = fixture_path("edge-cases");
    let unknown = fs::read_to_string(root.join("mods/unknown.pw.toml")).unwrap();
    let malformed = fs::read_to_string(root.join("mods/malformed.pw.toml")).unwrap();

    assert!(unknown.contains("[update]"));
    assert!(unknown.contains("pin = false"));
    assert!(malformed.contains("side = \"unsupported-side\""));
    assert!(malformed.contains("pin = \"sometimes\""));
    assert!(malformed.contains("url = \"not a trustworthy page URL\""));
}
