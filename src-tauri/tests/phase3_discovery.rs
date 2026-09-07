use cm_modpack_util_lib::db::{initialize, persist_discovery_result};
use cm_modpack_util_lib::discovery::pipeline::parse_candidates;
use cm_modpack_util_lib::domain::{
    CompatibilityEvidence, CompatibilityStatus, DiscoveryDiagnostics, DiscoveryOutcomeKind,
    DiscoveryResult, OperationStatus,
};
use rusqlite::params;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn candidate_parser_keeps_unknown_fields_explicit() {
    let candidates = parse_candidates(
        "example 1.0.0 -> 1.1.0\nUbe's Delight: ubesdelight-neoforge-1.21.1-0.4.13.jar -> ubesdelight-neoforge-1.21.1-0.4.14.jar\nnot a candidate\n",
    );
    assert_eq!(candidates.len(), 2);
    assert!(matches!(
        &candidates[1].identity,
        cm_modpack_util_lib::domain::Evidence::Observed(identity) if identity == "Ube's Delight"
    ));
    assert_eq!(
        candidates[0].version_change,
        cm_modpack_util_lib::domain::VersionChangeKind::Unknown
    );
    assert!(matches!(
        candidates[0].provider,
        cm_modpack_util_lib::domain::Evidence::Unknown
    ));
}

#[test]
fn discovery_observation_and_candidates_are_persisted() {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = PathBuf::from(std::env::temp_dir()).join(format!("cm-discovery-{suffix}.sqlite"));
    let database = initialize(&path).unwrap();
    {
        let connection = database.0.lock().unwrap();
        connection.execute("INSERT INTO modpacks (id, canonical_path, application_json, packwiz_json, validation_json, created_at, updated_at) VALUES (?1, ?2, '{}', '{}', '[]', '0', '0')", params!["modpack-test", "."]).unwrap();
    }
    let result = DiscoveryResult {
        status: OperationStatus::Succeeded,
        outcome: DiscoveryOutcomeKind::Normal,
        candidates: parse_candidates("example 1.0.0 -> 1.1.0"),
        diagnostics: DiscoveryDiagnostics {
            compatibility: CompatibilityEvidence {
                status: CompatibilityStatus::Unsupported,
                profile: None,
                executable_path: None,
                version_output: None,
                diagnostic: None,
            },
            process: None,
            fingerprint: None,
            messages: vec![],
        },
        error: None,
    };
    persist_discovery_result(&database, "modpack-test", &result).unwrap();
    let connection = database.0.lock().unwrap();
    let attempts: i64 = connection
        .query_row("SELECT COUNT(*) FROM discovery_attempts", [], |row| {
            row.get(0)
        })
        .unwrap();
    let candidates: i64 = connection
        .query_row("SELECT COUNT(*) FROM discovery_candidates", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(attempts, 1);
    assert_eq!(candidates, 1);
    let snapshots: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM snapshots WHERE modpack_id = 'modpack-test'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let snapshot_candidates: i64 = connection
        .query_row("SELECT COUNT(*) FROM snapshot_candidates", [], |row| {
            row.get(0)
        })
        .unwrap();
    let activity: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM modpack_activity WHERE event_type = 'snapshot_created'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(snapshots, 1);
    assert_eq!(snapshot_candidates, 1);
    assert_eq!(activity, 1);
    drop(connection);
    let _ = std::fs::remove_file(path);
}
