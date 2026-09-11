use super::{timestamp, unique_id, Database};
use crate::domain::{
    action_is_supported, preview_fingerprint, validate_confirmation, CleanupCandidate, CleanupPlan,
    CleanupRequest, CleanupResult, CleanupScope, CommandError, ImpactPreview, LifecycleAction,
    LifecycleOperationStatus, LifecycleRecordType, LifecycleRequest, LifecycleResult,
    OrphanClassification, ProtectedReference, StorageCategory, StorageReport,
    UnavailableDestination,
};
use rusqlite::{params, Connection};
use tauri::State;

const EXTERNAL_BOUNDARIES: [&str; 3] = [
    "Registered Packwiz project directories are never deleted",
    "Git metadata and history are never deleted",
    "User export destination files are never deleted",
];

#[tauri::command]
pub fn preview_lifecycle_action(
    state: State<'_, Database>,
    target_type: LifecycleRecordType,
    target_id: String,
    action: LifecycleAction,
) -> Result<ImpactPreview, CommandError> {
    let connection = state
        .0
        .lock()
        .map_err(|_| CommandError::new("database_unavailable", "Database is unavailable"))?;
    preview_lifecycle_action_inner(&connection, &target_type, &target_id, &action, None)
}

#[tauri::command]
pub fn apply_lifecycle_action(
    state: State<'_, Database>,
    request: LifecycleRequest,
) -> Result<LifecycleResult, CommandError> {
    let connection = state
        .0
        .lock()
        .map_err(|_| CommandError::new("database_unavailable", "Database is unavailable"))?;
    apply_lifecycle_action_inner(&connection, &request)
}

fn apply_lifecycle_action_inner(
    connection: &Connection,
    request: &LifecycleRequest,
) -> Result<LifecycleResult, CommandError> {
    if !action_is_supported(&request.target_type, &request.action) {
        return Err(CommandError::new(
            "unsupported_lifecycle_action",
            "This lifecycle action is not supported for the selected record",
        ));
    }
    let preview = preview_lifecycle_action_inner(
        &connection,
        &request.target_type,
        &request.target_id,
        &request.action,
        request.scope.as_ref(),
    )?;
    if preview.fingerprint != request.preview_fingerprint {
        return Err(CommandError::new(
            "stale_preview",
            "The record changed after the impact preview; preview it again",
        ));
    }
    if !preview.eligible {
        return Err(CommandError::new(
            "lifecycle_blocked",
            preview.blocked_reasons.join("; "),
        ));
    }
    if matches!(request.action, LifecycleAction::PermanentDelete) {
        validate_confirmation(&request.target_id, request.confirmation.as_deref())?;
    }
    let operation_id = unique_id("lifecycle");
    let now = timestamp();
    let transaction = connection.unchecked_transaction().map_err(|error| {
        CommandError::new(
            "database_write_failed",
            "Lifecycle transaction could not start",
        )
        .with_details(error.to_string())
    })?;
    let active: bool = transaction
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM lifecycle_operations WHERE record_type = ?1 AND record_id = ?2 AND status IN ('planned', 'running'))",
            params![record_type_value(&request.target_type), request.target_id],
            |row| row.get(0),
        )
        .map_err(|error| {
            CommandError::new("database_read_failed", "Lifecycle state could not be read")
                .with_details(error.to_string())
        })?;
    if active {
        return Err(CommandError::new(
            "lifecycle_operation_active",
            "Another lifecycle operation is already active for this record",
        ));
    }
    transaction
        .execute(
            "INSERT INTO lifecycle_operations (id, record_type, record_id, action, scope, preview_fingerprint, status, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'running', ?7)",
            params![
                operation_id,
                record_type_value(&request.target_type),
                request.target_id,
                action_value(&request.action),
                request.scope.as_ref().map(scope_value),
                request.preview_fingerprint,
                now
            ],
        )
        .map_err(|error| {
            CommandError::new("database_write_failed", "Lifecycle operation could not start")
                .with_details(error.to_string())
        })?;
    let (tombstoned, physically_deleted) = apply_action(&transaction, &request, &preview, &now)?;
    transaction
        .execute(
            "UPDATE lifecycle_operations SET status = 'succeeded', finished_at = ?1 WHERE id = ?2",
            params![timestamp(), operation_id],
        )
        .map_err(|error| {
            CommandError::new(
                "database_write_failed",
                "Lifecycle operation could not finish",
            )
            .with_details(error.to_string())
        })?;
    transaction.commit().map_err(|error| {
        CommandError::new(
            "database_write_failed",
            "Lifecycle operation could not commit",
        )
        .with_details(error.to_string())
    })?;
    Ok(LifecycleResult {
        target_type: request.target_type.clone(),
        target_id: request.target_id.clone(),
        action: request.action.clone(),
        status: LifecycleOperationStatus::Succeeded,
        tombstoned,
        physically_deleted,
        retained_references: preview.direct_references,
        message: if physically_deleted {
            "Application-owned record deleted; external files were not touched".into()
        } else if tombstoned {
            "Application record was retained as historical evidence".into()
        } else {
            "Application relationship was detached without deleting evidence".into()
        },
    })
}

#[tauri::command]
pub fn preview_cleanup(
    state: State<'_, Database>,
    scope: CleanupScope,
) -> Result<CleanupPlan, CommandError> {
    let connection = state
        .0
        .lock()
        .map_err(|_| CommandError::new("database_unavailable", "Database is unavailable"))?;
    cleanup_plan_inner(&connection, &scope)
}

#[tauri::command]
pub fn execute_cleanup(
    state: State<'_, Database>,
    request: CleanupRequest,
) -> Result<CleanupResult, CommandError> {
    let connection = state
        .0
        .lock()
        .map_err(|_| CommandError::new("database_unavailable", "Database is unavailable"))?;
    execute_cleanup_inner(&connection, &request)
}

