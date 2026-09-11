import { check, type DownloadEvent, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { saveSetting } from "./settings";
import { dismiss, toast, update } from "../components/ui/toast/toast.svelte";
import type { ToastAction } from "../components/ui/toast/types";
import type { UpdateErrorCategory, UpdateState } from "./updates";

export type UpdateSnapshot = {
  state: UpdateState;
  currentVersion: string;
  availableVersion: string | null;
  releaseDate: string | null;
  lastCheckedAt: string | null;
  downloadedBytes: number;
  contentLength: number | null;
  error: { category: UpdateErrorCategory; message: string } | null;
};

const initialSnapshot: UpdateSnapshot = {
  state: "idle",
  currentVersion: "",
  availableVersion: null,
  releaseDate: null,
  lastCheckedAt: null,
  downloadedBytes: 0,
  contentLength: null,
  error: null,
};

export const updateSnapshot = $state<UpdateSnapshot>({ ...initialSnapshot });

let currentUpdate: Update | null = null;
let operation: "check" | "install" | "restart" | null = null;
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
      if (updateToastId) {
        dismiss(updateToastId);
        updateToastId = null;
      }
    },
  };
}

function availableAction(): ToastAction {
  return {
    label: "Download and install",
    dismiss: false,
    onclick: () => beginInstall(),
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
    description: "Review the update before downloading and installing it.",
    duration: 0,
    action: availableAction(),
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
  if (operation) return updateSnapshot;
  operation = "check";
  updateSnapshot.error = null;
  setState("checking");
  try {
    const result = await check();
    updateSnapshot.lastCheckedAt = new Date().toISOString();
    if (!result) {
      currentUpdate = null;
      updateSnapshot.availableVersion = null;
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
    setState("available");

    const settings = await import("./settings").then(({ loadSettings }) => loadSettings());
    const lastNotified = settings["updates.lastNotifiedVersion"];
    const shouldNotify = notify && lastNotified !== result.version;
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

export async function beginInstall(): Promise<void> {
  if (operation || !currentUpdate || updateSnapshot.state === "ready_to_restart") return;
  if (typeof window !== "undefined" && !window.confirm("Download and install this update now?")) {
    setState("available");
    return;
  }

  operation = "install";
  updateSnapshot.error = null;
  updateSnapshot.downloadedBytes = 0;
  updateSnapshot.contentLength = null;
  setState("downloading");
  showToast({
    title: "Downloading update",
    description: "The verified installer will be downloaded before installation.",
    duration: 0,
    secondaryAction: laterAction(),
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
    setState("installing");
    update(updateToastId ?? "", { title: "Installing update", description: "Installation is in progress." });
    await currentUpdate.install({ restartAfterInstall: false });
    setState("ready_to_restart");
    showToast({
      title: "Update ready to restart",
      description: "The update is installed. Restart when you are ready.",
      duration: 0,
      action: {
        label: "Restart now",
        dismiss: false,
        onclick: () => restartNow(),
      },
      secondaryAction: laterAction(),
    });
  } catch (error) {
    showFailure(errorCategory(error) === "manifest_failure" ? "install_failure" : errorCategory(error));
  } finally {
    operation = null;
  }
}

export async function restartNow(): Promise<void> {
  if (operation || updateSnapshot.state !== "ready_to_restart") return;
  operation = "restart";
  setState("restarting");
  try {
    await relaunch();
  } catch (error) {
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

export function resetSessionUpdateState() {
  deferredThisSession = false;
  currentUpdate = null;
  Object.assign(updateSnapshot, initialSnapshot);
}
