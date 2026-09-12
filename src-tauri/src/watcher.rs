use crate::db::{self, Database};
use crate::discovery::fingerprint;
use crate::domain::{
    CommandError, ReleaseWorkspaceEvidenceFreshness, ReleaseWorkspaceLifecycle,
    ReleaseWorkspaceObservation,
};
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::sync::Mutex;
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, State};

const DEBOUNCE: Duration = Duration::from_millis(250);
const STABILITY_DELAY: Duration = Duration::from_millis(150);

struct WatchHandle {
    stop: Sender<()>,
}

#[derive(Default)]
pub struct ReleaseWatcherCoordinator {
    watchers: Mutex<HashMap<String, WatchHandle>>,
}

#[derive(Debug, Clone, Serialize)]
struct WatcherError {
    workspace_id: String,
    message: String,
}

impl ReleaseWatcherCoordinator {
    pub fn start(
        &self,
        app: AppHandle,
        workspace_id: String,
        root: PathBuf,
    ) -> Result<(), CommandError> {
        let mut watchers = self.watchers.lock().map_err(|_| {
            CommandError::new(
                "watcher_unavailable",
                "Release watcher coordination is unavailable",
            )
        })?;
        if watchers.contains_key(&workspace_id) {
            return Ok(());
        }
        let (stop, stop_receiver) = mpsc::channel();
        let worker_workspace_id = workspace_id.clone();
        thread::Builder::new()
            .name(format!("release-watcher-{workspace_id}"))
            .spawn(move || run_worker(app, worker_workspace_id, root, stop_receiver))
            .map_err(|error| {
                CommandError::new("watcher_start_failed", "Release watcher could not start")
                    .with_details(error.to_string())
            })?;
        watchers.insert(workspace_id, WatchHandle { stop });
        Ok(())
    }

    pub fn stop(&self, workspace_id: &str) {
        if let Ok(mut watchers) = self.watchers.lock() {
            if let Some(handle) = watchers.remove(workspace_id) {
                let _ = handle.stop.send(());
            }
        }
    }

    pub fn stop_all(&self) {
        if let Ok(mut watchers) = self.watchers.lock() {
            for (_, handle) in watchers.drain() {
                let _ = handle.stop.send(());
            }
        }
    }
}

fn run_worker(
    app: AppHandle,
    workspace_id: String,
    root: PathBuf,
    stop_receiver: mpsc::Receiver<()>,
) {
    let (event_sender, event_receiver) = mpsc::channel();
    let mut watcher = match RecommendedWatcher::new(event_sender, Config::default()) {
        Ok(watcher) => watcher,
        Err(error) => {
            emit_error(&app, &workspace_id, error.to_string());
            return;
        }
    };
    for (path, mode) in watch_scope(&root)
        .into_iter()
        .filter(|(path, _)| path.exists())
    {
        if let Err(error) = watcher.watch(&path, mode) {
            emit_error(&app, &workspace_id, error.to_string());
            return;
        }
    }

    loop {
        if stop_receiver.try_recv().is_ok() {
            break;
        }
        let first = match event_receiver.recv_timeout(Duration::from_millis(100)) {
            Ok(Ok(event)) => event,
            Ok(Err(error)) => {
                emit_error(&app, &workspace_id, error.to_string());
                continue;
            }
            Err(RecvTimeoutError::Timeout) => continue,
            Err(RecvTimeoutError::Disconnected) => break,
        };
        let mut paths = first.paths;
        while let Ok(result) = event_receiver.recv_timeout(DEBOUNCE) {
            if let Ok(event) = result {
                paths.extend(event.paths);
            }
        }
        paths.sort();
        paths.dedup();
        let relevant = paths
            .iter()
            .filter(|path| relevant_path(&root, path))
            .cloned()
            .collect::<Vec<_>>();
        if relevant.is_empty() {
            continue;
        }
        observe(&app, &workspace_id, &root, &relevant);
    }
}

