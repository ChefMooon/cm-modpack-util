import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { open, save } from "@tauri-apps/plugin-dialog";
import { openUrl } from "@tauri-apps/plugin-opener";
import type {
  ApplicationModpackMetadata,
  ApplyOperationReport,
  ApplyOperationRequest,
  InventoryEntry,
  ModpackOverview,
  ModpackObservation,
  ModpackRecord,
  RegistrationPreview,
  DiscoveryResult,
  DiscoveryProgress,
  ProcessEvidence,
  SnapshotRecord,
  SnapshotDecision,
  SnapshotNoteScope,
  OperationAttempt,
  PinOperationRequest,
  RecoveryAcknowledgement,
  ChangelogArtifact,
  ChangelogGenerationRequest,
  ChangelogRevision,
  ChangelogRevisionArchiveRequest,
  ChangelogRevisionRequest,
  ChangelogSelectionRevisionRequest,
  ChangelogExport,
  ChangelogExportRequest,
  ChangelogProgress,
  ReleaseComparison,
  ReleaseComparisonRequest,
  ReleaseCreateRequest,
  ReleaseRecord,
  ReleaseUpdateRequest,
  ReleaseWorkspace,
  StartReleaseWorkspaceRequest,
  RebaseReleaseWorkspaceRequest,
  ReleaseWorkspaceDecisionRequest,
  FinalizeReleaseWorkspaceRequest,
  SelectReleaseWorkspaceChangelogRequest,
  CreateReleaseWorkspaceChangelogRequest,
  PublishReleaseWorkspaceRequest,
  WithdrawReleaseWorkspaceRequest,
  ReleaseWorkspaceObservation,
  ReleaseWorkspaceObservationRecord,
  ReleaseWorkspaceEvidence,
  ReleaseWorkspaceActivityPage,
  CleanupPlan,
  CleanupRequest,
  CleanupResult,
  CleanupScope,
  ImpactPreview,
  LifecycleAction,
  LifecycleRecordType,
  LifecycleRequest,
  LifecycleResult,
  StorageReport,
} from "./domain";

export async function chooseModpackDirectory(): Promise<string | null> {
  const selected = await open({ directory: true, multiple: false, title: "Select a Packwiz modpack" });
  return typeof selected === "string" ? selected : null;
}

export function previewModpack(path: string): Promise<RegistrationPreview> {
  return invoke("preview_modpack", { path });
}

export function registerModpack(path: string, application?: ApplicationModpackMetadata): Promise<ModpackRecord> {
  return invoke("register_modpack", { path, application });
}

export function listModpacks(): Promise<ModpackRecord[]> {
  return invoke("list_modpacks");
}

export function refreshModpack(id: string): Promise<ModpackRecord> {
  return invoke("refresh_modpack", { id });
}

export function getModpackInventory(id: string): Promise<InventoryEntry[]> {
  return invoke("get_modpack_inventory", { id });
}

export function getModpackOverview(id: string): Promise<ModpackOverview> {
  return invoke("get_modpack_overview", { id });
}

export function getModpackObservation(id: string): Promise<ModpackObservation> {
  return invoke("get_modpack_observation", { id });
}

export function checkForUpdates(id: string): Promise<DiscoveryResult> {
  return invoke("check_for_updates", { modpackId: id });
}

export function cancelUpdateCheck(): Promise<"cancelled"> {
  return invoke("cancel_update_check");
}

export function listenUpdateCheckProgress(
  handler: (progress: DiscoveryProgress) => void,
): Promise<UnlistenFn> {
  return listen<DiscoveryProgress>("discovery-progress", (event) => handler(event.payload));
}

export function listenUpdateCheckProcess(
  handler: (process: ProcessEvidence) => void,
): Promise<UnlistenFn> {
  return listen<ProcessEvidence>("discovery-process", (event) => handler(event.payload));
}

export function openTrustedPage(url: string): Promise<void> {
  return openUrl(url);
}

export function updateModpackMetadata(id: string, application: ApplicationModpackMetadata): Promise<ModpackRecord> {
  return invoke("update_modpack_metadata", { id, application });
}

export function archiveModpack(id: string): Promise<ModpackRecord> {
  return invoke("archive_modpack", { id });
}

export function restoreModpack(id: string): Promise<ModpackRecord> {
  return invoke("restore_modpack", { id });
}

export function disconnectModpack(id: string): Promise<ModpackRecord> {
  return invoke("disconnect_modpack", { id });
}

export function reconnectModpack(id: string, path: string): Promise<ModpackRecord> {
  return invoke("reconnect_modpack", { id, path });
}

export function listModpackSnapshots(modpackId: string): Promise<SnapshotRecord[]> {
  return invoke("list_snapshots", { modpackId });
}