fn execute_cleanup_inner(
    connection: &Connection,
    request: &CleanupRequest,
) -> Result<CleanupResult, CommandError> {
    let plan = cleanup_plan_inner(&connection, &request.scope)?;
    if plan.fingerprint != request.preview_fingerprint {
        return Err(CommandError::new(
            "stale_preview",
            "Cleanup candidates changed after the preview; preview cleanup again",
        ));
    }
    let operation_id = unique_id("cleanup");
    let now = timestamp();
    let transaction = connection.unchecked_transaction().map_err(|error| {
        CommandError::new(
            "database_write_failed",
            "Cleanup transaction could not start",
        )
        .with_details(error.to_string())
    })?;
    let active: bool = transaction
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM cleanup_operations WHERE scope = ?1 AND status IN ('planned', 'running'))",
            [scope_value(&request.scope)],
            |row| row.get(0),
        )
        .map_err(|error| {
            CommandError::new("database_read_failed", "Cleanup state could not be read")
                .with_details(error.to_string())
        })?;
    if active {
        return Err(CommandError::new(
            "cleanup_operation_active",
            "Another cleanup operation is already active for this scope",
        ));
    }
    transaction
        .execute(
            "INSERT INTO cleanup_operations (id, scope, preview_fingerprint, status, created_at) VALUES (?1, ?2, ?3, 'running', ?4)",
            params![operation_id, scope_value(&request.scope), request.preview_fingerprint, now],
        )
        .map_err(|error| {
            CommandError::new("database_write_failed", "Cleanup operation could not start")
                .with_details(error.to_string())
        })?;
    let mut removed_count = 0_u64;
    let mut removed_bytes = 0_u64;
    for candidate in plan
        .candidates
        .iter()
        .filter(|candidate| !candidate.protected)
    {
        let changed = match candidate.record_type {
            LifecycleRecordType::ProviderCache => transaction.execute(
                "DELETE FROM changelog_cache WHERE id = ?1",
                [&candidate.record_id],
            ),
            LifecycleRecordType::ChangelogExport => transaction.execute(
                "DELETE FROM changelog_exports WHERE id = ?1",
                [&candidate.record_id],
            ),
            _ => Ok(0),
        }
        .map_err(|error| {
            CommandError::new(
                "database_write_failed",
                "Cleanup candidate could not be removed",
            )
            .with_details(error.to_string())
        })?;
        if changed > 0 {
            removed_count += 1;
            removed_bytes += candidate.bytes;
        }
    }
    transaction
        .execute(
            "UPDATE cleanup_operations SET status = 'succeeded', removed_count = ?1, protected_count = ?2, removed_bytes = ?3, finished_at = ?4 WHERE id = ?5",
            params![removed_count, plan.protected_count, removed_bytes, timestamp(), operation_id],
        )
        .map_err(|error| {
            CommandError::new("database_write_failed", "Cleanup operation could not finish")
                .with_details(error.to_string())
        })?;
    transaction.commit().map_err(|error| {
        CommandError::new(
            "database_write_failed",
            "Cleanup operation could not commit",
        )
        .with_details(error.to_string())
    })?;
    Ok(CleanupResult {
        scope: request.scope.clone(),
        status: LifecycleOperationStatus::Succeeded,
        removed_count,
        protected_count: plan.protected_count,
        removed_bytes,
        partial_failures: Vec::new(),
    })
}

#[tauri::command]
pub fn get_storage_report(state: State<'_, Database>) -> Result<StorageReport, CommandError> {
    let connection = state
        .0
        .lock()
        .map_err(|_| CommandError::new("database_unavailable", "Database is unavailable"))?;
    storage_report_inner(&connection)
}

fn preview_lifecycle_action_inner(
    connection: &Connection,
    target_type: &LifecycleRecordType,
    target_id: &str,
    action: &LifecycleAction,
    scope: Option<&CleanupScope>,
) -> Result<ImpactPreview, CommandError> {
    if !record_exists(connection, target_type, target_id)? {
        return Err(CommandError::new(
            "record_not_found",
            "The selected application record is not available",
        ));
    }
    let direct_references = references(connection, target_type, target_id)?;
    let active_operation = has_active_operation(connection, target_type, target_id)?;
    let orphan = orphan_classification(connection, target_type, target_id)?;
    let unavailable_destination = unavailable_destination(connection, target_type, target_id)?;
    let mut blocked_reasons = Vec::new();
    if active_operation {
        blocked_reasons.push("An active operation protects this record".into());
    }
    if orphan.is_some() {
        blocked_reasons.push("The record has a missing or tombstoned application parent".into());
    }
    if matches!(action, LifecycleAction::PermanentDelete)
        && !matches!(
            target_type,
            LifecycleRecordType::ProviderCache | LifecycleRecordType::ChangelogExport
        )
    {
        blocked_reasons
            .push("Permanent deletion is restricted to cache and obsolete export records".into());
    }
    if matches!(action, LifecycleAction::Detach) && direct_references.is_empty() {
        blocked_reasons.push("The record has no detachable application relationship".into());
    }
    let removable_records = if matches!(action, LifecycleAction::PermanentDelete) {
        vec![target_id.to_string()]
    } else {
        Vec::new()
    };
    let fingerprint = preview_fingerprint(
        target_type,
        target_id,
        action,
        scope,
        &direct_references,
        &removable_records,
        active_operation,
    );
    Ok(ImpactPreview {
        target_type: target_type.clone(),
        target_id: target_id.into(),
        action: action.clone(),
        scope: scope.cloned(),
        fingerprint,
        direct_references,
        retained_evidence: retained_evidence(target_type),
        removable_records,
        external_boundaries: EXTERNAL_BOUNDARIES
            .iter()
            .map(|item| (*item).into())
            .collect(),
        orphan,
        unavailable_destination,
        active_operation,
        eligible: blocked_reasons.is_empty(),
        requires_confirmation: matches!(action, LifecycleAction::PermanentDelete),
        blocked_reasons,
    })
}

