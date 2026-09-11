pub mod lifecycle;

use crate::domain::{
    inventory, validation, ActivityRecord, ApplicationModpackMetadata, CommandError,
    DiscoveryResult, InventoryEntry, ModpackLifecycle, ModpackOverview, ModpackRecord,
    RegistrationPreview, ReleaseRecord, ReleaseWorkspaceObservationRecord, SnapshotCandidateRecord,
    SnapshotDecision, SnapshotDecisionRecord, SnapshotLifecycle, SnapshotNoteRecord,
    SnapshotNoteScope, SnapshotRecheckRecord, SnapshotRecord, UpdateCandidate,
};
use rusqlite::{params, Connection};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::State;

pub struct Database(pub Mutex<Connection>);

pub fn new_changelog_attempt_id() -> String {
    unique_id("changelog-attempt")
}

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
    let baseline_capture_json =
        if matches!(result.outcome, crate::domain::DiscoveryOutcomeKind::Normal) {
            let path: String = connection
                .query_row(
                    "SELECT canonical_path FROM modpacks WHERE id = ?1",
                    [project_id],
                    |row| row.get(0),
                )
                .map_err(|error| {
                    CommandError::new("project_not_found", "Project is not registered")
                        .with_details(error.to_string())
                })?;
            crate::domain::capture::capture(
                std::path::Path::new(&path),
                crate::domain::ReleaseEligibility {
                    eligible: true,
                    provisional: false,
                    source: Some(crate::domain::ReleaseEligibilitySource::ReviewableSnapshot),
                    validation: Vec::new(),
                    diagnostic: None,
                },
                None,
                Vec::new(),
            )
            .ok()
            .map(|capture| serialize(&capture, "snapshot baseline capture"))
            .transpose()?
        } else {
            None
        };
    let outcome =
        serde_json::to_string(&result.outcome).unwrap_or_else(|_| "indeterminate".to_string());
    let transaction = connection.unchecked_transaction().map_err(|error| {
        CommandError::new(
            "database_write_failed",
            "Discovery observation could not be started",
        )
        .with_details(error.to_string())
    })?;
    let observed_at = timestamp();
    transaction.execute("INSERT INTO discovery_attempts (modpack_id, observed_at, outcome, result_json) VALUES (?1, ?2, ?3, ?4)", params![project_id, observed_at, outcome.trim_matches('"'), result_json]).map_err(|error| CommandError::new("database_write_failed", "Discovery observation could not be saved").with_details(error.to_string()))?;
    let attempt_id = connection.last_insert_rowid();
    for candidate in &result.candidates {
        transaction
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
    let snapshot_id = format!("snapshot-{project_id}-{attempt_id}");
    let lifecycle = if baseline_capture_json.is_some()
        && matches!(result.outcome, crate::domain::DiscoveryOutcomeKind::Normal)
    {
        SnapshotLifecycle::Reviewable
    } else if matches!(
        result.outcome,
        crate::domain::DiscoveryOutcomeKind::Cancelled
    ) {
        SnapshotLifecycle::Cancelled
    } else {
        SnapshotLifecycle::Draft
    };
    let lifecycle_json = serde_json::to_string(&lifecycle).unwrap_or_else(|_| "draft".to_string());
    transaction.execute("INSERT INTO snapshots (id, modpack_id, lifecycle, outcome, result_json, baseline_capture_json, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)", params![snapshot_id, project_id, lifecycle_json.trim_matches('"'), outcome.trim_matches('"'), result_json, baseline_capture_json, observed_at]).map_err(|error| CommandError::new("database_write_failed", "Snapshot could not be saved").with_details(error.to_string()))?;
    for (index, candidate) in result.candidates.iter().enumerate() {
        let candidate_id = format!("{snapshot_id}-candidate-{index}");
        transaction.execute("INSERT INTO snapshot_candidates (id, snapshot_id, candidate_json, observed_at) VALUES (?1, ?2, ?3, ?4)", params![candidate_id, snapshot_id, serialize(candidate, "snapshot candidate")?, observed_at]).map_err(|error| CommandError::new("database_write_failed", "Snapshot candidate could not be saved").with_details(error.to_string()))?;
    }
    transaction
        .execute(
            "INSERT INTO modpack_activity (modpack_id, event_type, occurred_at, message) VALUES (?1, ?2, ?3, ?4)",
            params![project_id, "snapshot_created", observed_at, format!("Discovery snapshot {snapshot_id} created")],
        )
        .map_err(|error| {
            CommandError::new("database_write_failed", "Snapshot activity could not be saved")
                .with_details(error.to_string())
        })?;
    transaction.commit().map_err(|error| {
        CommandError::new(
            "database_write_failed",
            "Discovery snapshot could not be committed",
        )
        .with_details(error.to_string())
    })?;
    Ok(())
}

pub fn registered_modpack_path(database: &Database, id: &str) -> Result<String, CommandError> {
    let connection = database
        .0
        .lock()
        .map_err(|_| CommandError::new("database_unavailable", "Database is unavailable"))?;
    connection
        .query_row(
            "SELECT canonical_path FROM modpacks WHERE id = ?1",
            [id],
            |row| row.get(0),
        )
        .map_err(|error| {
            CommandError::new("project_not_found", "Project is not registered")
                .with_details(error.to_string())
        })
}

pub fn load_snapshot(database: &Database, id: &str) -> Result<SnapshotRecord, CommandError> {
    let connection = database
        .0
        .lock()
        .map_err(|_| CommandError::new("database_unavailable", "Database is unavailable"))?;
    let snapshot = connection
        .query_row(
            "SELECT id, modpack_id, predecessor_id, lifecycle, outcome, label, result_json, baseline_capture_json, created_at, updated_at, closed_at FROM snapshots WHERE id = ?1",
            [id],
            row_to_snapshot,
        )
        .map_err(|error| CommandError::new("snapshot_not_found", "Snapshot is not available").with_details(error.to_string()))?;
    enrich_snapshot(&connection, snapshot)
}

pub fn persist_operation_attempt(
    database: &Database,
    attempt: &crate::domain::OperationAttempt,
    metadata_path: &std::path::Path,
    entry_id: &str,
    requested_pin: bool,
) -> Result<(), CommandError> {
    let connection = database
        .0
        .lock()
        .map_err(|_| CommandError::new("database_unavailable", "Database is unavailable"))?;
    let transaction = connection.unchecked_transaction().map_err(|error| {
        CommandError::new(
            "database_write_failed",
            "Operation transaction could not be started",
        )
        .with_details(error.to_string())
    })?;
    transaction
        .execute(
            "INSERT INTO operation_attempts (id, modpack_id, workspace_id, snapshot_id, predecessor_id, kind, status, outcome, recovery_json, process_json, before_fingerprint_json, after_fingerprint_json, verification_json, error_json, created_at, finished_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
            params![
                attempt.id,
                attempt.modpack_id,
                attempt.workspace_id,
                attempt.snapshot_id,
                attempt.predecessor_id,
                serialize(&attempt.kind, "operation kind")?.trim_matches('"'),
                serialize(&attempt.status, "operation status")?.trim_matches('"'),
                attempt.outcome.as_ref().map(|value| serialize(value, "operation outcome")).transpose()?.map(|value| value.trim_matches('"').to_string()),
                serialize(&attempt.recovery, "recovery observation")?,
                attempt.process.as_ref().map(|value| serialize(value, "process evidence")).transpose()?,
                attempt.before_fingerprint.as_ref().map(|value| serialize(value, "before fingerprint")).transpose()?,
                attempt.after_fingerprint.as_ref().map(|value| serialize(value, "after fingerprint")).transpose()?,
                attempt.verification.as_ref().map(|value| serialize(value, "operation verification")).transpose()?,
                attempt.error.as_ref().map(|value| serialize(value, "operation error")).transpose()?,
                attempt.created_at,
                attempt.finished_at,
            ],
        )
        .map_err(|error| CommandError::new("database_write_failed", "Operation could not be saved").with_details(error.to_string()))?;
    transaction
        .execute(
            "INSERT INTO operation_candidates (operation_id, entry_id, decision, observed_json) VALUES (?1, ?2, ?3, ?4)",
            params![attempt.id, entry_id, if requested_pin { "pin" } else { "unpin" }, "{}"],
        )
        .map_err(|error| CommandError::new("database_write_failed", "Operation target could not be saved").with_details(error.to_string()))?;
    if let Some(verification) = &attempt.verification {
        transaction
            .execute(
                "INSERT INTO pin_observations (operation_id, entry_id, metadata_path, requested_pin, observed_pin, observed_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![attempt.id, entry_id, metadata_path.to_string_lossy(), requested_pin as i32, verification.observed_state, attempt.finished_at.clone().unwrap_or_else(timestamp)],
            )
            .map_err(|error| CommandError::new("database_write_failed", "Pin observation could not be saved").with_details(error.to_string()))?;
    }
    transaction.commit().map_err(|error| {
        CommandError::new(
            "database_write_failed",
            "Operation evidence could not be committed",
        )
        .with_details(error.to_string())
    })?;
    Ok(())
}

pub fn persist_apply_attempt(
    database: &Database,
    attempt: &crate::domain::OperationAttempt,
    candidate_id: &str,
    entry_id: &str,
    observed_json: &str,
    candidate_outcome: crate::domain::ReleaseCandidateOutcome,
) -> Result<(), CommandError> {
    let connection = database
        .0
        .lock()
        .map_err(|_| CommandError::new("database_unavailable", "Database is unavailable"))?;
    let transaction = connection.unchecked_transaction().map_err(|error| {
        CommandError::new(
            "database_write_failed",
            "Apply evidence could not be started",
        )
        .with_details(error.to_string())
    })?;
    transaction
        .execute(
            "INSERT INTO operation_attempts (id, modpack_id, workspace_id, snapshot_id, predecessor_id, kind, status, outcome, recovery_json, process_json, before_fingerprint_json, after_fingerprint_json, verification_json, error_json, created_at, finished_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
            params![
                attempt.id,
                attempt.modpack_id,
                attempt.workspace_id,
                attempt.snapshot_id,
                attempt.predecessor_id,
                serialize(&attempt.kind, "operation kind")?.trim_matches('"'),
                serialize(&attempt.status, "operation status")?.trim_matches('"'),
                attempt.outcome.as_ref().map(|value| serialize(value, "operation outcome")).transpose()?.map(|value| value.trim_matches('"').to_string()),
                serialize(&attempt.recovery, "recovery observation")?,
                attempt.process.as_ref().map(|value| serialize(value, "process evidence")).transpose()?,
                attempt.before_fingerprint.as_ref().map(|value| serialize(value, "before fingerprint")).transpose()?,
                attempt.after_fingerprint.as_ref().map(|value| serialize(value, "after fingerprint")).transpose()?,
                attempt.verification.as_ref().map(|value| serialize(value, "operation verification")).transpose()?,
                attempt.error.as_ref().map(|value| serialize(value, "operation error")).transpose()?,
                attempt.created_at,
                attempt.finished_at,
            ],
        )
        .map_err(|error| CommandError::new("database_write_failed", "Apply operation could not be saved").with_details(error.to_string()))?;
    transaction
        .execute(
            "INSERT INTO operation_candidates (operation_id, candidate_id, entry_id, decision, outcome, observed_json) VALUES (?1, ?2, ?3, 'selected', ?5, ?4)",
            params![attempt.id, candidate_id, entry_id, observed_json, serialize(&candidate_outcome, "candidate outcome")?.trim_matches('"').to_string()],
        )
        .map_err(|error| CommandError::new("database_write_failed", "Apply target could not be saved").with_details(error.to_string()))?;
    transaction.commit().map_err(|error| {
        CommandError::new(
            "database_write_failed",
            "Apply evidence could not be committed",
        )
        .with_details(error.to_string())
    })
}

pub fn list_operation_attempts(
    database: &Database,
    project_id: &str,
) -> Result<Vec<crate::domain::OperationAttempt>, CommandError> {
    let connection = database
        .0
        .lock()
        .map_err(|_| CommandError::new("database_unavailable", "Database is unavailable"))?;
    let mut statement = connection
        .prepare("SELECT id, modpack_id, workspace_id, snapshot_id, predecessor_id, kind, status, outcome, recovery_json, process_json, before_fingerprint_json, after_fingerprint_json, verification_json, error_json, created_at, finished_at FROM operation_attempts WHERE modpack_id = ?1 ORDER BY created_at DESC")
        .map_err(|error| CommandError::new("database_read_failed", "Operations could not be read").with_details(error.to_string()))?;
    let result = statement
        .query_map([project_id], |row| {
            let kind: String = row.get(5)?;
            let status: String = row.get(6)?;
            let outcome: Option<String> = row.get(7)?;
            let parse = |index: usize| -> rusqlite::Result<Option<serde_json::Value>> {
                let value: Option<String> = row.get(index)?;
                value
                    .map(|json| {
                        serde_json::from_str(&json).map_err(|_| rusqlite::Error::InvalidQuery)
                    })
                    .transpose()
            };
            Ok(crate::domain::OperationAttempt {
                id: row.get(0)?,
                modpack_id: row.get(1)?,
                workspace_id: row.get(2)?,
                snapshot_id: row.get(3)?,
                predecessor_id: row.get(4)?,
                kind: serde_json::from_value(serde_json::Value::String(kind))
                    .map_err(|_| rusqlite::Error::InvalidQuery)?,
                status: serde_json::from_value(serde_json::Value::String(status))
                    .map_err(|_| rusqlite::Error::InvalidQuery)?,
                outcome: outcome
                    .map(|value| {
                        serde_json::from_value(serde_json::Value::String(value))
                            .map_err(|_| rusqlite::Error::InvalidQuery)
                    })
                    .transpose()?,
                recovery: serde_json::from_value(parse(8)?.ok_or(rusqlite::Error::InvalidQuery)?)
                    .map_err(|_| rusqlite::Error::InvalidQuery)?,
                process: parse(9)?
                    .map(|value| {
                        serde_json::from_value(value).map_err(|_| rusqlite::Error::InvalidQuery)
                    })
                    .transpose()?,
                before_fingerprint: parse(10)?
                    .map(|value| {
                        serde_json::from_value(value).map_err(|_| rusqlite::Error::InvalidQuery)
                    })
                    .transpose()?,
                after_fingerprint: parse(11)?
                    .map(|value| {
                        serde_json::from_value(value).map_err(|_| rusqlite::Error::InvalidQuery)
                    })
                    .transpose()?,
                verification: parse(12)?
                    .map(|value| {
                        serde_json::from_value(value).map_err(|_| rusqlite::Error::InvalidQuery)
                    })
                    .transpose()?,
                error: parse(13)?
                    .map(|value| {
                        serde_json::from_value(value).map_err(|_| rusqlite::Error::InvalidQuery)
                    })
                    .transpose()?,
                created_at: row.get(14)?,
                finished_at: row.get(15)?,
            })
        })
        .map_err(|error| {
            CommandError::new("database_read_failed", "Operations could not be read")
                .with_details(error.to_string())
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| {
            CommandError::new("database_record_invalid", "A stored operation is invalid")
                .with_details(error.to_string())
        });
    result
}

#[tauri::command]
pub fn record_recovery_acknowledgement(
    state: State<'_, Database>,
    acknowledgement: crate::domain::RecoveryAcknowledgement,
) -> Result<(), CommandError> {
    let connection = state
        .0
        .lock()
        .map_err(|_| CommandError::new("database_unavailable", "Database is unavailable"))?;
    let (before_fingerprint, recovery_json): (Option<String>, String) = connection
        .query_row(
            "SELECT before_fingerprint_json, recovery_json FROM operation_attempts WHERE id = ?1",
            [&acknowledgement.operation_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|error| {
            CommandError::new("database_read_failed", "Operation could not be read")
                .with_details(error.to_string())
        })?;
    let observed_recovery: crate::domain::RecoveryObservation =
        serde_json::from_str(&recovery_json).map_err(|_| {
            CommandError::new(
                "operation_record_invalid",
                "Stored recovery evidence is invalid",
            )
        })?;
    if observed_recovery.warning_category.as_deref()
        != Some(acknowledgement.warning_category.as_str())
    {
        return Err(CommandError::new(
            "acknowledgement_mismatch",
            "Acknowledgement warning does not match the operation",
        ));
    }
    if let Some(before_fingerprint) = before_fingerprint {
        if before_fingerprint != acknowledgement.modpack_fingerprint {
            return Err(CommandError::new(
                "acknowledgement_mismatch",
                "Acknowledgement fingerprint does not match the operation",
            ));
        }
    }
    connection
        .execute(
            "INSERT INTO operation_acknowledgements (operation_id, modpack_fingerprint, warning_category, recovery_state, acknowledged_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![acknowledgement.operation_id, acknowledgement.modpack_fingerprint, acknowledgement.warning_category, acknowledgement.recovery_state, acknowledgement.acknowledged_at],
        )
        .map_err(|error| CommandError::new("database_write_failed", "Recovery acknowledgement could not be saved").with_details(error.to_string()))?;
    Ok(())
}

#[tauri::command]
pub fn get_operation_history(
    state: State<'_, Database>,
    project_id: String,
) -> Result<Vec<crate::domain::OperationAttempt>, CommandError> {
    list_operation_attempts(&state, &project_id)
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
    let schema_metadata_exists: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'schema_metadata')",
        [],
        |row| row.get(0),
    )?;
    if schema_metadata_exists {
        let version: String = connection.query_row(
            "SELECT value FROM schema_metadata WHERE key = 'schema_version'",
            [],
            |row| row.get(0),
        )?;
        if version != "4" {
            return Err(rusqlite::Error::InvalidQuery);
        }
    } else {
        let has_existing_tables: bool = connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%')",
            [],
            |row| row.get(0),
        )?;
        if has_existing_tables {
            return Err(rusqlite::Error::InvalidQuery);
        }
        connection.execute_batch(include_str!("schema.sql"))?;
    }
    connection.execute(
        "UPDATE lifecycle_operations SET status = 'failed', finished_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), error_json = '{\"code\":\"stale_operation\",\"message\":\"Recovered after application restart\"}' WHERE status = 'running'",
        [],
    )?;
    connection.execute(
        "UPDATE cleanup_operations SET status = 'failed', finished_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), error_json = '{\"code\":\"stale_operation\",\"message\":\"Recovered after application restart\"}' WHERE status = 'running'",
        [],
    )?;
    Ok(Database(Mutex::new(connection)))
}

