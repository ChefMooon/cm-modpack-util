use super::{compatibility, fingerprint, operations::DiscoveryRuntime, process};
use crate::db::{self, Database};
use crate::domain::{
    ApplyOperationReport, ApplyOperationRequest, CommandError, Evidence, OperationAttempt,
    OperationKind, OperationOutcome, OperationStatus, OperationVerification, PinOperationRequest,
    RecoveryObservation, SnapshotDecision, SnapshotLifecycle,
};
use std::path::Path;
use tauri::State;

#[tauri::command]
pub fn apply_modpack(
    database: State<'_, Database>,
    runtime: State<'_, DiscoveryRuntime>,
    request: ApplyOperationRequest,
) -> Result<ApplyOperationReport, CommandError> {
    run_apply_operation(&database, &runtime, request)
}

#[tauri::command]
pub fn pin_modpack(
    database: State<'_, Database>,
    runtime: State<'_, DiscoveryRuntime>,
    request: PinOperationRequest,
) -> Result<OperationAttempt, CommandError> {
    run_pin_operation(&database, &runtime, request, true)
}

#[tauri::command]
pub fn unpin_modpack(
    database: State<'_, Database>,
    runtime: State<'_, DiscoveryRuntime>,
    request: PinOperationRequest,
) -> Result<OperationAttempt, CommandError> {
    run_pin_operation(&database, &runtime, request, false)
}

#[tauri::command]
pub fn cancel_operation(
    runtime: State<'_, DiscoveryRuntime>,
    operation_id: String,
) -> Result<(), CommandError> {
    runtime.cancel_operation(&operation_id)
}

fn run_pin_operation(
    database: &Database,
    runtime: &DiscoveryRuntime,
    request: PinOperationRequest,
    pin: bool,
) -> Result<OperationAttempt, CommandError> {
    let root = db::registered_modpack_path(database, &request.project_id)?;
    let root = Path::new(&root).canonicalize().map_err(|error| {
        CommandError::new(
            "project_unavailable",
            "Registered modpack root is unavailable",
        )
        .with_details(error.to_string())
    })?;
    let _lease = runtime.mutation_coordinator.acquire(&request.project_id)?;
    let inventory = crate::domain::inventory::read_inventory(&root)?;
    let entry = inventory
        .iter()
        .find(|entry| entry.local_id == request.entry_id)
        .ok_or_else(|| CommandError::new("entry_not_found", "Inventory entry is not registered"))?;
    let metadata_path = Path::new(&entry.metadata_path);
    let metadata_path =
        crate::safety::canonical_registered_path(&root, metadata_path).map_err(|_| {
            CommandError::new(
                "invalid_metadata_path",
                "Metadata path is outside the registered modpack",
            )
        })?;
    let slug = crate::domain::inventory::packwiz_slug(&metadata_path)?;
    let executable = compatibility::resolve_executable()?;
    let profile = compatibility::validate_executable(&executable);
    if profile.profile.is_none() {
        return Err(profile.diagnostic.unwrap_or_else(|| {
            CommandError::new(
                "unsupported_compatibility",
                "Packwiz profile is unsupported",
            )
        }));
    }
    let arguments = compatibility::pin_command(&slug, pin)?;
    let id = format!("operation-{}-{}", request.project_id, timestamp());
    let cancel = runtime.begin_operation_cancellation(&id)?;
    let before = fingerprint::collect(&root).ok();
    let recovery = recovery_observation(&root);
    let created_at = timestamp();
    let plan =
        process::command_plan_with_arguments(executable.clone(), arguments.clone(), root.clone())?;
    let output = process::run_probe_without_prompt_authorization(
        &plan,
        process::RunLimits {
            cancel: Some(cancel),
            ..Default::default()
        },
    );
    runtime.clear_operation_cancellation(&id);
    let output = output?;
    let stdout = output.stdout.clone();
    let stderr = output.stderr.clone();
    let after = fingerprint::collect(&root).ok();
    let observed = crate::domain::inventory::read_inventory(&root)
        .ok()
        .and_then(|entries| {
            entries
                .into_iter()
                .find(|item| item.local_id == request.entry_id)
        })
        .map(|item| item.pin);
    let observed_state = match observed {
        Some(Evidence::Observed(value)) => value.to_string(),
        Some(Evidence::Unknown) => "unknown".to_string(),
        Some(Evidence::Unavailable) => "unavailable".to_string(),
        Some(Evidence::Malformed { .. }) | None => "malformed".to_string(),
    };
    let verified = output.exit_code == Some(0)
        && output.cancellation == crate::domain::CancellationState::NotAttempted
        && observed_state == pin.to_string();
    let outcome = if verified {
        OperationOutcome::Complete
    } else if matches!(
        output.cancellation,
        crate::domain::CancellationState::UserCancelled
            | crate::domain::CancellationState::Terminated
    ) {
        OperationOutcome::Cancelled
    } else if output.exit_code == Some(0) {
        OperationOutcome::Indeterminate
    } else {
        OperationOutcome::Failed
    };
    let attempt = OperationAttempt {
        id,
        modpack_id: request.project_id,
        snapshot_id: None,
        predecessor_id: None,
        kind: if pin {
            OperationKind::Pin
        } else {
            OperationKind::Unpin
        },
        status: if verified {
            OperationStatus::Succeeded
        } else {
            OperationStatus::Failed
        },
        outcome: Some(outcome),
        recovery,
        process: Some(crate::domain::ProcessEvidence {
            executable: output.executable,
            arguments,
            working_directory: output.working_directory,
            stdout,
            stderr,
            exit_code: output.exit_code,
            started_at: output.started_at,
            finished_at: output.finished_at.clone(),
            output_truncated: output.output_truncated,
            prompt: output.prompt,
            cancellation: output.cancellation,
        }),
        before_fingerprint: before,
        after_fingerprint: after,
        verification: Some(OperationVerification {
            comparable: true,
            intended_state: pin.to_string(),
            observed_state,
            differences: Vec::new(),
            verified,
        }),
        error: if verified {
            None
        } else {
            Some(CommandError::new(
                "pin_verification_failed",
                "Packwiz pin state could not be verified",
            ))
        },
        created_at,
        finished_at: output.finished_at.clone().or_else(|| Some(timestamp())),
    };
    db::persist_operation_attempt(database, &attempt, &metadata_path, &request.entry_id, pin)?;
    Ok(attempt)
}

