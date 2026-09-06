export type OperationStatus =
  | "pending"
  | "running"
  | "succeeded"
  | "failed"
  | "cancelled"
  | "unavailable";

export type ValidationSeverity = "info" | "warning" | "error";

export type ProjectReference = {
  id: string;
  name: string;
  rootPath: string;
};
export type ValidationResult = {
  valid: boolean;
  severity: ValidationSeverity;
  code: string;
  message: string;
};

export type CommandError = {
  code: string;
  message: string;
  details?: string;
};

export type Observation<T> = { observed: T } | "unavailable";

export type ProjectLifecycle = "active" | "maintenance" | "archived" | "disconnected";

export type ApplicationProjectMetadata = {
  display_name: string;
  icon: string | null;
  theme: string | null;
  tags: string[];
  favorite: boolean;
  description: string | null;
  lifecycle: ProjectLifecycle;
};

export type PackwizObservations = {
  name: Observation<string>;
  author: Observation<string>;
  version: Observation<string>;
  pack_format: Observation<string>;
  index_file: Observation<string>;
  index_hash_format: Observation<string>;
  index_hash: Observation<string>;
  declared_versions: [string, string][];
};

export type RegistrationPreview = {
  canonical_path: string;
  application_defaults: ApplicationProjectMetadata;
  packwiz: PackwizObservations;
  validation: ValidationResult[];
};

export type ProjectRecord = {
  id: string;
  canonical_path: string;
  application: ApplicationProjectMetadata;
  packwiz: PackwizObservations;
  validation: ValidationResult[];
  created_at: string;
  updated_at: string;
  last_opened_at: string | null;
  last_refreshed_at: string | null;
};

export type Evidence<T> =
  | { observed: T }
  | "unknown"
  | "unavailable"
  | { malformed: { message: string } };

export type InventoryProvider = "modrinth" | "curseforge" | "unsupported" | "unknown";
export type InventorySide = "client" | "server" | "both" | "unknown";

export type TrustedPageLink = {
  url: string;
  provider: InventoryProvider;
};

export type InventoryEntry = {
  local_id: string;
  metadata_path: string;
  name: Evidence<string>;
  version: Evidence<string>;
  provider: Evidence<InventoryProvider>;
  side: Evidence<InventorySide>;
  pin: Evidence<boolean>;
  source_url: Evidence<string>;
  page_link: TrustedPageLink | null;
};

export type InventoryCounts = {
  total: number;
  provider_modrinth: number;
  provider_curseforge: number;
  provider_unsupported: number;
  provider_unknown: number;
  side_client: number;
  side_server: number;
  side_both: number;
  side_unknown: number;
  pinned: number;
  unpinned: number;
  pin_unknown: number;
  malformed: number;
};

export type GitWorkingTreeState =
  | "not_repository"
  | "clean"
  | "dirty"
  | "conflicted"
  | "unavailable";

export type GitStatusObservation = {
  state: GitWorkingTreeState;
  repository_root: string | null;
  merge_or_rebase_in_progress: boolean;
};

export type ActivityEventType =
  | "registered"
  | "opened"
  | "refresh_succeeded"
  | "refresh_failed"
  | "metadata_changed"
  | "lifecycle_changed";

export type ActivityRecord = {
  event_type: ActivityEventType;
  occurred_at: string;
  message: string;
};

export type ProjectOverview = {
  validation: ValidationResult[];
  minecraft_version: Evidence<string>;
  loader: Evidence<string>;
  inventory_counts: InventoryCounts;
  git: GitStatusObservation;
  known_update_count: number | null;
  activity: ActivityRecord[];
};

export type ObservationFreshness = "current" | "stale" | "unavailable";

export type RefreshResult = {
  status: OperationStatus;
  freshness: ObservationFreshness;
  observed_at: string | null;
  inventory: InventoryEntry[] | null;
  overview: ProjectOverview | null;
  error: CommandError | null;
};
