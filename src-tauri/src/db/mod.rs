use crate::domain::{
    inventory, validation, ActivityRecord, ApplicationProjectMetadata, CommandError,
    DiscoveryResult, InventoryEntry, ProjectLifecycle, ProjectOverview, ProjectRecord,
    RegistrationPreview,
};
use rusqlite::{params, Connection};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::State;

pub struct Database(pub Mutex<Connection>);

pub fn persist_discovery_result(
    database: &Database,
    project_id: &str,
    result: &DiscoveryResult,
) -> Result<(), CommandError> {
    let connection = database
        .0
        .lock()
        .map_err(|_| CommandError::new("database_unavailable", "Database is unavailable"))?;
    let result_json = serialize(result, "discovery result")?;
    let outcome =
        serde_json::to_string(&result.outcome).unwrap_or_else(|_| "indeterminate".to_string());
    connection.execute("INSERT INTO discovery_attempts (project_id, observed_at, outcome, result_json) VALUES (?1, ?2, ?3, ?4)", params![project_id, timestamp(), outcome.trim_matches('"'), result_json]).map_err(|error| CommandError::new("database_write_failed", "Discovery observation could not be saved").with_details(error.to_string()))?;
    let attempt_id = connection.last_insert_rowid();
    for candidate in &result.candidates {
        connection
            .execute(
                "INSERT INTO discovery_candidates (attempt_id, candidate_json) VALUES (?1, ?2)",
                params![attempt_id, serialize(candidate, "discovery candidate")?],
            )
            .map_err(|error| {
                CommandError::new(
                    "database_write_failed",
                    "Discovery candidate could not be saved",
                )
                .with_details(error.to_string())
            })?;
    }
    Ok(())
}

pub fn registered_project_path(database: &Database, id: &str) -> Result<String, CommandError> {
    let connection = database
        .0
        .lock()
        .map_err(|_| CommandError::new("database_unavailable", "Database is unavailable"))?;
    connection
        .query_row(
            "SELECT canonical_path FROM projects WHERE id = ?1",
            [id],
            |row| row.get(0),
        )
        .map_err(|error| {
            CommandError::new("project_not_found", "Project is not registered")
                .with_details(error.to_string())
        })
}

/// Read a boolean preference for native lifecycle decisions (tray/startup).
pub fn bool_setting(database: &Database, key: &str, default: bool) -> bool {
    let Ok(connection) = database.0.lock() else {
        return default;
    };
    let Ok(value) = connection.query_row(
        "SELECT value_json FROM settings WHERE key = ?1",
        [key],
        |row| row.get::<_, String>(0),
    ) else {
        return default;
    };
    serde_json::from_str(&value).unwrap_or(default)
}

pub fn initialize(path: &std::path::Path) -> Result<Database, rusqlite::Error> {
    let connection = Connection::open(path)?;
    connection.execute_batch("PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL;")?;
    connection.execute_batch("PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL;")?;
    connection.execute_batch(include_str!("schema.sql"))?;
    Ok(Database(Mutex::new(connection)))
}

fn timestamp() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs().to_string())
        .unwrap_or_else(|_| "0".to_string())
}

fn project_id(path: &str) -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    path.hash(&mut hasher);
    format!("project-{:x}", hasher.finish())
}

fn serialize<T: serde::Serialize>(value: &T, label: &str) -> Result<String, CommandError> {
    serde_json::to_string(value).map_err(|error| {
        CommandError::new(
            "serialization_failed",
            format!("{label} could not be serialized"),
        )
        .with_details(error.to_string())
    })
}

fn record_activity(
    connection: &Connection,
    project_id: &str,
    event_type: &str,
    message: &str,
) -> Result<(), CommandError> {
    connection
        .execute(
            "INSERT INTO project_activity (project_id, event_type, occurred_at, message) VALUES (?1, ?2, ?3, ?4)",
            params![project_id, event_type, timestamp(), message],
        )
        .map(|_| ())
        .map_err(|error| {
            CommandError::new("database_write_failed", "Project activity could not be saved")
                .with_details(error.to_string())
        })
}

