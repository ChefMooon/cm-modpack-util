import { getVersion } from "@tauri-apps/api/app";
import packageJson from "../../package.json";

export type AppVersionStatus = "loading" | "available" | "unavailable";

export const appVersion = $state<{
  value: string | null;
  status: AppVersionStatus;
}>({
  value: packageJson.version,
  status: "available",
});

let loadRequest: Promise<string | null> | undefined;

export function formatAppVersion(version: string | null): string {
  return version ? `v${version}` : "Version unavailable";
}

export function loadAppVersion(): Promise<string | null> {
  if (loadRequest) return loadRequest;

  loadRequest = getVersion()
    .then((version) => {
      appVersion.value = version;
      appVersion.status = "available";
      return version;
    })
    .catch(() => {
      appVersion.value = packageJson.version;
      appVersion.status = "available";
      return packageJson.version;
    });

  return loadRequest;
}
