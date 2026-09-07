use super::{compatibility, fingerprint, operations::DiscoveryRuntime, process};
use crate::db::{self, Database};
use crate::domain::{
    CommandError, CompatibilityEvidence, DiscoveryDiagnostics, DiscoveryOutcomeKind,
    DiscoveryResult, Evidence, InventoryProvider, InventorySide, OperationStatus, UpdateCandidate,
    VersionChangeKind,
};
use std::path::Path;
use tauri::{AppHandle, Emitter, State};

fn abnormal(
    outcome: DiscoveryOutcomeKind,
    compatibility: CompatibilityEvidence,
    message: impl Into<String>,
) -> DiscoveryResult {
    DiscoveryResult {
        status: OperationStatus::Failed,
        outcome,
        candidates: Vec::new(),
        diagnostics: DiscoveryDiagnostics {
            compatibility,
            process: None,
            fingerprint: None,
            messages: vec![message.into()],
        },
        error: None,
    }
}

#[tauri::command]
pub fn check_for_updates(
    app: AppHandle,
    database: State<'_, Database>,
    runtime: State<'_, DiscoveryRuntime>,
    modpack_id: String,
) -> Result<DiscoveryResult, CommandError> {
    let root = db::registered_modpack_path(&database, &modpack_id)?;
    let root = Path::new(&root).canonicalize().map_err(|error| {
        CommandError::new(
            "project_unavailable",
            "Registered modpack root is unavailable",
        )
        .with_details(error.to_string())
    })?;
    let executable = match compatibility::resolve_executable() {
        Ok(path) => path,
        Err(error) => {
            let result = abnormal(
                DiscoveryOutcomeKind::Unsupported,
                compatibility::unsupported(error.message.clone()),
                error.message,
            );
            db::persist_discovery_result(&database, &modpack_id, &result)?;
            return Ok(result);
        }
    };
    let compatibility = compatibility::validate_executable(&executable);
    if !matches!(
        compatibility.status,
        crate::domain::CompatibilityStatus::Supported
    ) {
        let result = abnormal(
            DiscoveryOutcomeKind::Unsupported,
            compatibility,
            "Packwiz executable is outside the tested profile",
        );
        db::persist_discovery_result(&database, &modpack_id, &result)?;
        return Ok(result);
    }
    let _lease = runtime.coordinator.acquire(&modpack_id)?;
    let cancellation = runtime.begin_cancellation()?;
    let _ = app.emit(
        "discovery-progress",
        crate::domain::DiscoveryProgress {
            project_id: modpack_id.clone(),
            kind: crate::domain::DiscoveryProgressKind::Starting,
            message: "Starting safe Packwiz discovery".to_string(),
            output_bytes: 0,
            cancellable: true,
        },
    );
    let before = fingerprint::collect(&root).ok();
    let plan = process::command_plan(executable, root.clone())?;
    let mut limits = process::RunLimits::default();
    limits.cancel = Some(cancellation.clone());
    let evidence = match process::run_probe(&plan, limits) {
        Ok(evidence) => evidence,
        Err(error) => {
            let result = abnormal(DiscoveryOutcomeKind::Failed, compatibility, error.message);
            db::persist_discovery_result(&database, &modpack_id, &result)?;
            return Ok(result);
        }
    };
    let _ = app.emit("discovery-process", &evidence);
    let _ = app.emit(
        "discovery-progress",
        crate::domain::DiscoveryProgress {
            project_id: modpack_id.clone(),
            kind: crate::domain::DiscoveryProgressKind::Cancelling,
            message: if matches!(evidence.prompt, crate::domain::PromptState::NoUpdates) {
                "Packwiz reported that all files are up to date".to_string()
            } else {
                "Packwiz cancellation result captured".to_string()
            },
            output_bytes: (evidence.stdout.len() + evidence.stderr.len()) as u64,
            cancellable: false,
        },
    );
    runtime.clear_cancellation();
    let after = fingerprint::collect(&root).ok();
    let comparison = before
        .zip(after)
        .map(|(before, after)| fingerprint::compare(before, after));
    let Some(comparison) = comparison else {
        let result = DiscoveryResult {
            status: OperationStatus::Failed,
            outcome: DiscoveryOutcomeKind::Indeterminate,
            candidates: Vec::new(),
            diagnostics: DiscoveryDiagnostics {
                compatibility,
                process: Some(evidence),
                fingerprint: None,
                messages: vec!["Before and after fingerprints were not comparable".to_string()],
            },
            error: None,
        };
        db::persist_discovery_result(&database, &modpack_id, &result)?;
        return Ok(result);
    };
    let no_updates = matches!(evidence.prompt, crate::domain::PromptState::NoUpdates)
        && matches!(
            evidence.cancellation,
            crate::domain::CancellationState::NotAttempted
        )
        && matches!(evidence.exit_code, Some(0) | None);
    let interactive_cancelled = matches!(evidence.prompt, crate::domain::PromptState::ExpectedSeen)
        && matches!(
            evidence.cancellation,
            crate::domain::CancellationState::Confirmed
        );
    let safe =
        (no_updates || interactive_cancelled) && comparison.unchanged && !evidence.output_truncated;
    let candidates = if safe {
        let candidates = parse_candidates(&evidence.stdout);
        enrich_candidates(&root, candidates)
    } else {
        Vec::new()
    };
    let malformed_candidates =
        safe && evidence.stdout.contains("Updates found:") && candidates.is_empty();
    let outcome = if malformed_candidates {
        DiscoveryOutcomeKind::Indeterminate
    } else if safe {
        DiscoveryOutcomeKind::Normal
    } else if !comparison.unchanged {
        DiscoveryOutcomeKind::Unsafe
    } else {
        DiscoveryOutcomeKind::Indeterminate
    };
    let output_bytes = evidence.stdout.len() as u64;
    let result = DiscoveryResult {
        status: if safe && !malformed_candidates {
            OperationStatus::Succeeded
        } else {
            OperationStatus::Failed
        },
        outcome,
        candidates,
        diagnostics: DiscoveryDiagnostics {
            compatibility,
            process: Some(evidence),
            fingerprint: Some(comparison),
            messages: Vec::new(),
        },
        error: None,
    };
    let _ = app.emit(
        "discovery-progress",
        crate::domain::DiscoveryProgress {
            project_id: modpack_id.clone(),
            kind: crate::domain::DiscoveryProgressKind::Finished,
            message: "Discovery finished with recorded safety evidence".to_string(),
            output_bytes,
            cancellable: false,
        },
    );
    db::persist_discovery_result(&database, &modpack_id, &result)?;
    Ok(result)
}