fn apply_action(
    transaction: &rusqlite::Transaction<'_>,
    request: &LifecycleRequest,
    preview: &ImpactPreview,
    now: &str,
) -> Result<(bool, bool), CommandError> {
    match request.action {
        LifecycleAction::Archive => {
            transaction
                .execute(
                    "INSERT INTO lifecycle_tombstones (record_type, record_id, state, reason, created_at, updated_at) VALUES (?1, ?2, 'archived', 'Archived by user', ?3, ?3) ON CONFLICT(record_type, record_id) DO UPDATE SET state = 'archived', updated_at = excluded.updated_at",
                    params![record_type_value(&request.target_type), request.target_id, now],
                )
                .map_err(db_write("Record could not be archived"))?;
            Ok((true, false))
        }
        LifecycleAction::Restore => {
            transaction
                .execute(
                    "DELETE FROM lifecycle_tombstones WHERE record_type = ?1 AND record_id = ?2",
                    params![record_type_value(&request.target_type), request.target_id],
                )
                .map_err(db_write("Record could not be restored"))?;
            Ok((false, false))
        }
        LifecycleAction::Detach => {
            detach_relationships(transaction, &request.target_type, &request.target_id)?;
            Ok((false, false))
        }
        LifecycleAction::PermanentDelete => {
            if !preview
                .removable_records
                .iter()
                .any(|id| id == &request.target_id)
            {
                return Err(CommandError::new(
                    "delete_not_allowlisted",
                    "The selected record is not in the physical-delete allowlist",
                ));
            }
            let query = match request.target_type {
                LifecycleRecordType::ProviderCache => "DELETE FROM changelog_cache WHERE id = ?1",
                LifecycleRecordType::ChangelogExport => {
                    "DELETE FROM changelog_exports WHERE id = ?1 AND status IN ('failed', 'unavailable')"
                }
                _ => unreachable!("preview blocks non-allowlisted physical deletion"),
            };
            transaction
                .execute(query, [&request.target_id])
                .map_err(db_write("Application record could not be deleted"))?;
            Ok((false, true))
        }
    }
}

fn detach_relationships(
    transaction: &rusqlite::Transaction<'_>,
    target_type: &LifecycleRecordType,
    target_id: &str,
) -> Result<(), CommandError> {
    let statements: &[&str] = match target_type {
        LifecycleRecordType::Snapshot => &[
            "DELETE FROM release_snapshots WHERE snapshot_id = ?1",
            "UPDATE release_workspaces SET source_snapshot_id = NULL WHERE source_snapshot_id = ?1",
            "UPDATE changelog_artifacts SET snapshot_id = NULL WHERE snapshot_id = ?1",
        ],
        LifecycleRecordType::ReleaseWorkspace => &[
            "UPDATE changelog_artifacts SET release_workspace_id = NULL WHERE release_workspace_id = ?1",
            "UPDATE release_workspaces SET source_snapshot_id = NULL, baseline_release_id = NULL, final_changelog_revision_id = NULL WHERE id = ?1",
        ],
        LifecycleRecordType::Release => &[
            "DELETE FROM release_snapshots WHERE release_id = ?1",
            "DELETE FROM release_changelog_artifacts WHERE release_id = ?1",
            "UPDATE release_workspaces SET baseline_release_id = NULL WHERE baseline_release_id = ?1",
        ],
        LifecycleRecordType::ChangelogArtifact => &[
            "DELETE FROM release_changelog_artifacts WHERE artifact_id = ?1",
            "UPDATE changelog_artifacts SET snapshot_id = NULL, release_workspace_id = NULL WHERE id = ?1",
        ],
        LifecycleRecordType::ChangelogRevision => &[
            "UPDATE changelog_exports SET revision_id = NULL WHERE revision_id = ?1",
            "UPDATE release_workspaces SET final_changelog_revision_id = NULL WHERE final_changelog_revision_id = ?1",
        ],
        LifecycleRecordType::ChangelogExport => &[
            "UPDATE changelog_exports SET revision_id = NULL WHERE id = ?1",
        ],
        _ => &[],
    };
    for statement in statements {
        transaction
            .execute(statement, [target_id])
            .map_err(db_write("Application relationship could not be detached"))?;
    }
    Ok(())
}

fn cleanup_plan_inner(
    connection: &Connection,
    scope: &CleanupScope,
) -> Result<CleanupPlan, CommandError> {
    let mut candidates = Vec::new();
    if matches!(
        scope,
        CleanupScope::ProviderCache | CleanupScope::ProviderCacheAndObsoleteExports
    ) {
        let mut statement = connection
            .prepare("SELECT id, length(raw_response), EXISTS(SELECT 1 FROM changelog_attempts WHERE status IN ('running', 'pending')) OR EXISTS(SELECT 1 FROM changelog_artifacts WHERE entries_json LIKE '%' || changelog_cache.id || '%' OR content LIKE '%' || changelog_cache.id || '%') FROM changelog_cache")
            .map_err(db_read("Provider cache could not be inspected"))?;
        let rows = statement
            .query_map([], |row| {
                let id: String = row.get(0)?;
                let bytes: i64 = row.get::<_, Option<i64>>(1)?.unwrap_or_default();
                let protected: bool = row.get(2)?;
                Ok(CleanupCandidate {
                    record_type: LifecycleRecordType::ProviderCache,
                    record_id: id,
                    bytes: bytes.max(0) as u64,
                    protected,
                    reason: if protected {
                        "An active changelog generation protects provider cache data".into()
                    } else {
                        "Unreferenced provider cache row".into()
                    },
                })
            })
            .map_err(db_read("Provider cache could not be inspected"))?;
        candidates.extend(rows.collect::<Result<Vec<_>, _>>().map_err(|error| {
            CommandError::new("database_read_failed", "Provider cache record is invalid")
                .with_details(error.to_string())
        })?);
    }
    if matches!(
        scope,
        CleanupScope::ObsoleteExports | CleanupScope::ProviderCacheAndObsoleteExports
    ) {
        let mut statement = connection
            .prepare("SELECT id, length(content), status FROM changelog_exports WHERE status IN ('failed', 'unavailable')")
            .map_err(db_read("Export history could not be inspected"))?;
        let rows = statement
            .query_map([], |row| {
                let status: String = row.get(2)?;
                Ok(CleanupCandidate {
                    record_type: LifecycleRecordType::ChangelogExport,
                    record_id: row.get(0)?,
                    bytes: row.get::<_, Option<i64>>(1)?.unwrap_or_default().max(0) as u64,
                    protected: false,
                    reason: format!("Obsolete export record with persisted status {status}"),
                })
            })
            .map_err(db_read("Export history could not be inspected"))?;
        candidates.extend(rows.collect::<Result<Vec<_>, _>>().map_err(|error| {
            CommandError::new("database_read_failed", "Export record is invalid")
                .with_details(error.to_string())
        })?);
    }
    let protected_count = candidates
        .iter()
        .filter(|candidate| candidate.protected)
        .count() as u64;
    let removable_bytes = candidates
        .iter()
        .filter(|candidate| !candidate.protected)
        .map(|candidate| candidate.bytes)
        .sum();
    let fingerprint = preview_fingerprint(
        &LifecycleRecordType::ProviderCache,
        "cleanup",
        &LifecycleAction::PermanentDelete,
        Some(scope),
        &[],
        &candidates
            .iter()
            .map(|candidate| candidate.record_id.clone())
            .collect::<Vec<_>>(),
        false,
    );
    Ok(CleanupPlan {
        scope: scope.clone(),
        fingerprint,
        candidates,
        protected_count,
        removable_bytes,
        blocked_reasons: Vec::new(),
    })
}

