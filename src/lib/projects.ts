import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
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