pub fn persist_release(database: &Database, release: &ReleaseRecord) -> Result<(), CommandError> {
    let connection = database
        .0
        .lock()
        .map_err(|_| CommandError::new("database_unavailable", "Database is unavailable"))?;
    let transaction = connection.unchecked_transaction().map_err(|error| {
        CommandError::new(
            "database_write_failed",
            "Release transaction could not be started",
        )
        .with_details(error.to_string())
    })?;
    let modpack_exists: bool = transaction
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM modpacks WHERE id = ?1)",
            [&release.modpack_id],
            |row| row.get(0),
        )
        .map_err(|error| {
            CommandError::new("project_not_found", "Project is not registered")
                .with_details(error.to_string())
        })?;
    if !modpack_exists {
        return Err(CommandError::new(
            "project_not_found",
            "Project is not registered",
        ));
    }
    if let Some(snapshot_id) = &release.capture.snapshot_id {
        let same_modpack: bool = transaction
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM snapshots WHERE id = ?1 AND modpack_id = ?2)",
                params![snapshot_id, release.modpack_id],
                |row| row.get(0),
            )
            .map_err(|error| {
                CommandError::new("snapshot_not_found", "Snapshot is not available")
                    .with_details(error.to_string())
            })?;
        if !same_modpack {
            return Err(CommandError::new(
                "snapshot_project_mismatch",
                "Snapshot does not belong to the release project",
            ));
        }
    }
    let status = serialize(&release.metadata.publication_status, "release status")?
        .trim_matches('"')
        .to_string();
    transaction
        .execute(
            "INSERT INTO releases (id, modpack_id, name, version, description, notes, publication_status, capture_json, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                release.id,
                release.modpack_id,
                release.metadata.name,
                release.metadata.version,
                release.metadata.description,
                release.metadata.notes,
                status,
                serialize(&release.capture, "release capture")?,
                release.created_at,
                release.updated_at,
            ],
        )
        .map_err(|error| CommandError::new("database_write_failed", "Release could not be saved").with_details(error.to_string()))?;
    if let Some(snapshot_id) = &release.capture.snapshot_id {
        transaction
            .execute(
                "INSERT INTO release_snapshots (release_id, snapshot_id) VALUES (?1, ?2)",
                params![release.id, snapshot_id],
            )
            .map_err(|error| {
                CommandError::new(
                    "database_write_failed",
                    "Release snapshot link could not be saved",
                )
                .with_details(error.to_string())
            })?;
    }
    for artifact_id in &release.capture.changelog_artifact_ids {
        let same_modpack: bool = transaction
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM changelog_artifacts WHERE id = ?1 AND modpack_id = ?2)",
                params![artifact_id, release.modpack_id],
                |row| row.get(0),
            )
            .map_err(|error| CommandError::new("changelog_artifact_not_found", "Changelog artifact is not available").with_details(error.to_string()))?;
        if !same_modpack {
            return Err(CommandError::new(
                "changelog_project_mismatch",
                "Changelog artifact does not belong to the release project",
            ));
        }
        transaction
            .execute(
                "INSERT INTO release_changelog_artifacts (release_id, artifact_id) VALUES (?1, ?2)",
                params![release.id, artifact_id],
            )
            .map_err(|error| {
                CommandError::new(
                    "database_write_failed",
                    "Release changelog link could not be saved",
                )
                .with_details(error.to_string())
            })?;
    }
    transaction.commit().map_err(|error| {
        CommandError::new("database_write_failed", "Release could not be committed")
            .with_details(error.to_string())
    })
}

pub fn load_release(database: &Database, id: &str) -> Result<ReleaseRecord, CommandError> {
    let connection = database
        .0
        .lock()
        .map_err(|_| CommandError::new("database_unavailable", "Database is unavailable"))?;
    connection
        .query_row(
            "SELECT id, modpack_id, name, version, description, notes, publication_status, capture_json, created_at, updated_at FROM releases WHERE id = ?1",
            [id],
            row_to_release,
        )
        .map_err(|error| CommandError::new("release_not_found", "Release is not available").with_details(error.to_string()))
}

pub fn list_releases(
    database: &Database,
    modpack_id: &str,
) -> Result<Vec<ReleaseRecord>, CommandError> {
    let connection = database
        .0
        .lock()
        .map_err(|_| CommandError::new("database_unavailable", "Database is unavailable"))?;
    let mut statement = connection
        .prepare("SELECT id, modpack_id, name, version, description, notes, publication_status, capture_json, created_at, updated_at FROM releases WHERE modpack_id = ?1 ORDER BY created_at DESC")
        .map_err(|error| CommandError::new("database_read_failed", "Releases could not be read").with_details(error.to_string()))?;
    let result = statement
        .query_map([modpack_id], row_to_release)
        .map_err(|error| {
            CommandError::new("database_read_failed", "Releases could not be read")
                .with_details(error.to_string())
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| {
            CommandError::new("release_record_invalid", "A stored release is invalid")
                .with_details(error.to_string())
        });
    result
}

fn workspace_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<crate::domain::ReleaseWorkspace> {
    let baseline_origin: String = row.get(3)?;
    let lifecycle: String = row.get(10)?;
    let evidence_status: String = row.get(11)?;
    let publication_status: String = row.get(12)?;
    Ok(crate::domain::ReleaseWorkspace {
        id: row.get(0)?,
        modpack_id: row.get(1)?,
        source_snapshot_id: row.get(2)?,
        baseline_origin: serde_json::from_value(serde_json::Value::String(baseline_origin))
            .map_err(|_| rusqlite::Error::InvalidQuery)?,
        baseline_release_id: row.get(4)?,
        baseline_capture: row
            .get::<_, Option<String>>(5)?
            .map(|value| serde_json::from_str(&value).map_err(|_| rusqlite::Error::InvalidQuery))
            .transpose()?,
        metadata: crate::domain::ReleaseMetadata {
            name: row.get(6)?,
            version: row.get(7)?,
            description: row.get(8)?,
            notes: row.get(9)?,
            publication_status: serde_json::from_value(serde_json::Value::String(
                publication_status.clone(),
            ))
            .map_err(|_| rusqlite::Error::InvalidQuery)?,
        },
        lifecycle: serde_json::from_value(serde_json::Value::String(lifecycle))
            .map_err(|_| rusqlite::Error::InvalidQuery)?,
        evidence_status: serde_json::from_value(serde_json::Value::String(evidence_status))
            .map_err(|_| rusqlite::Error::InvalidQuery)?,
        publication_status: serde_json::from_value(serde_json::Value::String(
            row.get::<_, String>(12)?,
        ))
        .map_err(|_| rusqlite::Error::InvalidQuery)?,
        final_capture: row
            .get::<_, Option<String>>(13)?
            .map(|value| serde_json::from_str(&value).map_err(|_| rusqlite::Error::InvalidQuery))
            .transpose()?,
        final_changelog_revision_id: row.get(14)?,
        finalization_receipt: row
            .get::<_, Option<String>>(15)?
            .map(|value| serde_json::from_str(&value).map_err(|_| rusqlite::Error::InvalidQuery))
            .transpose()?,
        phase: crate::domain::ReleaseWorkspacePhase::Review,
        blocking_reason: None,
        evidence_freshness: crate::domain::ReleaseWorkspaceEvidenceFreshness::Current,
        primary_next_action: "review_candidates".to_string(),
        created_at: row.get(16)?,
        updated_at: row.get(17)?,
        abandoned_at: row.get(18)?,
        candidates: Vec::new(),
        activity: Vec::new(),
        activity_count: 0,
    })
}

fn enrich_workspace(
    connection: &Connection,
    mut workspace: crate::domain::ReleaseWorkspace,
) -> Result<crate::domain::ReleaseWorkspace, CommandError> {
    workspace.candidates = connection
        .prepare("SELECT candidates.id, candidates.source_candidate_id, sources.candidate_json, candidates.decision, candidates.note, candidates.recorded_at FROM release_workspace_candidates candidates JOIN release_workspace_candidate_sources sources ON sources.id = candidates.source_candidate_id WHERE candidates.workspace_id = ?1 ORDER BY candidates.id")
        .map_err(|error| CommandError::new("database_read_failed", "Workspace candidates could not be read").with_details(error.to_string()))?
        .query_map([&workspace.id], |row| {
            let decision: String = row.get(3)?;
            Ok(crate::domain::ReleaseWorkspaceCandidate {
                id: row.get(0)?,
                source_candidate_id: row.get(1)?,
                candidate: serde_json::from_str(&row.get::<_, String>(2)?).map_err(|_| rusqlite::Error::InvalidQuery)?,
                decision: serde_json::from_value(serde_json::Value::String(decision)).map_err(|_| rusqlite::Error::InvalidQuery)?,
                note: row.get(4)?,
                recorded_at: row.get(5)?,
            })
        })
        .map_err(|error| CommandError::new("database_read_failed", "Workspace candidates could not be read").with_details(error.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| CommandError::new("database_record_invalid", "A stored workspace candidate is invalid").with_details(error.to_string()))?;
    workspace.activity_count = connection
        .query_row(
            "SELECT COUNT(*) FROM release_workspace_activity WHERE workspace_id = ?1",
            [&workspace.id],
            |row| row.get(0),
        )
        .map_err(|error| {
            CommandError::new(
                "database_read_failed",
                "Workspace activity count could not be read",
            )
            .with_details(error.to_string())
        })?;
    let (phase, blocking_reason, evidence_freshness, primary_next_action) =
        crate::domain::project_workspace_state(
            &workspace.lifecycle,
            &workspace.evidence_status,
            &workspace.candidates,
        );
    workspace.phase = phase;
    workspace.blocking_reason = blocking_reason;
    workspace.evidence_freshness = evidence_freshness;
    workspace.primary_next_action = primary_next_action;
    Ok(workspace)
}

pub(crate) fn load_release_workspace_inner(
    database: &Database,
    id: &str,
) -> Result<crate::domain::ReleaseWorkspace, CommandError> {
    let connection = database
        .0
        .lock()
        .map_err(|_| CommandError::new("database_unavailable", "Database is unavailable"))?;
    let workspace = connection
        .query_row(
            "SELECT id, modpack_id, source_snapshot_id, baseline_origin, baseline_release_id, baseline_capture_json, name, version, description, notes, lifecycle, evidence_status, publication_status, final_capture_json, final_changelog_revision_id, finalization_receipt_json, created_at, updated_at, abandoned_at FROM release_workspaces WHERE id = ?1",
            [id],
            workspace_from_row,
        )
        .map_err(|error| CommandError::new("workspace_not_found", "Release workspace is not available").with_details(error.to_string()))?;
    enrich_workspace(&connection, workspace)
}

pub(crate) fn list_release_workspaces_for_watch(
    database: &Database,
) -> Result<Vec<(String, String)>, CommandError> {
    let connection = database
        .0
        .lock()
        .map_err(|_| CommandError::new("database_unavailable", "Database is unavailable"))?;
    let mut statement = connection
        .prepare("SELECT id, modpack_id FROM release_workspaces WHERE lifecycle IN ('draft', 'applying', 'recovery_required', 'provisional', 'ready_to_finalize')")
        .map_err(|error| CommandError::new("database_read_failed", "Active release workspaces could not be read").with_details(error.to_string()))?;
    let result = statement
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .map_err(|error| {
            CommandError::new(
                "database_read_failed",
                "Active release workspaces could not be read",
            )
            .with_details(error.to_string())
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| {
            CommandError::new(
                "database_record_invalid",
                "An active release workspace is invalid",
            )
            .with_details(error.to_string())
        });
    result
}

pub(crate) fn record_workspace_observation(
    database: &Database,
    observation: &crate::domain::ReleaseWorkspaceObservation,
) -> Result<(), CommandError> {
    let connection = database
        .0
        .lock()
        .map_err(|_| CommandError::new("database_unavailable", "Database is unavailable"))?;
    let transaction = connection.unchecked_transaction().map_err(|error| {
        CommandError::new(
            "database_write_failed",
            "Workspace observation could not start",
        )
        .with_details(error.to_string())
    })?;
    let now = timestamp();
    let changed_scope_json = serialize(&observation.changed_scope, "workspace observation scope")?;
    let fingerprint_json = observation
        .fingerprint
        .as_ref()
        .map(|fingerprint| serialize(fingerprint, "workspace observation fingerprint"))
        .transpose()?;
    transaction
        .execute(
            "INSERT INTO release_workspace_observations (workspace_id, observed_at, changed_scope_json, evidence_freshness, blocking_reason, overlapped_operation, fingerprint_json) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                observation.workspace_id,
                now,
                changed_scope_json,
                serde_json::to_string(&observation.evidence_freshness).unwrap_or_else(|_| "\"unavailable\"".to_string()),
                observation.blocking_reason,
                observation.overlapped_operation,
                fingerprint_json,
            ],
        )
        .map_err(|error| CommandError::new("database_write_failed", "Workspace observation could not be saved").with_details(error.to_string()))?;
    let overlap = if observation.overlapped_operation {
        " during an application attempt"
    } else {
        ""
    };
    let message = format!(
        "External Packwiz change observed in {}{}",
        observation.changed_scope.join(", "),
        overlap
    );
    transaction
        .execute(
            "INSERT INTO release_workspace_activity (workspace_id, event_type, occurred_at, message) VALUES (?1, 'external_change_observed', ?2, ?3)",
            params![observation.workspace_id, now, message],
        )
        .map_err(|error| CommandError::new("database_write_failed", "Workspace observation activity could not be saved").with_details(error.to_string()))?;
    transaction.commit().map_err(|error| {
        CommandError::new(
            "database_write_failed",
            "Workspace observation could not be committed",
        )
        .with_details(error.to_string())
    })
}

#[tauri::command]
pub fn list_release_workspace_observations(
    state: State<'_, Database>,
    workspace_id: String,
) -> Result<Vec<ReleaseWorkspaceObservationRecord>, CommandError> {
    let connection = state
        .0
        .lock()
        .map_err(|_| CommandError::new("database_unavailable", "Database is unavailable"))?;
    let mut statement = connection
        .prepare(
            "SELECT id, observed_at, changed_scope_json, evidence_freshness, blocking_reason, overlapped_operation, fingerprint_json FROM release_workspace_observations WHERE workspace_id = ?1 ORDER BY id ASC",
        )
        .map_err(|error| CommandError::new("database_read_failed", "Workspace observations could not be read").with_details(error.to_string()))?;
    let rows = statement
        .query_map([workspace_id], |row| {
            let changed_scope_json: String = row.get(2)?;
            let freshness_json: String = row.get(3)?;
            let fingerprint_json: Option<String> = row.get(6)?;
            Ok(ReleaseWorkspaceObservationRecord {
                id: row.get(0)?,
                observed_at: row.get(1)?,
                changed_scope: serde_json::from_str(&changed_scope_json).map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(
                        changed_scope_json.len(),
                        rusqlite::types::Type::Text,
                        Box::new(error),
                    )
                })?,
                evidence_freshness: serde_json::from_str(&freshness_json).map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(
                        freshness_json.len(),
                        rusqlite::types::Type::Text,
                        Box::new(error),
                    )
                })?,
                blocking_reason: row.get(4)?,
                overlapped_operation: row.get(5)?,
                fingerprint: fingerprint_json
                    .map(|value| serde_json::from_str(&value))
                    .transpose()
                    .map_err(|error| {
                        rusqlite::Error::FromSqlConversionFailure(
                            0,
                            rusqlite::types::Type::Text,
                            Box::new(error),
                        )
                    })?,
            })
        })
        .map_err(|error| {
            CommandError::new(
                "database_read_failed",
                "Workspace observations could not be read",
            )
            .with_details(error.to_string())
        })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|error| {
        CommandError::new(
            "database_record_invalid",
            "A workspace observation is invalid",
        )
        .with_details(error.to_string())
    })
}

#[tauri::command]
pub fn get_release_workspace_evidence(
    state: State<'_, Database>,
    workspace_id: String,
) -> Result<crate::domain::ReleaseWorkspaceEvidence, CommandError> {
    let workspace = load_release_workspace_inner(&state, &workspace_id)?;
    let project_path = {
        let connection = state
            .0
            .lock()
            .map_err(|_| CommandError::new("database_unavailable", "Database is unavailable"))?;
        connection
            .query_row(
                "SELECT canonical_path FROM modpacks WHERE id = ?1",
                [&workspace.modpack_id],
                |row| row.get::<_, String>(0),
            )
            .map_err(|error| {
                CommandError::new(
                    "project_not_found",
                    "The workspace project is not registered",
                )
                .with_details(error.to_string())
            })?
    };
    let inventory = crate::domain::inventory::read_inventory(std::path::Path::new(&project_path))?;
    let observations = list_release_workspace_observations(state, workspace_id)?;
    Ok(crate::domain::ReleaseWorkspaceEvidence {
        workspace,
        inventory,
        observations,
    })
}

pub(crate) fn persist_finalized_release_workspace(
    database: &Database,
    workspace_id: &str,
    capture: &crate::domain::ReleaseCapture,
    receipt: &crate::domain::ReleaseReceipt,
) -> Result<(), CommandError> {
    let connection = database
        .0
        .lock()
        .map_err(|_| CommandError::new("database_unavailable", "Database is unavailable"))?;
    let capture_json = serialize(capture, "final release capture")?;
    let receipt_json = serialize(receipt, "release receipt")?;
    let now = timestamp();
    let transaction = connection.unchecked_transaction().map_err(|error| {
        CommandError::new("database_write_failed", "Finalization could not be started")
            .with_details(error.to_string())
    })?;
    let revision_context: (String, String, String, String, String, i64, Option<String>) = transaction
        .query_row(
            "SELECT a.id, a.release_workspace_id, a.snapshot_id, a.status, a.stage, r.frozen, r.archived_at FROM changelog_revisions r JOIN changelog_artifacts a ON a.id = r.artifact_id WHERE r.id = ?1",
            [&receipt.final_changelog_revision_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                ))
            },
        )
        .map_err(|error| {
            CommandError::new("final_changelog_required", "The final changelog revision is not available")
                .with_details(error.to_string())
        })?;
    if revision_context.1 != receipt.workspace_id
        || revision_context.3 != "complete"
        || revision_context.4 != "proposed"
        || revision_context.5 != 0
        || revision_context.6.is_some()
    {
        return Err(CommandError::new(
            "final_changelog_invalid",
            "The selected changelog revision is not an editable proposed revision for this workspace",
        ));
    }
    transaction
        .execute(
            "UPDATE changelog_artifacts SET stage = 'final', source_capture_fingerprint = ?1, updated_at = ?2 WHERE id = ?3",
            params![capture.capture_fingerprint, now, revision_context.0],
        )
        .map_err(|error| {
            CommandError::new("database_write_failed", "Final changelog artifact could not be frozen")
                .with_details(error.to_string())
        })?;
    transaction
        .execute(
            "UPDATE changelog_revisions SET frozen = 1 WHERE id = ?1",
            [&receipt.final_changelog_revision_id],
        )
        .map_err(|error| {
            CommandError::new(
                "database_write_failed",
                "Final changelog revision could not be frozen",
            )
            .with_details(error.to_string())
        })?;
    let changed = transaction
        .execute(
            "UPDATE release_workspaces SET lifecycle = 'finalized', evidence_status = 'baseline', final_capture_json = ?1, final_changelog_revision_id = ?2, finalization_receipt_json = ?3, updated_at = ?4 WHERE id = ?5 AND lifecycle IN ('ready_to_finalize', 'provisional') AND final_capture_json IS NULL",
            params![capture_json, receipt.final_changelog_revision_id, receipt_json, now, workspace_id],
        )
        .map_err(|error| {
            CommandError::new("database_write_failed", "Finalization could not be saved")
                .with_details(error.to_string())
        })?;
    if changed == 0 {
        return Err(CommandError::new(
            "invalid_finalization_state",
            "Workspace is not eligible for finalization or is already finalized",
        ));
    }
    transaction
        .execute(
            "INSERT INTO release_workspace_activity (workspace_id, event_type, occurred_at, message) VALUES (?1, 'finalized', ?2, ?3)",
            params![workspace_id, now, "Release workspace finalized"],
        )
        .map_err(|error| {
            CommandError::new("database_write_failed", "Finalization activity could not be saved")
                .with_details(error.to_string())
        })?;
    transaction.commit().map_err(|error| {
        CommandError::new(
            "database_write_failed",
            "Finalization could not be committed",
        )
        .with_details(error.to_string())
    })
}