fn enrich_candidates(root: &Path, candidates: Vec<UpdateCandidate>) -> Vec<UpdateCandidate> {
    let Ok(inventory) = crate::domain::inventory::read_inventory(root) else {
        return candidates;
    };
    candidates
        .into_iter()
        .map(|mut candidate| {
            let identity = match &candidate.identity {
                Evidence::Observed(value) => value,
                _ => return candidate,
            };
            if let Some(entry) = inventory.iter().find(|entry| {
                entry.local_id.contains(identity)
                    || matches!(&entry.name, Evidence::Observed(name) if name == identity)
            }) {
                candidate.local_path = Evidence::Observed(entry.metadata_path.clone());
                candidate.provider = entry.provider.clone();
                candidate.side = entry.side.clone();
                candidate.pin = entry.pin.clone();
                candidate.source_url = entry.source_url.clone();
                candidate.page_link = entry.page_link.clone();
            }
            candidate
        })
        .collect()
}

pub fn parse_candidates(output: &str) -> Vec<UpdateCandidate> {
    output
        .lines()
        .filter_map(|line| {
            let (left, available) = line.split_once(" -> ")?;
            let (identity, current) = left.rsplit_once(':').map_or_else(
                || {
                    let fields: Vec<&str> = line.split_whitespace().collect();
                    if fields.len() == 4 && fields[2] == "->" {
                        (fields[0], fields[1])
                    } else {
                        ("", "")
                    }
                },
                |(identity, current)| (identity.trim(), current.trim()),
            );
            if identity.is_empty() || current.is_empty() || available.is_empty() {
                return None;
            }
            let identity = identity.to_string();
            let current = current.to_string();
            let available = available.trim().to_string();
            Some(UpdateCandidate {
                identity: Evidence::Observed(identity),
                current_version: Evidence::Observed(current),
                available_version: Evidence::Observed(available),
                local_path: Evidence::Unknown,
                provider: Evidence::Unknown::<InventoryProvider>,
                side: Evidence::Unknown::<InventorySide>,
                pin: Evidence::Unknown,
                source_url: Evidence::Unknown,
                page_link: None,
                severity: Evidence::Unknown,
                version_change: VersionChangeKind::Unknown,
                output_evidence: line.to_string(),
            })
        })
        .collect()
}
