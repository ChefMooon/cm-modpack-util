import { invoke } from "@tauri-apps/api/core";

export async function loadSettings(): Promise<Record<string, string>> {
  return invoke<Record<string, string>>("get_settings");
}

export async function saveSetting(key: string, value: unknown): Promise<void> {
  await invoke("set_setting", { key, valueJson: JSON.stringify(value) });
}

export async function resetStoredSettings(): Promise<void> {
  await invoke("reset_settings");
}