#[tauri::command]
pub fn finalize_release_workspace(
    state: State<'_, Database>,
    request: crate::domain::FinalizeReleaseWorkspaceRequest,
) -> Result<crate::domain::ReleaseWorkspace, CommandError> {
    let workspace = load_release_workspace_inner(&state, &request.workspace_id)?;
    if !matches!(
        workspace.lifecycle,
        crate::domain::ReleaseWorkspaceLifecycle::ReadyToFinalize
            | crate::domain::ReleaseWorkspaceLifecycle::Provisional
    ) {
        return Err(CommandError::new(
            "invalid_finalization_state",
            "Workspace is not ready to finalize",
        ));
    }
    let baseline = workspace.baseline_capture.clone().ok_or_else(|| {
        CommandError::new(
            "baseline_capture_unavailable",
            "The release workspace has no validated baseline capture",
        )
    })?;
    let project_path = registered_modpack_path(&state, &workspace.modpack_id)?;
    let final_capture = crate::domain::capture::capture(
        std::path::Path::new(&project_path),
        baseline.eligibility.clone(),
        workspace.source_snapshot_id.clone(),
        Vec::new(),
    )?;
    let unresolved_candidates = workspace.candidates.iter().any(|candidate| {
        !matches!(
            candidate.decision,
            crate::domain::ReleaseCandidateDecision::Selected
                | crate::domain::ReleaseCandidateDecision::Skipped
                | crate::domain::ReleaseCandidateDecision::Pinned
        )
    });
    crate::domain::validate_finalization(
        &workspace.lifecycle,
        &baseline.capture_fingerprint,
        &final_capture,
        Some(&request.final_changelog_revision_id),
        unresolved_candidates,
    )?;
    let receipt = crate::domain::ReleaseReceipt {
        workspace_id: workspace.id.clone(),
        source_snapshot_id: workspace.source_snapshot_id.clone(),
        baseline_fingerprint: baseline.capture_fingerprint.clone(),
        final_capture_fingerprint: final_capture.capture_fingerprint.clone(),
        changed: true,
        stable: true,
        validated: true,
        final_changelog_revision_id: request.final_changelog_revision_id.clone(),
        finalized_at: timestamp(),
    };
    persist_finalized_release_workspace(&state, &workspace.id, &final_capture, &receipt)?;
    load_release_workspace_inner(&state, &workspace.id)
}

#[tauri::command]
pub fn publish_release_workspace(
    state: State<'_, Database>,
    request: crate::domain::PublishReleaseWorkspaceRequest,
) -> Result<crate::domain::ReleaseWorkspace, CommandError> {
    let workspace = load_release_workspace_inner(&state, &request.workspace_id)?;
    if !matches!(
        workspace.lifecycle,
        crate::domain::ReleaseWorkspaceLifecycle::Finalized
    ) {
        return Err(CommandError::new(
            "invalid_publication_state",
            "Only finalized workspaces can be published",
        ));
    }
    update_release_workspace_lifecycle(
        &state,
        &workspace.id,
        crate::domain::ReleaseWorkspaceLifecycle::Published,
        "Release workspace published",
    )?;
    load_release_workspace_inner(&state, &workspace.id)
}

#[tauri::command]
pub fn withdraw_release_workspace(
    state: State<'_, Database>,
    request: crate::domain::WithdrawReleaseWorkspaceRequest,
) -> Result<crate::domain::ReleaseWorkspace, CommandError> {
    let workspace = load_release_workspace_inner(&state, &request.workspace_id)?;
    if !matches!(
        workspace.lifecycle,
        crate::domain::ReleaseWorkspaceLifecycle::Published
    ) {
        return Err(CommandError::new(
            "invalid_withdrawal_state",
            "Only published workspaces can be withdrawn",
        ));
    }
    update_release_workspace_lifecycle(
        &state,
        &workspace.id,
        crate::domain::ReleaseWorkspaceLifecycle::Withdrawn,
        "Release workspace withdrawn",
    )?;
    load_release_workspace_inner(&state, &workspace.id)
}

pub(crate) fn update_release_workspace_lifecycle(
    database: &Database,
    workspace_id: &str,
    lifecycle: crate::domain::ReleaseWorkspaceLifecycle,
    message: &str,
) -> Result<(), CommandError> {
    let connection = database
        .0
        .lock()
        .map_err(|_| CommandError::new("database_unavailable", "Database is unavailable"))?;
    let now = timestamp();
    let lifecycle_value = serialize(&lifecycle, "workspace lifecycle")?
        .trim_matches('"')
        .to_string();
    let transaction = connection.unchecked_transaction().map_err(|error| {
        CommandError::new(
            "database_write_failed",
            "Workspace transition could not be started",
        )
        .with_details(error.to_string())
    })?;
    let changed = transaction
        .execute(
            "UPDATE release_workspaces SET lifecycle = ?1, publication_status = CASE WHEN ?1 IN ('published', 'withdrawn') THEN ?1 ELSE publication_status END, updated_at = ?2 WHERE id = ?3",
            params![lifecycle_value, now, workspace_id],
        )
        .map_err(|error| {
            CommandError::new(
                "database_write_failed",
                "Workspace transition could not be saved",
            )
            .with_details(error.to_string())
        })?;
    if changed == 0 {
        return Err(CommandError::new(
            "workspace_not_found",
            "Release workspace is not available",
        ));
    }
    transaction.execute("INSERT INTO release_workspace_activity (workspace_id, event_type, occurred_at, message) VALUES (?1, 'workspace_transition', ?2, ?3)", params![workspace_id, now, message]).map_err(|error| CommandError::new("database_write_failed", "Workspace activity could not be saved").with_details(error.to_string()))?;
    transaction.commit().map_err(|error| {
        CommandError::new(
            "database_write_failed",
            "Workspace transition could not be committed",
        )
        .with_details(error.to_string())
    })
}

pub(crate) fn start_release_workspace_operation(
    database: &Database,
    operation_id: &str,
    workspace_id: &str,
    snapshot_id: Option<&str>,
) -> Result<(), CommandError> {
    let connection = database
        .0
        .lock()
        .map_err(|_| CommandError::new("database_unavailable", "Database is unavailable"))?;
    connection
        .execute(
            "INSERT INTO release_workspace_operations (id, workspace_id, snapshot_id, status, created_at) VALUES (?1, ?2, ?3, 'running', ?4)",
            params![operation_id, workspace_id, snapshot_id, timestamp()],
        )
        .map_err(|error| {
            CommandError::new(
                "database_write_failed",
                "Workspace operation could not be started",
            )
            .with_details(error.to_string())
        })?;
    Ok(())
}

pub(crate) fn candidate_source_json(
    database: &Database,
    workspace_id: &str,
    candidate_id: &str,
) -> Result<String, CommandError> {
    let connection = database
        .0
        .lock()
        .map_err(|_| CommandError::new("database_unavailable", "Database is unavailable"))?;
    connection
        .query_row(
            "SELECT candidate_json FROM release_workspace_candidate_sources WHERE id = ?1 AND workspace_id = ?2",
            params![candidate_id, workspace_id],
            |row| row.get(0),
        )
        .map_err(|error| {
            CommandError::new("candidate_not_found", "Selected candidate does not belong to the release workspace")
                .with_details(error.to_string())
        })
}

pub(crate) fn finish_release_workspace_operation(
    database: &Database,
    operation_id: &str,
    outcome: &crate::domain::OperationOutcome,
) -> Result<(), CommandError> {
    let connection = database
        .0
        .lock()
        .map_err(|_| CommandError::new("database_unavailable", "Database is unavailable"))?;
    let changed = connection
        .execute(
            "UPDATE release_workspace_operations SET status = 'completed', outcome = ?1, finished_at = ?2 WHERE id = ?3",
            params![serialize(outcome, "operation outcome")?.trim_matches('"'), timestamp(), operation_id],
        )
        .map_err(|error| {
            CommandError::new(
                "database_write_failed",
                "Workspace operation could not be completed",
            )
            .with_details(error.to_string())
        })?;
    if changed == 0 {
        return Err(CommandError::new(
            "operation_not_found",
            "Workspace operation is not available",
        ));
    }
    Ok(())
}

#[tauri::command]
pub fn load_release_workspace(
    state: State<'_, Database>,
    id: String,
) -> Result<crate::domain::ReleaseWorkspace, CommandError> {
    load_release_workspace_inner(&state, &id)
}

#[tauri::command]
pub fn list_release_workspace_activity(
    state: State<'_, Database>,
    workspace_id: String,
    before_id: Option<i64>,
    limit: Option<u32>,
) -> Result<crate::domain::ReleaseWorkspaceActivityPage, CommandError> {
    let connection = state
        .0
        .lock()
        .map_err(|_| CommandError::new("database_unavailable", "Database is unavailable"))?;
    let requested_limit = i64::from(limit.unwrap_or(10).clamp(1, 10));
    let total_count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM release_workspace_activity WHERE workspace_id = ?1",
            [&workspace_id],
            |row| row.get(0),
        )
        .map_err(|error| {
            CommandError::new(
                "database_read_failed",
                "Workspace activity count could not be read",
            )
            .with_details(error.to_string())
        })?;
    let mut statement = if before_id.is_some() {
        connection
            .prepare("SELECT id, event_type, occurred_at, message FROM release_workspace_activity WHERE workspace_id = ?1 AND id < ?2 ORDER BY id DESC LIMIT ?3")
            .map_err(|error| CommandError::new("database_read_failed", "Workspace activity could not be read").with_details(error.to_string()))?
    } else {
        connection
            .prepare("SELECT id, event_type, occurred_at, message FROM release_workspace_activity WHERE workspace_id = ?1 ORDER BY id DESC LIMIT ?2")
            .map_err(|error| CommandError::new("database_read_failed", "Workspace activity could not be read").with_details(error.to_string()))?
    };
    let entries = if let Some(before_id) = before_id {
        statement
            .query_map(
                params![workspace_id, before_id, requested_limit + 1],
                activity_from_row,
            )
            .map_err(|error| {
                CommandError::new(
                    "database_read_failed",
                    "Workspace activity could not be read",
                )
                .with_details(error.to_string())
            })?
            .collect::<Result<Vec<_>, _>>()
    } else {
        statement
            .query_map(
                params![workspace_id, requested_limit + 1],
                activity_from_row,
            )
            .map_err(|error| {
                CommandError::new(
                    "database_read_failed",
                    "Workspace activity could not be read",
                )
                .with_details(error.to_string())
            })?
            .collect::<Result<Vec<_>, _>>()
    }
    .map_err(|error| {
        CommandError::new(
            "database_record_invalid",
            "A stored workspace activity record is invalid",
        )
        .with_details(error.to_string())
    })?;
    let has_more = entries.len() as i64 > requested_limit;
    let mut entries = entries;
    entries.truncate(requested_limit as usize);
    let next_cursor = if has_more {
        entries.last().map(|entry| entry.id)
    } else {
        None
    };
    Ok(crate::domain::ReleaseWorkspaceActivityPage {
        entries,
        next_cursor,
        has_more,
        total_count,
    })
}

fn activity_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<crate::domain::ReleaseWorkspaceActivity> {
    Ok(crate::domain::ReleaseWorkspaceActivity {
        id: row.get(0)?,
        event_type: row.get(1)?,
        occurred_at: row.get(2)?,
        message: row.get(3)?,
    })
}

#[tauri::command]
pub fn start_release_workspace(
    state: State<'_, Database>,
    request: crate::domain::StartReleaseWorkspaceRequest,
) -> Result<crate::domain::ReleaseWorkspace, CommandError> {
    let connection = state
        .0
        .lock()
        .map_err(|_| CommandError::new("database_unavailable", "Database is unavailable"))?;
    let (
        source_snapshot_id,
        baseline_origin,
        baseline_release_id,
        baseline_capture_json,
        candidate_snapshot_id,
    ) = if let Some(snapshot_id) = request.snapshot_id.as_deref() {
        let snapshot = connection
            .query_row(
                "SELECT modpack_id, lifecycle, baseline_capture_json FROM snapshots WHERE id = ?1",
                [snapshot_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Option<String>>(2)?,
                    ))
                },
            )
            .map_err(|error| {
                CommandError::new("snapshot_not_found", "Snapshot is not available")
                    .with_details(error.to_string())
            })?;
        if snapshot.0 != request.modpack_id {
            return Err(CommandError::new(
                "snapshot_project_mismatch",
                "Snapshot does not belong to the project",
            ));
        }
        if snapshot.1 != "reviewable" {
            return Err(CommandError::new(
                "snapshot_not_reviewable",
                "Only a reviewable snapshot can start a release workspace",
            ));
        }
        (
            Some(snapshot_id.to_string()),
            "snapshot",
            None,
            snapshot.2,
            Some(snapshot_id.to_string()),
        )
    } else if let Some(baseline_release_id) = request.baseline_release_id.as_deref() {
        let baseline = connection
            .query_row(
                "SELECT modpack_id, publication_status, capture_json FROM releases WHERE id = ?1",
                [baseline_release_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                },
            )
            .map_err(|error| {
                CommandError::new(
                    "baseline_release_not_found",
                    "Baseline release is not available",
                )
                .with_details(error.to_string())
            })?;
        if baseline.0 != request.modpack_id {
            return Err(CommandError::new(
                "baseline_release_project_mismatch",
                "Baseline release does not belong to the project",
            ));
        }
        if !matches!(baseline.1.as_str(), "published" | "provisional") {
            return Err(CommandError::new(
                "baseline_release_not_finalized",
                "Only a finalized release can be used as a baseline",
            ));
        }
        let _: crate::domain::ReleaseCapture =
            serde_json::from_str(&baseline.2).map_err(|error| {
                CommandError::new(
                    "baseline_capture_unavailable",
                    "The selected release has no valid immutable capture",
                )
                .with_details(error.to_string())
            })?;
        (
            None,
            "finalized_release",
            Some(baseline_release_id.to_string()),
            Some(baseline.2),
            None,
        )
    } else {
        let project_path: String = connection
            .query_row(
                "SELECT canonical_path FROM modpacks WHERE id = ?1",
                [&request.modpack_id],
                |row| row.get(0),
            )
            .map_err(|error| {
                CommandError::new("project_not_found", "Project is not registered")
                    .with_details(error.to_string())
            })?;
        let eligibility = crate::domain::ReleaseEligibility {
            eligible: true,
            provisional: false,
            source: Some(crate::domain::ReleaseEligibilitySource::ValidatedCurrentState),
            validation: Vec::new(),
            diagnostic: None,
        };
        let capture = crate::domain::capture::capture(
            std::path::Path::new(&project_path),
            eligibility,
            None,
            Vec::new(),
        )?;
        (
            None,
            "current_project",
            None,
            Some(serialize(&capture, "release baseline capture")?),
            None,
        )
    };
    if baseline_capture_json.is_none() {
        return Err(CommandError::new(
            "baseline_capture_unavailable",
            "A release workspace requires a stable immutable baseline capture",
        ));
    }
    let now = timestamp();
    let workspace_id = unique_id("release-workspace");
    let lifecycle = if matches!(
        request.metadata.publication_status,
        crate::domain::ReleasePublicationStatus::Provisional
    ) {
        crate::domain::ReleaseWorkspaceLifecycle::Provisional
    } else {
        crate::domain::ReleaseWorkspaceLifecycle::Draft
    };
    let transaction = connection.unchecked_transaction().map_err(|error| {
        CommandError::new(
            "database_write_failed",
            "Workspace transaction could not be started",
        )
        .with_details(error.to_string())
    })?;
    transaction.execute(
        "INSERT INTO release_workspaces (id, modpack_id, source_snapshot_id, baseline_origin, baseline_release_id, baseline_capture_json, name, version, description, notes, lifecycle, evidence_status, publication_status, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, 'baseline', ?12, ?13, ?13)",
        params![workspace_id, request.modpack_id, source_snapshot_id, baseline_origin, baseline_release_id, baseline_capture_json, request.metadata.name, request.metadata.version, request.metadata.description, request.metadata.notes, serialize(&lifecycle, "workspace lifecycle")?.trim_matches('"'), serialize(&request.metadata.publication_status, "publication status")?.trim_matches('"'), now],
    ).map_err(|error| CommandError::new("database_write_failed", "Release workspace could not be saved").with_details(error.to_string()))?;
    let mut candidates = transaction
        .prepare("SELECT id, candidate_json, observed_at FROM snapshot_candidates WHERE snapshot_id = ?1 ORDER BY id")
        .map_err(|error| {
            CommandError::new(
                "database_read_failed",
                "Snapshot candidates could not be read",
            )
            .with_details(error.to_string())
        })?;
    let candidate_sources = candidates
        .query_map([candidate_snapshot_id.as_deref().unwrap_or("")], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(|error| {
            CommandError::new(
                "database_read_failed",
                "Snapshot candidates could not be read",
            )
            .with_details(error.to_string())
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| {
            CommandError::new("database_record_invalid", "A snapshot candidate is invalid")
                .with_details(error.to_string())
        })?;
    drop(candidates);
    for (candidate_id, candidate_json, observed_at) in candidate_sources {
        let source_id = format!("{workspace_id}:{candidate_id}");
        transaction.execute("INSERT INTO release_workspace_candidate_sources (id, workspace_id, source_snapshot_candidate_id, candidate_json, observed_at) VALUES (?1, ?2, ?3, ?4, ?5)", params![source_id, workspace_id, candidate_id, candidate_json, observed_at]).map_err(|error| CommandError::new("database_write_failed", "Workspace candidate source could not be saved").with_details(error.to_string()))?;
        transaction.execute("INSERT INTO release_workspace_candidates (workspace_id, source_candidate_id, decision, recorded_at) VALUES (?1, ?2, 'undecided', ?3)", params![workspace_id, source_id, now]).map_err(|error| CommandError::new("database_write_failed", "Workspace candidate could not be saved").with_details(error.to_string()))?;
    }
    let baseline_message = match (&source_snapshot_id, &baseline_release_id) {
        (Some(snapshot_id), _) => {
            format!("Release workspace {workspace_id} started from snapshot {snapshot_id}")
        }
        (None, Some(release_id)) => {
            format!("Release workspace {workspace_id} started from finalized release {release_id}")
        }
        (None, None) => {
            format!("Release workspace {workspace_id} started with a current-project baseline")
        }
    };
    transaction.execute("INSERT INTO release_workspace_activity (workspace_id, event_type, occurred_at, message) VALUES (?1, 'workspace_started', ?2, ?3)", params![workspace_id, now, baseline_message]).map_err(|error| CommandError::new("database_write_failed", "Workspace activity could not be saved").with_details(error.to_string()))?;
    transaction.commit().map_err(|error| {
        CommandError::new(
            "database_write_failed",
            "Release workspace could not be committed",
        )
        .with_details(error.to_string())
    })?;
    drop(connection);
    load_release_workspace_inner(&state, &workspace_id)
}

#[tauri::command]
pub fn list_release_workspaces(
    state: State<'_, Database>,
    modpack_id: String,
) -> Result<Vec<crate::domain::ReleaseWorkspace>, CommandError> {
    let connection = state
        .0
        .lock()
        .map_err(|_| CommandError::new("database_unavailable", "Database is unavailable"))?;
    let rows = connection.prepare("SELECT id, modpack_id, source_snapshot_id, baseline_origin, baseline_release_id, baseline_capture_json, name, version, description, notes, lifecycle, evidence_status, publication_status, final_capture_json, final_changelog_revision_id, finalization_receipt_json, created_at, updated_at, abandoned_at FROM release_workspaces WHERE modpack_id = ?1 ORDER BY created_at DESC").map_err(|error| CommandError::new("database_read_failed", "Release workspaces could not be read").with_details(error.to_string()))?.query_map([modpack_id], workspace_from_row).map_err(|error| CommandError::new("database_read_failed", "Release workspaces could not be read").with_details(error.to_string()))?.collect::<Result<Vec<_>, _>>().map_err(|error| CommandError::new("database_record_invalid", "A stored release workspace is invalid").with_details(error.to_string()))?;
    rows.into_iter()
        .map(|workspace| enrich_workspace(&connection, workspace))
        .collect()
}

#[tauri::command]
pub fn abandon_release_workspace(
    state: State<'_, Database>,
    workspace_id: String,
) -> Result<crate::domain::ReleaseWorkspace, CommandError> {
    let connection = state
        .0
        .lock()
        .map_err(|_| CommandError::new("database_unavailable", "Database is unavailable"))?;
    let now = timestamp();
    let transaction = connection.unchecked_transaction().map_err(|error| {
        CommandError::new(
            "database_write_failed",
            "Workspace transaction could not be started",
        )
        .with_details(error.to_string())
    })?;
    let changed = transaction.execute("UPDATE release_workspaces SET lifecycle = 'abandoned', abandoned_at = ?1, updated_at = ?1 WHERE id = ?2 AND lifecycle NOT IN ('abandoned', 'finalized', 'published', 'withdrawn')", params![now, workspace_id]).map_err(|error| CommandError::new("database_write_failed", "Workspace could not be abandoned").with_details(error.to_string()))?;
    if changed == 0 {
        return Err(CommandError::new(
            "workspace_not_editable",
            "Release workspace cannot be abandoned",
        ));
    }
    transaction.execute("INSERT INTO release_workspace_activity (workspace_id, event_type, occurred_at, message) VALUES (?1, 'workspace_abandoned', ?2, 'Workspace explicitly abandoned; project files were not reverted')", params![workspace_id, now]).map_err(|error| CommandError::new("database_write_failed", "Workspace activity could not be saved").with_details(error.to_string()))?;
    transaction.commit().map_err(|error| {
        CommandError::new(
            "database_write_failed",
            "Workspace abandonment could not be committed",
        )
        .with_details(error.to_string())
    })?;
    drop(connection);
    load_release_workspace_inner(&state, &workspace_id)
}

#[tauri::command]
pub fn unlink_snapshot_from_release_workspace(
    state: State<'_, Database>,
    workspace_id: String,
) -> Result<crate::domain::ReleaseWorkspace, CommandError> {
    let connection = state
        .0
        .lock()
        .map_err(|_| CommandError::new("database_unavailable", "Database is unavailable"))?;
    let transaction = connection.unchecked_transaction().map_err(|error| {
        CommandError::new("database_write_failed", "Snapshot unlink could not start")
            .with_details(error.to_string())
    })?;
    let now = timestamp();
    let active_operation: bool = transaction
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM operation_attempts WHERE workspace_id = ?1 AND status IN ('running', 'applying'))",
            [&workspace_id],
            |row| row.get(0),
        )
        .map_err(|error| CommandError::new("database_read_failed", "Workspace operation state could not be read").with_details(error.to_string()))?;
    if active_operation {
        return Err(CommandError::new(
            "workspace_operation_active",
            "Snapshot cannot be unlinked while a workspace operation is active",
        ));
    }
    let changed = transaction
        .execute(
            "UPDATE release_workspaces SET source_snapshot_id = NULL, baseline_origin = 'detached_snapshot', final_changelog_revision_id = NULL, finalization_receipt_json = NULL, updated_at = ?1 WHERE id = ?2 AND source_snapshot_id IS NOT NULL AND lifecycle NOT IN ('abandoned', 'applying', 'finalized', 'published', 'withdrawn')",
            params![now, workspace_id],
        )
        .map_err(|error| CommandError::new("database_write_failed", "Snapshot unlink could not be saved").with_details(error.to_string()))?;
    if changed == 0 {
        return Err(CommandError::new(
            "workspace_snapshot_unlink_not_allowed",
            "Only an editable workspace with a linked snapshot can unlink its snapshot",
        ));
    }
    transaction
        .execute(
            "DELETE FROM release_workspace_candidates WHERE workspace_id = ?1",
            [&workspace_id],
        )
        .map_err(|error| {
            CommandError::new(
                "database_write_failed",
                "Workspace candidate decisions could not be cleared",
            )
            .with_details(error.to_string())
        })?;
    transaction
        .execute(
            "DELETE FROM release_workspace_candidate_sources WHERE workspace_id = ?1",
            [&workspace_id],
        )
        .map_err(|error| {
            CommandError::new(
                "database_write_failed",
                "Workspace candidate sources could not be cleared",
            )
            .with_details(error.to_string())
        })?;
    transaction
        .execute(
            "UPDATE changelog_artifacts SET release_workspace_id = NULL WHERE release_workspace_id = ?1 AND stage = 'proposed'",
            [&workspace_id],
        )
        .map_err(|error| CommandError::new("database_write_failed", "Workspace changelog proposals could not be detached").with_details(error.to_string()))?;
    transaction
        .execute(
            "INSERT INTO release_workspace_activity (workspace_id, event_type, occurred_at, message) VALUES (?1, 'snapshot_unlinked', ?2, 'Discovery snapshot unlinked; immutable snapshot baseline retained')",
            params![workspace_id, now],
        )
        .map_err(|error| CommandError::new("database_write_failed", "Snapshot unlink activity could not be saved").with_details(error.to_string()))?;
    transaction.commit().map_err(|error| {
        CommandError::new(
            "database_write_failed",
            "Snapshot unlink could not be committed",
        )
        .with_details(error.to_string())
    })?;
    drop(connection);
    load_release_workspace_inner(&state, &workspace_id)
}

