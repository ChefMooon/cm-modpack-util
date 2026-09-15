CREATE TABLE IF NOT EXISTS schema_metadata (
    key TEXT PRIMARY KEY NOT NULL,
    value TEXT NOT NULL
);

INSERT OR IGNORE INTO schema_metadata (key, value) VALUES ('schema_version', '1');

CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY NOT NULL,
    value_json TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS modpacks (
    id TEXT PRIMARY KEY NOT NULL,
    canonical_path TEXT NOT NULL UNIQUE,
    application_json TEXT NOT NULL,
    packwiz_json TEXT NOT NULL,
    validation_json TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    last_opened_at TEXT,
    last_refreshed_at TEXT
);

CREATE TABLE IF NOT EXISTS inventory_observations (
    modpack_id TEXT PRIMARY KEY NOT NULL,
    observed_at TEXT NOT NULL,
    freshness TEXT NOT NULL,
    inventory_json TEXT NOT NULL,
    overview_json TEXT NOT NULL,
    FOREIGN KEY (modpack_id) REFERENCES modpacks(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS modpack_activity (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    modpack_id TEXT NOT NULL,
    event_type TEXT NOT NULL,
    occurred_at TEXT NOT NULL,
    message TEXT NOT NULL,
    FOREIGN KEY (modpack_id) REFERENCES modpacks(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS discovery_attempts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    modpack_id TEXT NOT NULL,
    observed_at TEXT NOT NULL,
    outcome TEXT NOT NULL,
    result_json TEXT NOT NULL,
    FOREIGN KEY (modpack_id) REFERENCES modpacks(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS discovery_candidates (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    attempt_id INTEGER NOT NULL,
    candidate_json TEXT NOT NULL,
    FOREIGN KEY (attempt_id) REFERENCES discovery_attempts(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS snapshots (
    id TEXT PRIMARY KEY NOT NULL,
    modpack_id TEXT NOT NULL,
    predecessor_id TEXT,
    lifecycle TEXT NOT NULL,
    outcome TEXT NOT NULL,
    label TEXT,
    result_json TEXT NOT NULL,
    baseline_capture_json TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    closed_at TEXT,
    FOREIGN KEY (modpack_id) REFERENCES modpacks(id) ON DELETE RESTRICT,
    FOREIGN KEY (predecessor_id) REFERENCES snapshots(id) ON DELETE RESTRICT
);

CREATE TABLE IF NOT EXISTS snapshot_candidates (
    id TEXT PRIMARY KEY NOT NULL,
    snapshot_id TEXT NOT NULL,
    candidate_json TEXT NOT NULL,
    observed_at TEXT NOT NULL,
    FOREIGN KEY (snapshot_id) REFERENCES snapshots(id) ON DELETE RESTRICT
);

CREATE TABLE IF NOT EXISTS snapshot_decisions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    snapshot_id TEXT NOT NULL,
    candidate_id TEXT NOT NULL,
    decision TEXT NOT NULL,
    note TEXT,
    recorded_at TEXT NOT NULL,
    FOREIGN KEY (snapshot_id) REFERENCES snapshots(id) ON DELETE RESTRICT,
    FOREIGN KEY (candidate_id) REFERENCES release_workspace_candidate_sources(id) ON DELETE RESTRICT
);

CREATE TABLE IF NOT EXISTS snapshot_notes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    modpack_id TEXT NOT NULL,
    snapshot_id TEXT,
    candidate_id TEXT,
    scope TEXT NOT NULL,
    note TEXT NOT NULL,
    is_current INTEGER NOT NULL DEFAULT 1,
    recorded_at TEXT NOT NULL,
    FOREIGN KEY (modpack_id) REFERENCES modpacks(id) ON DELETE RESTRICT,
    FOREIGN KEY (snapshot_id) REFERENCES snapshots(id) ON DELETE RESTRICT,
    FOREIGN KEY (candidate_id) REFERENCES snapshot_candidates(id) ON DELETE RESTRICT
);

CREATE TABLE IF NOT EXISTS snapshot_rechecks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    snapshot_id TEXT NOT NULL,
    comparable INTEGER NOT NULL,
    unchanged INTEGER NOT NULL,
    differences_json TEXT NOT NULL,
    checked_at TEXT NOT NULL,
    FOREIGN KEY (snapshot_id) REFERENCES snapshots(id) ON DELETE RESTRICT
);

CREATE TABLE IF NOT EXISTS operation_attempts (
    id TEXT PRIMARY KEY NOT NULL,
    modpack_id TEXT NOT NULL,
    workspace_id TEXT,
    snapshot_id TEXT,
    predecessor_id TEXT,
    kind TEXT NOT NULL,
    status TEXT NOT NULL,
    outcome TEXT,
    recovery_json TEXT NOT NULL,
    process_json TEXT,
    before_fingerprint_json TEXT,
    after_fingerprint_json TEXT,
    verification_json TEXT,
    error_json TEXT,
    created_at TEXT NOT NULL,
    finished_at TEXT,
    FOREIGN KEY (modpack_id) REFERENCES modpacks(id) ON DELETE RESTRICT,
    FOREIGN KEY (workspace_id) REFERENCES release_workspaces(id) ON DELETE RESTRICT,
    FOREIGN KEY (snapshot_id) REFERENCES snapshots(id) ON DELETE RESTRICT,
    FOREIGN KEY (predecessor_id) REFERENCES operation_attempts(id) ON DELETE RESTRICT
);

CREATE TABLE IF NOT EXISTS release_workspace_operations (
    id TEXT PRIMARY KEY NOT NULL,
    workspace_id TEXT NOT NULL,
    snapshot_id TEXT,
    status TEXT NOT NULL,
    outcome TEXT,
    created_at TEXT NOT NULL,
    finished_at TEXT,
    FOREIGN KEY (workspace_id) REFERENCES release_workspaces(id) ON DELETE RESTRICT,
    FOREIGN KEY (snapshot_id) REFERENCES snapshots(id) ON DELETE RESTRICT
);

CREATE TABLE IF NOT EXISTS operation_candidates (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    operation_id TEXT NOT NULL,
    candidate_id TEXT,
    entry_id TEXT,
    decision TEXT NOT NULL,
    outcome TEXT NOT NULL DEFAULT 'pending',
    observed_json TEXT NOT NULL,
    FOREIGN KEY (operation_id) REFERENCES operation_attempts(id) ON DELETE RESTRICT,
    FOREIGN KEY (candidate_id) REFERENCES snapshot_candidates(id) ON DELETE RESTRICT
);

CREATE TABLE IF NOT EXISTS operation_acknowledgements (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    operation_id TEXT NOT NULL,
    modpack_fingerprint TEXT NOT NULL,
    warning_category TEXT NOT NULL,
    recovery_state TEXT NOT NULL,
    acknowledged_at TEXT NOT NULL,
    UNIQUE (operation_id),
    FOREIGN KEY (operation_id) REFERENCES operation_attempts(id) ON DELETE RESTRICT
);

CREATE TABLE IF NOT EXISTS pin_observations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    operation_id TEXT NOT NULL,
    entry_id TEXT NOT NULL,
    metadata_path TEXT NOT NULL,
    requested_pin INTEGER NOT NULL,
    observed_pin TEXT NOT NULL,
    observed_at TEXT NOT NULL,
    FOREIGN KEY (operation_id) REFERENCES operation_attempts(id) ON DELETE RESTRICT
);

CREATE TABLE IF NOT EXISTS changelog_cache (
    id TEXT PRIMARY KEY NOT NULL,
    cache_key_json TEXT NOT NULL UNIQUE,
    raw_response TEXT NOT NULL,
    association_json TEXT NOT NULL,
    retrieved_at TEXT NOT NULL,
    request_context TEXT NOT NULL,
    response_context TEXT NOT NULL,
    normalized_query TEXT NOT NULL DEFAULT '',
    page_count INTEGER NOT NULL DEFAULT 1,
    complete INTEGER NOT NULL DEFAULT 1,
    loader TEXT,
    game_version TEXT
);

CREATE TABLE IF NOT EXISTS changelog_attempts (
    id TEXT PRIMARY KEY NOT NULL,
    modpack_id TEXT NOT NULL,
    snapshot_id TEXT,
    request_json TEXT NOT NULL,
    request_fingerprint TEXT NOT NULL,
    status TEXT NOT NULL,
    error_json TEXT,
    created_at TEXT NOT NULL,
    finished_at TEXT,
    FOREIGN KEY (modpack_id) REFERENCES modpacks(id) ON DELETE RESTRICT,
    FOREIGN KEY (snapshot_id) REFERENCES snapshots(id) ON DELETE RESTRICT
);

CREATE TABLE IF NOT EXISTS changelog_artifacts (
    id TEXT PRIMARY KEY NOT NULL,
    modpack_id TEXT NOT NULL,
    snapshot_id TEXT,
    release_workspace_id TEXT,
    stage TEXT NOT NULL DEFAULT 'proposed',
    source_capture_fingerprint TEXT,
    attempt_id TEXT NOT NULL UNIQUE,
    status TEXT NOT NULL,
    introduction TEXT,
    selected_version_ids_json TEXT NOT NULL DEFAULT '[]',
    content TEXT NOT NULL,
    entries_json TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (modpack_id) REFERENCES modpacks(id) ON DELETE RESTRICT,
    FOREIGN KEY (snapshot_id) REFERENCES snapshots(id) ON DELETE RESTRICT,
    FOREIGN KEY (release_workspace_id) REFERENCES release_workspaces(id) ON DELETE RESTRICT,
    FOREIGN KEY (attempt_id) REFERENCES changelog_attempts(id) ON DELETE RESTRICT
);

CREATE TABLE IF NOT EXISTS changelog_revisions (
    id TEXT PRIMARY KEY NOT NULL,
    artifact_id TEXT NOT NULL,
    prior_revision_id TEXT,
    content TEXT NOT NULL,
    introduction TEXT,
    selected_version_ids_json TEXT NOT NULL DEFAULT '[]',
    created_at TEXT NOT NULL,
    is_current INTEGER NOT NULL DEFAULT 1,
    frozen INTEGER NOT NULL DEFAULT 0,
    archived_at TEXT,
    FOREIGN KEY (artifact_id) REFERENCES changelog_artifacts(id) ON DELETE RESTRICT,
    FOREIGN KEY (prior_revision_id) REFERENCES changelog_revisions(id) ON DELETE RESTRICT
);

CREATE TABLE IF NOT EXISTS changelog_exports (
    id TEXT PRIMARY KEY NOT NULL,
    artifact_id TEXT NOT NULL,
    revision_id TEXT,
    destination TEXT NOT NULL,
    format TEXT NOT NULL,
    content TEXT NOT NULL,
    content_hash TEXT NOT NULL,
    status TEXT NOT NULL,
    exported_at TEXT NOT NULL,
    diagnostic_json TEXT,
    FOREIGN KEY (artifact_id) REFERENCES changelog_artifacts(id) ON DELETE RESTRICT,
    FOREIGN KEY (revision_id) REFERENCES changelog_revisions(id) ON DELETE RESTRICT
);

CREATE TABLE IF NOT EXISTS releases (
    id TEXT PRIMARY KEY NOT NULL,
    modpack_id TEXT NOT NULL,
    name TEXT NOT NULL,
    version TEXT,
    description TEXT,
    notes TEXT,
    publication_status TEXT NOT NULL,
    capture_json TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (modpack_id) REFERENCES modpacks(id) ON DELETE RESTRICT
);

CREATE TABLE IF NOT EXISTS release_snapshots (
    release_id TEXT PRIMARY KEY NOT NULL,
    snapshot_id TEXT NOT NULL UNIQUE,
    FOREIGN KEY (release_id) REFERENCES releases(id) ON DELETE RESTRICT,
    FOREIGN KEY (snapshot_id) REFERENCES snapshots(id) ON DELETE RESTRICT
);

CREATE TABLE IF NOT EXISTS release_changelog_artifacts (
    release_id TEXT NOT NULL,
    artifact_id TEXT NOT NULL,
    PRIMARY KEY (release_id, artifact_id),
    FOREIGN KEY (release_id) REFERENCES releases(id) ON DELETE RESTRICT,
    FOREIGN KEY (artifact_id) REFERENCES changelog_artifacts(id) ON DELETE RESTRICT
);

CREATE TABLE IF NOT EXISTS release_workspaces (
    id TEXT PRIMARY KEY NOT NULL,
    modpack_id TEXT NOT NULL,
    source_snapshot_id TEXT,
    baseline_origin TEXT NOT NULL DEFAULT 'current_project',
    baseline_release_id TEXT,
    baseline_capture_json TEXT,
    name TEXT NOT NULL,
    version TEXT,
    description TEXT,
    notes TEXT,
    lifecycle TEXT NOT NULL,
    evidence_status TEXT NOT NULL,
    publication_status TEXT NOT NULL,
    final_capture_json TEXT,
    final_changelog_revision_id TEXT,
    finalization_receipt_json TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    abandoned_at TEXT,
    FOREIGN KEY (modpack_id) REFERENCES modpacks(id) ON DELETE RESTRICT,
    FOREIGN KEY (source_snapshot_id) REFERENCES snapshots(id) ON DELETE RESTRICT,
    FOREIGN KEY (baseline_release_id) REFERENCES releases(id) ON DELETE RESTRICT
);

CREATE TABLE IF NOT EXISTS release_workspace_candidates (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    workspace_id TEXT NOT NULL,
    source_candidate_id TEXT NOT NULL,
    decision TEXT NOT NULL,
    note TEXT,
    recorded_at TEXT NOT NULL,
    FOREIGN KEY (workspace_id) REFERENCES release_workspaces(id) ON DELETE RESTRICT,
    FOREIGN KEY (source_candidate_id) REFERENCES release_workspace_candidate_sources(id) ON DELETE RESTRICT
);

CREATE TABLE IF NOT EXISTS release_workspace_candidate_sources (
    id TEXT PRIMARY KEY NOT NULL,
    workspace_id TEXT NOT NULL,
    source_snapshot_candidate_id TEXT,
    candidate_json TEXT NOT NULL,
    observed_at TEXT NOT NULL,
    FOREIGN KEY (workspace_id) REFERENCES release_workspaces(id) ON DELETE RESTRICT,
    FOREIGN KEY (source_snapshot_candidate_id) REFERENCES snapshot_candidates(id) ON DELETE RESTRICT
);

CREATE TABLE IF NOT EXISTS release_workspace_activity (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    workspace_id TEXT NOT NULL,
    event_type TEXT NOT NULL,
    occurred_at TEXT NOT NULL,
    message TEXT NOT NULL,
    FOREIGN KEY (workspace_id) REFERENCES release_workspaces(id) ON DELETE RESTRICT
);

CREATE INDEX IF NOT EXISTS idx_release_workspace_activity_workspace_id_id
    ON release_workspace_activity (workspace_id, id);

CREATE TABLE IF NOT EXISTS release_workspace_observations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    workspace_id TEXT NOT NULL,
    observed_at TEXT NOT NULL,
    changed_scope_json TEXT NOT NULL,
    evidence_freshness TEXT NOT NULL,
    blocking_reason TEXT,
    overlapped_operation INTEGER NOT NULL DEFAULT 0,
    fingerprint_json TEXT,
    FOREIGN KEY (workspace_id) REFERENCES release_workspaces(id) ON DELETE RESTRICT
);

CREATE TABLE IF NOT EXISTS lifecycle_tombstones (
    record_type TEXT NOT NULL,
    record_id TEXT NOT NULL,
    state TEXT NOT NULL,
    reason TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    PRIMARY KEY (record_type, record_id)
);

CREATE TABLE IF NOT EXISTS lifecycle_operations (
    id TEXT PRIMARY KEY NOT NULL,
    record_type TEXT NOT NULL,
    record_id TEXT NOT NULL,
    action TEXT NOT NULL,
    scope TEXT,
    preview_fingerprint TEXT NOT NULL,
    status TEXT NOT NULL,
    created_at TEXT NOT NULL,
    finished_at TEXT,
    error_json TEXT
);

CREATE INDEX IF NOT EXISTS idx_lifecycle_operations_target
    ON lifecycle_operations (record_type, record_id, status);

CREATE TABLE IF NOT EXISTS cleanup_operations (
    id TEXT PRIMARY KEY NOT NULL,
    scope TEXT NOT NULL,
    preview_fingerprint TEXT NOT NULL,
    status TEXT NOT NULL,
    removed_count INTEGER NOT NULL DEFAULT 0,
    protected_count INTEGER NOT NULL DEFAULT 0,
    removed_bytes INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    finished_at TEXT,
    error_json TEXT
);