fn storage_report_inner(connection: &Connection) -> Result<StorageReport, CommandError> {
    let sqlite_bytes: i64 = connection
        .query_row(
            "SELECT page_count * page_size FROM pragma_page_count(), pragma_page_size()",
            [],
            |row| row.get(0),
        )
        .map_err(db_read("Database size could not be measured"))?;
    let cache_bytes = sum_length(
        connection,
        "SELECT COALESCE(SUM(length(raw_response)), 0) FROM changelog_cache",
    )?;
    let artifact_bytes = sum_length(
        connection,
        "SELECT COALESCE(SUM(length(content) + length(entries_json)), 0) FROM changelog_artifacts",
    )?;
    let export_bytes = sum_length(
        connection,
        "SELECT COALESCE(SUM(length(content)), 0) FROM changelog_exports",
    )?;
    let removable_cache = sum_length(
        connection,
        "SELECT COALESCE(SUM(length(raw_response)), 0) FROM changelog_cache WHERE NOT EXISTS (SELECT 1 FROM changelog_attempts WHERE status IN ('running', 'pending'))",
    )?;
    let removable_exports = sum_length(
        connection,
        "SELECT COALESCE(SUM(length(content)), 0) FROM changelog_exports WHERE status IN ('failed', 'unavailable')",
    )?;
    let categories = vec![
        StorageCategory {
            name: "sqlite_database".into(),
            bytes: sqlite_bytes.max(0) as u64,
            removable: false,
            protected: true,
            availability: "available".into(),
        },
        StorageCategory {
            name: "provider_cache".into(),
            bytes: cache_bytes,
            removable: true,
            protected: false,
            availability: "available".into(),
        },
        StorageCategory {
            name: "changelog_artifacts".into(),
            bytes: artifact_bytes,
            removable: false,
            protected: true,
            availability: "available".into(),
        },
        StorageCategory {
            name: "export_records".into(),
            bytes: export_bytes,
            removable: true,
            protected: false,
            availability: "available".into(),
        },
    ];
    Ok(StorageReport {
        application_owned_bytes: categories.iter().map(|category| category.bytes).sum(),
        removable_bytes: removable_cache + removable_exports,
        protected_bytes: categories
            .iter()
            .filter(|category| category.protected)
            .map(|category| category.bytes)
            .sum(),
        categories,
        unavailable_categories: vec![
            "External Packwiz project sizes are not measured".into(),
            "External Git history sizes are not measured".into(),
            "User export destination sizes are not measured".into(),
        ],
        external_boundaries: EXTERNAL_BOUNDARIES
            .iter()
            .map(|item| (*item).into())
            .collect(),
    })
}

fn record_exists(
    connection: &Connection,
    record_type: &LifecycleRecordType,
    record_id: &str,
) -> Result<bool, CommandError> {
    let table = match record_type {
        LifecycleRecordType::Modpack => "modpacks",
        LifecycleRecordType::Snapshot => "snapshots",
        LifecycleRecordType::Operation => "operation_attempts",
        LifecycleRecordType::ReleaseWorkspace => "release_workspaces",
        LifecycleRecordType::Release => "releases",
        LifecycleRecordType::ChangelogArtifact => "changelog_artifacts",
        LifecycleRecordType::ChangelogRevision => "changelog_revisions",
        LifecycleRecordType::ChangelogExport => "changelog_exports",
        LifecycleRecordType::ProviderCache => "changelog_cache",
    };
    connection
        .query_row(
            &format!("SELECT EXISTS(SELECT 1 FROM {table} WHERE id = ?1)"),
            [record_id],
            |row| row.get(0),
        )
        .map_err(db_read("Record existence could not be checked"))
}