#[tauri::command]
pub fn rebase_release_workspace(
    state: State<'_, Database>,
    request: crate::domain::RebaseReleaseWorkspaceRequest,
) -> Result<crate::domain::ReleaseWorkspace, CommandError> {
    let connection = state
        .0
        .lock()
        .map_err(|_| CommandError::new("database_unavailable", "Database is unavailable"))?;
    let (project_path, lifecycle): (String, String) = connection
        .query_row(
            "SELECT modpacks.canonical_path, release_workspaces.lifecycle FROM modpacks JOIN release_workspaces ON release_workspaces.modpack_id = modpacks.id WHERE release_workspaces.id = ?1",
            [&request.workspace_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|error| CommandError::new("workspace_not_found", "Release workspace is not available").with_details(error.to_string()))?;
    if matches!(
        lifecycle.as_str(),
        "abandoned" | "applying" | "finalized" | "published" | "withdrawn"
    ) {
        return Err(CommandError::new(
            "workspace_not_editable",
            "Only an editable release workspace can rebase its baseline",
        ));
    }
    let active_operation: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM operation_attempts WHERE workspace_id = ?1 AND status IN ('running', 'applying'))",
            [&request.workspace_id],
            |row| row.get(0),
        )
        .map_err(|error| CommandError::new("database_read_failed", "Workspace operation state could not be read").with_details(error.to_string()))?;
    if active_operation {
        return Err(CommandError::new(
            "workspace_operation_active",
            "Baseline cannot be rebased while a workspace operation is active",
        ));
    }
    let capture = crate::domain::capture::capture(
        std::path::Path::new(&project_path),
        crate::domain::ReleaseEligibility {
            eligible: true,
            provisional: false,
            source: Some(crate::domain::ReleaseEligibilitySource::ValidatedCurrentState),
            validation: Vec::new(),
            diagnostic: None,
        },
        None,
        Vec::new(),
    )?;
    let now = timestamp();
    let transaction = connection.unchecked_transaction().map_err(|error| {
        CommandError::new("database_write_failed", "Baseline rebase could not start")
            .with_details(error.to_string())
    })?;
    transaction
        .execute(
            "DELETE FROM release_workspace_candidates WHERE workspace_id = ?1",
            [&request.workspace_id],
        )
        .map_err(|error| {
            CommandError::new(
                "database_write_failed",
                "Workspace candidate decisions could not be cleared",
            )
            .with_details(error.to_string())
        })?;
    transaction
        .execute(
            "DELETE FROM release_workspace_candidate_sources WHERE workspace_id = ?1",
            [&request.workspace_id],
        )
        .map_err(|error| {
            CommandError::new(
                "database_write_failed",
                "Workspace candidate sources could not be cleared",
            )
            .with_details(error.to_string())
        })?;
    transaction
        .execute(
            "UPDATE changelog_artifacts SET release_workspace_id = NULL WHERE release_workspace_id = ?1 AND stage = 'proposed'",
            [&request.workspace_id],
        )
        .map_err(|error| CommandError::new("database_write_failed", "Workspace changelog proposals could not be detached").with_details(error.to_string()))?;
    transaction.execute(
        "UPDATE release_workspaces SET source_snapshot_id = NULL, baseline_origin = 'current_project', baseline_release_id = NULL, baseline_capture_json = ?1, evidence_status = 'baseline', lifecycle = 'draft', final_capture_json = NULL, final_changelog_revision_id = NULL, finalization_receipt_json = NULL, updated_at = ?2 WHERE id = ?3",
        params![serialize(&capture, "release baseline capture")?, now, request.workspace_id],
    ).map_err(|error| CommandError::new("database_write_failed", "Baseline rebase could not be saved").with_details(error.to_string()))?;
    transaction.execute(
        "INSERT INTO release_workspace_activity (workspace_id, event_type, occurred_at, message) VALUES (?1, 'baseline_rebased', ?2, 'Release baseline explicitly rebased from current project files')",
        params![request.workspace_id, now],
    ).map_err(|error| CommandError::new("database_write_failed", "Baseline rebase activity could not be saved").with_details(error.to_string()))?;
    transaction.commit().map_err(|error| {
        CommandError::new(
            "database_write_failed",
            "Baseline rebase could not be committed",
        )
        .with_details(error.to_string())
    })?;
    drop(connection);
    load_release_workspace_inner(&state, &request.workspace_id)
}

#[tauri::command]
pub fn link_snapshot_to_release_workspace(
    state: State<'_, Database>,
    workspace_id: String,
    snapshot_id: String,
) -> Result<crate::domain::ReleaseWorkspace, CommandError> {
    let connection = state
        .0
        .lock()
        .map_err(|_| CommandError::new("database_unavailable", "Database is unavailable"))?;
    let transaction = connection.unchecked_transaction().map_err(|error| {
        CommandError::new("database_write_failed", "Snapshot link could not start")
            .with_details(error.to_string())
    })?;
    let workspace: (String, Option<String>, String) = transaction
        .query_row(
            "SELECT modpack_id, source_snapshot_id, lifecycle FROM release_workspaces WHERE id = ?1",
            [&workspace_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .map_err(|error| {
            CommandError::new("workspace_not_found", "Release workspace is not available")
                .with_details(error.to_string())
        })?;
    if workspace.1.is_some() {
        return Err(CommandError::new(
            "workspace_snapshot_already_linked",
            "This release workspace already has a discovery snapshot",
        ));
    }
    if matches!(
        workspace.2.as_str(),
        "abandoned" | "finalized" | "published" | "withdrawn"
    ) {
        return Err(CommandError::new(
            "workspace_not_editable",
            "Only an editable release workspace can receive a discovery snapshot",
        ));
    }
    let snapshot: (String, String, Option<String>) = transaction
        .query_row(
            "SELECT modpack_id, lifecycle, baseline_capture_json FROM snapshots WHERE id = ?1",
            [&snapshot_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .map_err(|error| {
            CommandError::new("snapshot_not_found", "Snapshot is not available")
                .with_details(error.to_string())
        })?;
    if snapshot.0 != workspace.0 {
        return Err(CommandError::new(
            "snapshot_project_mismatch",
            "Snapshot does not belong to the release project",
        ));
    }
    if snapshot.1 != "reviewable" || snapshot.2.is_none() {
        return Err(CommandError::new(
            "snapshot_not_reviewable",
            "Only a reviewable snapshot with stable baseline evidence can be linked",
        ));
    }
    let now = timestamp();
    transaction
        .execute(
            "UPDATE release_workspaces SET source_snapshot_id = ?1, updated_at = ?2 WHERE id = ?3 AND source_snapshot_id IS NULL",
            params![snapshot_id, now, workspace_id],
        )
        .map_err(|error| {
            CommandError::new("database_write_failed", "Snapshot link could not be saved")
                .with_details(error.to_string())
        })?;
    let mut candidates = transaction
        .prepare("SELECT id, candidate_json, observed_at FROM snapshot_candidates WHERE snapshot_id = ?1 ORDER BY id")
        .map_err(|error| {
            CommandError::new("database_read_failed", "Snapshot candidates could not be read")
                .with_details(error.to_string())
        })?;
    let candidate_sources = candidates
        .query_map([&snapshot_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(|error| {
            CommandError::new(
                "database_read_failed",
                "Snapshot candidates could not be read",
            )
            .with_details(error.to_string())
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| {
            CommandError::new("database_record_invalid", "A snapshot candidate is invalid")
                .with_details(error.to_string())
        })?;
    drop(candidates);
    for (candidate_id, candidate_json, observed_at) in candidate_sources {
        let source_id = format!("{workspace_id}:{candidate_id}");
        transaction
            .execute(
                "INSERT INTO release_workspace_candidate_sources (id, workspace_id, source_snapshot_candidate_id, candidate_json, observed_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![source_id, workspace_id, candidate_id, candidate_json, observed_at],
            )
            .map_err(|error| {
                CommandError::new("database_write_failed", "Workspace candidate source could not be linked")
                    .with_details(error.to_string())
            })?;
        transaction
            .execute(
                "INSERT INTO release_workspace_candidates (workspace_id, source_candidate_id, decision, recorded_at) VALUES (?1, ?2, 'undecided', ?3)",
                params![workspace_id, source_id, now],
            )
            .map_err(|error| {
                CommandError::new("database_write_failed", "Workspace candidate could not be linked")
                    .with_details(error.to_string())
            })?;
    }
    transaction
        .execute(
            "INSERT INTO release_workspace_activity (workspace_id, event_type, occurred_at, message) VALUES (?1, 'snapshot_linked', ?2, ?3)",
            params![workspace_id, now, format!("Discovery snapshot {snapshot_id} linked; release baseline was preserved")],
        )
        .map_err(|error| {
            CommandError::new("database_write_failed", "Snapshot link activity could not be saved")
                .with_details(error.to_string())
        })?;
    transaction.commit().map_err(|error| {
        CommandError::new(
            "database_write_failed",
            "Snapshot link could not be committed",
        )
        .with_details(error.to_string())
    })?;
    drop(connection);
    load_release_workspace_inner(&state, &workspace_id)
}

#[tauri::command]
pub fn set_release_workspace_decision(
    state: State<'_, Database>,
    request: crate::domain::ReleaseWorkspaceDecisionRequest,
) -> Result<crate::domain::ReleaseWorkspace, CommandError> {
    let connection = state
        .0
        .lock()
        .map_err(|_| CommandError::new("database_unavailable", "Database is unavailable"))?;
    let now = timestamp();
    let transaction = connection.unchecked_transaction().map_err(|error| {
        CommandError::new(
            "database_write_failed",
            "Workspace transaction could not be started",
        )
        .with_details(error.to_string())
    })?;
    let valid_target: bool = transaction
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM release_workspace_candidates WHERE workspace_id = ?1 AND source_candidate_id = ?2) AND EXISTS(SELECT 1 FROM release_workspaces WHERE id = ?1 AND lifecycle NOT IN ('abandoned', 'finalized', 'published', 'withdrawn'))",
            params![request.workspace_id, request.source_candidate_id],
            |row| row.get(0),
        )
        .map_err(|error| CommandError::new("database_read_failed", "Workspace decision target could not be read").with_details(error.to_string()))?;
    if !valid_target {
        return Err(CommandError::new(
            "workspace_decision_not_allowed",
            "Candidate is not editable in this release workspace",
        ));
    }
    transaction
        .execute(
            "INSERT INTO release_workspace_candidates (workspace_id, source_candidate_id, decision, note, recorded_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![request.workspace_id, request.source_candidate_id, serialize(&request.decision, "candidate decision")?.trim_matches('"'), request.note, now],
        )
        .map_err(|error| CommandError::new("database_write_failed", "Workspace decision could not be saved").with_details(error.to_string()))?;
    transaction
        .execute(
            "INSERT INTO release_workspace_activity (workspace_id, event_type, occurred_at, message) VALUES (?1, 'candidate_decision_changed', ?2, ?3)",
            params![request.workspace_id, now, format!("Candidate {} decision changed", request.source_candidate_id)],
        )
        .map_err(|error| CommandError::new("database_write_failed", "Workspace activity could not be saved").with_details(error.to_string()))?;
    transaction.commit().map_err(|error| {
        CommandError::new(
            "database_write_failed",
            "Workspace decision could not be committed",
        )
        .with_details(error.to_string())
    })?;
    drop(connection);
    load_release_workspace_inner(&state, &request.workspace_id)
}

#[tauri::command]
pub fn select_release_workspace_changelog(
    state: State<'_, Database>,
    request: crate::domain::SelectReleaseWorkspaceChangelogRequest,
) -> Result<crate::domain::ReleaseWorkspace, CommandError> {
    let connection = state
        .0
        .lock()
        .map_err(|_| CommandError::new("database_unavailable", "Database is unavailable"))?;
    if let Some(revision_id) = &request.changelog_revision_id {
        let valid: bool = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM changelog_revisions r JOIN changelog_artifacts a ON a.id = r.artifact_id WHERE r.id = ?1 AND a.release_workspace_id = ?2 AND a.stage = 'proposed' AND r.frozen = 0 AND r.archived_at IS NULL)",
                params![revision_id, request.workspace_id],
                |row| row.get(0),
            )
            .map_err(|error| CommandError::new("database_read_failed", "The changelog revision could not be verified").with_details(error.to_string()))?;
        if !valid {
            return Err(CommandError::new(
                "changelog_revision_mismatch",
                "The selected changelog revision is not an editable proposed revision for this workspace",
            ));
        }
    }
    connection
        .execute(
            "UPDATE release_workspaces SET final_changelog_revision_id = ?1, updated_at = ?2 WHERE id = ?3 AND lifecycle NOT IN ('finalized', 'published', 'withdrawn', 'abandoned')",
            params![request.changelog_revision_id, timestamp(), request.workspace_id],
        )
        .map_err(|error| CommandError::new("database_write_failed", "The changelog selection could not be saved").with_details(error.to_string()))?;
    drop(connection);
    load_release_workspace_inner(&state, &request.workspace_id)
}

#[tauri::command]
pub fn create_release_workspace_changelog(
    state: State<'_, Database>,
    request: crate::domain::CreateReleaseWorkspaceChangelogRequest,
) -> Result<crate::domain::ChangelogArtifact, CommandError> {
    let workspace = load_release_workspace_inner(&state, &request.workspace_id)?;
    let connection = state
        .0
        .lock()
        .map_err(|_| CommandError::new("database_unavailable", "Database is unavailable"))?;
    if matches!(
        workspace.lifecycle,
        crate::domain::ReleaseWorkspaceLifecycle::Finalized
            | crate::domain::ReleaseWorkspaceLifecycle::Published
            | crate::domain::ReleaseWorkspaceLifecycle::Withdrawn
            | crate::domain::ReleaseWorkspaceLifecycle::Abandoned
    ) {
        return Err(CommandError::new(
            "invalid_changelog_state",
            "This workspace cannot create a changelog proposal",
        ));
    }
    let now = timestamp();
    let attempt_id = unique_id("changelog-attempt");
    let artifact_id = unique_id("changelog-artifact");
    connection.execute("INSERT INTO changelog_attempts (id, modpack_id, snapshot_id, request_json, request_fingerprint, status, created_at, finished_at) VALUES (?1, ?2, ?3, ?4, ?5, 'complete', ?6, ?6)", params![attempt_id, workspace.modpack_id, workspace.source_snapshot_id, "{}", artifact_id, now]).map_err(|error| CommandError::new("database_write_failed", "Changelog proposal could not be saved").with_details(error.to_string()))?;
    connection.execute("INSERT INTO changelog_artifacts (id, modpack_id, snapshot_id, release_workspace_id, stage, source_capture_fingerprint, attempt_id, status, introduction, content, entries_json, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, 'proposed', NULL, ?5, 'complete', ?6, '', '[]', ?7, ?7)", params![artifact_id, workspace.modpack_id, workspace.source_snapshot_id, workspace.id, attempt_id, request.introduction, now]).map_err(|error| CommandError::new("database_write_failed", "Changelog proposal could not be saved").with_details(error.to_string()))?;
    drop(connection);
    load_changelog_artifact(&state, &artifact_id)
}

#[tauri::command]
pub fn create_release(
    _state: State<'_, Database>,
    _request: crate::domain::ReleaseCreateRequest,
) -> Result<ReleaseRecord, CommandError> {
    Err(CommandError::new(
        "release_workspace_required",
        "New releases must start from a reviewable snapshot workspace",
    ))
}

#[tauri::command]
pub fn get_release(state: State<'_, Database>, id: String) -> Result<ReleaseRecord, CommandError> {
    load_release(&state, &id)
}

#[tauri::command]
pub fn list_releases_command(
    state: State<'_, Database>,
    modpack_id: String,
) -> Result<Vec<ReleaseRecord>, CommandError> {
    list_releases(&state, &modpack_id)
}

#[tauri::command]
pub fn update_release(
    state: State<'_, Database>,
    request: crate::domain::ReleaseUpdateRequest,
) -> Result<ReleaseRecord, CommandError> {
    let existing = load_release(&state, &request.release_id)?;
    crate::domain::validate_publication_transition(
        &existing.metadata.publication_status,
        &request.metadata.publication_status,
        false,
    )?;
    if request.metadata.name.trim().is_empty() {
        return Err(CommandError::new(
            "invalid_release_name",
            "Release name is required",
        ));
    }
    let connection = state
        .0
        .lock()
        .map_err(|_| CommandError::new("database_unavailable", "Database is unavailable"))?;
    let updated_at = timestamp();
    connection
        .execute(
            "UPDATE releases SET name = ?1, version = ?2, description = ?3, notes = ?4, publication_status = ?5, updated_at = ?6 WHERE id = ?7",
            params![
                request.metadata.name,
                request.metadata.version,
                request.metadata.description,
                request.metadata.notes,
                serialize(&request.metadata.publication_status, "release status")?.trim_matches('"'),
                updated_at,
                request.release_id,
            ],
        )
        .map_err(|error| CommandError::new("database_write_failed", "Release could not be updated").with_details(error.to_string()))?;
    drop(connection);
    load_release(&state, &request.release_id)
}

#[tauri::command]
pub fn compare_releases(
    state: State<'_, Database>,
    request: crate::domain::ReleaseComparisonRequest,
) -> Result<crate::domain::ReleaseComparison, CommandError> {
    let before = load_release(&state, &request.before_release_id)?;
    let after = load_release(&state, &request.after_release_id)?;
    Ok(crate::domain::comparison::compare(&before, &after))
}

fn row_to_release(row: &rusqlite::Row<'_>) -> rusqlite::Result<ReleaseRecord> {
    let status: String = row.get(6)?;
    let capture_json: String = row.get(7)?;
    Ok(ReleaseRecord {
        id: row.get(0)?,
        modpack_id: row.get(1)?,
        metadata: crate::domain::ReleaseMetadata {
            name: row.get(2)?,
            version: row.get(3)?,
            description: row.get(4)?,
            notes: row.get(5)?,
            publication_status: serde_json::from_value(serde_json::Value::String(status))
                .map_err(|_| rusqlite::Error::InvalidQuery)?,
        },
        capture: serde_json::from_str(&capture_json).map_err(|_| rusqlite::Error::InvalidQuery)?,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
    })
}

pub fn persist_changelog_artifact(
    database: &Database,
    request: &crate::domain::ChangelogGenerationRequest,
    artifact: &crate::domain::ChangelogArtifact,
) -> Result<(), CommandError> {
    let connection = database
        .0
        .lock()
        .map_err(|_| CommandError::new("database_unavailable", "Database is unavailable"))?;
    let transaction = connection.unchecked_transaction().map_err(|error| {
        CommandError::new(
            "database_write_failed",
            "Changelog attempt could not be started",
        )
        .with_details(error.to_string())
    })?;
    let now = timestamp();
    let status = serde_json::to_string(&artifact.status)
        .map_err(|error| {
            CommandError::new(
                "serialization_failed",
                "Changelog status could not be serialized",
            )
            .with_details(error.to_string())
        })?
        .trim_matches('"')
        .to_string();
    transaction
        .execute(
            "INSERT INTO changelog_attempts (id, modpack_id, snapshot_id, request_json, request_fingerprint, status, created_at, finished_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
            params![artifact.attempt_id, request.modpack_id, request.snapshot_id, serialize(request, "changelog request")?, request.request_fingerprint, status, now],
        )
        .map_err(|error| CommandError::new("database_write_failed", "Changelog attempt could not be saved").with_details(error.to_string()))?;
    transaction
        .execute(
            "INSERT INTO changelog_artifacts (id, modpack_id, snapshot_id, release_workspace_id, stage, source_capture_fingerprint, attempt_id, status, introduction, content, entries_json, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?12)",
            params![artifact.id, artifact.modpack_id, artifact.snapshot_id, artifact.release_workspace_id, serialize(&artifact.stage, "changelog stage")?.trim_matches('"'), artifact.source_capture_fingerprint, artifact.attempt_id, status, artifact.introduction, artifact.content, serialize(&artifact.entries, "changelog entries")?, now],
        )
        .map_err(|error| CommandError::new("database_write_failed", "Changelog artifact could not be saved").with_details(error.to_string()))?;
    transaction.commit().map_err(|error| {
        CommandError::new(
            "database_write_failed",
            "Changelog artifact could not be committed",
        )
        .with_details(error.to_string())
    })
}

pub fn persist_changelog_revision(
    database: &Database,
    request: &crate::domain::ChangelogRevisionRequest,
) -> Result<crate::domain::ChangelogRevision, CommandError> {
    let connection = database
        .0
        .lock()
        .map_err(|_| CommandError::new("database_unavailable", "Database is unavailable"))?;
    let transaction = connection.unchecked_transaction().map_err(|error| {
        CommandError::new("database_write_failed", "Revision could not be started")
            .with_details(error.to_string())
    })?;
    let artifact_exists: bool = transaction
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM changelog_artifacts WHERE id = ?1)",
            [&request.artifact_id],
            |row| row.get(0),
        )
        .map_err(|error| {
            CommandError::new(
                "database_read_failed",
                "The source artifact could not be verified",
            )
            .with_details(error.to_string())
        })?;
    if !artifact_exists {
        return Err(CommandError::new(
            "changelog_not_found",
            "The source changelog artifact is not available",
        ));
    }
    let artifact_frozen: bool = transaction
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM changelog_revisions WHERE artifact_id = ?1 AND frozen = 1)",
            [&request.artifact_id],
            |row| row.get(0),
        )
        .map_err(|error| {
            CommandError::new("database_read_failed", "The artifact freeze state could not be verified")
                .with_details(error.to_string())
        })?;
    if artifact_frozen {
        return Err(CommandError::new(
            "changelog_revision_frozen",
            "Final changelog revisions cannot be edited",
        ));
    }
    if let Some(prior_revision_id) = &request.prior_revision_id {
        let belongs_to_artifact: bool = transaction
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM changelog_revisions WHERE id = ?1 AND artifact_id = ?2)",
                params![prior_revision_id, request.artifact_id],
                |row| row.get(0),
            )
            .map_err(|error| {
                CommandError::new("database_read_failed", "The prior revision could not be verified")
                    .with_details(error.to_string())
            })?;
        if !belongs_to_artifact {
            return Err(CommandError::new(
                "revision_mismatch",
                "The prior revision does not belong to this artifact",
            ));
        }
    }
    let now = timestamp();
    let id = unique_id("revision");
    transaction
        .execute(
            "UPDATE changelog_revisions SET is_current = 0 WHERE artifact_id = ?1",
            [&request.artifact_id],
        )
        .map_err(|error| {
            CommandError::new(
                "database_write_failed",
                "Revision history could not be updated",
            )
            .with_details(error.to_string())
        })?;
    transaction
        .execute(
            "INSERT INTO changelog_revisions (id, artifact_id, prior_revision_id, content, introduction, selected_version_ids_json, created_at, is_current, frozen, archived_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 1, 0, NULL)",
            params![id, request.artifact_id, request.prior_revision_id, request.content, request.introduction, serialize(&request.selected_version_ids, "selected changelog versions")?, now],
        )
        .map_err(|error| CommandError::new("database_write_failed", "Revision could not be saved").with_details(error.to_string()))?;
    transaction.commit().map_err(|error| {
        CommandError::new("database_write_failed", "Revision could not be committed")
            .with_details(error.to_string())
    })?;
    Ok(crate::domain::ChangelogRevision {
        id,
        artifact_id: request.artifact_id.clone(),
        prior_revision_id: request.prior_revision_id.clone(),
        content: request.content.clone(),
        introduction: request.introduction.clone(),
        selected_version_ids: request.selected_version_ids.clone(),
        created_at: now,
        is_current: true,
        frozen: false,
        archived_at: None,
    })
}

