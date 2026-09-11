export type UpdateState =
  | "idle"
  | "checking"
  | "up_to_date"
  | "available"
  | "download_confirm"
  | "downloading"
  | "installing"
  | "ready_to_restart"
  | "restarting"
  | "later"
  | "offline"
  | "failed";

export type UpdateErrorCategory =
  | "manifest_failure"
  | "invalid_metadata"
  | "signature_rejection"
  | "download_failure"
  | "install_failure"
  | "relaunch_failure"
  | "runtime_unavailable";

export type UpdateErrorAction =
  | "retry_check"
  | "review_release"
  | "retry_download"
  | "retry_install"
  | "retry_restart"
  | "continue_using_app";

export type UpdateError = {
  category: UpdateErrorCategory;
  message: string;
  action: UpdateErrorAction;
};

export type UpdateStateGuidance = {
  owner: "update_coordinator" | "shared_toast" | "native_runtime";
  meaning: string;
  retryAction: UpdateErrorAction;
};

export const updateStateGuidance: Record<UpdateState, UpdateStateGuidance> = {
  idle: {
    owner: "update_coordinator",
    meaning: "No update operation is currently running.",
    retryAction: "retry_check",
  },
  checking: {
    owner: "native_runtime",
    meaning: "Release metadata is being checked without downloading an installer.",
    retryAction: "retry_check",
  },
  up_to_date: {
    owner: "shared_toast",
    meaning: "The installed version is the newest stable release.",
    retryAction: "retry_check",
  },
  available: {
    owner: "shared_toast",
    meaning: "A newer stable release is available for review.",
    retryAction: "review_release",
  },
  download_confirm: {
    owner: "shared_toast",
    meaning: "The user must explicitly approve downloading and installing the update.",
    retryAction: "retry_download",
  },
  downloading: {
    owner: "native_runtime",
    meaning: "The approved updater artifact is downloading.",
    retryAction: "retry_download",
  },
  installing: {
    owner: "native_runtime",
    meaning: "The verified updater artifact is being installed.",
    retryAction: "retry_install",
  },
  ready_to_restart: {
    owner: "shared_toast",
    meaning: "Installation completed and the app remains open until restart is chosen.",
    retryAction: "retry_restart",
  },
  restarting: {
    owner: "native_runtime",
    meaning: "The native relaunch request is being processed.",
    retryAction: "retry_restart",
  },
  later: {
    owner: "shared_toast",
    meaning: "The user deferred the restart for the current session.",
    retryAction: "retry_check",
  },
  offline: {
    owner: "update_coordinator",
    meaning: "The release feed is unavailable, so normal app workflows continue.",
    retryAction: "retry_check",
  },
  failed: {
    owner: "shared_toast",
    meaning: "The last update operation failed and exposes an explicit recovery action.",
    retryAction: "retry_check",
  },
};

export const updateErrorGuidance: Record<
  UpdateErrorCategory,
  { message: string; action: UpdateErrorAction }
> = {
  manifest_failure: {
    message: "The release feed could not be reached.",
    action: "retry_check",
  },
  invalid_metadata: {
    message: "The release feed returned invalid update metadata.",
    action: "review_release",
  },
  signature_rejection: {
    message: "The downloaded update did not pass signature verification.",
    action: "continue_using_app",
  },
  download_failure: {
    message: "The update download did not complete.",
    action: "retry_download",
  },
  install_failure: {
    message: "The update could not be installed.",
    action: "retry_install",
  },
  relaunch_failure: {
    message: "The installed update could not be relaunched.",
    action: "retry_restart",
  },
  runtime_unavailable: {
    message: "Updates are unavailable outside the Tauri desktop runtime.",
    action: "continue_using_app",
  },
};

const allowedTransitions: Record<UpdateState, readonly UpdateState[]> = {
  idle: ["checking", "later"],
  checking: ["idle", "up_to_date", "available", "offline", "failed"],
  up_to_date: ["checking", "idle"],
  available: ["download_confirm", "checking", "later", "idle"],
  download_confirm: ["downloading", "available", "later"],
  downloading: ["installing", "available", "failed"],
  installing: ["ready_to_restart", "failed"],
  ready_to_restart: ["restarting", "later"],
  restarting: ["idle", "failed"],
  later: ["checking", "idle"],
  offline: ["checking", "idle"],
  failed: ["checking", "download_confirm", "restarting", "idle"],
};

export function canTransitionUpdateState(from: UpdateState, to: UpdateState): boolean {
  return allowedTransitions[from].includes(to);
}
