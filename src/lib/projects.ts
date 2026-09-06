import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { openUrl } from "@tauri-apps/plugin-opener";
import type {
  ApplicationProjectMetadata,
  InventoryEntry,
  ProjectOverview,
  ProjectRecord,
  RegistrationPreview,
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