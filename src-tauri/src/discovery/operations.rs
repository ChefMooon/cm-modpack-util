use super::{
    compatibility, fingerprint,
    process::{self, OperationCoordinator, RunLimits},
};
use crate::db::{self, Database};
use crate::domain::{CommandError, DiscoveryProgress, DiscoveryProgressKind, OperationStatus};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, State};

pub struct DiscoveryRuntime {
    pub coordinator: OperationCoordinator,
    cancellation: Mutex<Option<Arc<AtomicBool>>>,
}
impl Default for DiscoveryRuntime {
    fn default() -> Self {
        Self {
            coordinator: OperationCoordinator::default(),
            cancellation: Mutex::new(None),
        }
    }
}

impl DiscoveryRuntime {
    pub fn begin_cancellation(&self) -> Result<Arc<AtomicBool>, CommandError> {
        let flag = Arc::new(AtomicBool::new(false));
        *self.cancellation.lock().map_err(|_| {
            CommandError::new("operation_unavailable", "Operation state is unavailable")
        })? = Some(flag.clone());
        Ok(flag)
    }

    pub fn clear_cancellation(&self) {
        if let Ok(mut cancellation) = self.cancellation.lock() {
            *cancellation = None;
        }
    }
}

fn emit_progress(
    app: &AppHandle,
    project_id: &str,
    kind: DiscoveryProgressKind,
    message: &str,
    output_bytes: u64,
    cancellable: bool,
) {
    let _ = app.emit(
        "discovery-progress",
        DiscoveryProgress {
            project_id: project_id.to_string(),
            kind,
            message: message.to_string(),
            output_bytes,
            cancellable,
        },
    );
}

#[tauri::command]
pub fn start_update_check(
    app: AppHandle,
    database: State<'_, Database>,
    runtime: State<'_, DiscoveryRuntime>,
    project_id: String,
) -> Result<OperationStatus, CommandError> {
    let root = db::registered_project_path(&database, &project_id)?;
    let executable = compatibility::resolve_executable()?;
    let profile = compatibility::validate_executable(&executable);
    if !matches!(
        profile.status,
        crate::domain::CompatibilityStatus::Supported
    ) {
        return Err(profile.diagnostic.unwrap_or_else(|| {
            CommandError::new(
                "unsupported_compatibility",
                "Packwiz profile is unsupported",
            )
        }));
    }
    let root_path = Path::new(&root).canonicalize().map_err(|error| {
        CommandError::new(
            "project_unavailable",
            "Registered project root is unavailable",
        )
        .with_details(error.to_string())
    })?;
    let plan = process::command_plan(executable, root_path.clone())?;
    let lease = runtime.coordinator.acquire(&project_id)?;
    let _cancel = runtime.begin_cancellation()?;
    emit_progress(
        &app,
        &project_id,
        DiscoveryProgressKind::Starting,
        "Starting safe Packwiz discovery probe",
        0,
        true,
    );
    let runtime_cancel = runtime
        .cancellation
        .lock()
        .unwrap()
        .as_ref()
        .cloned()
        .unwrap();
    let project = project_id.clone();
    std::thread::spawn(move || {
        let _lease = lease;
        let before = fingerprint::collect(&root_path);
        emit_progress(
            &app,
            &project,
            DiscoveryProgressKind::Running,
            "Observing Packwiz output",
            0,
            true,
        );
        let evidence = if runtime_cancel.load(Ordering::Relaxed) {
            None
        } else {
            let mut limits = RunLimits::default();
            limits.cancel = Some(runtime_cancel.clone());
            process::run_probe(&plan, limits).ok()
        };
        if let Some(ref evidence) = evidence {
            let _ = app.emit("discovery-process", evidence);
            emit_progress(
                &app,
                &project,
                DiscoveryProgressKind::Cancelling,
                "Packwiz cancellation result captured",
                evidence.stdout.len() as u64 + evidence.stderr.len() as u64,
                false,
            );
        }
        let after = fingerprint::collect(&root_path);
        let unchanged = before
            .ok()
            .zip(after.ok())
            .map(|(before, after)| fingerprint::compare(before, after).unchanged)
            .unwrap_or(false);
        let message = if unchanged {
            "Probe finished with unchanged project evidence"
        } else {
            "Probe finished without a comparable unchanged fingerprint"
        };
        emit_progress(
            &app,
            &project,
            DiscoveryProgressKind::Finished,
            message,
            evidence
                .as_ref()
                .map(|item| item.stdout.len() as u64)
                .unwrap_or(0),
            false,
        );
    });
    Ok(OperationStatus::Running)
}

#[tauri::command]
pub fn cancel_update_check(
    runtime: State<'_, DiscoveryRuntime>,
) -> Result<OperationStatus, CommandError> {
    let cancellation = runtime.cancellation.lock().map_err(|_| {
        CommandError::new("operation_unavailable", "Operation state is unavailable")
    })?;
    if let Some(flag) = cancellation.as_ref() {
        flag.store(true, Ordering::Relaxed);
        Ok(OperationStatus::Cancelled)
    } else {
        Err(CommandError::new(
            "operation_not_running",
            "No discovery operation is running",
        ))
    }
}
