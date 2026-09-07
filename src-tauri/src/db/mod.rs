use crate::domain::{
    inventory, validation, ActivityRecord, ApplicationModpackMetadata, CommandError,
    DiscoveryResult, InventoryEntry, ModpackLifecycle, ModpackOverview, ModpackRecord,
    RegistrationPreview, SnapshotCandidateRecord, SnapshotDecision, SnapshotDecisionRecord,
    SnapshotLifecycle, SnapshotNoteRecord, SnapshotNoteScope, SnapshotRecheckRecord,
    SnapshotRecord, UpdateCandidate,
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
    let lifecycle = if matches!(result.outcome, crate::domain::DiscoveryOutcomeKind::Normal) {
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
    transaction.execute("INSERT INTO snapshots (id, modpack_id, lifecycle, outcome, result_json, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)", params![snapshot_id, project_id, lifecycle_json.trim_matches('"'), outcome.trim_matches('"'), result_json, observed_at]).map_err(|error| CommandError::new("database_write_failed", "Snapshot could not be saved").with_details(error.to_string()))?;
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
            "SELECT id, modpack_id, predecessor_id, lifecycle, outcome, label, result_json, created_at, updated_at, closed_at FROM snapshots WHERE id = ?1",
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
            "INSERT INTO operation_attempts (id, modpack_id, snapshot_id, predecessor_id, kind, status, outcome, recovery_json, process_json, before_fingerprint_json, after_fingerprint_json, verification_json, error_json, created_at, finished_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                attempt.id,
                attempt.modpack_id,
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
            "INSERT INTO operation_attempts (id, modpack_id, snapshot_id, predecessor_id, kind, status, outcome, recovery_json, process_json, before_fingerprint_json, after_fingerprint_json, verification_json, error_json, created_at, finished_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                attempt.id,
                attempt.modpack_id,
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
            "INSERT INTO operation_candidates (operation_id, candidate_id, entry_id, decision, observed_json) VALUES (?1, ?2, ?3, 'selected', ?4)",
            params![attempt.id, candidate_id, entry_id, observed_json],
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
        .prepare("SELECT id, modpack_id, snapshot_id, predecessor_id, kind, status, outcome, recovery_json, process_json, before_fingerprint_json, after_fingerprint_json, verification_json, error_json, created_at, finished_at FROM operation_attempts WHERE modpack_id = ?1 ORDER BY created_at DESC")
        .map_err(|error| CommandError::new("database_read_failed", "Operations could not be read").with_details(error.to_string()))?;
    let result = statement
        .query_map([project_id], |row| {
            let kind: String = row.get(4)?;
            let status: String = row.get(5)?;
            let outcome: Option<String> = row.get(6)?;
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
                snapshot_id: row.get(2)?,
                predecessor_id: row.get(3)?,
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
                recovery: serde_json::from_value(parse(7)?.ok_or(rusqlite::Error::InvalidQuery)?)
                    .map_err(|_| rusqlite::Error::InvalidQuery)?,
                process: parse(8)?
                    .map(|value| {
                        serde_json::from_value(value).map_err(|_| rusqlite::Error::InvalidQuery)
                    })
                    .transpose()?,
                before_fingerprint: parse(9)?
                    .map(|value| {
                        serde_json::from_value(value).map_err(|_| rusqlite::Error::InvalidQuery)
                    })
                    .transpose()?,
                after_fingerprint: parse(10)?
                    .map(|value| {
                        serde_json::from_value(value).map_err(|_| rusqlite::Error::InvalidQuery)
                    })
                    .transpose()?,
                verification: parse(11)?
                    .map(|value| {
                        serde_json::from_value(value).map_err(|_| rusqlite::Error::InvalidQuery)
                    })
                    .transpose()?,
                error: parse(12)?
                    .map(|value| {
                        serde_json::from_value(value).map_err(|_| rusqlite::Error::InvalidQuery)
                    })
                    .transpose()?,
                created_at: row.get(13)?,
                finished_at: row.get(14)?,
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
    connection.execute_batch("PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL;")?;
    connection.execute_batch(include_str!("schema.sql"))?;
    Ok(Database(Mutex::new(connection)))
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
            "INSERT INTO changelog_artifacts (id, modpack_id, snapshot_id, attempt_id, status, introduction, content, entries_json, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)",
            params![artifact.id, artifact.modpack_id, artifact.snapshot_id, artifact.attempt_id, status, artifact.introduction, artifact.content, serialize(&artifact.entries, "changelog entries")?, now],
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
            "INSERT INTO changelog_revisions (id, artifact_id, prior_revision_id, content, introduction, created_at, is_current) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1)",
            params![id, request.artifact_id, request.prior_revision_id, request.content, request.introduction, now],
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
        created_at: now,
        is_current: true,
    })
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
    connection.query_row("SELECT id, modpack_id, snapshot_id, attempt_id, status, introduction, content, entries_json, created_at, updated_at FROM changelog_artifacts WHERE id = ?1", [id], |row| {
        let status: String = row.get(4)?;
        let entries: String = row.get(7)?;
        Ok(crate::domain::ChangelogArtifact {
            id: row.get(0)?, modpack_id: row.get(1)?, snapshot_id: row.get(2)?, attempt_id: row.get(3)?,
            status: serde_json::from_value(serde_json::Value::String(status)).map_err(|_| rusqlite::Error::InvalidQuery)?,
            introduction: row.get(5)?, content: row.get(6)?,
            entries: serde_json::from_str(&entries).map_err(|_| rusqlite::Error::InvalidQuery)?,
            created_at: row.get(8)?, updated_at: row.get(9)?,
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
    let mut statement = connection.prepare("SELECT id, artifact_id, prior_revision_id, content, introduction, created_at, is_current FROM changelog_revisions WHERE artifact_id = ?1 ORDER BY created_at DESC").map_err(|error| CommandError::new("database_read_failed", "Revision history could not be read").with_details(error.to_string()))?;
    let rows = statement
        .query_map([artifact_id], |row| {
            Ok(crate::domain::ChangelogRevision {
                id: row.get(0)?,
                artifact_id: row.get(1)?,
                prior_revision_id: row.get(2)?,
                content: row.get(3)?,
                introduction: row.get(4)?,
                created_at: row.get(5)?,
                is_current: row.get::<_, i64>(6)? != 0,
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
    let connection = database
        .0
        .lock()
        .map_err(|_| CommandError::new("database_unavailable", "Database is unavailable"))?;
    let key_json = serialize(key, "cache key")?;
    let association_json = serialize(association, "cache association")?;
    use sha2::{Digest, Sha256};
    let cache_id = format!("cache-{:x}", Sha256::digest(key_json.as_bytes()));
    connection.execute("INSERT OR REPLACE INTO changelog_cache (id, cache_key_json, raw_response, association_json, retrieved_at, request_context, response_context) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)", params![cache_id, key_json, response, association_json, timestamp(), "exact changelog request", "modrinth version response"])
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
    let connection = database
        .0
        .lock()
        .map_err(|_| CommandError::new("database_unavailable", "Database is unavailable"))?;
    let mut statement = connection
        .prepare("SELECT cache_key_json, raw_response, association_json FROM changelog_cache WHERE json_extract(cache_key_json, '$.provider') = 'modrinth' AND json_extract(cache_key_json, '$.project_id') = ?1")
        .map_err(|error| {
            CommandError::new("database_read_failed", "Provider cache could not be read")
                .with_details(error.to_string())
        })?;
    let rows = statement
        .query_map([project_id], |row| {
            let key_json: String = row.get(0)?;
            let response: String = row.get(1)?;
            let association_json: String = row.get(2)?;
            let key: crate::domain::ChangelogCacheKey =
                serde_json::from_str(&key_json).map_err(|_| rusqlite::Error::InvalidQuery)?;
            let association: crate::domain::ProviderMatchEvidence =
                serde_json::from_str(&association_json)
                    .map_err(|_| rusqlite::Error::InvalidQuery)?;
            Ok((key, response, association))
        })
        .map_err(|error| {
            CommandError::new("database_read_failed", "Provider cache could not be read")
                .with_details(error.to_string())
        })?;
    for row in rows {
        let (_key, response, association) = row.map_err(|error| {
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
            return Ok(Some((response, association)));
        }
    }
    Ok(None)
}

fn timestamp() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs().to_string())
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
    Ok(SnapshotRecord {
        id: row.get(0)?,
        modpack_id: row.get(1)?,
        predecessor_id: row.get(2)?,
        lifecycle: serde_json::from_value(serde_json::Value::String(lifecycle))
            .map_err(|_| rusqlite::Error::InvalidQuery)?,
        outcome: serde_json::from_value(serde_json::Value::String(outcome))
            .map_err(|_| rusqlite::Error::InvalidQuery)?,
        label: row.get(5)?,
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
        closed_at: row.get(9)?,
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
        .prepare("SELECT id, modpack_id, predecessor_id, lifecycle, outcome, label, result_json, created_at, updated_at, closed_at FROM snapshots WHERE modpack_id = ?1 ORDER BY created_at DESC")
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
            "SELECT id, modpack_id, predecessor_id, lifecycle, outcome, label, result_json, created_at, updated_at, closed_at FROM snapshots WHERE id = ?1",
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
        let snapshot = connection.query_row("SELECT id, modpack_id, predecessor_id, lifecycle, outcome, label, result_json, created_at, updated_at, closed_at FROM snapshots WHERE id = ?1", [&id], row_to_snapshot).map_err(|error| CommandError::new("snapshot_not_found", "Snapshot is not available").with_details(error.to_string()))?;
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
    let snapshot = connection.query_row("SELECT id, modpack_id, predecessor_id, lifecycle, outcome, label, result_json, created_at, updated_at, closed_at FROM snapshots WHERE id = ?1", [&id], row_to_snapshot).map_err(|error| CommandError::new("snapshot_not_found", "Snapshot is not available").with_details(error.to_string()))?;
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
        .query_row("SELECT id, modpack_id, predecessor_id, lifecycle, outcome, label, result_json, created_at, updated_at, closed_at FROM snapshots WHERE id = ?1", [&retry_id], row_to_snapshot)
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
    let snapshot = connection.query_row("SELECT id, modpack_id, predecessor_id, lifecycle, outcome, label, result_json, created_at, updated_at, closed_at FROM snapshots WHERE id = ?1", [id], row_to_snapshot).map_err(|error| CommandError::new("snapshot_not_found", "Snapshot is not available").with_details(error.to_string()))?;
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
        let registered = super::register_modpack_inner(&database, path.clone(), None)
            .expect("valid fixture should register");
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
}
