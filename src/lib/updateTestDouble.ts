export type UpdateRequestKind = "manifest" | "artifact";

export type UpdateRequestDouble = {
  requests: UpdateRequestKind[];
  check: () => Promise<null>;
  downloadAndInstall: () => Promise<void>;
};

export function createUpdateRequestDouble(): UpdateRequestDouble {
  const requests: UpdateRequestKind[] = [];
  return {
    requests,
    async check() {
      requests.push("manifest");
      return null;
    },
    async downloadAndInstall() {
      requests.push("artifact");
    },
  };
}
