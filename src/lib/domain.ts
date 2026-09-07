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
export type CandidateSeverity = "critical" | "warning" | "informational" | "unknown";

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
  severity: Evidence<CandidateSeverity>;
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
  | "lifecycle_changed"
  | "snapshot_created";

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

export type ChangelogSourceKind = "discovery_candidates" | "applied_operation";
export type ChangelogGenerationStatus =
  | "pending"
  | "running"
  | "complete"
  | "partial"
  | "failed"
  | "cancelled"
  | "indeterminate";
export type ProviderMatchConfidence = "exact" | "high" | "ambiguous" | "unresolved" | "unknown";
export type LocalEntryIdentity = {
  entry_id: string;
  metadata_path: string;
  local_provider: InventoryProvider;
  local_identity: Evidence<string>;
  local_version: Evidence<string>;
};
export type ModrinthProjectIdentity = { project_id: string; slug: string | null; title: string | null };
export type ModrinthVersionIdentity = {
  version_id: string;
  version_number: string | null;
  game_versions: string[];
  loaders: string[];
};
export type VersionAssociationEvidence = {
  local_version: Evidence<string>;
  provider_version: string | null;
  matched_exactly: boolean;
  details: string[];
};
export type ProviderMatchEvidence = {
  confidence: ProviderMatchConfidence;
  project: ModrinthProjectIdentity | null;
  version: ModrinthVersionIdentity | null;
  association: VersionAssociationEvidence | null;
  candidates: string[];
  reason: string;
};
export type ChangelogRetrievalStatus =
  | "not_requested"
  | "cached"
  | "cached_stale"
  | "retrieved"
  | "missing"
  | "ambiguous"
  | "unresolved"
  | "failed"
  | "offline_unavailable";
export type ChangelogEntryResult = {
  local: LocalEntryIdentity;
  match_evidence: ProviderMatchEvidence;
  retrieval: ChangelogRetrievalStatus;
  changelog: string | null;
  diagnostic: CommandError | null;
};
export type ChangelogGenerationRequest = {
  project_id: string;
  snapshot_id: string;
  introduction: string | null;
  offline: boolean;
  source: ChangelogSourceKind;
  request_fingerprint: string;
};
export type ChangelogProgress = {
  attempt_id: string;
  completed: number;
  total: number;
  message: string;
  cancellable: boolean;
};
export type ChangelogArtifact = {
  id: string;
  project_id: string;
  snapshot_id: string;
  attempt_id: string;
  status: ChangelogGenerationStatus;
  introduction: string | null;
  content: string;
  entries: ChangelogEntryResult[];
  created_at: string;
  updated_at: string;
};
export type ChangelogRevision = {
  id: string;
  artifact_id: string;
  prior_revision_id: string | null;
  content: string;
  introduction: string | null;
  created_at: string;
  is_current: boolean;
};
export type ChangelogRevisionRequest = {
  artifact_id: string;
  prior_revision_id: string | null;
  content: string;
  introduction: string | null;
};
export type ChangelogExportStatus = "exported" | "unavailable" | "failed";
export type ChangelogExport = {
  id: string;
  artifact_id: string;
  revision_id: string | null;
  destination: string;
  format: string;
  content: string;
  content_hash: string;
  status: ChangelogExportStatus;
  exported_at: string;
  diagnostic: CommandError | null;
};
export type ChangelogExportRequest = {
  artifact_id: string;
  revision_id: string | null;
  destination: string;
  content: string;
};

export type CompatibilityStatus = "supported" | "unsupported";
export type VersionEvidence = { observed: string } | "unavailable";
export type CompatibilityProfile = {
  id: string;
  executable_name: string;
  version: VersionEvidence;
  command: string[];
  prompt_patterns: string[];
  cancellation_response: string;
  cancellation_markers: string[];
  no_update_markers: string[];
  expected_exit_codes: number[];
  max_output_bytes: number;
  timeout_seconds: number;
  supported_platforms: string[];
  known_limitations: string[];
};
export type CompatibilityEvidence = {
  status: CompatibilityStatus;
  profile: CompatibilityProfile | null;
  executable_path: string | null;
  version_output: string | null;
  diagnostic: CommandError | null;
};
export type PromptState = "not_seen" | "expected_seen" | "no_updates" | "missing" | "ambiguous" | "unexpected";
export type CancellationState =
  | "not_attempted"
  | "sent_n"
  | "confirmed"
  | "write_failed"
  | "premature_exit"
  | "terminated"
  | "user_cancelled";
