import { check, type DownloadEvent, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { saveSetting } from "./settings";
import { dismiss, toast, update } from "../components/ui/toast/toast.svelte";
import type { ToastAction } from "../components/ui/toast/types";
import { canStartUpdateOperation, shouldNotifyAvailableUpdate, type UpdateErrorCategory, type UpdateOperation, type UpdateState } from "./updates";

export type UpdateSnapshot = {
  state: UpdateState;
  currentVersion: string;
  availableVersion: string | null;
  releaseDate: string | null;
  releaseNotes: string | null;
  releaseUrl: string | null;
  lastCheckedAt: string | null;
  downloadedBytes: number;
  contentLength: number | null;
  downloadReady: boolean;
  installPromptOpen: boolean;
  error: { category: UpdateErrorCategory; message: string } | null;
  detailsOpen: boolean;
};

const initialSnapshot: UpdateSnapshot = {
  state: "idle",
  currentVersion: "",
  availableVersion: null,
  releaseDate: null,
  releaseNotes: null,
  releaseUrl: null,
  lastCheckedAt: null,
  downloadedBytes: 0,
  contentLength: null,
  downloadReady: false,
  installPromptOpen: false,
  error: null,
  detailsOpen: false,
};

export const updateSnapshot = $state<UpdateSnapshot>({ ...initialSnapshot });

let currentUpdate: Update | null = null;
let operation: UpdateOperation | null = null;
let updateToastId: string | null = null;
let deferredThisSession = false;

function setState(state: UpdateState) {
  updateSnapshot.state = state;
}

function errorCategory(error: unknown): UpdateErrorCategory {
  const message = error instanceof Error ? error.message.toLowerCase() : String(error).toLowerCase();
  if (message.includes("signature") || message.includes("public key")) return "signature_rejection";
  if (message.includes("download") || message.includes("network")) return "download_failure";
  if (message.includes("install")) return "install_failure";
  if (message.includes("json") || message.includes("manifest") || message.includes("version")) return "invalid_metadata";
  return "manifest_failure";
}

function errorMessage(category: UpdateErrorCategory) {
  return {
    manifest_failure: "The release feed could not be reached.",
    invalid_metadata: "The release feed returned invalid update metadata.",
    signature_rejection: "The downloaded update did not pass signature verification.",
    download_failure: "The update download did not complete.",
    install_failure: "The update could not be installed.",
    relaunch_failure: "The installed update could not be relaunched.",
    runtime_unavailable: "Updates are unavailable outside the Tauri desktop runtime.",
  }[category];
}

function laterAction(): ToastAction {
  return {
    label: "Later",
    dismiss: false,
    onclick: () => {
      deferredThisSession = true;
      setState("later");
      updateSnapshot.installPromptOpen = false;
      if (updateSnapshot.availableVersion) {
        void saveSetting("updates.lastDeferredVersion", updateSnapshot.availableVersion);
      }
      if (updateToastId) {
        dismiss(updateToastId);
        updateToastId = null;
      }
    },
  };
}

function viewUpdateAction(): ToastAction {
  return {
    label: "View update",
    dismiss: false,
    onclick: () => openUpdateDetails(),
  };
}

function showToast(options: Parameters<typeof toast>[0]) {
  if (updateToastId) {
    update(updateToastId, options);
    return;
  }
  updateToastId = toast(options);
}

function showAvailableToast() {
  if (deferredThisSession) return;
  showToast({
    title: `Version ${updateSnapshot.availableVersion} is available`,
    description: "Review the release notes before downloading and installing it.",
    duration: 0,
    action: viewUpdateAction(),
    secondaryAction: laterAction(),
  });
}

function showFailure(category: UpdateErrorCategory) {
  updateSnapshot.error = { category, message: errorMessage(category) };
  setState(category === "manifest_failure" ? "offline" : "failed");
  showToast({
    title: "Update check failed",
    description: updateSnapshot.error.message,
    severity: "error",
    duration: 0,
    action: {
      label: "Retry",
      dismiss: false,
      onclick: async () => {
        await checkForUpdates(true);
      },
    },
  });
}

export async function checkForUpdates(notify = false): Promise<UpdateSnapshot> {
  if (!canStartUpdateOperation(operation, "check")) return updateSnapshot;
  operation = "check";
  updateSnapshot.error = null;
  setState("checking");
  try {
    const result = await check();
    updateSnapshot.lastCheckedAt = new Date().toISOString();
    if (!result) {
      currentUpdate = null;
      updateSnapshot.availableVersion = null;
      updateSnapshot.releaseDate = null;
      updateSnapshot.releaseNotes = null;
      updateSnapshot.releaseUrl = null;
      setState("up_to_date");
      if (updateToastId) {
        dismiss(updateToastId);
        updateToastId = null;
      }
      return updateSnapshot;
    }

    currentUpdate = result;
    updateSnapshot.currentVersion = result.currentVersion;
    updateSnapshot.availableVersion = result.version;
    updateSnapshot.releaseDate = result.date ?? null;
    updateSnapshot.releaseNotes = result.body ?? null;
    updateSnapshot.releaseUrl =
      typeof result.rawJson.url === "string"
        ? result.rawJson.url
        : "https://github.com/ChefMooon/cm-modpack-util/releases";
    setState("available");

    const settings = await import("./settings").then(({ loadSettings }) => loadSettings());
    const lastNotified = settings["updates.lastNotifiedVersion"];
    const lastDeferred = settings["updates.lastDeferredVersion"];
    const shouldNotify = shouldNotifyAvailableUpdate(notify, lastNotified, lastDeferred, result.version);
    if (shouldNotify) {
      await saveSetting("updates.lastNotifiedVersion", result.version);
      showAvailableToast();
    }
  } catch (error) {
    const category = errorCategory(error);
    if (typeof window === "undefined" || !("__TAURI_INTERNALS__" in window)) {
      updateSnapshot.error = { category: "runtime_unavailable", message: errorMessage("runtime_unavailable") };
      setState("failed");
    } else {
      showFailure(category);
    }
  } finally {
    operation = null;
  }
  return updateSnapshot;
}

export async function downloadUpdate(): Promise<void> {
  if (!canStartUpdateOperation(operation, "install") || !currentUpdate || updateSnapshot.state === "ready_to_restart") return;

  operation = "install";
  updateSnapshot.detailsOpen = false;
  updateSnapshot.error = null;
  updateSnapshot.downloadedBytes = 0;
  updateSnapshot.contentLength = null;
  updateSnapshot.downloadReady = false;
  setState("downloading");
  showToast({
    title: "Downloading update",
    description: "The verified installer will be downloaded before installation.",
    duration: 0,
  });
  try {
    await currentUpdate.download((event: DownloadEvent) => {
      if (event.event === "Started") {
        updateSnapshot.contentLength = event.data.contentLength ?? null;
      } else if (event.event === "Progress") {
        updateSnapshot.downloadedBytes += event.data.chunkLength;
      }
      update(updateToastId ?? "", {
        description: updateSnapshot.contentLength
          ? `Downloaded ${Math.round((updateSnapshot.downloadedBytes / updateSnapshot.contentLength) * 100)}%`
          : `Downloaded ${updateSnapshot.downloadedBytes} bytes`,
      });
    });
    updateSnapshot.downloadReady = true;
    updateSnapshot.installPromptOpen = true;
    setState("install_confirm");
    showToast({
      title: "Update ready to install",
      description: "Install and restart now, or keep working and install it later.",
      duration: 0,
      action: {
        label: "Install and restart",
        dismiss: false,
        onclick: () => installAndRestart(),
      },
      secondaryAction: laterAction(),
    });
  } catch (error) {
    showFailure(errorCategory(error) === "manifest_failure" ? "install_failure" : errorCategory(error));
  } finally {
    operation = null;
  }
}

export async function installAndRestart(): Promise<void> {
  if (!canStartUpdateOperation(operation, "install") || !currentUpdate || !updateSnapshot.downloadReady) return;
  operation = "install";
  updateSnapshot.installPromptOpen = false;
  setState("installing");
  showToast({
    title: "Installing update",
    description: "The app will close and restart into the installed version.",
    duration: 0,
  });
  try {
    await currentUpdate.install({ restartAfterInstall: true });
  } catch (error) {
    showFailure(errorCategory(error) === "manifest_failure" ? "install_failure" : errorCategory(error));
  } finally {
    operation = null;
  }
}

export async function restartNow(): Promise<void> {
  if (!canStartUpdateOperation(operation, "restart") || updateSnapshot.state !== "ready_to_restart") return;
  operation = "restart";
  setState("restarting");
  try {
    await relaunch();
  } catch {
    showFailure("relaunch_failure");
  } finally {
    operation = null;
  }
}

export function deferRestart() {
  if (updateSnapshot.state === "ready_to_restart") {
    deferredThisSession = true;
    setState("later");
  }
}

export function openUpdateDetails() {
  if (updateSnapshot.availableVersion) {
    updateSnapshot.detailsOpen = true;
    setState("download_confirm");
  }
}

export function closeUpdateDetails() {
  updateSnapshot.detailsOpen = false;
  if (updateSnapshot.state === "download_confirm") setState("available");
}

export function deferUpdateReview() {
  deferredThisSession = true;
  updateSnapshot.detailsOpen = false;
  if (updateSnapshot.state === "download_confirm" || updateSnapshot.state === "available") {
    setState("later");
  }
  if (updateToastId) {
    dismiss(updateToastId);
    updateToastId = null;
  }
  if (updateSnapshot.availableVersion) {
    void saveSetting("updates.lastDeferredVersion", updateSnapshot.availableVersion);
  }
}

export function closeInstallPrompt() {
  updateSnapshot.installPromptOpen = false;
}

export function resetSessionUpdateState() {
  deferredThisSession = false;
  currentUpdate = null;
  Object.assign(updateSnapshot, initialSnapshot);
}