fn observe(app: &AppHandle, workspace_id: &str, root: &Path, paths: &[PathBuf]) {
    let database = app.state::<Database>();
    let workspace = match db::load_release_workspace_inner(&database, workspace_id) {
        Ok(workspace) => workspace,
        Err(error) => {
            emit_error(app, workspace_id, error.message);
            return;
        }
    };
    if !watchable(&workspace.lifecycle) {
        return;
    }
    let overlapped_operation = matches!(workspace.lifecycle, ReleaseWorkspaceLifecycle::Applying);
    let scope = paths
        .iter()
        .filter_map(|path| path.strip_prefix(root).ok())
        .map(|path| path.to_string_lossy().replace('\\', "/"))
        .collect::<Vec<_>>();
    let first = fingerprint::collect(root);
    thread::sleep(STABILITY_DELAY);
    let second = fingerprint::collect(root);
    let (freshness, fingerprint_changed, observed_fingerprint) = match (first, second) {
        (Ok(before), Ok(after)) => {
            let comparison = fingerprint::compare(before, after);
            let returned_to_baseline = workspace
                .baseline_capture
                .as_ref()
                .map(|capture| capture.state.source_fingerprint.clone())
                .as_ref()
                .map(|baseline| matches_baseline(Some(baseline), &comparison.after))
                .unwrap_or(false);
            if comparison.unchanged && returned_to_baseline {
                (
                    ReleaseWorkspaceEvidenceFreshness::Current,
                    false,
                    Some(comparison.after),
                )
            } else {
                (
                    ReleaseWorkspaceEvidenceFreshness::Stale,
                    true,
                    Some(comparison.after),
                )
            }
        }
        _ => (ReleaseWorkspaceEvidenceFreshness::Unavailable, false, None),
    };
    let blocking_reason = observation_blocking_reason(
        fingerprint_changed,
        overlapped_operation,
        observed_fingerprint.is_some(),
    )
    .map(str::to_string);
    let observation = ReleaseWorkspaceObservation {
        workspace_id: workspace_id.to_string(),
        changed_scope: scope,
        evidence_freshness: freshness,
        blocking_reason,
        overlapped_operation,
        fingerprint: observed_fingerprint,
    };
    if let Err(error) = db::record_workspace_observation(&database, &observation) {
        emit_error(app, workspace_id, error.message);
        return;
    }
    let _ = app.emit("release-workspace-observation", observation);
}

fn matches_baseline(
    baseline: Option<&crate::domain::ModpackFingerprint>,
    observed: &crate::domain::ModpackFingerprint,
) -> bool {
    baseline
        .map(|baseline| fingerprint::compare(baseline.clone(), observed.clone()).unchanged)
        .unwrap_or(false)
}

fn observation_blocking_reason(
    fingerprint_changed: bool,
    overlapped_operation: bool,
    fingerprint_available: bool,
) -> Option<&'static str> {
    if fingerprint_changed {
        Some(if overlapped_operation {
            "app_owned_apply_overlap"
        } else {
            "external_change_detected"
        })
    } else if !fingerprint_available {
        Some("external_evidence_unavailable")
    } else {
        None
    }
}

fn relevant_path(root: &Path, path: &Path) -> bool {
    let relative = path.strip_prefix(root).unwrap_or(path);
    let normalized = relative.to_string_lossy().replace('\\', "/");
    if normalized == "pack.toml" || normalized == "index.toml" {
        return true;
    }
    if normalized.ends_with(".pw.toml") {
        return true;
    }
    fingerprint::RELEVANT_DIRECTORY_ROOTS
        .iter()
        .any(|directory| {
            normalized == *directory || normalized.starts_with(&format!("{directory}/"))
        })
}

fn watch_scope(root: &Path) -> Vec<(PathBuf, RecursiveMode)> {
    let mut paths = Vec::new();
    let pack = root.join("pack.toml");
    paths.push((pack.clone(), RecursiveMode::NonRecursive));
    let index = std::fs::read_to_string(&pack)
        .ok()
        .and_then(|text| text.parse::<toml::Value>().ok())
        .and_then(|pack| {
            pack.get("index")?
                .get("file")?
                .as_str()
                .map(|name| root.join(name))
        });
    if let Some(index_path) = index {
        paths.push((index_path.clone(), RecursiveMode::NonRecursive));
        if let Ok(text) = std::fs::read_to_string(index_path) {
            if let Ok(index) = text.parse::<toml::Value>() {
                if let Some(files) = index.get("files").and_then(toml::Value::as_array) {
                    for metadata in files.iter().filter_map(toml::Value::as_table) {
                        if metadata.get("metafile").and_then(toml::Value::as_bool) == Some(true) {
                            if let Some(name) = metadata.get("file").and_then(toml::Value::as_str) {
                                paths.push((root.join(name), RecursiveMode::NonRecursive));
                            }
                        }
                    }
                }
            }
        }
    }
    for directory in fingerprint::RELEVANT_DIRECTORY_ROOTS {
        paths.push((root.join(directory), RecursiveMode::Recursive));
    }
    paths
}

