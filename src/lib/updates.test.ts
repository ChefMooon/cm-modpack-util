import { describe, expect, it } from "vitest";
import {
  canStartUpdateOperation,
  canTransitionUpdateState,
  shouldNotifyAvailableUpdate,
} from "./updates";
import { createUpdateRequestDouble } from "./updateTestDouble";

describe("update state transitions", () => {
  it("allows the explicit install and restart path", () => {
    expect(canTransitionUpdateState("available", "download_confirm")).toBe(true);
    expect(canTransitionUpdateState("download_confirm", "downloading")).toBe(true);
    expect(canTransitionUpdateState("downloading", "install_confirm")).toBe(true);
    expect(canTransitionUpdateState("install_confirm", "installing")).toBe(true);
    expect(canTransitionUpdateState("installing", "ready_to_restart")).toBe(true);
    expect(canTransitionUpdateState("ready_to_restart", "restarting")).toBe(true);
  });

  it("does not allow installation or restart before their prerequisites", () => {
    expect(canTransitionUpdateState("available", "installing")).toBe(false);
    expect(canTransitionUpdateState("downloading", "installing")).toBe(false);
    expect(canTransitionUpdateState("installing", "restarting")).toBe(false);
    expect(canTransitionUpdateState("ready_to_restart", "downloading")).toBe(false);
  });
});

describe("update operation guard", () => {
  it("rejects every duplicate operation while one is active", () => {
    expect(canStartUpdateOperation(null, "check")).toBe(true);
    expect(canStartUpdateOperation("check", "check")).toBe(false);
    expect(canStartUpdateOperation("check", "install")).toBe(false);
    expect(canStartUpdateOperation("install", "restart")).toBe(false);
  });
});

describe("update request boundary", () => {
  it("keeps startup checks on metadata until install is explicitly requested", async () => {
    const request = createUpdateRequestDouble();

    await request.check();
    expect(request.requests).toEqual(["manifest"]);

    await request.downloadAndInstall();
    expect(request.requests).toEqual(["manifest", "artifact"]);
  });
});

describe("update notification suppression", () => {
  it("suppresses a deferred version after a route or module remount", () => {
    expect(shouldNotifyAvailableUpdate(true, undefined, "0.0.14", "0.0.14")).toBe(false);
    expect(shouldNotifyAvailableUpdate(true, "0.0.14", undefined, "0.0.14")).toBe(false);
    expect(shouldNotifyAvailableUpdate(true, undefined, undefined, "0.0.14")).toBe(true);
  });
});