export function getSnapshot(id: string): Promise<SnapshotRecord> {
  return invoke("get_snapshot", { id });
}

export function setSnapshotDecision(snapshotId: string, candidateId: string, decision: SnapshotDecision, note?: string): Promise<void> {
  return invoke("set_snapshot_decision", { snapshotId, candidateId, decision, note });
}

export function saveModpackSnapshotNote(modpackId: string, scope: SnapshotNoteScope, note: string, snapshotId?: string, candidateId?: string): Promise<void> {
  return invoke("save_snapshot_note", { modpackId, scope, note, snapshotId, candidateId });
}

export function closeSnapshot(id: string, cancelled: boolean): Promise<SnapshotRecord> {
  return invoke("close_snapshot", { id, cancelled });
}

export function linkSnapshotRetry(predecessorId: string, retryId: string): Promise<SnapshotRecord> {
  return invoke("link_snapshot_retry", { predecessorId, retryId });
}

export function recheckSnapshot(id: string): Promise<SnapshotRecord> {
  return invoke("recheck_snapshot", { id });
}

export function pinModpackEntry(request: PinOperationRequest): Promise<OperationAttempt> {
  return invoke("pin_modpack", { request });
}

export function unpinModpackEntry(request: PinOperationRequest): Promise<OperationAttempt> {
  return invoke("unpin_modpack", { request });
}

export function cancelOperation(operationId: string): Promise<void> {
  return invoke("cancel_operation", { operationId });
}

export function getModpackOperationHistory(modpackId: string): Promise<OperationAttempt[]> {
  return invoke("get_operation_history", { modpackId });
}

export function recordRecoveryAcknowledgement(acknowledgement: RecoveryAcknowledgement): Promise<void> {
  return invoke("record_recovery_acknowledgement", { acknowledgement });
}

export function applyModpack(request: ApplyOperationRequest): Promise<ApplyOperationReport> {
  return invoke("apply_modpack", { request });
}

export function generateChangelog(request: ChangelogGenerationRequest): Promise<ChangelogArtifact> {
  return invoke("generate_changelog", { request });
}

export function cancelChangelogGeneration(): Promise<void> {
  return invoke("cancel_changelog_generation");
}

export function listenChangelogProgress(
  handler: (progress: ChangelogProgress) => void,
): Promise<UnlistenFn> {
  return listen<ChangelogProgress>("changelog-progress", (event) => handler(event.payload));
}

export function createChangelogRevision(request: ChangelogRevisionRequest): Promise<ChangelogRevision> {
  return invoke("create_changelog_revision", { request });
}

export function createChangelogSelectionRevision(request: ChangelogSelectionRevisionRequest): Promise<ChangelogRevision> {
  return invoke("create_changelog_selection_revision", { request });
}

export function archiveChangelogRevision(request: ChangelogRevisionArchiveRequest): Promise<ChangelogRevision> {
  return invoke("archive_changelog_revision", { request });
}

export function exportChangelog(request: ChangelogExportRequest): Promise<ChangelogExport> {
  return invoke("export_changelog", { request });
}

export async function chooseChangelogDestination(): Promise<string | null> {
  return save({ title: "Export Markdown changelog", defaultPath: "CHANGELOG.md", filters: [{ name: "Markdown", extensions: ["md"] }] });
}

export function getChangelogArtifact(artifactId: string): Promise<ChangelogArtifact> {
  return invoke("get_changelog_artifact", { artifactId });
}

export function listChangelogArtifacts(modpackId: string): Promise<ChangelogArtifact[]> {
  return invoke("list_changelog_artifacts", { modpackId });
}

export function listChangelogRevisions(artifactId: string): Promise<ChangelogRevision[]> {
  return invoke("list_changelog_revisions", { artifactId });
}

export function listChangelogExports(artifactId: string): Promise<ChangelogExport[]> {
  return invoke("list_changelog_exports", { artifactId });
}

export function createRelease(request: ReleaseCreateRequest): Promise<ReleaseRecord> {
  return invoke("create_release", { request });
}

export function getRelease(id: string): Promise<ReleaseRecord> {
  return invoke("get_release", { id });
}

export function listReleases(modpackId: string): Promise<ReleaseRecord[]> {
  return invoke("list_releases_command", { modpackId });
}

export function updateRelease(request: ReleaseUpdateRequest): Promise<ReleaseRecord> {
  return invoke("update_release", { request });
}

export function compareReleases(request: ReleaseComparisonRequest): Promise<ReleaseComparison> {
  return invoke("compare_releases", { request });
}

export function startReleaseWorkspace(request: StartReleaseWorkspaceRequest): Promise<ReleaseWorkspace> {
  return invoke("start_release_workspace", { request });
}

