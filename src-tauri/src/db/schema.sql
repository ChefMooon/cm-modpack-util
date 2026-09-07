CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY NOT NULL,
    value_json TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS projects (
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
    project_id TEXT PRIMARY KEY NOT NULL,
    observed_at TEXT NOT NULL,
    freshness TEXT NOT NULL,
    inventory_json TEXT NOT NULL,
    overview_json TEXT NOT NULL,
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS project_activity (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id TEXT NOT NULL,
    event_type TEXT NOT NULL,
    occurred_at TEXT NOT NULL,
    message TEXT NOT NULL,
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS discovery_attempts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id TEXT NOT NULL,
    observed_at TEXT NOT NULL,
    outcome TEXT NOT NULL,
    result_json TEXT NOT NULL,
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS discovery_candidates (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    attempt_id INTEGER NOT NULL,
    candidate_json TEXT NOT NULL,
    FOREIGN KEY (attempt_id) REFERENCES discovery_attempts(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS snapshots (
    id TEXT PRIMARY KEY NOT NULL,
    project_id TEXT NOT NULL,
    predecessor_id TEXT,
    lifecycle TEXT NOT NULL,
    outcome TEXT NOT NULL,
    label TEXT,
    result_json TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    closed_at TEXT,
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE RESTRICT,
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
    FOREIGN KEY (candidate_id) REFERENCES snapshot_candidates(id) ON DELETE RESTRICT
);

CREATE TABLE IF NOT EXISTS snapshot_notes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id TEXT NOT NULL,
    snapshot_id TEXT,
    candidate_id TEXT,
    scope TEXT NOT NULL,
    note TEXT NOT NULL,
    is_current INTEGER NOT NULL DEFAULT 1,
    recorded_at TEXT NOT NULL,
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE RESTRICT,
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
    project_id TEXT NOT NULL,
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
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE RESTRICT,
    FOREIGN KEY (snapshot_id) REFERENCES snapshots(id) ON DELETE RESTRICT,
    FOREIGN KEY (predecessor_id) REFERENCES operation_attempts(id) ON DELETE RESTRICT
);

CREATE TABLE IF NOT EXISTS operation_candidates (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    operation_id TEXT NOT NULL,
    candidate_id TEXT,
    entry_id TEXT,
    decision TEXT NOT NULL,
    observed_json TEXT NOT NULL,
    FOREIGN KEY (operation_id) REFERENCES operation_attempts(id) ON DELETE RESTRICT,
    FOREIGN KEY (candidate_id) REFERENCES snapshot_candidates(id) ON DELETE RESTRICT
);

CREATE TABLE IF NOT EXISTS operation_acknowledgements (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    operation_id TEXT NOT NULL,
    project_fingerprint TEXT NOT NULL,
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