fn references(
    connection: &Connection,
    record_type: &LifecycleRecordType,
    record_id: &str,
) -> Result<Vec<ProtectedReference>, CommandError> {
    let mut result = Vec::new();
    let queries: &[(&str, LifecycleRecordType, &str, &str)] = match record_type {
        LifecycleRecordType::Snapshot => &[
            (
                "SELECT release_id FROM release_snapshots WHERE snapshot_id = ?1",
                LifecycleRecordType::Release,
                "release_id",
                "release snapshot link",
            ),
            (
                "SELECT id FROM release_workspaces WHERE source_snapshot_id = ?1",
                LifecycleRecordType::ReleaseWorkspace,
                "id",
                "workspace baseline",
            ),
            (
                "SELECT id FROM changelog_artifacts WHERE snapshot_id = ?1",
                LifecycleRecordType::ChangelogArtifact,
                "id",
                "changelog source",
            ),
        ],
        LifecycleRecordType::ReleaseWorkspace => &[
            (
                "SELECT id FROM operation_attempts WHERE workspace_id = ?1",
                LifecycleRecordType::Operation,
                "id",
                "workspace operation",
            ),
            (
                "SELECT id FROM changelog_artifacts WHERE release_workspace_id = ?1",
                LifecycleRecordType::ChangelogArtifact,
                "id",
                "workspace changelog",
            ),
        ],
        LifecycleRecordType::Release => &[
            (
                "SELECT snapshot_id FROM release_snapshots WHERE release_id = ?1",
                LifecycleRecordType::Snapshot,
                "snapshot_id",
                "release snapshot",
            ),
            (
                "SELECT artifact_id FROM release_changelog_artifacts WHERE release_id = ?1",
                LifecycleRecordType::ChangelogArtifact,
                "artifact_id",
                "release changelog",
            ),
            (
                "SELECT id FROM release_workspaces WHERE baseline_release_id = ?1",
                LifecycleRecordType::ReleaseWorkspace,
                "id",
                "workspace baseline",
            ),
        ],
        LifecycleRecordType::ChangelogArtifact => &[
            (
                "SELECT revision_id FROM changelog_revisions WHERE artifact_id = ?1",
                LifecycleRecordType::ChangelogRevision,
                "revision_id",
                "artifact revision",
            ),
            (
                "SELECT release_id FROM release_changelog_artifacts WHERE artifact_id = ?1",
                LifecycleRecordType::Release,
                "release_id",
                "release changelog",
            ),
            (
                "SELECT id FROM changelog_exports WHERE artifact_id = ?1",
                LifecycleRecordType::ChangelogExport,
                "id",
                "export record",
            ),
        ],
        LifecycleRecordType::ChangelogRevision => &[
            (
                "SELECT id FROM changelog_exports WHERE revision_id = ?1",
                LifecycleRecordType::ChangelogExport,
                "id",
                "export revision",
            ),
            (
                "SELECT id FROM release_workspaces WHERE final_changelog_revision_id = ?1",
                LifecycleRecordType::ReleaseWorkspace,
                "id",
                "final workspace revision",
            ),
        ],
        LifecycleRecordType::ChangelogExport => &[],
        LifecycleRecordType::Modpack => &[
            (
                "SELECT id FROM snapshots WHERE modpack_id = ?1",
                LifecycleRecordType::Snapshot,
                "id",
                "modpack snapshot",
            ),
            (
                "SELECT id FROM releases WHERE modpack_id = ?1",
                LifecycleRecordType::Release,
                "id",
                "modpack release",
            ),
            (
                "SELECT id FROM release_workspaces WHERE modpack_id = ?1",
                LifecycleRecordType::ReleaseWorkspace,
                "id",
                "modpack workspace",
            ),
        ],
        LifecycleRecordType::Operation | LifecycleRecordType::ProviderCache => &[],
    };
    for (query, source_type, _, relationship) in queries {
        let mut statement = connection
            .prepare(query)
            .map_err(db_read("References could not be read"))?;
        let rows = statement
            .query_map([record_id], |row| row.get::<_, String>(0))
            .map_err(db_read("References could not be read"))?;
        for row in rows {
            result.push(ProtectedReference {
                source_type: source_type.clone(),
                source_id: row.map_err(db_read("Reference record is invalid"))?,
                relationship: (*relationship).into(),
                reason: "Referenced application history is retained".into(),
            });
        }
    }
    Ok(result)
}

fn orphan_classification(
    connection: &Connection,
    record_type: &LifecycleRecordType,
    record_id: &str,
) -> Result<Option<OrphanClassification>, CommandError> {
    let parent = match record_type {
        LifecycleRecordType::ChangelogRevision => Some((
            LifecycleRecordType::ChangelogArtifact,
            connection
                .query_row(
                    "SELECT artifact_id FROM changelog_revisions WHERE id = ?1",
                    [record_id],
                    |row| row.get::<_, String>(0),
                )
                .map_err(db_read("Revision parent could not be read"))?,
        )),
        LifecycleRecordType::ChangelogExport => Some((
            LifecycleRecordType::ChangelogArtifact,
            connection
                .query_row(
                    "SELECT artifact_id FROM changelog_exports WHERE id = ?1",
                    [record_id],
                    |row| row.get::<_, String>(0),
                )
                .map_err(db_read("Export parent could not be read"))?,
        )),
        _ => None,
    };
    let Some((parent_type, parent_id)) = parent else {
        return Ok(None);
    };
    let parent_exists = record_exists(connection, &parent_type, &parent_id)?;
    let parent_tombstoned: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM lifecycle_tombstones WHERE record_type = ?1 AND record_id = ?2)",
            params![record_type_value(&parent_type), parent_id],
            |row| row.get(0),
        )
        .map_err(db_read("Parent lifecycle state could not be read"))?;
    if parent_exists && !parent_tombstoned {
        Ok(None)
    } else {
        Ok(Some(OrphanClassification {
            record_type: record_type.clone(),
            record_id: record_id.into(),
            missing_parent_type: parent_type,
            missing_parent_id: parent_id,
            reason: "Required application parent is missing or tombstoned".into(),
        }))
    }
}

fn unavailable_destination(
    connection: &Connection,
    record_type: &LifecycleRecordType,
    record_id: &str,
) -> Result<Option<UnavailableDestination>, CommandError> {
    if !matches!(record_type, LifecycleRecordType::ChangelogExport) {
        return Ok(None);
    }
    connection
        .query_row(
            "SELECT destination, status, diagnostic_json FROM changelog_exports WHERE id = ?1 AND status IN ('failed', 'unavailable')",
            [record_id],
            |row| {
                Ok(UnavailableDestination {
                    export_id: record_id.into(),
                    destination: row.get(0)?,
                    status: row.get(1)?,
                    diagnostic: row.get(2)?,
                })
            },
        )
        .optional()
        .map_err(db_read("Export availability could not be read"))
}

fn has_active_operation(
    connection: &Connection,
    record_type: &LifecycleRecordType,
    record_id: &str,
) -> Result<bool, CommandError> {
    connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM lifecycle_operations WHERE record_type = ?1 AND record_id = ?2 AND status IN ('planned', 'running'))",
            params![record_type_value(record_type), record_id],
            |row| row.get(0),
        )
        .map_err(db_read("Active lifecycle state could not be read"))
}