export function loadReleaseWorkspace(id: string): Promise<ReleaseWorkspace> {
  return invoke("load_release_workspace", { id });
}

export function listReleaseWorkspaceActivity(workspaceId: string, beforeId: number | null = null, limit = 10): Promise<ReleaseWorkspaceActivityPage> {
  return invoke("list_release_workspace_activity", { workspaceId, beforeId, limit });
}

export function listReleaseWorkspaceObservations(workspaceId: string): Promise<ReleaseWorkspaceObservationRecord[]> {
  return invoke("list_release_workspace_observations", { workspaceId });
}

export function getReleaseWorkspaceEvidence(workspaceId: string): Promise<ReleaseWorkspaceEvidence> {
  return invoke("get_release_workspace_evidence", { workspaceId });
}

export function listReleaseWorkspaces(modpackId: string): Promise<ReleaseWorkspace[]> {
  return invoke("list_release_workspaces", { modpackId });
}

export function abandonReleaseWorkspace(workspaceId: string): Promise<ReleaseWorkspace> {
  return invoke("abandon_release_workspace", { workspaceId });
}

export function unlinkSnapshotFromReleaseWorkspace(workspaceId: string): Promise<ReleaseWorkspace> {
  return invoke("unlink_snapshot_from_release_workspace", { workspaceId });
}

export function rebaseReleaseWorkspace(request: RebaseReleaseWorkspaceRequest): Promise<ReleaseWorkspace> {
  return invoke("rebase_release_workspace", { request });
}

export function linkSnapshotToReleaseWorkspace(workspaceId: string, snapshotId: string): Promise<ReleaseWorkspace> {
  return invoke("link_snapshot_to_release_workspace", { workspaceId, snapshotId });
}

export function setReleaseWorkspaceDecision(request: ReleaseWorkspaceDecisionRequest): Promise<ReleaseWorkspace> {
  return invoke("set_release_workspace_decision", { request });
}

export function finalizeReleaseWorkspace(request: FinalizeReleaseWorkspaceRequest): Promise<ReleaseWorkspace> {
  return invoke("finalize_release_workspace", { request });
}

export function selectReleaseWorkspaceChangelog(request: SelectReleaseWorkspaceChangelogRequest): Promise<ReleaseWorkspace> {
  return invoke("select_release_workspace_changelog", { request });
}

export function createReleaseWorkspaceChangelog(request: CreateReleaseWorkspaceChangelogRequest): Promise<ChangelogArtifact> {
  return invoke("create_release_workspace_changelog", { request });
}

export function publishReleaseWorkspace(request: PublishReleaseWorkspaceRequest): Promise<ReleaseWorkspace> {
  return invoke("publish_release_workspace", { request });
}

export function withdrawReleaseWorkspace(request: WithdrawReleaseWorkspaceRequest): Promise<ReleaseWorkspace> {
  return invoke("withdraw_release_workspace", { request });
}

export function startReleaseWorkspaceWatcher(workspaceId: string): Promise<void> {
  return invoke("start_release_workspace_watcher", { workspaceId });
}

export function stopReleaseWorkspaceWatcher(workspaceId: string): Promise<void> {
  return invoke("stop_release_workspace_watcher", { workspaceId });
}

export function reconcileReleaseWorkspaceWatchers(): Promise<number> {
  return invoke("reconcile_release_workspace_watchers");
}

export function listenReleaseWorkspaceObservation(
  handler: (observation: ReleaseWorkspaceObservation) => void,
): Promise<UnlistenFn> {
  return listen<ReleaseWorkspaceObservation>("release-workspace-observation", (event) => handler(event.payload));
}

export function listenReleaseWorkspaceWatcherError(
  handler: (error: { workspace_id: string; message: string }) => void,
): Promise<UnlistenFn> {
  return listen<{ workspace_id: string; message: string }>("release-workspace-watcher-error", (event) => handler(event.payload));
}

export function previewLifecycleAction(
  targetType: LifecycleRecordType,
  targetId: string,
  action: LifecycleAction,
): Promise<ImpactPreview> {
  return invoke("preview_lifecycle_action", { targetType, targetId, action });
}

export function applyLifecycleAction(request: LifecycleRequest): Promise<LifecycleResult> {
  return invoke("apply_lifecycle_action", { request });
}

export function previewCleanup(scope: CleanupScope): Promise<CleanupPlan> {
  return invoke("preview_cleanup", { scope });
}

export function executeCleanup(request: CleanupRequest): Promise<CleanupResult> {
  return invoke("execute_cleanup", { request });
}

export function getStorageReport(): Promise<StorageReport> {
  return invoke("get_storage_report");
}