fn run_apply_operation(
    database: &Database,
    runtime: &DiscoveryRuntime,
    request: ApplyOperationRequest,
) -> Result<ApplyOperationReport, CommandError> {
    if request.operation_id.trim().is_empty() || request.candidate_ids.is_empty() {
        return Err(CommandError::new(
            "invalid_apply_request",
            "An apply operation requires an ID and at least one selected candidate",
        ));
    }
    let snapshot = db::load_snapshot(database, &request.snapshot_id)?;
    if snapshot.lifecycle != SnapshotLifecycle::Reviewable
        || !matches!(
            snapshot.outcome,
            crate::domain::DiscoveryOutcomeKind::Normal
        )
    {
        return Err(CommandError::new(
            "snapshot_not_reviewable",
            "Only a current reviewable snapshot can be applied",
        ));
    }
    let project_root = Path::new(&db::registered_modpack_path(
        database,
        &snapshot.project_id,
    )?)
    .canonicalize()
    .map_err(|error| {
        CommandError::new("project_unavailable", "Project root is unavailable")
            .with_details(error.to_string())
    })?;
    let before_snapshot = snapshot
        .result
        .diagnostics
        .fingerprint
        .as_ref()
        .ok_or_else(|| {
            CommandError::new(
                "snapshot_fingerprint_unavailable",
                "Snapshot has no usable before-state fingerprint",
            )
        })?
        .before
        .clone();
    let current = fingerprint::collect(&project_root).map_err(|error| {
        CommandError::new(
            "project_changed",
            "Project changed or could not be fingerprinted",
        )
        .with_details(error)
    })?;
    let comparison = fingerprint::compare(before_snapshot, current);
    if !comparison.comparable || !comparison.unchanged {
        return Err(CommandError::new(
            "snapshot_stale",
            "Project changed since the selected snapshot",
        ));
    }
    let executable = compatibility::resolve_executable()?;
    if compatibility::validate_executable(&executable)
        .profile
        .is_none()
    {
        return Err(CommandError::new(
            "unsupported_compatibility",
            "Packwiz executable is outside the tested mutation profile",
        ));
    }
    let _lease = runtime.mutation_coordinator.acquire(&snapshot.project_id)?;
    let cancel = runtime.begin_operation_cancellation(&request.operation_id)?;
    let mut attempts = Vec::new();
    let mut succeeded = 0usize;
    let mut cancelled = false;
    for (index, candidate_id) in request.candidate_ids.iter().enumerate() {
        if cancel.load(std::sync::atomic::Ordering::Relaxed) {
            cancelled = true;
            break;
        }
        let candidate = snapshot
            .candidates
            .iter()
            .find(|candidate| candidate.id == *candidate_id)
            .ok_or_else(|| {
                CommandError::new(
                    "candidate_not_found",
                    "Selected candidate does not belong to the snapshot",
                )
            })?;
        let decision = snapshot
            .decisions
            .iter()
            .filter(|decision| decision.candidate_id == *candidate_id)
            .last()
            .map(|decision| &decision.decision);
        if decision != Some(&SnapshotDecision::Selected) {
            return Err(CommandError::new(
                "candidate_not_selected",
                "Every apply target must have a selected decision",
            ));
        }
        let expected_path = match &candidate.candidate.local_path {
            Evidence::Observed(path) => Path::new(path).canonicalize().map_err(|_| {
                CommandError::new(
                    "candidate_path_unavailable",
                    "Selected candidate metadata is unavailable",
                )
            })?,
            _ => {
                return Err(CommandError::new(
                    "candidate_identity_unknown",
                    "Selected candidate has no verified local metadata path",
                ))
            }
        };
        let inventory = crate::domain::inventory::read_inventory(&project_root)?;
        let entry = inventory
            .iter()
            .find(|entry| {
                Path::new(&entry.metadata_path).canonicalize().ok().as_ref() == Some(&expected_path)
            })
            .ok_or_else(|| {
                CommandError::new(
                    "candidate_changed",
                    "Selected candidate no longer matches current inventory",
                )
            })?;
        let metadata_path = crate::safety::canonical_registered_path(
            &project_root,
            Path::new(&entry.metadata_path),
        )
        .map_err(|_| {
            CommandError::new(
                "invalid_metadata_path",
                "Candidate metadata is outside the registered modpack",
            )
        })?;
        let slug = crate::domain::inventory::packwiz_slug(&metadata_path)?;
        let arguments = compatibility::update_command(&slug)?;
        let before = fingerprint::collect(&project_root).ok();
        let created_at = timestamp();
        let plan = process::command_plan_with_arguments(
            executable.clone(),
            arguments.clone(),
            project_root.clone(),
        )?;
        let output = process::run_probe_without_prompt_authorization(
            &plan,
            process::RunLimits {
                cancel: Some(cancel.clone()),
                ..Default::default()
            },
        )?;
        let after = fingerprint::collect(&project_root).ok();
        let updated_entry = crate::domain::inventory::read_inventory(&project_root)
            .ok()
            .and_then(|entries| {
                entries
                    .into_iter()
                    .find(|item| item.local_id == entry.local_id)
            });
        let observed_filename = updated_entry.as_ref().and_then(|item| {
            crate::domain::inventory::metadata_filename(Path::new(&item.metadata_path)).ok()
        });
        let observed_version = updated_entry.as_ref().and_then(|item| match &item.version {
            Evidence::Observed(value) => Some(value.clone()),
            _ => None,
        });
        let available_artifact = match &candidate.candidate.available_version {
            Evidence::Observed(value) => value.clone(),
            _ => "unknown".to_string(),
        };
        let current_artifact = match &candidate.candidate.current_version {
            Evidence::Observed(value) => value.as_str(),
            _ => "",
        };
        let current_version = match &entry.version {
            Evidence::Observed(value) => value.as_str(),
            _ => "",
        };
        let expected_state =
            normalize_target_version(current_artifact, &available_artifact, current_version)
                .unwrap_or_else(|| available_artifact.clone());
        let observed_state = observed_version
            .or_else(|| observed_filename.clone())
            .unwrap_or_else(|| "unavailable".to_string());
        let verified_state = observed_filename
            .as_deref()
            .is_some_and(|filename| filename == available_artifact)
            || observed_state == expected_state;
        let verification = OperationVerification {
            comparable: before.is_some() && after.is_some(),
            intended_state: expected_state.clone(),
            observed_state: observed_state.clone(),
            differences: if verified_state {
                Vec::new()
            } else {
                vec!["target version does not match selected snapshot".to_string()]
            },
            verified: output.exit_code == Some(0)
                && output.cancellation == crate::domain::CancellationState::NotAttempted
                && verified_state,
        };
        let verified = verification.verified;
        if verified {
            succeeded += 1;
        }
        let outcome = if verified {
            OperationOutcome::Complete
        } else if matches!(
            output.cancellation,
            crate::domain::CancellationState::UserCancelled
                | crate::domain::CancellationState::Terminated
        ) {
            cancelled = true;
            OperationOutcome::Cancelled
        } else if output.exit_code == Some(0) {
            OperationOutcome::Indeterminate
        } else {
            OperationOutcome::Failed
        };
        let attempt = OperationAttempt {
            id: format!("{}-{}", request.operation_id, index),
            modpack_id: snapshot.project_id.clone(),
            snapshot_id: Some(snapshot.id.clone()),
            predecessor_id: None,
            kind: OperationKind::Apply,
            status: if verified {
                OperationStatus::Succeeded
            } else if matches!(outcome, OperationOutcome::Cancelled) {
                OperationStatus::Cancelled
            } else {
                OperationStatus::Failed
            },
            outcome: Some(outcome),
            recovery: recovery_observation(&project_root),
            process: Some(crate::domain::ProcessEvidence {
                executable: output.executable,
                arguments,
                working_directory: output.working_directory,
                stdout: output.stdout,
                stderr: output.stderr,
                exit_code: output.exit_code,
                started_at: output.started_at,
                finished_at: output.finished_at.clone(),
                output_truncated: output.output_truncated,
                prompt: output.prompt,
                cancellation: output.cancellation,
            }),
            before_fingerprint: before,
            after_fingerprint: after,
            verification: Some(verification),
            error: if verified {
                None
            } else {
                Some(CommandError::new(
                    "apply_verification_failed",
                    "Selected update could not be verified",
                ))
            },
            created_at,
            finished_at: output.finished_at.or_else(|| Some(timestamp())),
        };
        db::persist_apply_attempt(
            database,
            &attempt,
            candidate_id,
            &entry.local_id,
            &serde_json::to_string(&entry).unwrap_or_else(|_| "{}".to_string()),
        )?;
        attempts.push(attempt);
        if cancelled {
            break;
        }
    }
    runtime.clear_operation_cancellation(&request.operation_id);
    let outcome = if cancelled {
        OperationOutcome::Cancelled
    } else if succeeded == request.candidate_ids.len() {
        OperationOutcome::Complete
    } else if succeeded > 0 {
        OperationOutcome::Partial
    } else {
        OperationOutcome::Failed
    };
    Ok(ApplyOperationReport {
        snapshot_id: request.snapshot_id,
        attempts,
        outcome,
    })
}