pub fn persist_changelog_selection_revision(
    database: &Database,
    request: &crate::domain::ChangelogSelectionRevisionRequest,
) -> Result<crate::domain::ChangelogRevision, CommandError> {
    let artifact = load_changelog_artifact(database, &request.artifact_id)?;
    let workspace_id = request.workspace_id.trim();
    if workspace_id.is_empty() || artifact.release_workspace_id.as_deref() != Some(workspace_id) {
        return Err(CommandError::new(
            "changelog_workspace_mismatch",
            "The changelog artifact does not belong to this release workspace",
        ));
    }
    {
        let connection = database
            .0
            .lock()
            .map_err(|_| CommandError::new("database_unavailable", "Database is unavailable"))?;
        let (lifecycle, modpack_id): (String, String) = connection
            .query_row(
                "SELECT lifecycle, modpack_id FROM release_workspaces WHERE id = ?1",
                [workspace_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(|_| {
                CommandError::new(
                    "changelog_workspace_mismatch",
                    "The release workspace is not available",
                )
            })?;
        if modpack_id != artifact.modpack_id
            || matches!(
                lifecycle.as_str(),
                "finalized" | "published" | "withdrawn" | "abandoned"
            )
        {
            return Err(CommandError::new(
                "invalid_changelog_state",
                "This release workspace cannot change its proposed changelog",
            ));
        }
    }
    let selected: std::collections::HashSet<&str> = request
        .selected_version_ids
        .iter()
        .map(String::as_str)
        .collect();
    let available_ids: std::collections::HashSet<&str> = artifact
        .entries
        .iter()
        .flat_map(|entry| entry.versions.iter())
        .filter(|version| {
            version.included
                && !version.is_current
                && version.content_status == crate::domain::ChangelogContentStatus::Available
        })
        .map(|version| version.version.version_id.as_str())
        .collect();
    if selected.iter().any(|id| !available_ids.contains(id)) {
        return Err(CommandError::new(
            "invalid_changelog_selection",
            "Selection contains an unknown or unavailable provider version",
        ));
    }
    let mut content = artifact
        .introduction
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or_default()
        .to_string();
    if !content.is_empty() {
        content.push_str("\n\n");
    }
    for entry in &artifact.entries {
        let versions = entry
            .versions
            .iter()
            .filter(|version| selected.contains(version.version.version_id.as_str()))
            .collect::<Vec<_>>();
        if versions.iter().any(|version| version.changelog.is_none()) {
            return Err(CommandError::new(
                "empty_changelog_selection",
                "A selected provider version has no changelog text",
            ));
        }
        if versions.is_empty() {
            continue;
        }
        let name = match &entry.local.local_identity {
            crate::domain::Evidence::Observed(value) => value.as_str(),
            _ => "Unknown mod",
        };
        content.push_str(&format!("## {name}\n\n"));
        for version in versions {
            content.push_str(&format!(
                "### {}\n\n{}\n\n",
                version
                    .version
                    .version_number
                    .as_deref()
                    .unwrap_or("Provider release"),
                version.changelog.as_deref().unwrap_or_default()
            ));
        }
    }
    persist_changelog_revision(
        database,
        &crate::domain::ChangelogRevisionRequest {
            artifact_id: request.artifact_id.clone(),
            prior_revision_id: request.prior_revision_id.clone(),
            content,
            introduction: artifact.introduction,
            selected_version_ids: request.selected_version_ids.clone(),
        },
    )
}

pub fn set_changelog_revision_archived(
    database: &Database,
    request: &crate::domain::ChangelogRevisionArchiveRequest,
) -> Result<crate::domain::ChangelogRevision, CommandError> {
    let connection = database
        .0
        .lock()
        .map_err(|_| CommandError::new("database_unavailable", "Database is unavailable"))?;
    let now = timestamp();
    let mut statement = connection
        .prepare("SELECT id, artifact_id, prior_revision_id, content, introduction, selected_version_ids_json, created_at, is_current, frozen, archived_at FROM changelog_revisions WHERE id = ?1")
        .map_err(|error| CommandError::new("database_read_failed", "Revision could not be read").with_details(error.to_string()))?;
    let revision = statement
        .query_row([&request.revision_id], |row| {
            Ok(crate::domain::ChangelogRevision {
                id: row.get(0)?,
                artifact_id: row.get(1)?,
                prior_revision_id: row.get(2)?,
                content: row.get(3)?,
                introduction: row.get(4)?,
                selected_version_ids: serde_json::from_str(&row.get::<_, String>(5)?)
                    .unwrap_or_default(),
                created_at: row.get(6)?,
                is_current: row.get::<_, i64>(7)? != 0,
                frozen: row.get::<_, i64>(8)? != 0,
                archived_at: row.get(9)?,
            })
        })
        .map_err(|_| {
            CommandError::new(
                "changelog_not_found",
                "The changelog revision is not available",
            )
        })?;
    if revision.frozen {
        return Err(CommandError::new(
            "changelog_revision_frozen",
            "Final changelog revisions cannot be archived",
        ));
    }
    let archived_at = if request.archived { Some(now) } else { None };
    connection
        .execute(
            "UPDATE changelog_revisions SET archived_at = ?1 WHERE id = ?2",
            params![archived_at, request.revision_id],
        )
        .map_err(|error| {
            CommandError::new(
                "database_write_failed",
                "Revision archive state could not be saved",
            )
            .with_details(error.to_string())
        })?;
    drop(statement);
    load_changelog_revision(&connection, &request.revision_id)
}

fn load_changelog_revision(
    connection: &Connection,
    revision_id: &str,
) -> Result<crate::domain::ChangelogRevision, CommandError> {
    connection
        .query_row(
            "SELECT id, artifact_id, prior_revision_id, content, introduction, selected_version_ids_json, created_at, is_current, frozen, archived_at FROM changelog_revisions WHERE id = ?1",
            [revision_id],
            |row| {
                Ok(crate::domain::ChangelogRevision {
                    id: row.get(0)?,
                    artifact_id: row.get(1)?,
                    prior_revision_id: row.get(2)?,
                    content: row.get(3)?,
                    introduction: row.get(4)?,
                    selected_version_ids: serde_json::from_str(&row.get::<_, String>(5)?).unwrap_or_default(),
                    created_at: row.get(6)?,
                    is_current: row.get::<_, i64>(7)? != 0,
                    frozen: row.get::<_, i64>(8)? != 0,
                    archived_at: row.get(9)?,
                })
            },
        )
        .map_err(|error| CommandError::new("database_read_failed", "Revision could not be read").with_details(error.to_string()))
}

pub fn persist_changelog_export(
    database: &Database,
    request: &crate::domain::ChangelogExportRequest,
    status: crate::domain::ChangelogExportStatus,
    diagnostic: Option<CommandError>,
) -> Result<crate::domain::ChangelogExport, CommandError> {
    let connection = database
        .0
        .lock()
        .map_err(|_| CommandError::new("database_unavailable", "Database is unavailable"))?;
    let artifact_exists: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM changelog_artifacts WHERE id = ?1)",
            [&request.artifact_id],
            |row| row.get(0),
        )
        .map_err(|error| {
            CommandError::new(
                "database_read_failed",
                "The source artifact could not be verified",
            )
            .with_details(error.to_string())
        })?;
    if !artifact_exists {
        return Err(CommandError::new(
            "changelog_not_found",
            "The source changelog artifact is not available",
        ));
    }
    if let Some(revision_id) = &request.revision_id {
        let belongs_to_artifact: bool = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM changelog_revisions WHERE id = ?1 AND artifact_id = ?2)",
                params![revision_id, request.artifact_id],
                |row| row.get(0),
            )
            .map_err(|error| {
                CommandError::new("database_read_failed", "The revision could not be verified")
                    .with_details(error.to_string())
            })?;
        if !belongs_to_artifact {
            return Err(CommandError::new(
                "revision_mismatch",
                "The revision does not belong to this artifact",
            ));
        }
    }
    let exported_at = timestamp();
    let id = unique_id("export");
    use sha2::{Digest, Sha256};
    let content_hash = format!("{:x}", Sha256::digest(request.content.as_bytes()));
    let status_value = serde_json::to_string(&status)
        .unwrap_or_else(|_| "failed".into())
        .trim_matches('"')
        .to_string();
    connection.execute("INSERT INTO changelog_exports (id, artifact_id, revision_id, destination, format, content, content_hash, status, exported_at, diagnostic_json) VALUES (?1, ?2, ?3, ?4, 'markdown', ?5, ?6, ?7, ?8, ?9)", params![id, request.artifact_id, request.revision_id, request.destination, request.content, content_hash, status_value, exported_at, diagnostic.as_ref().map(|value| serialize(value, "export diagnostic")).transpose()?]).map_err(|error| CommandError::new("database_write_failed", "Export record could not be saved").with_details(error.to_string()))?;
    Ok(crate::domain::ChangelogExport {
        id,
        artifact_id: request.artifact_id.clone(),
        revision_id: request.revision_id.clone(),
        destination: request.destination.clone(),
        format: "markdown".into(),
        content: request.content.clone(),
        content_hash,
        status,
        exported_at,
        diagnostic,
    })
}

pub fn load_changelog_artifact(
    database: &Database,
    id: &str,
) -> Result<crate::domain::ChangelogArtifact, CommandError> {
    let connection = database
        .0
        .lock()
        .map_err(|_| CommandError::new("database_unavailable", "Database is unavailable"))?;
    connection.query_row("SELECT id, modpack_id, snapshot_id, attempt_id, release_workspace_id, stage, source_capture_fingerprint, status, introduction, content, entries_json, created_at, updated_at FROM changelog_artifacts WHERE id = ?1", [id], |row| {
        let status: String = row.get(7)?;
        let entries: String = row.get(10)?;
        Ok(crate::domain::ChangelogArtifact {
            id: row.get(0)?, modpack_id: row.get(1)?, snapshot_id: row.get(2)?, attempt_id: row.get(3)?,
            release_workspace_id: row.get(4)?,
            stage: serde_json::from_value(serde_json::Value::String(row.get(5)?)).map_err(|_| rusqlite::Error::InvalidQuery)?,
            source_capture_fingerprint: row.get(6)?,
            status: serde_json::from_value(serde_json::Value::String(status)).map_err(|_| rusqlite::Error::InvalidQuery)?,
            introduction: row.get(8)?, content: row.get(9)?,
            entries: serde_json::from_str(&entries).map_err(|_| rusqlite::Error::InvalidQuery)?,
            created_at: row.get(11)?, updated_at: row.get(12)?,
        })
    }).map_err(|error| CommandError::new("changelog_not_found", "Changelog artifact is not available").with_details(error.to_string()))
}