fn retained_evidence(record_type: &LifecycleRecordType) -> Vec<String> {
    match record_type {
        LifecycleRecordType::Snapshot => vec!["Discovery candidates and baseline capture".into()],
        LifecycleRecordType::Operation => {
            vec!["Process, fingerprint, and recovery evidence".into()]
        }
        LifecycleRecordType::ReleaseWorkspace => {
            vec!["Workspace decisions, activity, and captures".into()]
        }
        LifecycleRecordType::Release => vec!["Captured Packwiz and Git provenance".into()],
        LifecycleRecordType::ChangelogArtifact | LifecycleRecordType::ChangelogRevision => {
            vec!["Provider evidence and immutable changelog history".into()]
        }
        LifecycleRecordType::ChangelogExport => {
            vec!["Persisted destination and export result".into()]
        }
        LifecycleRecordType::Modpack | LifecycleRecordType::ProviderCache => Vec::new(),
    }
}

fn record_type_value(value: &LifecycleRecordType) -> &'static str {
    match value {
        LifecycleRecordType::Modpack => "modpack",
        LifecycleRecordType::Snapshot => "snapshot",
        LifecycleRecordType::Operation => "operation",
        LifecycleRecordType::ReleaseWorkspace => "release_workspace",
        LifecycleRecordType::Release => "release",
        LifecycleRecordType::ChangelogArtifact => "changelog_artifact",
        LifecycleRecordType::ChangelogRevision => "changelog_revision",
        LifecycleRecordType::ChangelogExport => "changelog_export",
        LifecycleRecordType::ProviderCache => "provider_cache",
    }
}

fn action_value(value: &LifecycleAction) -> &'static str {
    match value {
        LifecycleAction::Archive => "archive",
        LifecycleAction::Restore => "restore",
        LifecycleAction::Detach => "detach",
        LifecycleAction::PermanentDelete => "permanent_delete",
    }
}

fn scope_value(value: &CleanupScope) -> &'static str {
    match value {
        CleanupScope::ProviderCache => "provider_cache",
        CleanupScope::ObsoleteExports => "obsolete_exports",
        CleanupScope::ProviderCacheAndObsoleteExports => "provider_cache_and_obsolete_exports",
    }
}

fn sum_length(connection: &Connection, query: &str) -> Result<u64, CommandError> {
    connection
        .query_row(query, [], |row| row.get::<_, i64>(0))
        .map(|value| value.max(0) as u64)
        .map_err(db_read("Storage category could not be measured"))
}

fn db_read(message: &'static str) -> impl Fn(rusqlite::Error) -> CommandError {
    move |error| CommandError::new("database_read_failed", message).with_details(error.to_string())
}

fn db_write(message: &'static str) -> impl Fn(rusqlite::Error) -> CommandError {
    move |error| CommandError::new("database_write_failed", message).with_details(error.to_string())
}

trait OptionalRow<T> {
    fn optional(self) -> rusqlite::Result<Option<T>>;
}