fn recovery_observation(root: &Path) -> RecoveryObservation {
    let git = crate::domain::inventory::observe_git(root);
    let recovery_available = !matches!(git.state, crate::domain::GitWorkingTreeState::Unavailable);
    RecoveryObservation {
        git,
        recovery_available,
        warning_category: if recovery_available {
            None
        } else {
            Some("git_unavailable".to_string())
        },
        diagnostic: None,
    }
}

fn timestamp() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos().to_string())
        .unwrap_or_else(|_| "0".to_string())
}

fn normalize_target_version(
    current_artifact: &str,
    available_artifact: &str,
    current_version: &str,
) -> Option<String> {
    let current_artifact = current_artifact.strip_suffix(".jar")?;
    let available_artifact = available_artifact.strip_suffix(".jar")?;
    let version_start = current_artifact.find(current_version)?;
    let version_end = version_start + current_version.len();
    let prefix = &current_artifact[..version_start];
    let suffix = &current_artifact[version_end..];
    let available_version_start = available_artifact
        .strip_prefix(prefix)?
        .strip_suffix(suffix)?
        .len();
    let available_without_prefix = available_artifact.strip_prefix(prefix)?;
    let available_version = &available_without_prefix[..available_version_start];
    (!available_version.is_empty()).then(|| available_version.to_string())
}

#[cfg(test)]
mod tests {
    use super::compatibility;

    #[test]
    fn mutation_profile_is_not_interactive() {
        assert_eq!(
            compatibility::pin_command("ubes-delight", true).unwrap(),
            ["pin", "ubes-delight"]
        );
    }

    #[test]
    fn normalizes_packwiz_artifact_names_to_metadata_versions() {
        assert_eq!(
            super::normalize_target_version(
                "ubesdelight-neoforge-1.21.1-0.4.13.jar",
                "ubesdelight-neoforge-1.21.1-0.4.14.jar",
                "0.4.13",
            ),
            Some("0.4.14".to_string())
        );
    }
}