export type ProcessEvidence = {
  executable: string;
  arguments: string[];
  working_directory: string;
  stdout: string;
  stderr: string;
  exit_code: number | null;
  started_at: string;
  finished_at: string | null;
  output_truncated: boolean;
  prompt: PromptState;
  cancellation: CancellationState;
};
export type FingerprintEntry = {
  relative_path: string;
  kind: string;
  size: number | null;
  modified_ns: number | null;
  content_hash: string | null;
};
export type ProjectFingerprint = {
  root: string;
  entries: FingerprintEntry[];
  complete: boolean;
  diagnostic: string | null;
};
export type FingerprintComparison = {
  before: ProjectFingerprint;
  after: ProjectFingerprint;
  unchanged: boolean;
  comparable: boolean;
  differences: string[];
};
export type VersionChangeKind = "major" | "minor" | "bugfix" | "unknown";
export type UpdateCandidate = {
  identity: Evidence<string>;
  current_version: Evidence<string>;
  available_version: Evidence<string>;
  local_path: Evidence<string>;
  provider: Evidence<InventoryProvider>;
  side: Evidence<InventorySide>;
  pin: Evidence<boolean>;
  source_url: Evidence<string>;
  page_link: TrustedPageLink | null;
  severity: Evidence<CandidateSeverity>;
  version_change: VersionChangeKind;
  output_evidence: string;
};
export type DiscoveryOutcomeKind = "normal" | "unsupported" | "unsafe" | "indeterminate" | "failed" | "cancelled";
export type DiscoveryDiagnostics = {
  compatibility: CompatibilityEvidence;
  process: ProcessEvidence | null;
  fingerprint: FingerprintComparison | null;
  messages: string[];
};
export type DiscoveryResult = {
  status: OperationStatus;
  outcome: DiscoveryOutcomeKind;
  candidates: UpdateCandidate[];
  diagnostics: DiscoveryDiagnostics;
  error: CommandError | null;
};

export type OperationKind = "apply" | "pin" | "unpin";
export type OperationOutcome = "complete" | "partial" | "failed" | "cancelled" | "unsafe" | "indeterminate";
export type OperationVerification = {
  comparable: boolean;
  intended_state: string;
  observed_state: string;
  differences: string[];
  verified: boolean;
};
export type RecoveryObservation = {
  git: GitStatusObservation;
  recovery_available: boolean;
  warning_category: string | null;
  diagnostic: string | null;
};
export type RecoveryAcknowledgement = {
  operation_id: string;
  project_fingerprint: string;
  warning_category: string;
  recovery_state: string;
  acknowledged_at: string;
};
export type OperationAttempt = {
  id: string;
  project_id: string;
  snapshot_id: string | null;
  predecessor_id: string | null;
  kind: OperationKind;
  status: OperationStatus;
  outcome: OperationOutcome | null;
  recovery: RecoveryObservation;
  process: ProcessEvidence | null;
  before_fingerprint: ProjectFingerprint | null;
  after_fingerprint: ProjectFingerprint | null;
  verification: OperationVerification | null;
  error: CommandError | null;
  created_at: string;
  finished_at: string | null;
};
export type PinOperationRequest = {
  project_id: string;
  entry_id: string;
};
export type ApplyOperationRequest = {
  operation_id: string;
  snapshot_id: string;
  candidate_ids: string[];
};
export type ApplyOperationReport = {
  snapshot_id: string;
  attempts: OperationAttempt[];
  outcome: OperationOutcome;
};
export type SnapshotLifecycle = "draft" | "reviewable" | "closed" | "cancelled" | "stale";
export type SnapshotDecision = "undecided" | "selected" | "skipped" | "blocked" | "deferred";
export type SnapshotNoteScope = "project" | "snapshot" | "candidate";
export type SnapshotRecord = {
  id: string;
  project_id: string;
  predecessor_id: string | null;
  lifecycle: SnapshotLifecycle;
  outcome: DiscoveryOutcomeKind;
  label: string | null;
  created_at: string;
  updated_at: string;
  closed_at: string | null;
  result: DiscoveryResult;
  candidates: SnapshotCandidateRecord[];
  decisions: SnapshotDecisionRecord[];
  notes: SnapshotNoteRecord[];
  rechecks: SnapshotRecheckRecord[];
};
export type SnapshotCandidateRecord = { id: string; candidate: UpdateCandidate; observed_at: string };
export type SnapshotDecisionRecord = { id: number; candidate_id: string; decision: SnapshotDecision; note: string | null; recorded_at: string };
export type SnapshotNoteRecord = { id: number; project_id: string; snapshot_id: string | null; candidate_id: string | null; scope: SnapshotNoteScope; note: string; is_current: boolean; recorded_at: string };
export type SnapshotRecheckRecord = { id: number; comparable: boolean; unchanged: boolean; differences: string[]; checked_at: string };
export type DiscoveryProgressKind = "starting" | "running" | "prompt_detected" | "cancelling" | "finished";
export type DiscoveryProgress = {
  project_id: string;
  kind: DiscoveryProgressKind;
  message: string;
  output_bytes: number;
  cancellable: boolean;
};