fn watchable(lifecycle: &ReleaseWorkspaceLifecycle) -> bool {
    matches!(
        lifecycle,
        ReleaseWorkspaceLifecycle::Draft
            | ReleaseWorkspaceLifecycle::Applying
            | ReleaseWorkspaceLifecycle::RecoveryRequired
            | ReleaseWorkspaceLifecycle::Provisional
            | ReleaseWorkspaceLifecycle::ReadyToFinalize
    )
}

fn emit_error(app: &AppHandle, workspace_id: &str, message: String) {
    let _ = app.emit(
        "release-workspace-watcher-error",
        WatcherError {
            workspace_id: workspace_id.to_string(),
            message,
        },
    );
}

#[tauri::command]
pub fn start_release_workspace_watcher(
    app: AppHandle,
    database: State<'_, Database>,
    coordinator: State<'_, ReleaseWatcherCoordinator>,
    workspace_id: String,
) -> Result<(), CommandError> {
    let workspace = db::load_release_workspace_inner(&database, &workspace_id)?;
    if !watchable(&workspace.lifecycle) {
        return Err(CommandError::new(
            "watcher_not_allowed",
            "Only editable release workspaces can be observed",
        ));
    }
    let path = db::registered_modpack_path(&database, &workspace.modpack_id)?;
    coordinator.start(app, workspace_id, PathBuf::from(path))
}

#[tauri::command]
pub fn stop_release_workspace_watcher(
    coordinator: State<'_, ReleaseWatcherCoordinator>,
    workspace_id: String,
) -> Result<(), CommandError> {
    coordinator.stop(&workspace_id);
    Ok(())
}

#[tauri::command]
pub fn reconcile_release_workspace_watchers(
    app: AppHandle,
    database: State<'_, Database>,
    coordinator: State<'_, ReleaseWatcherCoordinator>,
) -> Result<u32, CommandError> {
    let workspaces = db::list_release_workspaces_for_watch(&database)?;
    let mut started = 0;
    for (workspace_id, modpack_id) in workspaces {
        let path = db::registered_modpack_path(&database, &modpack_id)?;
        coordinator.start(app.clone(), workspace_id, PathBuf::from(path))?;
        started += 1;
    }
    Ok(started)
}

#[cfg(test)]
mod tests {
    use super::{
        matches_baseline, observation_blocking_reason, relevant_path, watch_scope, watchable,
    };
    use crate::domain::{FingerprintEntry, ModpackFingerprint, ReleaseWorkspaceLifecycle};
    use std::path::Path;

    #[test]
    fn relevant_path_matches_packwiz_boundary_only() {
        let root = Path::new("C:/packs/demo");
        assert!(relevant_path(root, &root.join("pack.toml")));
        assert!(relevant_path(root, &root.join("mods/example.pw.toml")));
        assert!(relevant_path(root, &root.join("resourcepacks/assets/a")));
        assert!(!relevant_path(root, &root.join("README.md")));
    }

    #[test]
    fn historical_workspaces_are_not_watchable() {
        assert!(watchable(&ReleaseWorkspaceLifecycle::Draft));
        assert!(!watchable(&ReleaseWorkspaceLifecycle::Finalized));
        assert!(!watchable(&ReleaseWorkspaceLifecycle::Published));
        assert!(!watchable(&ReleaseWorkspaceLifecycle::Withdrawn));
        assert!(!watchable(&ReleaseWorkspaceLifecycle::Abandoned));
    }

    #[test]
    fn watch_scope_does_not_watch_the_registered_root() {
        let paths = watch_scope(Path::new("C:/packs/demo"));
        assert!(paths
            .iter()
            .all(|(path, _)| path != Path::new("C:/packs/demo")));
    }

    #[test]
    fn baseline_match_clears_reverted_external_change() {
        let baseline = ModpackFingerprint {
            root: "root".into(),
            entries: vec![FingerprintEntry {
                relative_path: "mods/example.pw.toml".into(),
                kind: "file".into(),
                size: Some(1),
                modified_ns: Some(1),
                content_hash: Some("same".into()),
            }],
            complete: true,
            diagnostic: None,
        };
        assert!(matches_baseline(Some(&baseline), &baseline));
        assert!(!matches_baseline(None, &baseline));
    }

    #[test]
    fn distinguishes_app_owned_overlap_from_external_change() {
        assert_eq!(
            observation_blocking_reason(true, true, true),
            Some("app_owned_apply_overlap")
        );
        assert_eq!(
            observation_blocking_reason(true, false, true),
            Some("external_change_detected")
        );
        assert_eq!(
            observation_blocking_reason(false, true, false),
            Some("external_evidence_unavailable")
        );
        assert_eq!(observation_blocking_reason(false, true, true), None);
    }
}