impl<T> OptionalRow<T> for rusqlite::Result<T> {
    fn optional(self) -> rusqlite::Result<Option<T>> {
        match self {
            Ok(value) => Ok(Some(value)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(error) => Err(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn connection() -> Connection {
        let connection = Connection::open_in_memory().expect("database should open");
        connection
            .execute_batch(include_str!("schema.sql"))
            .expect("schema should initialize");
        connection
    }

    #[test]
    fn preview_reports_protected_snapshot_relationships() {
        let connection = connection();
        connection
                .execute(
                    "INSERT INTO modpacks (id, canonical_path, application_json, packwiz_json, validation_json, created_at, updated_at) VALUES ('pack-1', 'C:/pack', '{}', '{}', '[]', '1', '1')",
                    [],
                )
                .unwrap();
        connection
                .execute(
                    "INSERT INTO snapshots (id, modpack_id, lifecycle, outcome, result_json, created_at, updated_at) VALUES ('snapshot-1', 'pack-1', 'reviewable', 'normal', '{}', '1', '1')",
                    [],
                )
                .unwrap();
        connection
                .execute(
                    "INSERT INTO release_workspaces (id, modpack_id, name, lifecycle, evidence_status, publication_status, created_at, updated_at) VALUES ('workspace-1', 'pack-1', 'Release', 'draft', 'baseline', 'draft', '1', '1')",
                    [],
                )
                .unwrap();
        connection
                .execute(
                    "UPDATE release_workspaces SET source_snapshot_id = 'snapshot-1' WHERE id = 'workspace-1'",
                    [],
                )
                .unwrap();
        let preview = preview_lifecycle_action_inner(
            &connection,
            &LifecycleRecordType::Snapshot,
            "snapshot-1",
            &LifecycleAction::Archive,
            None,
        )
        .unwrap();
        assert!(!preview.direct_references.is_empty());
        assert!(preview.eligible);
        assert!(!preview.fingerprint.is_empty());
    }

    #[test]
    fn cleanup_plan_counts_cache_bytes_and_active_protection() {
        let connection = connection();
        connection
                .execute(
                    "INSERT INTO changelog_cache (id, cache_key_json, raw_response, association_json, retrieved_at, request_context, response_context) VALUES ('cache-1', '{}', '12345', '{}', '1', '{}', '{}')",
                    [],
                )
                .unwrap();
        let plan = cleanup_plan_inner(&connection, &CleanupScope::ProviderCache).unwrap();
        assert_eq!(plan.candidates.len(), 1);
        assert_eq!(plan.removable_bytes, 5);
        assert_eq!(plan.protected_count, 0);
    }

    #[test]
    fn cleanup_protects_cache_referenced_by_retained_artifact() {
        let connection = connection();
        connection
            .execute(
                "INSERT INTO modpacks (id, canonical_path, application_json, packwiz_json, validation_json, created_at, updated_at) VALUES ('pack-1', 'C:/pack', '{}', '{}', '[]', '1', '1')",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO changelog_attempts (id, modpack_id, request_json, request_fingerprint, status, created_at) VALUES ('attempt-1', 'pack-1', '{}', 'fingerprint', 'complete', '1')",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO changelog_cache (id, cache_key_json, raw_response, association_json, retrieved_at, request_context, response_context) VALUES ('cache-1', '{}', '12345', '{}', '1', '{}', '{}')",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO changelog_artifacts (id, modpack_id, attempt_id, status, content, entries_json, created_at, updated_at) VALUES ('artifact-1', 'pack-1', 'attempt-1', 'complete', '', '[\"cache-1\"]', '1', '1')",
                [],
            )
            .unwrap();
        let plan = cleanup_plan_inner(&connection, &CleanupScope::ProviderCache).unwrap();
        assert_eq!(plan.protected_count, 1);
        assert_eq!(plan.removable_bytes, 0);
    }

    #[test]
    fn storage_report_separates_external_boundaries() {
        let connection = connection();
        let report = storage_report_inner(&connection).unwrap();
        assert!(report.application_owned_bytes > 0);
        assert_eq!(report.external_boundaries.len(), 3);
        assert!(report
            .unavailable_categories
            .iter()
            .any(|category| category.contains("Packwiz")));
    }

    #[test]
    fn preview_blocks_active_lifecycle_operation() {
        let connection = connection();
        connection
            .execute(
                "INSERT INTO modpacks (id, canonical_path, application_json, packwiz_json, validation_json, created_at, updated_at) VALUES ('pack-1', 'C:/pack', '{}', '{}', '[]', '1', '1')",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO lifecycle_operations (id, record_type, record_id, action, preview_fingerprint, status, created_at) VALUES ('operation-1', 'modpack', 'pack-1', 'archive', 'fingerprint', 'running', '1')",
                [],
            )
            .unwrap();
        let preview = preview_lifecycle_action_inner(
            &connection,
            &LifecycleRecordType::Modpack,
            "pack-1",
            &LifecycleAction::Archive,
            None,
        )
        .unwrap();
        assert!(preview.active_operation);
        assert!(!preview.eligible);
        assert!(preview
            .blocked_reasons
            .iter()
            .any(|reason| reason.contains("active operation")));
    }

    #[test]
    fn lifecycle_transitions_retain_history_and_protect_external_files() {
        let connection = connection();
        let fixture =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/packwiz/valid");
        let before = std::fs::read(fixture.join("pack.toml")).unwrap();
        let export_path = std::env::temp_dir().join(format!(
            "cm-modpack-util-export-{}-{}.md",
            std::process::id(),
            unique_id("test")
        ));
        std::fs::write(&export_path, "user-owned export").unwrap();

        connection
            .execute(
                "INSERT INTO modpacks (id, canonical_path, application_json, packwiz_json, validation_json, created_at, updated_at) VALUES ('pack-1', ?1, '{}', '{}', '[]', '1', '1')",
                [fixture.to_string_lossy().as_ref()],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO snapshots (id, modpack_id, lifecycle, outcome, result_json, created_at, updated_at) VALUES ('snapshot-1', 'pack-1', 'reviewable', 'normal', '{}', '1', '1')",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO release_workspaces (id, modpack_id, source_snapshot_id, name, lifecycle, evidence_status, publication_status, created_at, updated_at) VALUES ('workspace-1', 'pack-1', 'snapshot-1', 'Release', 'draft', 'baseline', 'draft', '1', '1')",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO changelog_attempts (id, modpack_id, request_json, request_fingerprint, status, created_at) VALUES ('attempt-1', 'pack-1', '{}', 'fingerprint', 'complete', '1')",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO changelog_artifacts (id, modpack_id, attempt_id, status, content, entries_json, created_at, updated_at) VALUES ('artifact-1', 'pack-1', 'attempt-1', 'complete', 'history', '[]', '1', '1')",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO changelog_exports (id, artifact_id, destination, format, content, content_hash, status, exported_at) VALUES ('export-1', 'artifact-1', ?1, 'markdown', 'history', 'hash', 'unavailable', '1')",
                [export_path.to_string_lossy().as_ref()],
            )
            .unwrap();

        let archive_preview = preview_lifecycle_action_inner(
            &connection,
            &LifecycleRecordType::Snapshot,
            "snapshot-1",
            &LifecycleAction::Archive,
            None,
        )
        .unwrap();
        assert_eq!(archive_preview.direct_references.len(), 1);
        let archived = apply_lifecycle_action_inner(
            &connection,
            &LifecycleRequest {
                target_type: LifecycleRecordType::Snapshot,
                target_id: "snapshot-1".into(),
                action: LifecycleAction::Archive,
                scope: None,
                preview_fingerprint: archive_preview.fingerprint,
                confirmation: None,
            },
        )
        .unwrap();
        assert!(archived.tombstoned);
        assert_eq!(
            connection
                .query_row(
                    "SELECT state FROM lifecycle_tombstones WHERE record_type = 'snapshot' AND record_id = 'snapshot-1'",
                    [],
                    |row| row.get::<_, String>(0)
                )
                .unwrap(),
            "archived"
        );

        let restore_preview = preview_lifecycle_action_inner(
            &connection,
            &LifecycleRecordType::Snapshot,
            "snapshot-1",
            &LifecycleAction::Restore,
            None,
        )
        .unwrap();
        apply_lifecycle_action_inner(
            &connection,
            &LifecycleRequest {
                target_type: LifecycleRecordType::Snapshot,
                target_id: "snapshot-1".into(),
                action: LifecycleAction::Restore,
                scope: None,
                preview_fingerprint: restore_preview.fingerprint,
                confirmation: None,
            },
        )
        .unwrap();
        let detach_preview = preview_lifecycle_action_inner(
            &connection,
            &LifecycleRecordType::Snapshot,
            "snapshot-1",
            &LifecycleAction::Detach,
            None,
        )
        .unwrap();
        apply_lifecycle_action_inner(
            &connection,
            &LifecycleRequest {
                target_type: LifecycleRecordType::Snapshot,
                target_id: "snapshot-1".into(),
                action: LifecycleAction::Detach,
                scope: None,
                preview_fingerprint: detach_preview.fingerprint,
                confirmation: None,
            },
        )
        .unwrap();
        let detached_source: Option<String> = connection
            .query_row(
                "SELECT source_snapshot_id FROM release_workspaces WHERE id = 'workspace-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(detached_source.is_none());

        let export_preview = preview_lifecycle_action_inner(
            &connection,
            &LifecycleRecordType::ChangelogExport,
            "export-1",
            &LifecycleAction::PermanentDelete,
            None,
        )
        .unwrap();
        let confirmation_error = apply_lifecycle_action_inner(
            &connection,
            &LifecycleRequest {
                target_type: LifecycleRecordType::ChangelogExport,
                target_id: "export-1".into(),
                action: LifecycleAction::PermanentDelete,
                scope: None,
                preview_fingerprint: export_preview.fingerprint.clone(),
                confirmation: Some("DELETE wrong-id".into()),
            },
        )
        .unwrap_err();
        assert_eq!(confirmation_error.code, "confirmation_required");
        let deleted = apply_lifecycle_action_inner(
            &connection,
            &LifecycleRequest {
                target_type: LifecycleRecordType::ChangelogExport,
                target_id: "export-1".into(),
                action: LifecycleAction::PermanentDelete,
                scope: None,
                preview_fingerprint: export_preview.fingerprint,
                confirmation: Some("DELETE export-1".into()),
            },
        )
        .unwrap();
        assert!(deleted.physically_deleted);
        assert_eq!(std::fs::read(&export_path).unwrap(), b"user-owned export");
        assert_eq!(std::fs::read(fixture.join("pack.toml")).unwrap(), before);
        let _ = std::fs::remove_file(export_path);
    }

    #[test]
    fn preview_reports_orphaned_unavailable_export_and_stale_fingerprints() {
        let connection = connection();
        connection
            .execute(
                "INSERT INTO modpacks (id, canonical_path, application_json, packwiz_json, validation_json, created_at, updated_at) VALUES ('pack-1', 'C:/pack', '{}', '{}', '[]', '1', '1')",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO changelog_attempts (id, modpack_id, request_json, request_fingerprint, status, created_at) VALUES ('attempt-1', 'pack-1', '{}', 'fingerprint', 'complete', '1')",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO changelog_artifacts (id, modpack_id, attempt_id, status, content, entries_json, created_at, updated_at) VALUES ('artifact-1', 'pack-1', 'attempt-1', 'complete', '', '[]', '1', '1')",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO changelog_exports (id, artifact_id, destination, format, content, content_hash, status, exported_at) VALUES ('export-1', 'artifact-1', 'C:/missing/export.md', 'markdown', 'x', 'hash', 'unavailable', '1')",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO lifecycle_tombstones (record_type, record_id, state, reason, created_at, updated_at) VALUES ('changelog_artifact', 'artifact-1', 'archived', 'test', '1', '1')",
                [],
            )
            .unwrap();
        let preview = preview_lifecycle_action_inner(
            &connection,
            &LifecycleRecordType::ChangelogExport,
            "export-1",
            &LifecycleAction::Archive,
            None,
        )
        .unwrap();
        assert!(preview.orphan.is_some());
        assert!(preview.unavailable_destination.is_some());
        assert!(!preview.eligible);

        connection
            .execute(
                "INSERT INTO changelog_cache (id, cache_key_json, raw_response, association_json, retrieved_at, request_context, response_context) VALUES ('cache-1', '{}', 'cache', '{}', '1', '{}', '{}')",
                [],
            )
            .unwrap();
        let cache_preview = preview_lifecycle_action_inner(
            &connection,
            &LifecycleRecordType::ProviderCache,
            "cache-1",
            &LifecycleAction::PermanentDelete,
            None,
        )
        .unwrap();
        connection
            .execute(
                "INSERT INTO lifecycle_operations (id, record_type, record_id, action, preview_fingerprint, status, created_at) VALUES ('operation-1', 'provider_cache', 'cache-1', 'permanent_delete', 'changed', 'running', '1')",
                [],
            )
            .unwrap();
        let stale = apply_lifecycle_action_inner(
            &connection,
            &LifecycleRequest {
                target_type: LifecycleRecordType::ProviderCache,
                target_id: "cache-1".into(),
                action: LifecycleAction::PermanentDelete,
                scope: None,
                preview_fingerprint: cache_preview.fingerprint,
                confirmation: Some("DELETE cache-1".into()),
            },
        )
        .unwrap_err();
        assert_eq!(stale.code, "stale_preview");
    }

    #[test]
    fn cleanup_is_idempotent_and_storage_reconciles_owned_categories() {
        let connection = connection();
        connection
            .execute(
                "INSERT INTO changelog_cache (id, cache_key_json, raw_response, association_json, retrieved_at, request_context, response_context) VALUES ('cache-1', '{}', 'cache', '{}', '1', '{}', '{}')",
                [],
            )
            .unwrap();
        let plan = cleanup_plan_inner(&connection, &CleanupScope::ProviderCache).unwrap();
        let result = execute_cleanup_inner(
            &connection,
            &CleanupRequest {
                scope: CleanupScope::ProviderCache,
                preview_fingerprint: plan.fingerprint,
            },
        )
        .unwrap();
        assert_eq!(result.removed_count, 1);
        let second_plan = cleanup_plan_inner(&connection, &CleanupScope::ProviderCache).unwrap();
        let second_result = execute_cleanup_inner(
            &connection,
            &CleanupRequest {
                scope: CleanupScope::ProviderCache,
                preview_fingerprint: second_plan.fingerprint,
            },
        )
        .unwrap();
        assert_eq!(second_result.removed_count, 0);
        let report = storage_report_inner(&connection).unwrap();
        assert_eq!(
            report.application_owned_bytes,
            report
                .categories
                .iter()
                .map(|category| category.bytes)
                .sum::<u64>()
        );
        assert!(report
            .external_boundaries
            .iter()
            .all(|boundary| boundary.contains("never deleted")));
    }
}