fn read_activity(
    connection: &Connection,
    project_id: &str,
) -> Result<Vec<ActivityRecord>, CommandError> {
    let mut statement = connection
        .prepare("SELECT event_type, occurred_at, message FROM project_activity WHERE project_id = ?1 ORDER BY occurred_at DESC LIMIT 10")
        .map_err(|error| CommandError::new("database_read_failed", "Project activity could not be read").with_details(error.to_string()))?;
    let records = statement
        .query_map([project_id], |row| {
            let event_type: String = row.get(0)?;
            let event_type = serde_json::from_value(serde_json::Value::String(event_type))
                .map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(
                        0,
                        rusqlite::types::Type::Text,
                        Box::new(error),
                    )
                })?;
            Ok(ActivityRecord {
                event_type,
                occurred_at: row.get(1)?,
                message: row.get(2)?,
            })
        })
        .map_err(|error| {
            CommandError::new("database_read_failed", "Project activity could not be read")
                .with_details(error.to_string())
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| {
            CommandError::new("database_read_failed", "Project activity could not be read")
                .with_details(error.to_string())
        })?;
    Ok(records)
}

fn row_to_project(row: &rusqlite::Row<'_>) -> rusqlite::Result<ProjectRecord> {
    let application: String = row.get(2)?;
    let packwiz: String = row.get(3)?;
    let validation: String = row.get(4)?;
    Ok(ProjectRecord {
        id: row.get(0)?,
        canonical_path: row.get(1)?,
        application: serde_json::from_str(&application)
            .map_err(|_| rusqlite::Error::InvalidQuery)?,
        packwiz: serde_json::from_str(&packwiz).map_err(|_| rusqlite::Error::InvalidQuery)?,
        validation: serde_json::from_str(&validation).map_err(|_| rusqlite::Error::InvalidQuery)?,
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
        last_opened_at: row.get(7)?,
        last_refreshed_at: row.get(8)?,
    })
}

#[tauri::command]
pub fn preview_project(path: String) -> Result<RegistrationPreview, CommandError> {
    validation::preview(&path)
}

#[tauri::command]
pub fn register_project(
    state: State<'_, Database>,
    path: String,
    application: Option<ApplicationProjectMetadata>,
) -> Result<ProjectRecord, CommandError> {
    register_project_inner(&state, path, application)
}

fn register_project_inner(
    database: &Database,
    path: String,
    application: Option<ApplicationProjectMetadata>,
) -> Result<ProjectRecord, CommandError> {
    let preview = validation::preview(&path)?;
    if validation::has_errors(&preview.validation) {
        return Err(CommandError::new(
            "project_validation_failed",
            "Project evidence did not pass validation",
        ));
    }
    let application = application.unwrap_or(preview.application_defaults);
    let id = project_id(&preview.canonical_path);
    let now = timestamp();
    let connection = database
        .0
        .lock()
        .map_err(|_| CommandError::new("database_unavailable", "Database is unavailable"))?;
    let application_json = serialize(&application, "application metadata")?;
    let packwiz_json = serialize(&preview.packwiz, "Packwiz observations")?;
    let validation_json = serialize(&preview.validation, "validation results")?;
    connection.execute(
        "INSERT INTO projects (id, canonical_path, application_json, packwiz_json, validation_json, created_at, updated_at, last_opened_at, last_refreshed_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6, ?6, ?6)",
        params![id, preview.canonical_path, application_json, packwiz_json, validation_json, now],
    ).map_err(|error| {
        if matches!(error, rusqlite::Error::SqliteFailure(_, _)) { CommandError::new("duplicate_project", "This project directory is already registered").with_details(error.to_string()) } else { CommandError::new("database_write_failed", "Project could not be registered").with_details(error.to_string()) }
    })?;
    record_activity(&connection, &id, "registered", "Project registered")?;
    Ok(ProjectRecord {
        id,
        canonical_path: preview.canonical_path,
        application,
        packwiz: preview.packwiz,
        validation: preview.validation,
        created_at: now.clone(),
        updated_at: now.clone(),
        last_opened_at: Some(now.clone()),
        last_refreshed_at: Some(now),
    })
}

