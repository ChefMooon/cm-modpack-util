use cm_modpack_util_lib::db::{cache_changelog_response, cached_changelog_response, initialize};
use cm_modpack_util_lib::discovery::{compatibility, fingerprint};
use cm_modpack_util_lib::domain::validation;
use cm_modpack_util_lib::domain::{
    ChangelogCacheKey, Evidence, ProviderMatchConfidence, ProviderMatchEvidence,
    VersionAssociationEvidence,
};
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
fn phase5_fixture_matrix_preserves_external_evidence_and_reports_malformed_inputs() {
    let valid = fixture_path("valid");
    let before = fs::read(valid.join("pack.toml")).unwrap();
    let first = fingerprint::collect(&valid).unwrap();
    let second = fingerprint::collect(&valid).unwrap();
    assert!(fingerprint::compare(first, second).unchanged);

    let edge = validation::preview(fixture_path("edge-cases").to_string_lossy().as_ref())
        .expect("edge-case fixture should remain inspectable");
    assert!(edge
        .validation
        .iter()
        .any(|result| result.code == "unknown_mod_side"));

    let malformed = validation::preview(fixture_path("malformed").to_string_lossy().as_ref())
        .expect_err("missing referenced index should be reported as unavailable");
    assert_eq!(malformed.code, "invalid_index_reference");

    let disconnected = validation::preview(
        std::env::temp_dir()
            .join("cm-modpack-util-disconnected-project")
            .to_string_lossy()
            .as_ref(),
    )
    .expect_err("missing project should remain disconnected");
    assert_eq!(disconnected.code, "project_unavailable");
    assert_eq!(fs::read(valid.join("pack.toml")).unwrap(), before);
}

#[test]
fn phase5_process_fixture_requires_confirmed_cancellation() {
    let supported =
        fs::read_to_string(fixture_path("discovery").join("supported-cancel.txt")).unwrap();
    let prompt = compatibility::classify_prompt(&supported);
    assert_eq!(
        compatibility::classify_cancellation(&supported, Some(0), &prompt),
        cm_modpack_util_lib::domain::CancellationState::Confirmed
    );

    let timeout = fs::read_to_string(fixture_path("discovery").join("timeout.txt")).unwrap();
    let timeout_prompt = compatibility::classify_prompt(&timeout);
    assert_ne!(
        compatibility::classify_cancellation(&timeout, None, &timeout_prompt),
        cm_modpack_util_lib::domain::CancellationState::Confirmed
    );
}

#[test]
fn phase5_offline_provider_cache_round_trip_is_application_owned() {
    let database = initialize(Path::new(":memory:")).unwrap();
    let key = ChangelogCacheKey {
        provider: "modrinth".into(),
        endpoint: "versions".into(),
        request_shape: "exact-version".into(),
        project_id: "project".into(),
        version_id: "version".into(),
        game_version: Some("1.21.1".into()),
        loader: Some("neoforge".into()),
        requested_fields: vec!["changelog".into()],
    };
    let association = ProviderMatchEvidence {
        confidence: ProviderMatchConfidence::Exact,
        project: None,
        version: None,
        association: Some(VersionAssociationEvidence {
            local_version: Evidence::Observed("1.0.0".into()),
            provider_version: Some("1.0.0".into()),
            matched_exactly: true,
            details: vec!["offline fixture".into()],
        }),
        candidates: Vec::new(),
        reason: "exact cached fixture".into(),
    };
    cache_changelog_response(&database, &key, r#"{"changelog":"offline"}"#, &association).unwrap();
    let cached = cached_changelog_response(&database, &key)
        .unwrap()
        .expect("offline cache should be available");
    assert_eq!(cached.0, r#"{"changelog":"offline"}"#);
    assert_eq!(cached.1.confidence, ProviderMatchConfidence::Exact);
}

#[cfg(windows)]
#[test]
fn phase5_windows_path_spellings_remain_inside_registered_root() {
    use cm_modpack_util_lib::safety::canonical_registered_path;
    let base = std::env::temp_dir().join(format!(
        "cm-modpack-util-windows-path-{}",
        std::process::id()
    ));
    let root = base.join("Registered");
    let child = root.join("Mods");
    fs::create_dir_all(&child).unwrap();
    let equivalent = root.join("Mods").join("..").join("Mods");
    assert_eq!(
        canonical_registered_path(&root, &child).unwrap(),
        canonical_registered_path(&root, &equivalent).unwrap()
    );
    let _ = fs::remove_dir_all(base);
}