pub fn list_changelog_artifacts(
    database: &Database,
    project_id: &str,
) -> Result<Vec<crate::domain::ChangelogArtifact>, CommandError> {
    let ids = {
        let connection = database
            .0
            .lock()
            .map_err(|_| CommandError::new("database_unavailable", "Database is unavailable"))?;
        let mut statement = connection
            .prepare(
                "SELECT id FROM changelog_artifacts WHERE modpack_id = ?1 ORDER BY created_at DESC",
            )
            .map_err(|error| {
                CommandError::new(
                    "database_read_failed",
                    "Changelog history could not be read",
                )
                .with_details(error.to_string())
            })?;
        let rows = statement
            .query_map([project_id], |row| row.get::<_, String>(0))
            .map_err(|error| {
                CommandError::new(
                    "database_read_failed",
                    "Changelog history could not be read",
                )
                .with_details(error.to_string())
            })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|error| {
            CommandError::new(
                "database_read_failed",
                "Changelog history could not be read",
            )
            .with_details(error.to_string())
        })?
    };
    ids.into_iter()
        .map(|id| load_changelog_artifact(database, &id))
        .collect()
}

pub fn list_changelog_revisions(
    database: &Database,
    artifact_id: &str,
) -> Result<Vec<crate::domain::ChangelogRevision>, CommandError> {
    let connection = database
        .0
        .lock()
        .map_err(|_| CommandError::new("database_unavailable", "Database is unavailable"))?;
    let mut statement = connection.prepare("SELECT id, artifact_id, prior_revision_id, content, introduction, selected_version_ids_json, created_at, is_current, frozen, archived_at FROM changelog_revisions WHERE artifact_id = ?1 ORDER BY created_at DESC").map_err(|error| CommandError::new("database_read_failed", "Revision history could not be read").with_details(error.to_string()))?;
    let rows = statement
        .query_map([artifact_id], |row| {
            Ok(crate::domain::ChangelogRevision {
                id: row.get(0)?,
                artifact_id: row.get(1)?,
                prior_revision_id: row.get(2)?,
                content: row.get(3)?,
                introduction: row.get(4)?,
                selected_version_ids: serde_json::from_str(&row.get::<_, String>(5)?)
                    .unwrap_or_default(),
                created_at: row.get(6)?,
                is_current: row.get::<_, i64>(7)? != 0,
                frozen: row.get::<_, i64>(8)? != 0,
                archived_at: row.get(9)?,
            })
        })
        .map_err(|error| {
            CommandError::new("database_read_failed", "Revision history could not be read")
                .with_details(error.to_string())
        })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|error| {
        CommandError::new("database_read_failed", "Revision history could not be read")
            .with_details(error.to_string())
    })
}

pub fn list_changelog_exports(
    database: &Database,
    artifact_id: &str,
) -> Result<Vec<crate::domain::ChangelogExport>, CommandError> {
    let connection = database
        .0
        .lock()
        .map_err(|_| CommandError::new("database_unavailable", "Database is unavailable"))?;
    let mut statement = connection.prepare("SELECT id, artifact_id, revision_id, destination, format, content, content_hash, status, exported_at, diagnostic_json FROM changelog_exports WHERE artifact_id = ?1 ORDER BY exported_at DESC").map_err(|error| CommandError::new("database_read_failed", "Export history could not be read").with_details(error.to_string()))?;
    let rows = statement
        .query_map([artifact_id], |row| {
            let status: String = row.get(7)?;
            let diagnostic: Option<String> = row.get(9)?;
            Ok(crate::domain::ChangelogExport {
                id: row.get(0)?,
                artifact_id: row.get(1)?,
                revision_id: row.get(2)?,
                destination: row.get(3)?,
                format: row.get(4)?,
                content: row.get(5)?,
                content_hash: row.get(6)?,
                status: serde_json::from_value(serde_json::Value::String(status))
                    .map_err(|_| rusqlite::Error::InvalidQuery)?,
                exported_at: row.get(8)?,
                diagnostic: diagnostic
                    .map(|value| {
                        serde_json::from_str(&value).map_err(|_| rusqlite::Error::InvalidQuery)
                    })
                    .transpose()?,
            })
        })
        .map_err(|error| {
            CommandError::new("database_read_failed", "Export history could not be read")
                .with_details(error.to_string())
        })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|error| {
        CommandError::new("database_read_failed", "Export history could not be read")
            .with_details(error.to_string())
    })
}

pub fn cache_changelog_response(
    database: &Database,
    key: &crate::domain::ChangelogCacheKey,
    response: &str,
    association: &crate::domain::ProviderMatchEvidence,
) -> Result<(), CommandError> {
    cache_changelog_response_with_metadata(database, key, response, association, 1, true)
}

pub fn cache_changelog_response_with_metadata(
    database: &Database,
    key: &crate::domain::ChangelogCacheKey,
    response: &str,
    association: &crate::domain::ProviderMatchEvidence,
    page_count: u32,
    complete: bool,
) -> Result<(), CommandError> {
    let connection = database
        .0
        .lock()
        .map_err(|_| CommandError::new("database_unavailable", "Database is unavailable"))?;
    let key_json = serialize(key, "cache key")?;
    let association_json = serialize(association, "cache association")?;
    use sha2::{Digest, Sha256};
    let cache_id = format!("cache-{:x}", Sha256::digest(key_json.as_bytes()));
    let retrieved_at = timestamp();
    let normalized_query = format!(
        "project={};loader={:?};game_version={:?};limit={}",
        key.project_id, key.loader, key.game_version, key.request_shape
    );
    connection.execute("INSERT OR REPLACE INTO changelog_cache (id, cache_key_json, raw_response, association_json, retrieved_at, request_context, response_context, normalized_query, page_count, complete, loader, game_version) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)", params![cache_id, key_json, response, association_json, retrieved_at, "version-set request", "modrinth version response", normalized_query, page_count, complete, key.loader, key.game_version])
        .map(|_| ())
        .map_err(|error| CommandError::new("database_write_failed", "Provider response could not be cached").with_details(error.to_string()))
}

pub fn cached_changelog_response(
    database: &Database,
    key: &crate::domain::ChangelogCacheKey,
) -> Result<Option<(String, crate::domain::ProviderMatchEvidence)>, CommandError> {
    let connection = database
        .0
        .lock()
        .map_err(|_| CommandError::new("database_unavailable", "Database is unavailable"))?;
    let key_json = serialize(key, "cache key")?;
    let result = connection.query_row(
        "SELECT raw_response, association_json FROM changelog_cache WHERE cache_key_json = ?1",
        [key_json],
        |row| {
            let response: String = row.get(0)?;
            let association: String = row.get(1)?;
            Ok((
                response,
                serde_json::from_str(&association).map_err(|_| rusqlite::Error::InvalidQuery)?,
            ))
        },
    );
    match result {
        Ok(value) => Ok(Some(value)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(error) => Err(CommandError::new(
            "database_read_failed",
            "Provider cache could not be read",
        )
        .with_details(error.to_string())),
    }
}

pub fn cached_changelog_response_for_local_version(
    database: &Database,
    project_id: &str,
    local_version: &str,
) -> Result<Option<(String, crate::domain::ProviderMatchEvidence)>, CommandError> {
    cached_changelog_response_for_local_version_with_freshness(database, project_id, local_version)
        .map(|value| {
            value.map(|(response, association, _stale, _complete, _page_count)| {
                (response, association)
            })
        })
}

pub fn cached_changelog_response_for_local_version_with_freshness(
    database: &Database,
    project_id: &str,
    local_version: &str,
) -> Result<
    Option<(
        String,
        crate::domain::ProviderMatchEvidence,
        bool,
        bool,
        u32,
    )>,
    CommandError,
> {
    cached_changelog_response_for_local_version_with_query(
        database,
        project_id,
        local_version,
        None,
        None,
    )
}

pub fn cached_changelog_response_for_local_version_with_query(
    database: &Database,
    project_id: &str,
    local_version: &str,
    loader: Option<&str>,
    game_version: Option<&str>,
) -> Result<
    Option<(
        String,
        crate::domain::ProviderMatchEvidence,
        bool,
        bool,
        u32,
    )>,
    CommandError,
> {
    let connection = database
        .0
        .lock()
        .map_err(|_| CommandError::new("database_unavailable", "Database is unavailable"))?;
    let now = timestamp().parse::<f64>().unwrap_or_default();
    let mut statement = connection
        .prepare("SELECT cache_key_json, raw_response, association_json, complete, page_count FROM changelog_cache WHERE json_extract(cache_key_json, '$.provider') = 'modrinth' AND json_extract(cache_key_json, '$.project_id') = ?1 AND (?2 IS NULL OR json_extract(cache_key_json, '$.loader') = ?2) AND (?3 IS NULL OR json_extract(cache_key_json, '$.game_version') = ?3)")
        .map_err(|error| {
            CommandError::new("database_read_failed", "Provider cache could not be read")
                .with_details(error.to_string())
        })?;
    let rows = statement
        .query_map(rusqlite::params![project_id, loader, game_version], |row| {
            let key_json: String = row.get(0)?;
            let response: String = row.get(1)?;
            let association_json: String = row.get(2)?;
            let complete: bool = row.get(3)?;
            let page_count: u32 = row.get(4)?;
            let key: crate::domain::ChangelogCacheKey =
                serde_json::from_str(&key_json).map_err(|_| rusqlite::Error::InvalidQuery)?;
            let association: crate::domain::ProviderMatchEvidence =
                serde_json::from_str(&association_json)
                    .map_err(|_| rusqlite::Error::InvalidQuery)?;
            Ok((key, response, association, complete, page_count))
        })
        .map_err(|error| {
            CommandError::new("database_read_failed", "Provider cache could not be read")
                .with_details(error.to_string())
        })?;
    for row in rows {
        let (_key, response, association, complete, page_count) = row.map_err(|error| {
            CommandError::new(
                "database_record_invalid",
                "A cached provider response is invalid",
            )
            .with_details(error.to_string())
        })?;
        let eligible = matches!(
            association.confidence,
            crate::domain::ProviderMatchConfidence::Exact
                | crate::domain::ProviderMatchConfidence::High
        ) && association.association.as_ref().and_then(|value| {
            match &value.local_version {
                crate::domain::Evidence::Observed(version) => Some(version.as_str()),
                _ => None,
            }
        }) == Some(local_version)
            && association.association.is_some();
        if eligible {
            let retrieved_at = connection
                .query_row(
                    "SELECT retrieved_at FROM changelog_cache WHERE cache_key_json = ?1",
                    [&serde_json::to_string(&_key).unwrap_or_default()],
                    |row| row.get::<_, String>(0),
                )
                .unwrap_or_else(|_| "0".into())
                .parse::<f64>()
                .unwrap_or_default();
            return Ok(Some((
                response,
                association,
                now - retrieved_at > 86_400.0,
                complete,
                page_count,
            )));
        }
    }
    Ok(None)
}

fn timestamp() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| format!("{}.{:09}", duration.as_secs(), duration.subsec_nanos()))
        .unwrap_or_else(|_| "0".to_string())
}