#[tauri::command]
pub fn list_projects(state: State<'_, Database>) -> Result<Vec<ProjectRecord>, CommandError> {
    let connection = state
        .0
        .lock()
        .map_err(|_| CommandError::new("database_unavailable", "Database is unavailable"))?;
    let mut statement = connection.prepare("SELECT id, canonical_path, application_json, packwiz_json, validation_json, created_at, updated_at, last_opened_at, last_refreshed_at FROM projects ORDER BY updated_at DESC").map_err(|error| CommandError::new("database_read_failed", "Projects could not be read").with_details(error.to_string()))?;
    let rows = statement.query_map([], row_to_project).map_err(|error| {
        CommandError::new("database_read_failed", "Projects could not be read")
            .with_details(error.to_string())
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|error| {
        CommandError::new("database_record_invalid", "A stored project is invalid")
            .with_details(error.to_string())
    })
}

#[tauri::command]
pub fn open_project(state: State<'_, Database>, id: String) -> Result<ProjectRecord, CommandError> {
    let connection = state
        .0
        .lock()
        .map_err(|_| CommandError::new("database_unavailable", "Database is unavailable"))?;
    let mut project = connection.query_row("SELECT id, canonical_path, application_json, packwiz_json, validation_json, created_at, updated_at, last_opened_at, last_refreshed_at FROM projects WHERE id = ?1", [&id], row_to_project).map_err(|error| CommandError::new("project_not_found", "Project is not registered").with_details(error.to_string()))?;
    let current = validation::preview(&project.canonical_path).map_err(|error| {
        CommandError::new(
            "project_state_unavailable",
            "Registered project files could not be reopened",
        )
        .with_details(format!("{}: {}", error.code, error.message))
    })?;
    project.packwiz = current.packwiz;
    project.validation = current.validation;
    let now = timestamp();
    connection
        .execute(
            "UPDATE projects SET last_opened_at = ?1, updated_at = ?1 WHERE id = ?2",
            params![now, id],
        )
        .map_err(|error| {
            CommandError::new(
                "database_write_failed",
                "Project open timestamp could not be saved",
            )
            .with_details(error.to_string())
        })?;
    project.last_opened_at = Some(now.clone());
    project.updated_at = now;
    record_activity(&connection, &project.id, "opened", "Project opened")?;
    Ok(project)
}

fn refresh_record(
    connection: &Connection,
    id: &str,
    preview: &RegistrationPreview,
) -> Result<ProjectRecord, CommandError> {
    let existing = connection.query_row("SELECT id, canonical_path, application_json, packwiz_json, validation_json, created_at, updated_at, last_opened_at, last_refreshed_at FROM projects WHERE id = ?1", [id], row_to_project).map_err(|error| CommandError::new("project_not_found", "Project is not registered").with_details(error.to_string()))?;
    let inventory = inventory::read_inventory(std::path::Path::new(&preview.canonical_path))?;
    let overview = inventory::read_overview(std::path::Path::new(&preview.canonical_path))?;
    let inventory_json = serialize(&inventory, "inventory")?;
    let overview_json = serialize(&overview, "overview")?;
    let now = timestamp();
    connection.execute("UPDATE projects SET canonical_path = ?1, packwiz_json = ?2, validation_json = ?3, updated_at = ?4, last_refreshed_at = ?4 WHERE id = ?5", params![preview.canonical_path, serialize(&preview.packwiz, "Packwiz observations")?, serialize(&preview.validation, "validation results")?, now, id]).map_err(|error| CommandError::new("database_write_failed", "Project refresh could not be saved").with_details(error.to_string()))?;
    connection
        .execute(
            "INSERT INTO inventory_observations (project_id, observed_at, freshness, inventory_json, overview_json) VALUES (?1, ?2, 'current', ?3, ?4) ON CONFLICT(project_id) DO UPDATE SET observed_at = excluded.observed_at, freshness = excluded.freshness, inventory_json = excluded.inventory_json, overview_json = excluded.overview_json",
            params![
                id,
                now,
                inventory_json,
                overview_json
            ],
        )
        .map_err(|error| {
            CommandError::new("database_write_failed", "Inventory observation could not be saved")
                .with_details(error.to_string())
        })?;
    record_activity(
        &connection,
        id,
        "refresh_succeeded",
        "Project inventory refreshed",
    )?;
    Ok(ProjectRecord {
        canonical_path: preview.canonical_path.clone(),
        packwiz: preview.packwiz.clone(),
        validation: preview.validation.clone(),
        updated_at: now.clone(),
        last_refreshed_at: Some(now),
        ..existing
    })
}

#[tauri::command]
pub fn refresh_project(
    state: State<'_, Database>,
    id: String,
) -> Result<ProjectRecord, CommandError> {
    refresh_project_inner(&state, &id)
}

fn refresh_project_inner(database: &Database, id: &str) -> Result<ProjectRecord, CommandError> {
    let connection = database
        .0
        .lock()
        .map_err(|_| CommandError::new("database_unavailable", "Database is unavailable"))?;
    let path: String = connection
        .query_row(
            "SELECT canonical_path FROM projects WHERE id = ?1",
            [id],
            |row| row.get(0),
        )
        .map_err(|error| {
            CommandError::new("project_not_found", "Project is not registered")
                .with_details(error.to_string())
        })?;
    let preview = match validation::preview(&path) {
        Ok(preview) => preview,
        Err(error) => {
            let _ = connection.execute(
                "UPDATE inventory_observations SET freshness = 'stale' WHERE project_id = ?1",
                [id],
            );
            let _ = record_activity(&connection, id, "refresh_failed", &error.message);
            return Err(error);
        }
    };
    refresh_record(&connection, id, &preview)
}

#[tauri::command]
pub fn get_project_inventory(
    state: State<'_, Database>,
    id: String,
) -> Result<Vec<InventoryEntry>, CommandError> {
    let connection = state
        .0
        .lock()
        .map_err(|_| CommandError::new("database_unavailable", "Database is unavailable"))?;
    let path: String = connection
        .query_row(
            "SELECT canonical_path FROM projects WHERE id = ?1",
            [&id],
            |row| row.get(0),
        )
        .map_err(|error| {
            CommandError::new("project_not_found", "Project is not registered")
                .with_details(error.to_string())
        })?;
    inventory::read_inventory(std::path::Path::new(&path))
}

#[tauri::command]
pub fn get_project_overview(
    state: State<'_, Database>,
    id: String,
) -> Result<ProjectOverview, CommandError> {
    let connection = state
        .0
        .lock()
        .map_err(|_| CommandError::new("database_unavailable", "Database is unavailable"))?;
    let path: String = connection
        .query_row(
            "SELECT canonical_path FROM projects WHERE id = ?1",
            [&id],
            |row| row.get(0),
        )
        .map_err(|error| {
            CommandError::new("project_not_found", "Project is not registered")
                .with_details(error.to_string())
        })?;
    let mut overview = inventory::read_overview(std::path::Path::new(&path))?;
    overview.activity = read_activity(&connection, &id)?;
    Ok(overview)
}

#[tauri::command]
pub fn update_project_metadata(
    state: State<'_, Database>,
    id: String,
    application: ApplicationProjectMetadata,
) -> Result<ProjectRecord, CommandError> {
    let connection = state
        .0
        .lock()
        .map_err(|_| CommandError::new("database_unavailable", "Database is unavailable"))?;
    let existing = connection.query_row("SELECT id, canonical_path, application_json, packwiz_json, validation_json, created_at, updated_at, last_opened_at, last_refreshed_at FROM projects WHERE id = ?1", [&id], row_to_project).map_err(|error| CommandError::new("project_not_found", "Project is not registered").with_details(error.to_string()))?;
    let now = timestamp();
    connection
        .execute(
            "UPDATE projects SET application_json = ?1, updated_at = ?2 WHERE id = ?3",
            params![serialize(&application, "application metadata")?, now, id],
        )
        .map_err(|error| {
            CommandError::new(
                "database_write_failed",
                "Project metadata could not be saved",
            )
            .with_details(error.to_string())
        })?;
    Ok(ProjectRecord {
        application,
        updated_at: now,
        ..existing
    })
}

fn set_lifecycle(
    database: &Database,
    id: String,
    lifecycle: ProjectLifecycle,
) -> Result<ProjectRecord, CommandError> {
    let connection = database
        .0
        .lock()
        .map_err(|_| CommandError::new("database_unavailable", "Database is unavailable"))?;
    let mut project = connection.query_row("SELECT id, canonical_path, application_json, packwiz_json, validation_json, created_at, updated_at, last_opened_at, last_refreshed_at FROM projects WHERE id = ?1", [&id], row_to_project).map_err(|error| CommandError::new("project_not_found", "Project is not registered").with_details(error.to_string()))?;
    project.application.lifecycle = lifecycle;
    let now = timestamp();
    connection
        .execute(
            "UPDATE projects SET application_json = ?1, updated_at = ?2 WHERE id = ?3",
            params![
                serialize(&project.application, "application metadata")?,
                now,
                id
            ],
        )
        .map_err(|error| {
            CommandError::new(
                "database_write_failed",
                "Project lifecycle could not be saved",
            )
            .with_details(error.to_string())
        })?;
    project.updated_at = now;
    Ok(project)
}

#[tauri::command]
pub fn archive_project(
    state: State<'_, Database>,
    id: String,
) -> Result<ProjectRecord, CommandError> {
    set_lifecycle(&state, id, ProjectLifecycle::Archived)
}

#[tauri::command]
pub fn restore_project(
    state: State<'_, Database>,
    id: String,
) -> Result<ProjectRecord, CommandError> {
    set_lifecycle(&state, id, ProjectLifecycle::Active)
}

#[tauri::command]
pub fn disconnect_project(
    state: State<'_, Database>,
    id: String,
) -> Result<ProjectRecord, CommandError> {
    set_lifecycle(&state, id, ProjectLifecycle::Disconnected)
}

#[tauri::command]
pub fn reconnect_project(
    state: State<'_, Database>,
    id: String,
    path: String,
) -> Result<ProjectRecord, CommandError> {
    reconnect_project_inner(&state, id, path)
}

fn reconnect_project_inner(
    database: &Database,
    id: String,
    path: String,
) -> Result<ProjectRecord, CommandError> {
    let preview = validation::preview(&path)?;
    if validation::has_errors(&preview.validation) {
        return Err(CommandError::new(
            "project_validation_failed",
            "The selected directory did not pass validation",
        ));
    }
    let connection = database
        .0
        .lock()
        .map_err(|_| CommandError::new("database_unavailable", "Database is unavailable"))?;
    refresh_record(&connection, &id, &preview)
}

#[tauri::command]
pub fn get_settings(state: State<'_, Database>) -> Result<HashMap<String, String>, CommandError> {
    let connection = state
        .0
        .lock()
        .map_err(|_| CommandError::new("database_unavailable", "Database is unavailable"))?;
    let mut statement = connection
        .prepare("SELECT key, value_json FROM settings")
        .map_err(|error| {
            CommandError::new("database_read_failed", "Settings could not be read")
                .with_details(error.to_string())
        })?;

    let rows = statement
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .map_err(|error| {
            CommandError::new("database_read_failed", "Settings could not be read")
                .with_details(error.to_string())
        })?;

    rows.collect::<Result<HashMap<_, _>, _>>().map_err(|error| {
        CommandError::new("database_read_failed", "Settings could not be read")
            .with_details(error.to_string())
    })
}

#[tauri::command]
pub fn set_setting(
    state: State<'_, Database>,
    key: String,
    value_json: String,
) -> Result<(), CommandError> {
    if key.trim().is_empty() {
        return Err(CommandError::new(
            "invalid_setting_key",
            "Setting key cannot be empty",
        ));
    }
    serde_json::from_str::<serde_json::Value>(&value_json).map_err(|error| {
        CommandError::new("invalid_setting_value", "Setting value is not valid JSON")
            .with_details(error.to_string())
    })?;

    let connection = state
        .0
        .lock()
        .map_err(|_| CommandError::new("database_unavailable", "Database is unavailable"))?;
    connection
        .execute(
            "INSERT INTO settings (key, value_json, updated_at) VALUES (?1, ?2, ?3)
             ON CONFLICT(key) DO UPDATE SET value_json = excluded.value_json, updated_at = excluded.updated_at",
            params![key, value_json, timestamp()],
        )
        .map_err(|error| CommandError::new("database_write_failed", "Setting could not be saved").with_details(error.to_string()))?;
    Ok(())
}

#[tauri::command]
pub fn delete_setting(state: State<'_, Database>, key: String) -> Result<(), CommandError> {
    let connection = state
        .0
        .lock()
        .map_err(|_| CommandError::new("database_unavailable", "Database is unavailable"))?;
    connection
        .execute("DELETE FROM settings WHERE key = ?1", [key])
        .map_err(|error| {
            CommandError::new("database_write_failed", "Setting could not be deleted")
                .with_details(error.to_string())
        })?;
    Ok(())
}

#[tauri::command]
pub fn reset_settings(state: State<'_, Database>) -> Result<(), CommandError> {
    let connection = state
        .0
        .lock()
        .map_err(|_| CommandError::new("database_unavailable", "Database is unavailable"))?;
    connection
        .execute("DELETE FROM settings", [])
        .map_err(|error| {
            CommandError::new("database_write_failed", "Settings could not be reset")
                .with_details(error.to_string())
        })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::initialize;
    use std::path::Path;

    #[test]
    fn creates_settings_schema_and_supports_updates() {
        let database = initialize(Path::new(":memory:")).expect("database should initialize");
        let connection = database
            .0
            .lock()
            .expect("database lock should be available");
        let project_table: String = connection
            .query_row(
                "SELECT name FROM sqlite_master WHERE type = 'table' AND name = 'projects'",
                [],
                |row| row.get(0),
            )
            .expect("project schema should be initialized");
        assert_eq!(project_table, "projects");
        connection
            .execute(
                "INSERT INTO settings (key, value_json, updated_at) VALUES (?1, ?2, ?3)",
                ["appearance.theme", "\\\"dark\\\"", "0"],
            )
            .expect("setting should be inserted");
        connection
            .execute(
                "UPDATE settings SET value_json = ?1 WHERE key = ?2",
                ["\\\"light\\\"", "appearance.theme"],
            )
            .expect("setting should be updated");
        let value: String = connection
            .query_row(
                "SELECT value_json FROM settings WHERE key = ?1",
                ["appearance.theme"],
                |row| row.get(0),
            )
            .expect("setting should be readable");
        assert_eq!(value, "\\\"light\\\"");
    }

    #[test]
    fn registration_lifecycle_preserves_identity_without_deleting_external_files() {
        let database = initialize(Path::new(":memory:")).expect("database should initialize");
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/packwiz/valid");
        let path = fixture.to_string_lossy().into_owned();
        let registered = super::register_project_inner(&database, path.clone(), None)
            .expect("valid fixture should register");
        let duplicate = super::register_project_inner(&database, path.clone(), None)
            .expect_err("equivalent project should be rejected");
        assert_eq!(duplicate.code, "duplicate_project");
        let disconnected = super::set_lifecycle(
            &database,
            registered.id.clone(),
            crate::domain::ProjectLifecycle::Disconnected,
        )
        .expect("project should disconnect");
        assert_eq!(
            disconnected.application.lifecycle,
            crate::domain::ProjectLifecycle::Disconnected
        );
        let reconnected = super::reconnect_project_inner(&database, registered.id.clone(), path)
            .expect("valid fixture should reconnect");
        assert_eq!(reconnected.id, registered.id);
        assert!(fixture.join("pack.toml").is_file());
    }

    #[test]
    fn refresh_persists_current_inventory_and_activity() {
        let database = initialize(Path::new(":memory:")).expect("database should initialize");
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/packwiz/mixed");
        let path = fixture.to_string_lossy().into_owned();
        let registered = super::register_project_inner(&database, path.clone(), None)
            .expect("valid fixture should register");
        let preview = crate::domain::validation::preview(&path).expect("fixture should preview");
        let connection = database
            .0
            .lock()
            .expect("database lock should be available");
        let refreshed = super::refresh_record(&connection, &registered.id, &preview)
            .expect("refresh should persist");
        assert_eq!(refreshed.last_refreshed_at.is_some(), true);
        let observation_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM inventory_observations WHERE project_id = ?1",
                [&registered.id],
                |row| row.get(0),
            )
            .unwrap();
        let activity_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM project_activity WHERE project_id = ?1 AND event_type = 'refresh_succeeded'",
                [&registered.id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(observation_count, 1);
        assert_eq!(activity_count, 1);
    }

    #[test]
    fn failed_refresh_marks_last_observation_stale_without_replacing_it() {
        let database = initialize(Path::new(":memory:")).expect("database should initialize");
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/packwiz/mixed");
        let path = fixture.to_string_lossy().into_owned();
        let registered = super::register_project_inner(&database, path.clone(), None)
            .expect("valid fixture should register");
        let preview = crate::domain::validation::preview(&path).expect("fixture should preview");
        let connection = database.0.lock().unwrap();
        super::refresh_record(&connection, &registered.id, &preview).unwrap();
        connection
            .execute(
                "UPDATE projects SET canonical_path = ?1 WHERE id = ?2",
                ["C:/missing-project", registered.id.as_str()],
            )
            .unwrap();
        drop(connection);

        let error = super::refresh_project_inner(&database, &registered.id).unwrap_err();
        assert_eq!(error.code, "project_unavailable");
        let connection = database.0.lock().unwrap();
        let freshness: String = connection
            .query_row(
                "SELECT freshness FROM inventory_observations WHERE project_id = ?1",
                [&registered.id],
                |row| row.get(0),
            )
            .unwrap();
        let failures: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM project_activity WHERE project_id = ?1 AND event_type = 'refresh_failed'",
                [&registered.id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(freshness, "stale");
        assert_eq!(failures, 1);
    }
}
