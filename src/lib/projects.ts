import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { open, save } from "@tauri-apps/plugin-dialog";
import { openUrl } from "@tauri-apps/plugin-opener";
import type {
  ApplicationProjectMetadata,
  ApplyOperationReport,
  ApplyOperationRequest,
  InventoryEntry,
  ProjectOverview,
  ProjectRecord,
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
  ChangelogRevisionRequest,
  ChangelogExport,
  ChangelogExportRequest,
  ChangelogProgress,
} from "./domain";

export async function chooseProjectDirectory(): Promise<string | null> {
  const selected = await open({ directory: true, multiple: false, title: "Select a Packwiz project" });
  return typeof selected === "string" ? selected : null;
}

export function previewProject(path: string): Promise<RegistrationPreview> {
  return invoke("preview_project", { path });
}

export function registerProject(path: string, application?: ApplicationProjectMetadata): Promise<ProjectRecord> {
  return invoke("register_project", { path, application });
}

export function listProjects(): Promise<ProjectRecord[]> {
  return invoke("list_projects");
}

export function refreshProject(id: string): Promise<ProjectRecord> {
  return invoke("refresh_project", { id });
}

export function getProjectInventory(id: string): Promise<InventoryEntry[]> {
  return invoke("get_project_inventory", { id });
}

export function getProjectOverview(id: string): Promise<ProjectOverview> {
  return invoke("get_project_overview", { id });
}

export function checkForUpdates(id: string): Promise<DiscoveryResult> {
  return invoke("check_for_updates", { projectId: id });
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

export function updateProjectMetadata(id: string, application: ApplicationProjectMetadata): Promise<ProjectRecord> {
  return invoke("update_project_metadata", { id, application });
}

export function archiveProject(id: string): Promise<ProjectRecord> {
  return invoke("archive_project", { id });
}

export function restoreProject(id: string): Promise<ProjectRecord> {
  return invoke("restore_project", { id });
}

export function disconnectProject(id: string): Promise<ProjectRecord> {
  return invoke("disconnect_project", { id });
}

export function reconnectProject(id: string, path: string): Promise<ProjectRecord> {
  return invoke("reconnect_project", { id, path });
}

export function listSnapshots(projectId: string): Promise<SnapshotRecord[]> {
  return invoke("list_snapshots", { projectId });
}

export function getSnapshot(id: string): Promise<SnapshotRecord> {
  return invoke("get_snapshot", { id });
}

export function setSnapshotDecision(snapshotId: string, candidateId: string, decision: SnapshotDecision, note?: string): Promise<void> {
  return invoke("set_snapshot_decision", { snapshotId, candidateId, decision, note });
}

export function saveSnapshotNote(projectId: string, scope: SnapshotNoteScope, note: string, snapshotId?: string, candidateId?: string): Promise<void> {
  return invoke("save_snapshot_note", { projectId, scope, note, snapshotId, candidateId });
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

export function pinProject(request: PinOperationRequest): Promise<OperationAttempt> {
  return invoke("pin_project", { request });
}

export function unpinProject(request: PinOperationRequest): Promise<OperationAttempt> {
  return invoke("unpin_project", { request });
}

export function cancelOperation(operationId: string): Promise<void> {
  return invoke("cancel_operation", { operationId });
}

export function getOperationHistory(projectId: string): Promise<OperationAttempt[]> {
  return invoke("get_operation_history", { projectId });
}

export function recordRecoveryAcknowledgement(acknowledgement: RecoveryAcknowledgement): Promise<void> {
  return invoke("record_recovery_acknowledgement", { acknowledgement });
}

export function applyProject(request: ApplyOperationRequest): Promise<ApplyOperationReport> {
  return invoke("apply_project", { request });
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

export function exportChangelog(request: ChangelogExportRequest): Promise<ChangelogExport> {
  return invoke("export_changelog", { request });
}

export async function chooseChangelogDestination(): Promise<string | null> {
  return save({ title: "Export Markdown changelog", defaultPath: "CHANGELOG.md", filters: [{ name: "Markdown", extensions: ["md"] }] });
}

export function getChangelogArtifact(artifactId: string): Promise<ChangelogArtifact> {
  return invoke("get_changelog_artifact", { artifactId });
}

export function listChangelogArtifacts(projectId: string): Promise<ChangelogArtifact[]> {
  return invoke("list_changelog_artifacts", { projectId });
}

export function listChangelogRevisions(artifactId: string): Promise<ChangelogRevision[]> {
  return invoke("list_changelog_revisions", { artifactId });
}

export function listChangelogExports(artifactId: string): Promise<ChangelogExport[]> {
  return invoke("list_changelog_exports", { artifactId });
}