fn unique_id(prefix: &str) -> String {
    static SEQUENCE: AtomicU64 = AtomicU64::new(0);
    format!(
        "{}-{}-{}",
        prefix,
        timestamp(),
        SEQUENCE.fetch_add(1, Ordering::Relaxed)
    )
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
            "INSERT INTO modpack_activity (modpack_id, event_type, occurred_at, message) VALUES (?1, ?2, ?3, ?4)",
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
        .prepare("SELECT event_type, occurred_at, message FROM modpack_activity WHERE modpack_id = ?1 ORDER BY occurred_at DESC LIMIT 10")
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

fn row_to_modpack(row: &rusqlite::Row<'_>) -> rusqlite::Result<ModpackRecord> {
    let application: String = row.get(2)?;
    let packwiz: String = row.get(3)?;
    let validation: String = row.get(4)?;
    Ok(ModpackRecord {
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
pub fn preview_modpack(path: String) -> Result<RegistrationPreview, CommandError> {
    validation::preview(&path)
}

#[tauri::command]
pub fn register_modpack(
    state: State<'_, Database>,
    path: String,
    application: Option<ApplicationModpackMetadata>,
) -> Result<ModpackRecord, CommandError> {
    register_modpack_inner(&state, path, application)
}

fn register_modpack_inner(
    database: &Database,
    path: String,
    application: Option<ApplicationModpackMetadata>,
) -> Result<ModpackRecord, CommandError> {
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
        "INSERT INTO modpacks (id, canonical_path, application_json, packwiz_json, validation_json, created_at, updated_at, last_opened_at, last_refreshed_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6, ?6, ?6)",
        params![id, preview.canonical_path, application_json, packwiz_json, validation_json, now],
    ).map_err(|error| {
        if matches!(error, rusqlite::Error::SqliteFailure(_, _)) { CommandError::new("duplicate_modpack", "This modpack directory is already registered").with_details(error.to_string()) } else { CommandError::new("database_write_failed", "Modpack could not be registered").with_details(error.to_string()) }
    })?;
    record_activity(&connection, &id, "registered", "Project registered")?;
    Ok(ModpackRecord {
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
pub fn list_modpacks(state: State<'_, Database>) -> Result<Vec<ModpackRecord>, CommandError> {
    let connection = state
        .0
        .lock()
        .map_err(|_| CommandError::new("database_unavailable", "Database is unavailable"))?;
    let mut statement = connection.prepare("SELECT id, canonical_path, application_json, packwiz_json, validation_json, created_at, updated_at, last_opened_at, last_refreshed_at FROM modpacks ORDER BY updated_at DESC").map_err(|error| CommandError::new("database_read_failed", "Modpacks could not be read").with_details(error.to_string()))?;
    let rows = statement.query_map([], row_to_modpack).map_err(|error| {
        CommandError::new("database_read_failed", "Projects could not be read")
            .with_details(error.to_string())
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|error| {
        CommandError::new("database_record_invalid", "A stored project is invalid")
            .with_details(error.to_string())
    })
}

fn row_to_snapshot(row: &rusqlite::Row<'_>) -> rusqlite::Result<SnapshotRecord> {
    let lifecycle: String = row.get(3)?;
    let outcome: String = row.get(4)?;
    let result_json: String = row.get(6)?;
    let baseline_capture_json: Option<String> = row.get(7)?;
    Ok(SnapshotRecord {
        id: row.get(0)?,
        modpack_id: row.get(1)?,
        predecessor_id: row.get(2)?,
        lifecycle: serde_json::from_value(serde_json::Value::String(lifecycle))
            .map_err(|_| rusqlite::Error::InvalidQuery)?,
        outcome: serde_json::from_value(serde_json::Value::String(outcome))
            .map_err(|_| rusqlite::Error::InvalidQuery)?,
        label: row.get(5)?,
        baseline_capture: baseline_capture_json
            .map(|json| serde_json::from_str(&json).map_err(|_| rusqlite::Error::InvalidQuery))
            .transpose()?,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
        closed_at: row.get(10)?,
        result: serde_json::from_str(&result_json).map_err(|_| rusqlite::Error::InvalidQuery)?,
        candidates: Vec::new(),
        decisions: Vec::new(),
        notes: Vec::new(),
        rechecks: Vec::new(),
    })
}

fn enrich_snapshot(
    connection: &Connection,
    mut snapshot: SnapshotRecord,
) -> Result<SnapshotRecord, CommandError> {
    let mut candidates = connection
        .prepare("SELECT id, candidate_json, observed_at FROM snapshot_candidates WHERE snapshot_id = ?1 ORDER BY id")
        .map_err(|error| CommandError::new("database_read_failed", "Snapshot candidates could not be read").with_details(error.to_string()))?
        .query_map([&snapshot.id], |row| {
            let candidate_json: String = row.get(1)?;
            Ok(SnapshotCandidateRecord {
                id: row.get(0)?,
                candidate: serde_json::from_str::<UpdateCandidate>(&candidate_json)
                    .map_err(|_| rusqlite::Error::InvalidQuery)?,
                observed_at: row.get(2)?,
            })
        })
        .map_err(|error| CommandError::new("database_read_failed", "Snapshot candidates could not be read").with_details(error.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| CommandError::new("database_record_invalid", "A stored snapshot candidate is invalid").with_details(error.to_string()))?;
    snapshot.candidates.append(&mut candidates);

    let mut decisions = connection
        .prepare("SELECT id, candidate_id, decision, note, recorded_at FROM snapshot_decisions WHERE snapshot_id = ?1 ORDER BY id")
        .map_err(|error| CommandError::new("database_read_failed", "Snapshot decisions could not be read").with_details(error.to_string()))?
        .query_map([&snapshot.id], |row| {
            let decision: String = row.get(2)?;
            Ok(SnapshotDecisionRecord {
                id: row.get(0)?,
                candidate_id: row.get(1)?,
                decision: serde_json::from_value(serde_json::Value::String(decision))
                    .map_err(|_| rusqlite::Error::InvalidQuery)?,
                note: row.get(3)?,
                recorded_at: row.get(4)?,
            })
        })
        .map_err(|error| CommandError::new("database_read_failed", "Snapshot decisions could not be read").with_details(error.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| CommandError::new("database_record_invalid", "A stored snapshot decision is invalid").with_details(error.to_string()))?;
    snapshot.decisions.append(&mut decisions);

    let mut notes = connection
        .prepare("SELECT id, modpack_id, snapshot_id, candidate_id, scope, note, is_current, recorded_at FROM snapshot_notes WHERE modpack_id = (SELECT modpack_id FROM snapshots WHERE id = ?1) AND (snapshot_id IS NULL OR snapshot_id = ?1) ORDER BY id")
        .map_err(|error| CommandError::new("database_read_failed", "Snapshot notes could not be read").with_details(error.to_string()))?
        .query_map([&snapshot.id], |row| {
            let scope: String = row.get(4)?;
            Ok(SnapshotNoteRecord {
                id: row.get(0)?,
                modpack_id: row.get(1)?,
                snapshot_id: row.get(2)?,
                candidate_id: row.get(3)?,
                scope: serde_json::from_value(serde_json::Value::String(scope))
                    .map_err(|_| rusqlite::Error::InvalidQuery)?,
                note: row.get(5)?,
                is_current: row.get::<_, i64>(6)? != 0,
                recorded_at: row.get(7)?,
            })
        })
        .map_err(|error| CommandError::new("database_read_failed", "Snapshot notes could not be read").with_details(error.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| CommandError::new("database_record_invalid", "A stored snapshot note is invalid").with_details(error.to_string()))?;
    snapshot.notes.append(&mut notes);
    let mut rechecks = connection
        .prepare("SELECT id, comparable, unchanged, differences_json, checked_at FROM snapshot_rechecks WHERE snapshot_id = ?1 ORDER BY id")
        .map_err(|error| CommandError::new("database_read_failed", "Snapshot freshness checks could not be read").with_details(error.to_string()))?
        .query_map([&snapshot.id], |row| {
            let differences_json: String = row.get(3)?;
            Ok(SnapshotRecheckRecord {
                id: row.get(0)?,
                comparable: row.get::<_, i64>(1)? != 0,
                unchanged: row.get::<_, i64>(2)? != 0,
                differences: serde_json::from_str(&differences_json)
                    .map_err(|_| rusqlite::Error::InvalidQuery)?,
                checked_at: row.get(4)?,
            })
        })
        .map_err(|error| CommandError::new("database_read_failed", "Snapshot freshness checks could not be read").with_details(error.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| CommandError::new("database_record_invalid", "A stored freshness check is invalid").with_details(error.to_string()))?;
    snapshot.rechecks.append(&mut rechecks);
    Ok(snapshot)
}

#[tauri::command]
pub fn list_snapshots(
    state: State<'_, Database>,
    modpack_id: String,
) -> Result<Vec<SnapshotRecord>, CommandError> {
    let connection = state
        .0
        .lock()
        .map_err(|_| CommandError::new("database_unavailable", "Database is unavailable"))?;
    let mut statement = connection
        .prepare("SELECT id, modpack_id, predecessor_id, lifecycle, outcome, label, result_json, baseline_capture_json, created_at, updated_at, closed_at FROM snapshots WHERE modpack_id = ?1 ORDER BY created_at DESC")
        .map_err(|error| CommandError::new("database_read_failed", "Snapshots could not be read").with_details(error.to_string()))?;
    let snapshots = statement
        .query_map([modpack_id], row_to_snapshot)
        .map_err(|error| {
            CommandError::new("database_read_failed", "Snapshots could not be read")
                .with_details(error.to_string())
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| {
            CommandError::new("database_record_invalid", "A stored snapshot is invalid")
                .with_details(error.to_string())
        });
    let snapshots = snapshots?;
    snapshots
        .into_iter()
        .map(|snapshot| enrich_snapshot(&connection, snapshot))
        .collect()
}

#[tauri::command]
pub fn get_snapshot(
    state: State<'_, Database>,
    id: String,
) -> Result<SnapshotRecord, CommandError> {
    let connection = state
        .0
        .lock()
        .map_err(|_| CommandError::new("database_unavailable", "Database is unavailable"))?;
    let snapshot = connection
        .query_row(
            "SELECT id, modpack_id, predecessor_id, lifecycle, outcome, label, result_json, baseline_capture_json, created_at, updated_at, closed_at FROM snapshots WHERE id = ?1",
            [id],
            row_to_snapshot,
        )
        .map_err(|error| CommandError::new("snapshot_not_found", "Snapshot is not available").with_details(error.to_string()))?;
    enrich_snapshot(&connection, snapshot)
}

fn snapshot_state(connection: &Connection, id: &str) -> Result<(String, String), CommandError> {
    connection
        .query_row(
            "SELECT lifecycle, result_json FROM snapshots WHERE id = ?1",
            [id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|error| {
            CommandError::new("snapshot_not_found", "Snapshot is not available")
                .with_details(error.to_string())
        })
}

fn ensure_writable_snapshot(connection: &Connection, id: &str) -> Result<(), CommandError> {
    let (lifecycle, result_json) = snapshot_state(connection, id)?;
    let outcome: crate::domain::DiscoveryOutcomeKind = serde_json::from_str(&result_json)
        .ok()
        .map(|result: DiscoveryResult| result.outcome)
        .unwrap_or(crate::domain::DiscoveryOutcomeKind::Indeterminate);
    if lifecycle != "reviewable" || !matches!(outcome, crate::domain::DiscoveryOutcomeKind::Normal)
    {
        return Err(CommandError::new(
            "snapshot_not_reviewable",
            "Snapshot is not available for review writes",
        ));
    }
    Ok(())
}

#[tauri::command]
pub fn set_snapshot_decision(
    state: State<'_, Database>,
    snapshot_id: String,
    candidate_id: String,
    decision: SnapshotDecision,
    note: Option<String>,
) -> Result<(), CommandError> {
    let connection = state
        .0
        .lock()
        .map_err(|_| CommandError::new("database_unavailable", "Database is unavailable"))?;
    ensure_writable_snapshot(&connection, &snapshot_id)?;
    let candidate_exists = connection
        .query_row(
            "SELECT EXISTS (SELECT 1 FROM snapshot_candidates WHERE id = ?1 AND snapshot_id = ?2)",
            params![candidate_id, snapshot_id],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|error| {
            CommandError::new(
                "database_read_failed",
                "Snapshot candidate could not be checked",
            )
            .with_details(error.to_string())
        })?;
    if candidate_exists == 0 {
        return Err(CommandError::new(
            "candidate_not_found",
            "Candidate does not belong to this snapshot",
        ));
    }
    let decision = serde_json::to_string(&decision).unwrap_or_else(|_| "undecided".into());
    connection.execute("INSERT INTO snapshot_decisions (snapshot_id, candidate_id, decision, note, recorded_at) VALUES (?1, ?2, ?3, ?4, ?5)", params![snapshot_id, candidate_id, decision.trim_matches('"'), note, timestamp()]).map_err(|error| CommandError::new("database_write_failed", "Snapshot decision could not be saved").with_details(error.to_string()))?;
    Ok(())
}

#[tauri::command]
pub fn save_snapshot_note(
    state: State<'_, Database>,
    modpack_id: String,
    snapshot_id: Option<String>,
    candidate_id: Option<String>,
    scope: SnapshotNoteScope,
    note: String,
) -> Result<(), CommandError> {
    let note = note.trim().to_string();
    if note.is_empty() || note.len() > 4000 {
        return Err(CommandError::new(
            "invalid_note",
            "Notes must contain between 1 and 4000 characters",
        ));
    }
    let connection = state
        .0
        .lock()
        .map_err(|_| CommandError::new("database_unavailable", "Database is unavailable"))?;
    if let Some(id) = snapshot_id.as_deref() {
        ensure_writable_snapshot(&connection, id)?;
    }
    if let Some(candidate) = candidate_id.as_deref() {
        let belongs_to_snapshot = snapshot_id
            .as_deref()
            .map(|snapshot| {
                connection
                    .query_row(
                        "SELECT EXISTS (SELECT 1 FROM snapshot_candidates WHERE id = ?1 AND snapshot_id = ?2)",
                        params![candidate, snapshot],
                        |row| row.get::<_, i64>(0),
                    )
                    .unwrap_or(0)
                    == 1
            })
            .unwrap_or(false);
        if !belongs_to_snapshot {
            return Err(CommandError::new(
                "candidate_not_found",
                "Candidate does not belong to this snapshot",
            ));
        }
    }
    connection.execute("UPDATE snapshot_notes SET is_current = 0 WHERE modpack_id = ?1 AND ((snapshot_id = ?2) OR (snapshot_id IS NULL AND ?2 IS NULL)) AND ((candidate_id = ?3) OR (candidate_id IS NULL AND ?3 IS NULL))", params![modpack_id, snapshot_id, candidate_id]).map_err(|error| CommandError::new("database_write_failed", "Previous note could not be archived").with_details(error.to_string()))?;
    let scope = serde_json::to_string(&scope).unwrap_or_else(|_| "project".into());
    connection.execute("INSERT INTO snapshot_notes (modpack_id, snapshot_id, candidate_id, scope, note, recorded_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)", params![modpack_id, snapshot_id, candidate_id, scope.trim_matches('"'), note, timestamp()]).map_err(|error| CommandError::new("database_write_failed", "Snapshot note could not be saved").with_details(error.to_string()))?;
    Ok(())
}

#[tauri::command]
pub fn close_snapshot(
    state: State<'_, Database>,
    id: String,
    cancelled: bool,
) -> Result<SnapshotRecord, CommandError> {
    let connection = state
        .0
        .lock()
        .map_err(|_| CommandError::new("database_unavailable", "Database is unavailable"))?;
    let requested = if cancelled { "cancelled" } else { "closed" };
    let current: String = connection
        .query_row(
            "SELECT lifecycle FROM snapshots WHERE id = ?1",
            [&id],
            |row| row.get(0),
        )
        .map_err(|error| {
            CommandError::new("snapshot_not_found", "Snapshot is not available")
                .with_details(error.to_string())
        })?;
    if current == requested {
        let snapshot = connection.query_row("SELECT id, modpack_id, predecessor_id, lifecycle, outcome, label, result_json, baseline_capture_json, created_at, updated_at, closed_at FROM snapshots WHERE id = ?1", [&id], row_to_snapshot).map_err(|error| CommandError::new("snapshot_not_found", "Snapshot is not available").with_details(error.to_string()))?;
        return enrich_snapshot(&connection, snapshot);
    }
    if !matches!(current.as_str(), "draft" | "reviewable") {
        return Err(CommandError::new(
            "snapshot_transition_rejected",
            "Snapshot can no longer be closed or cancelled",
        ));
    }
    connection
        .execute(
            "UPDATE snapshots SET lifecycle = ?1, updated_at = ?2, closed_at = ?2 WHERE id = ?3",
            params![requested, timestamp(), id],
        )
        .map_err(|error| {
            CommandError::new("database_write_failed", "Snapshot could not be closed")
                .with_details(error.to_string())
        })?;
    let snapshot = connection.query_row("SELECT id, modpack_id, predecessor_id, lifecycle, outcome, label, result_json, baseline_capture_json, created_at, updated_at, closed_at FROM snapshots WHERE id = ?1", [&id], row_to_snapshot).map_err(|error| CommandError::new("snapshot_not_found", "Snapshot is not available").with_details(error.to_string()))?;
    enrich_snapshot(&connection, snapshot)
}

#[tauri::command]
pub fn link_snapshot_retry(
    state: State<'_, Database>,
    predecessor_id: String,
    retry_id: String,
) -> Result<SnapshotRecord, CommandError> {
    let connection = state
        .0
        .lock()
        .map_err(|_| CommandError::new("database_unavailable", "Database is unavailable"))?;
    let predecessor_project: String = connection
        .query_row(
            "SELECT modpack_id FROM snapshots WHERE id = ?1",
            [&predecessor_id],
            |row| row.get(0),
        )
        .map_err(|error| {
            CommandError::new(
                "snapshot_not_found",
                "Predecessor snapshot is not available",
            )
            .with_details(error.to_string())
        })?;
    let retry_project: String = connection
        .query_row(
            "SELECT modpack_id FROM snapshots WHERE id = ?1",
            [&retry_id],
            |row| row.get(0),
        )
        .map_err(|error| {
            CommandError::new("snapshot_not_found", "Retry snapshot is not available")
                .with_details(error.to_string())
        })?;
    if predecessor_project != retry_project || predecessor_id == retry_id {
        return Err(CommandError::new(
            "retry_link_rejected",
            "Retry and predecessor must belong to the same project",
        ));
    }
    connection
        .execute(
            "UPDATE snapshots SET predecessor_id = ?1, updated_at = ?2 WHERE id = ?3 AND predecessor_id IS NULL",
            params![predecessor_id, timestamp(), retry_id],
        )
        .map_err(|error| CommandError::new("database_write_failed", "Retry link could not be saved").with_details(error.to_string()))?;
    let snapshot = connection
        .query_row("SELECT id, modpack_id, predecessor_id, lifecycle, outcome, label, result_json, baseline_capture_json, created_at, updated_at, closed_at FROM snapshots WHERE id = ?1", [&retry_id], row_to_snapshot)
        .map_err(|error| CommandError::new("snapshot_not_found", "Retry snapshot is not available").with_details(error.to_string()))?;
    enrich_snapshot(&connection, snapshot)
}

#[tauri::command]
pub fn recheck_snapshot(
    state: State<'_, Database>,
    id: String,
) -> Result<SnapshotRecord, CommandError> {
    let (project_path, result_json) = {
        let connection = state
            .0
            .lock()
            .map_err(|_| CommandError::new("database_unavailable", "Database is unavailable"))?;
        connection.query_row("SELECT modpacks.canonical_path, snapshots.result_json FROM snapshots JOIN modpacks ON modpacks.id = snapshots.modpack_id WHERE snapshots.id = ?1", [&id], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))).map_err(|_| CommandError::new("snapshot_not_found", "Snapshot is not available"))?
    };
    let result: DiscoveryResult = serde_json::from_str(&result_json).map_err(|_| {
        CommandError::new("snapshot_invalid", "Stored snapshot evidence is invalid")
    })?;
    let before = result
        .diagnostics
        .fingerprint
        .as_ref()
        .map(|comparison| comparison.before.clone());
    let current = crate::discovery::fingerprint::collect(std::path::Path::new(&project_path)).ok();
    let (comparable, unchanged, differences) = before
        .map(|before| {
            current
                .map(|after| {
                    let comparison = crate::discovery::fingerprint::compare(before, after);
                    (
                        comparison.comparable,
                        comparison.unchanged,
                        comparison.differences,
                    )
                })
                .unwrap_or((
                    false,
                    false,
                    vec!["Current fingerprint could not be collected".into()],
                ))
        })
        .unwrap_or((
            false,
            false,
            vec!["Snapshot has no comparable before fingerprint".into()],
        ));
    let stale = !comparable || !unchanged;
    let connection = state
        .0
        .lock()
        .map_err(|_| CommandError::new("database_unavailable", "Database is unavailable"))?;
    let differences_json = serialize(&differences, "freshness differences")?;
    connection
        .execute(
            "INSERT INTO snapshot_rechecks (snapshot_id, comparable, unchanged, differences_json, checked_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, comparable as i64, unchanged as i64, differences_json, timestamp()],
        )
        .map_err(|error| CommandError::new("database_write_failed", "Snapshot freshness could not be saved").with_details(error.to_string()))?;
    if stale {
        connection
            .execute(
                "UPDATE snapshots SET lifecycle = 'stale', updated_at = ?1 WHERE id = ?2",
                params![timestamp(), id],
            )
            .map_err(|error| {
                CommandError::new(
                    "database_write_failed",
                    "Snapshot freshness could not be saved",
                )
                .with_details(error.to_string())
            })?;
    }
    let snapshot = connection.query_row("SELECT id, modpack_id, predecessor_id, lifecycle, outcome, label, result_json, baseline_capture_json, created_at, updated_at, closed_at FROM snapshots WHERE id = ?1", [id], row_to_snapshot).map_err(|error| CommandError::new("snapshot_not_found", "Snapshot is not available").with_details(error.to_string()))?;
    enrich_snapshot(&connection, snapshot)
}

#[tauri::command]
pub fn open_modpack(state: State<'_, Database>, id: String) -> Result<ModpackRecord, CommandError> {
    let connection = state
        .0
        .lock()
        .map_err(|_| CommandError::new("database_unavailable", "Database is unavailable"))?;
    let mut project = connection.query_row("SELECT id, canonical_path, application_json, packwiz_json, validation_json, created_at, updated_at, last_opened_at, last_refreshed_at FROM modpacks WHERE id = ?1", [&id], row_to_modpack).map_err(|error| CommandError::new("modpack_not_found", "Modpack is not registered").with_details(error.to_string()))?;
    let current = validation::preview(&project.canonical_path).map_err(|error| {
        CommandError::new(
            "project_state_unavailable",
            "Registered modpack files could not be reopened",
        )
        .with_details(format!("{}: {}", error.code, error.message))
    })?;
    project.packwiz = current.packwiz;
    project.validation = current.validation;
    let now = timestamp();
    connection
        .execute(
            "UPDATE modpacks SET last_opened_at = ?1, updated_at = ?1 WHERE id = ?2",
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
) -> Result<ModpackRecord, CommandError> {
    let existing = connection.query_row("SELECT id, canonical_path, application_json, packwiz_json, validation_json, created_at, updated_at, last_opened_at, last_refreshed_at FROM modpacks WHERE id = ?1", [id], row_to_modpack).map_err(|error| CommandError::new("modpack_not_found", "Modpack is not registered").with_details(error.to_string()))?;
    let inventory = inventory::read_inventory(std::path::Path::new(&preview.canonical_path))?;
    let overview = inventory::read_overview(std::path::Path::new(&preview.canonical_path))?;
    let inventory_json = serialize(&inventory, "inventory")?;
    let overview_json = serialize(&overview, "overview")?;
    let now = timestamp();
    connection.execute("UPDATE modpacks SET canonical_path = ?1, packwiz_json = ?2, validation_json = ?3, updated_at = ?4, last_refreshed_at = ?4 WHERE id = ?5", params![preview.canonical_path, serialize(&preview.packwiz, "Packwiz observations")?, serialize(&preview.validation, "validation results")?, now, id]).map_err(|error| CommandError::new("database_write_failed", "Modpack refresh could not be saved").with_details(error.to_string()))?;
    connection
        .execute(
            "INSERT INTO inventory_observations (modpack_id, observed_at, freshness, inventory_json, overview_json) VALUES (?1, ?2, 'current', ?3, ?4) ON CONFLICT(modpack_id) DO UPDATE SET observed_at = excluded.observed_at, freshness = excluded.freshness, inventory_json = excluded.inventory_json, overview_json = excluded.overview_json",
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
    Ok(ModpackRecord {
        canonical_path: preview.canonical_path.clone(),
        packwiz: preview.packwiz.clone(),
        validation: preview.validation.clone(),
        updated_at: now.clone(),
        last_refreshed_at: Some(now),
        ..existing
    })
}

#[tauri::command]
pub fn refresh_modpack(
    state: State<'_, Database>,
    id: String,
) -> Result<ModpackRecord, CommandError> {
    refresh_modpack_inner(&state, &id)
}

fn refresh_modpack_inner(database: &Database, id: &str) -> Result<ModpackRecord, CommandError> {
    let connection = database
        .0
        .lock()
        .map_err(|_| CommandError::new("database_unavailable", "Database is unavailable"))?;
    let path: String = connection
        .query_row(
            "SELECT canonical_path FROM modpacks WHERE id = ?1",
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
                "UPDATE inventory_observations SET freshness = 'stale' WHERE modpack_id = ?1",
                [id],
            );
            let _ = record_activity(&connection, id, "refresh_failed", &error.message);
            return Err(error);
        }
    };
    refresh_record(&connection, id, &preview)
}

#[tauri::command]
pub fn get_modpack_inventory(
    state: State<'_, Database>,
    id: String,
) -> Result<Vec<InventoryEntry>, CommandError> {
    let connection = state
        .0
        .lock()
        .map_err(|_| CommandError::new("database_unavailable", "Database is unavailable"))?;
    let path: String = connection
        .query_row(
            "SELECT canonical_path FROM modpacks WHERE id = ?1",
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
pub fn get_modpack_overview(
    state: State<'_, Database>,
    id: String,
) -> Result<ModpackOverview, CommandError> {
    let connection = state
        .0
        .lock()
        .map_err(|_| CommandError::new("database_unavailable", "Database is unavailable"))?;
    let path: String = connection
        .query_row(
            "SELECT canonical_path FROM modpacks WHERE id = ?1",
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
pub fn update_modpack_metadata(
    state: State<'_, Database>,
    id: String,
    application: ApplicationModpackMetadata,
) -> Result<ModpackRecord, CommandError> {
    let connection = state
        .0
        .lock()
        .map_err(|_| CommandError::new("database_unavailable", "Database is unavailable"))?;
    let existing = connection.query_row("SELECT id, canonical_path, application_json, packwiz_json, validation_json, created_at, updated_at, last_opened_at, last_refreshed_at FROM modpacks WHERE id = ?1", [&id], row_to_modpack).map_err(|error| CommandError::new("modpack_not_found", "Modpack is not registered").with_details(error.to_string()))?;
    let now = timestamp();
    connection
        .execute(
            "UPDATE modpacks SET application_json = ?1, updated_at = ?2 WHERE id = ?3",
            params![serialize(&application, "application metadata")?, now, id],
        )
        .map_err(|error| {
            CommandError::new(
                "database_write_failed",
                "Modpack metadata could not be saved",
            )
            .with_details(error.to_string())
        })?;
    Ok(ModpackRecord {
        application,
        updated_at: now,
        ..existing
    })
}

fn set_lifecycle(
    database: &Database,
    id: String,
    lifecycle: ModpackLifecycle,
) -> Result<ModpackRecord, CommandError> {
    let connection = database
        .0
        .lock()
        .map_err(|_| CommandError::new("database_unavailable", "Database is unavailable"))?;
    let mut project = connection.query_row("SELECT id, canonical_path, application_json, packwiz_json, validation_json, created_at, updated_at, last_opened_at, last_refreshed_at FROM modpacks WHERE id = ?1", [&id], row_to_modpack).map_err(|error| CommandError::new("modpack_not_found", "Modpack is not registered").with_details(error.to_string()))?;
    project.application.lifecycle = lifecycle;
    let now = timestamp();
    connection
        .execute(
            "UPDATE modpacks SET application_json = ?1, updated_at = ?2 WHERE id = ?3",
            params![
                serialize(&project.application, "application metadata")?,
                now,
                id
            ],
        )
        .map_err(|error| {
            CommandError::new(
                "database_write_failed",
                "Modpack lifecycle could not be saved",
            )
            .with_details(error.to_string())
        })?;
    project.updated_at = now;
    Ok(project)
}

#[tauri::command]
pub fn archive_modpack(
    state: State<'_, Database>,
    id: String,
) -> Result<ModpackRecord, CommandError> {
    set_lifecycle(&state, id, ModpackLifecycle::Archived)
}

#[tauri::command]
pub fn restore_modpack(
    state: State<'_, Database>,
    id: String,
) -> Result<ModpackRecord, CommandError> {
    set_lifecycle(&state, id, ModpackLifecycle::Active)
}

#[tauri::command]
pub fn disconnect_modpack(
    state: State<'_, Database>,
    id: String,
) -> Result<ModpackRecord, CommandError> {
    set_lifecycle(&state, id, ModpackLifecycle::Disconnected)
}

#[tauri::command]
pub fn reconnect_modpack(
    state: State<'_, Database>,
    id: String,
    path: String,
) -> Result<ModpackRecord, CommandError> {
    reconnect_modpack_inner(&state, id, path)
}

fn reconnect_modpack_inner(
    database: &Database,
    id: String,
    path: String,
) -> Result<ModpackRecord, CommandError> {
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
    use rusqlite::params;
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
                "SELECT name FROM sqlite_master WHERE type = 'table' AND name = 'modpacks'",
                [],
                |row| row.get(0),
            )
            .expect("project schema should be initialized");
        assert_eq!(project_table, "modpacks");
        for table in [
            "operation_attempts",
            "operation_candidates",
            "operation_acknowledgements",
            "pin_observations",
            "changelog_cache",
            "changelog_attempts",
            "changelog_artifacts",
            "changelog_revisions",
            "changelog_exports",
            "lifecycle_tombstones",
            "lifecycle_operations",
            "cleanup_operations",
        ] {
            let exists: i64 = connection
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
                    [table],
                    |row| row.get(0),
                )
                .expect("operation schema should be queryable");
            assert_eq!(exists, 1, "missing operation table {table}");
        }
        let has_selection_column: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('changelog_revisions') WHERE name = 'selected_version_ids_json'",
                [],
                |row| row.get(0),
            )
            .expect("revision selection column should be queryable");
        assert_eq!(has_selection_column, 1);
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
        let schema_version: String = connection
            .query_row(
                "SELECT value FROM schema_metadata WHERE key = 'schema_version'",
                [],
                |row| row.get(0),
            )
            .expect("schema version should be readable");
        assert_eq!(schema_version, "4");
    }

    #[test]
    fn rejects_existing_version_three_database_without_migration() {
        let path =
            std::env::temp_dir().join(format!("cm-modpack-util-v3-{}.sqlite", std::process::id()));
        {
            let connection = rusqlite::Connection::open(&path).expect("database should open");
            connection
                .execute_batch(
                    "CREATE TABLE schema_metadata (key TEXT PRIMARY KEY, value TEXT NOT NULL); INSERT INTO schema_metadata (key, value) VALUES ('schema_version', '3');",
                )
                .expect("legacy sentinel should be created");
        }
        assert!(initialize(&path).is_err());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn recovers_persisted_running_lifecycle_and_cleanup_operations() {
        let path = std::env::temp_dir().join(format!(
            "cm-modpack-util-recovery-{}-{}.sqlite",
            std::process::id(),
            super::unique_id("test")
        ));
        {
            let database = initialize(&path).expect("database should initialize");
            let connection = database
                .0
                .lock()
                .expect("database lock should be available");
            connection
                .execute(
                    "INSERT INTO lifecycle_operations (id, record_type, record_id, action, preview_fingerprint, status, created_at) VALUES ('lifecycle-1', 'snapshot', 'snapshot-1', 'archive', 'fingerprint', 'running', '1')",
                    [],
                )
                .expect("running lifecycle operation should be insertable");
            connection
                .execute(
                    "INSERT INTO cleanup_operations (id, scope, preview_fingerprint, status, created_at) VALUES ('cleanup-1', 'provider_cache', 'fingerprint', 'running', '1')",
                    [],
                )
                .expect("running cleanup operation should be insertable");
        }
        let database = initialize(&path).expect("database should recover stale operations");
        let connection = database
            .0
            .lock()
            .expect("database lock should be available");
        for table in ["lifecycle_operations", "cleanup_operations"] {
            let status: String = connection
                .query_row(
                    &format!("SELECT status FROM {table} WHERE id = ?1"),
                    [if table == "lifecycle_operations" {
                        "lifecycle-1"
                    } else {
                        "cleanup-1"
                    }],
                    |row| row.get(0),
                )
                .expect("recovered operation should be readable");
            assert_eq!(status, "failed");
        }
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(path.with_extension("sqlite-wal"));
        let _ = std::fs::remove_file(path.with_extension("sqlite-shm"));
    }

    #[test]
    fn workspace_schema_allows_multiple_active_records_and_preserves_history() {
        let database = initialize(Path::new(":memory:")).expect("database should initialize");
        let connection = database
            .0
            .lock()
            .expect("database lock should be available");
        connection
            .execute(
                "INSERT INTO modpacks (id, canonical_path, application_json, packwiz_json, validation_json, created_at, updated_at) VALUES ('project', '.', '{}', '{}', '[]', '0', '0')",
                [],
            )
            .expect("project should be insertable");
        connection
            .execute(
                "INSERT INTO snapshots (id, modpack_id, lifecycle, outcome, result_json, baseline_capture_json, created_at, updated_at) VALUES ('snapshot', 'project', 'reviewable', 'normal', '{}', '{}', '0', '0')",
                [],
            )
            .expect("snapshot should be insertable");
        let workspace = |id: &str| {
            connection.execute(
                "INSERT INTO release_workspaces (id, modpack_id, source_snapshot_id, name, lifecycle, evidence_status, publication_status, created_at, updated_at) VALUES (?1, 'project', 'snapshot', 'Release', 'draft', 'baseline', 'draft', '0', '0')",
                [id],
            )
        };
        workspace("first").expect("first workspace should be insertable");
        workspace("second").expect("multiple active workspaces should be insertable");
        connection
            .execute(
                "UPDATE release_workspaces SET lifecycle = 'abandoned', abandoned_at = '1' WHERE id = 'first'",
                [],
            )
            .expect("workspace should be abandonable");
        drop(connection);
        let loaded = super::load_release_workspace_inner(&database, "second")
            .expect("a newly stored workspace should load with nullable finalization fields");
        assert_eq!(
            loaded.publication_status,
            crate::domain::ReleasePublicationStatus::Draft
        );
        assert!(loaded.final_capture.is_none());
        assert!(loaded.final_changelog_revision_id.is_none());
        assert!(loaded.finalization_receipt.is_none());
    }

    #[test]
    fn workspace_activity_cursor_query_returns_newest_entries_and_older_pages() {
        let database = initialize(Path::new(":memory:")).expect("database should initialize");
        let connection = database
            .0
            .lock()
            .expect("database lock should be available");
        connection
            .execute(
                "INSERT INTO modpacks (id, canonical_path, application_json, packwiz_json, validation_json, created_at, updated_at) VALUES ('project', '.', '{}', '{}', '[]', '0', '0')",
                [],
            )
            .expect("project should be insertable");
        connection
            .execute(
                "INSERT INTO release_workspaces (id, modpack_id, name, lifecycle, evidence_status, publication_status, created_at, updated_at) VALUES ('workspace', 'project', 'Release', 'draft', 'baseline', 'draft', '0', '0')",
                [],
            )
            .expect("workspace should be insertable");
        for index in 1..=5 {
            connection
                .execute(
                    "INSERT INTO release_workspace_activity (workspace_id, event_type, occurred_at, message) VALUES ('workspace', 'event', ?1, ?2)",
                    params![format!("legacy-{}", index), format!("Event {index}")],
                )
                .expect("activity should be insertable");
        }

        let first_page = connection
            .prepare("SELECT id, event_type, occurred_at, message FROM release_workspace_activity WHERE workspace_id = 'workspace' ORDER BY id DESC LIMIT 4")
            .expect("first page query should prepare")
            .query_map([], super::activity_from_row)
            .expect("first page query should run")
            .collect::<Result<Vec<_>, _>>()
            .expect("first page should decode");
        assert_eq!(first_page.len(), 4);
        assert!(first_page[0].id > first_page[1].id);
        let cursor = first_page[2].id;
        let older_page = connection
            .prepare("SELECT id, event_type, occurred_at, message FROM release_workspace_activity WHERE workspace_id = 'workspace' AND id < ?1 ORDER BY id DESC LIMIT 4")
            .expect("older page query should prepare")
            .query_map([cursor], super::activity_from_row)
            .expect("older page query should run")
            .collect::<Result<Vec<_>, _>>()
            .expect("older page should decode");
        assert_eq!(older_page.len(), 2);
        assert!(older_page.iter().all(|entry| entry.id < cursor));
    }

    #[test]
    fn timestamp_includes_fractional_seconds_and_legacy_values_remain_strings() {
        let current = super::timestamp();
        let (seconds, fraction) = current
            .split_once('.')
            .expect("timestamp should be precise");
        assert!(seconds.parse::<u64>().is_ok());
        assert_eq!(fraction.len(), 9);
        assert!(fraction.parse::<u32>().is_ok());
        let legacy = "1760000000";
        assert_eq!(
            legacy
                .parse::<u64>()
                .expect("legacy timestamp should parse"),
            1_760_000_000
        );
    }

    #[test]
    fn registration_lifecycle_preserves_identity_without_deleting_external_files() {
        let database = initialize(Path::new(":memory:")).expect("database should initialize");
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/packwiz/valid");
        let path = fixture.to_string_lossy().into_owned();
        let registered = super::register_modpack_inner(&database, path.clone(), None)
            .expect("valid fixture should register");
        assert_eq!(registered.application.theme.as_deref(), Some("cyan"));
        let duplicate = super::register_modpack_inner(&database, path.clone(), None)
            .expect_err("equivalent project should be rejected");
        assert_eq!(duplicate.code, "duplicate_modpack");
        let disconnected = super::set_lifecycle(
            &database,
            registered.id.clone(),
            crate::domain::ModpackLifecycle::Disconnected,
        )
        .expect("project should disconnect");
        assert_eq!(
            disconnected.application.lifecycle,
            crate::domain::ModpackLifecycle::Disconnected
        );
        let reconnected = super::reconnect_modpack_inner(&database, registered.id.clone(), path)
            .expect("valid fixture should reconnect");
        assert_eq!(reconnected.id, registered.id);
        assert!(fixture.join("pack.toml").is_file());
    }

    #[test]
    fn refresh_persists_current_inventory_and_activity() {
        let database = initialize(Path::new(":memory:")).expect("database should initialize");
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/packwiz/mixed");
        let path = fixture.to_string_lossy().into_owned();
        let registered = super::register_modpack_inner(&database, path.clone(), None)
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
                "SELECT COUNT(*) FROM inventory_observations WHERE modpack_id = ?1",
                [&registered.id],
                |row| row.get(0),
            )
            .unwrap();
        let activity_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM modpack_activity WHERE modpack_id = ?1 AND event_type = 'refresh_succeeded'",
                [&registered.id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(observation_count, 1);
        assert_eq!(activity_count, 1);
    }

    #[test]
    fn reads_snapshot_created_activity() {
        let database = initialize(Path::new(":memory:")).expect("database should initialize");
        let connection = database
            .0
            .lock()
            .expect("database lock should be available");
        connection
            .execute(
                "INSERT INTO modpacks (id, canonical_path, application_json, packwiz_json, validation_json, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
                params!["modpack-test", ".", "{}", "{}", "[]", "2026-09-06T00:00:00Z"],
            )
            .expect("test project should be inserted");
        connection
            .execute(
                "INSERT INTO modpack_activity (modpack_id, event_type, occurred_at, message) VALUES (?1, ?2, ?3, ?4)",
                params!["modpack-test", "snapshot_created", "2026-09-06T00:00:00Z", "Snapshot created"],
            )
            .expect("snapshot activity should be inserted");

        let activity = super::read_activity(&connection, "modpack-test")
            .expect("snapshot activity should be readable");

        assert_eq!(activity.len(), 1);
        assert_eq!(
            activity[0].event_type,
            crate::domain::ActivityEventType::SnapshotCreated
        );
    }

    #[test]
    fn failed_refresh_marks_last_observation_stale_without_replacing_it() {
        let database = initialize(Path::new(":memory:")).expect("database should initialize");
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/packwiz/mixed");
        let path = fixture.to_string_lossy().into_owned();
        let registered = super::register_modpack_inner(&database, path.clone(), None)
            .expect("valid fixture should register");
        let preview = crate::domain::validation::preview(&path).expect("fixture should preview");
        let connection = database.0.lock().unwrap();
        super::refresh_record(&connection, &registered.id, &preview).unwrap();
        connection
            .execute(
                "UPDATE modpacks SET canonical_path = ?1 WHERE id = ?2",
                ["C:/missing-modpack", registered.id.as_str()],
            )
            .unwrap();
        drop(connection);

        let error = super::refresh_modpack_inner(&database, &registered.id).unwrap_err();
        assert_eq!(error.code, "project_unavailable");
        let connection = database.0.lock().unwrap();
        let freshness: String = connection
            .query_row(
                "SELECT freshness FROM inventory_observations WHERE modpack_id = ?1",
                [&registered.id],
                |row| row.get(0),
            )
            .unwrap();
        let failures: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM modpack_activity WHERE modpack_id = ?1 AND event_type = 'refresh_failed'",
                [&registered.id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(freshness, "stale");
        assert_eq!(failures, 1);
    }

    #[test]
    fn cache_round_trip_preserves_exact_association() {
        let database = initialize(Path::new(":memory:")).expect("database should initialize");
        let association = crate::domain::ProviderMatchEvidence {
            confidence: crate::domain::ProviderMatchConfidence::Exact,
            project: Some(crate::domain::ModrinthProjectIdentity {
                modpack_id: "project-id".into(),
                slug: Some("example".into()),
                title: None,
            }),
            version: Some(crate::domain::ModrinthVersionIdentity {
                version_id: "version-id".into(),
                version_number: Some("1.2.3".into()),
                game_versions: vec![],
                loaders: vec![],
            }),
            association: Some(crate::domain::VersionAssociationEvidence {
                local_version: crate::domain::Evidence::Observed("1.2.2".into()),
                provider_version: Some("1.2.3".into()),
                matched_exactly: true,
                details: vec!["fixture".into()],
            }),
            candidates: vec![],
            reason: "exact fixture association".into(),
        };
        let key = crate::domain::ChangelogCacheKey {
            provider: "modrinth".into(),
            endpoint: "versions".into(),
            request_shape: "changelog".into(),
            project_id: "project-id".into(),
            version_id: "version-id".into(),
            game_version: None,
            loader: None,
            requested_fields: vec!["changelog".into()],
        };
        super::cache_changelog_response(&database, &key, r#"{"changelog":"fixed"}"#, &association)
            .unwrap();
        let cached = super::cached_changelog_response(&database, &key)
            .unwrap()
            .unwrap();
        assert_eq!(cached.0, r#"{"changelog":"fixed"}"#);
        assert_eq!(cached.1, association);
        super::cache_changelog_response_with_metadata(
            &database,
            &key,
            r#"[{"id":"version-id","version_number":"1.2.3","changelog":"fixed"}]"#,
            &association,
            7,
            false,
        )
        .unwrap();
        let metadata = super::cached_changelog_response_for_local_version_with_freshness(
            &database,
            "project-id",
            "1.2.2",
        )
        .unwrap()
        .unwrap();
        assert!(!metadata.2);
        assert!(!metadata.3);
        assert_eq!(metadata.4, 7);
        database
            .0
            .lock()
            .unwrap()
            .execute(
                "UPDATE changelog_cache SET retrieved_at = '0' WHERE cache_key_json = ?1",
                [&serde_json::to_string(&key).unwrap()],
            )
            .unwrap();
        let stale = super::cached_changelog_response_for_local_version_with_freshness(
            &database,
            "project-id",
            "1.2.2",
        )
        .unwrap()
        .unwrap();
        assert!(stale.2);
        assert!(!stale.3);
        assert!(super::cached_changelog_response_for_local_version(
            &database,
            "project-id",
            "1.2.2"
        )
        .unwrap()
        .is_some());
        assert!(super::cached_changelog_response_for_local_version(
            &database,
            "project-id",
            "9.9.9"
        )
        .unwrap()
        .is_none());
        let mut mismatched_key = key.clone();
        mismatched_key.requested_fields = vec!["title".into()];
        assert!(super::cached_changelog_response(&database, &mismatched_key)
            .unwrap()
            .is_none());
    }

    #[test]
    fn release_capture_round_trips_without_current_tree_reads() {
        let database = initialize(Path::new(":memory:")).expect("database should initialize");
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/packwiz/valid");
        let path = fixture.to_string_lossy().into_owned();
        let registered = super::register_modpack_inner(&database, path.clone(), None)
            .expect("valid fixture should register");
        let capture = crate::domain::capture::capture(
            &fixture,
            crate::domain::ReleaseEligibility {
                eligible: true,
                provisional: false,
                source: Some(crate::domain::ReleaseEligibilitySource::ValidatedCurrentState),
                validation: Vec::new(),
                diagnostic: None,
            },
            None,
            Vec::new(),
        )
        .expect("release capture should succeed");
        let release = crate::domain::ReleaseRecord {
            id: "release-test".into(),
            modpack_id: registered.id.clone(),
            metadata: crate::domain::ReleaseMetadata {
                name: "Test release".into(),
                version: Some("1".into()),
                description: None,
                notes: Some("captured".into()),
                publication_status: crate::domain::ReleasePublicationStatus::Draft,
            },
            capture,
            created_at: "2026-09-07T00:00:00Z".into(),
            updated_at: "2026-09-07T00:00:00Z".into(),
        };
        super::persist_release(&database, &release).expect("release should persist");
        let loaded = super::load_release(&database, &release.id).expect("release should reload");
        assert_eq!(
            loaded.capture.capture_fingerprint,
            release.capture.capture_fingerprint
        );
        assert_eq!(
            super::list_releases(&database, &registered.id)
                .unwrap()
                .len(),
            1
        );
    }
}
