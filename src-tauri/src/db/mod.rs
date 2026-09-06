use crate::domain::CommandError;
use rusqlite::{params, Connection};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::State;

pub struct Database(pub Mutex<Connection>);

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
}
