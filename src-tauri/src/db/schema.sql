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
