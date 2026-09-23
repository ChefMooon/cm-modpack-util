export type OperationStatus =
  | "pending"
  | "running"
  | "succeeded"
  | "failed"
  | "cancelled"
  | "unavailable";

export type ValidationSeverity = "info" | "warning" | "error";

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

export type LifecycleRecordType =
  | "modpack"
  | "snapshot"
  | "operation"
  | "release_workspace"
  | "release"
  | "changelog_artifact"
  | "changelog_revision"
  | "changelog_export"
  | "provider_cache";

export type LifecycleAction = "archive" | "restore" | "detach" | "permanent_delete";
export type CleanupScope = "provider_cache" | "obsolete_exports" | "provider_cache_and_obsolete_exports";
export type LifecycleOperationStatus = "planned" | "running" | "succeeded" | "partial" | "blocked" | "failed";

export type ProtectedReference = {
  source_type: LifecycleRecordType;
  source_id: string;
  relationship: string;
  reason: string;
};

export type OrphanClassification = {
  record_type: LifecycleRecordType;
  record_id: string;
  missing_parent_type: LifecycleRecordType;
  missing_parent_id: string;
  reason: string;
};

export type UnavailableDestination = {
  export_id: string;
  destination: string;
  status: string;
  diagnostic: string | null;
};

export type ImpactPreview = {
  target_type: LifecycleRecordType;
  target_id: string;
  action: LifecycleAction;
  scope: CleanupScope | null;
  fingerprint: string;
  direct_references: ProtectedReference[];
  retained_evidence: string[];
  removable_records: string[];
  external_boundaries: string[];
  orphan: OrphanClassification | null;
  unavailable_destination: UnavailableDestination | null;
  active_operation: boolean;
  blocked_reasons: string[];
  eligible: boolean;
  requires_confirmation: boolean;
};

export type LifecycleRequest = {
  target_type: LifecycleRecordType;
  target_id: string;
  action: LifecycleAction;
  scope: CleanupScope | null;
  preview_fingerprint: string;
  confirmation: string | null;
};

export type LifecycleResult = {
  target_type: LifecycleRecordType;
  target_id: string;
  action: LifecycleAction;
  status: LifecycleOperationStatus;
  tombstoned: boolean;
  physically_deleted: boolean;
  retained_references: ProtectedReference[];
  message: string;
};

export type CleanupCandidate = {
  record_type: LifecycleRecordType;
  record_id: string;
  bytes: number;
  protected: boolean;
  reason: string;
};

export type CleanupPlan = {
  scope: CleanupScope;
  fingerprint: string;
  candidates: CleanupCandidate[];
  protected_count: number;
  removable_bytes: number;
  blocked_reasons: string[];
};

export type CleanupRequest = {
  scope: CleanupScope;
  preview_fingerprint: string;
};

export type CleanupResult = {
  scope: CleanupScope;
  status: LifecycleOperationStatus;
  removed_count: number;
  protected_count: number;
  removed_bytes: number;
  partial_failures: string[];
};

export type StorageCategory = {
  name: string;
  bytes: number;
  removable: boolean;
  protected: boolean;
  availability: string;
};

export type StorageReport = {
  categories: StorageCategory[];
  application_owned_bytes: number;
  removable_bytes: number;
  protected_bytes: number;
  unavailable_categories: string[];
  external_boundaries: string[];
};

export type Observation<T> = { observed: T } | "unavailable";

export type ModpackLifecycle = "active" | "maintenance" | "archived" | "disconnected";

export type ApplicationModpackMetadata = {
  display_name: string;
  icon: string | null;
  theme: string | null;
  tags: string[];
  favorite: boolean;
  description: string | null;
  lifecycle: ModpackLifecycle;
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
  application_defaults: ApplicationModpackMetadata;
  packwiz: PackwizObservations;
  validation: ValidationResult[];
};

export type ModpackRecord = {
  id: string;
  canonical_path: string;
  application: ApplicationModpackMetadata;
  packwiz: PackwizObservations;
  validation: ValidationResult[];
  created_at: string;
  updated_at: string;
  last_opened_at: string | null;
  last_refreshed_at: string | null;
};

export type ObservationFreshness = "current" | "stale" | "unavailable";

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
  | "snapshot_created"
  | "release_published";

export type ActivityRecord = {
  event_type: ActivityEventType;
  occurred_at: string;
  message: string;
};

export type ModpackOverview = {
  validation: ValidationResult[];
  minecraft_version: Evidence<string>;
  loader: Evidence<string>;
  inventory_counts: InventoryCounts;
  git: GitStatusObservation;
  known_update_count: number | null;
  activity: ActivityRecord[];
};

export type ModpackObservation = {
  observed_at: string;
  freshness: ObservationFreshness;
  inventory: InventoryEntry[];
  overview: ModpackOverview;
};

export type ReleaseWorkspaceWatcherStatus = "stopped" | "starting" | "observing" | "stopping" | "error";

export type RefreshResult = {
  status: OperationStatus;
  freshness: ObservationFreshness;
  observed_at: string | null;
  inventory: InventoryEntry[] | null;
  overview: ModpackOverview | null;
  error: CommandError | null;
};

export const RELEASE_CAPTURE_SCHEMA_VERSION = 1;
export type ReleasePublicationStatus = "draft" | "provisional" | "published" | "withdrawn";
export type ReleaseMetadata = {
  name: string;
  version: string | null;
  description: string | null;
  notes: string | null;
  publication_status: ReleasePublicationStatus;
};
export type ReleaseEligibilitySource =
  | "validated_current_state"
  | "reviewable_snapshot"
  | "completed_operation"
  | "partial_operation";
export type ReleaseEligibility = {
  eligible: boolean;
  provisional: boolean;
  source: ReleaseEligibilitySource | null;
  validation: ValidationResult[];
  diagnostic: string | null;
};
export type CapturedRuntime = {
  minecraft_version: Evidence<string>;
  loader: Evidence<string>;
};
export type CapturedEntry = {
  local_id: string;
  metadata_path: string;
  name: Evidence<string>;
  version: Evidence<string>;
  provider: Evidence<InventoryProvider>;
  side: Evidence<InventorySide>;
  source_url: Evidence<string>;
  pin: Evidence<boolean>;
  capture_provenance: string;
};
export type CapturedPackwizState = {
  schema_version: number;
  pack_name: Evidence<string>;
  pack_author: Evidence<string>;
  pack_version: Evidence<string>;
  pack_format: Evidence<string>;
  index_file: Evidence<string>;
  runtime: CapturedRuntime;
  entries: CapturedEntry[];
  source_fingerprint: ModpackFingerprint;
};
export type GitProvenance = {
  repository_root: string | null;
  branch: Evidence<string>;
  commit: Evidence<string>;
  tag: Evidence<string>;
  remote: Evidence<string>;
  working_tree: GitWorkingTreeState;
  merge_or_rebase_in_progress: boolean;
  history_available: boolean;
  diagnostic: string | null;
};
export type ReleaseCapture = {
  schema_version: number;
  captured_at: string;
  capture_fingerprint: string;
  eligibility: ReleaseEligibility;
  state: CapturedPackwizState;
  provenance: GitProvenance;
  snapshot_id: string | null;
  changelog_artifact_ids: string[];
};
export type ReleaseRecord = {
  id: string;
  modpack_id: string;
  metadata: ReleaseMetadata;
  capture: ReleaseCapture;
  created_at: string;
  updated_at: string;
};
export type ReleaseWorkspaceLifecycle =
  | "draft"
  | "applying"
  | "recovery_required"
  | "provisional"
  | "ready_to_finalize"
  | "finalized"
  | "published"
  | "withdrawn"
  | "abandoned";
export type ReleaseWorkspaceEvidenceStatus = "baseline" | "stale" | "unavailable" | "unverified";
export type ReleaseWorkspacePhase = "review" | "apply" | "resolve" | "verify" | "finalize" | "publish";
export type ReleaseWorkspaceEvidenceFreshness = "current" | "stale" | "unavailable";
export type ReleaseBaselineOrigin = "current_project" | "snapshot" | "finalized_release" | "detached_snapshot";
export type ReleaseCandidateDecision =
  | "undecided"
  | "selected"
  | "skipped"
  | "deferred"
  | "blocked"
  | "pinned"
  | "uncertain";
export type ReleaseWorkspaceCandidate = {
  id: number;
  source_candidate_id: string;
  candidate: UpdateCandidate;
  decision: ReleaseCandidateDecision;
  note: string | null;
  recorded_at: string;
};
export type ReleaseWorkspaceActivity = {
  id: number;
  event_type: string;
  occurred_at: string;
  message: string;
};
export type ReleaseWorkspaceActivityPage = {
  entries: ReleaseWorkspaceActivity[];
  next_cursor: number | null;
  has_more: boolean;
  total_count: number;
};
export type ReleaseWorkspaceObservation = {
  workspace_id: string;
  changed_scope: string[];
  evidence_freshness: ReleaseWorkspaceEvidenceFreshness;
  blocking_reason: string | null;
  overlapped_operation: boolean;
  fingerprint: ModpackFingerprint | null;
};
export type ReleaseWorkspaceObservationRecord = ReleaseWorkspaceObservation & {
  id: number;
  observed_at: string;
};
export type ReleaseWorkspaceEvidence = {
  workspace: ReleaseWorkspace;
  inventory: InventoryEntry[];
  observations: ReleaseWorkspaceObservationRecord[];
};
export type ReleaseWorkspace = {
  id: string;
  modpack_id: string;
  source_snapshot_id: string | null;
  baseline_origin: ReleaseBaselineOrigin;
  baseline_release_id: string | null;
  baseline_capture: ReleaseCapture | null;
  metadata: ReleaseMetadata;
  lifecycle: ReleaseWorkspaceLifecycle;
  evidence_status: ReleaseWorkspaceEvidenceStatus;
  publication_status: ReleasePublicationStatus;
  final_capture: unknown | null;
  final_changelog_revision_id: string | null;
  finalization_receipt: unknown | null;
  phase: ReleaseWorkspacePhase;
  blocking_reason: string | null;
  evidence_freshness: ReleaseWorkspaceEvidenceFreshness;
  primary_next_action: string;
  created_at: string;
  updated_at: string;
  abandoned_at: string | null;
  candidates: ReleaseWorkspaceCandidate[];
  activity: ReleaseWorkspaceActivity[];
  activity_count: number;
};
export type StartReleaseWorkspaceRequest = {
  modpack_id: string;
  snapshot_id: string | null;
  baseline_release_id: string | null;
  metadata: ReleaseMetadata;
};

export type FinalizeReleaseWorkspaceRequest = {
  workspace_id: string;
  final_changelog_revision_id: string;
};
export type SelectReleaseWorkspaceChangelogRequest = {
  workspace_id: string;
  changelog_revision_id: string | null;
};
export type CreateReleaseWorkspaceChangelogRequest = {
  workspace_id: string;
  introduction: string | null;
};
export type PublishReleaseWorkspaceRequest = { workspace_id: string };
export type WithdrawReleaseWorkspaceRequest = { workspace_id: string };
export type RebaseReleaseWorkspaceRequest = { workspace_id: string };
export type ReleaseWorkspaceDecisionRequest = {
  workspace_id: string;
  source_candidate_id: string;
  decision: ReleaseCandidateDecision;
  note: string | null;
};
export type ReleaseCreateRequest = {
  modpack_id: string;
  metadata: ReleaseMetadata;
  snapshot_id: string | null;
  changelog_artifact_ids: string[];
  provisional: boolean;
};
export type ReleaseUpdateRequest = {
  release_id: string;
  metadata: ReleaseMetadata;
};
export type ReleaseComparisonRequest = {
  before_release_id: string;
  after_release_id: string;
};
export type ReleaseStateChange =
  | "added"
  | "removed"
  | "version_changed"
  | "provider_changed"
  | "source_changed"
  | "side_changed"
  | "unchanged"
  | "ambiguous";
export type ReleaseComparisonEntry = {
  identity: string;
  change: ReleaseStateChange;
  before: CapturedEntry | null;
  after: CapturedEntry | null;
  diagnostic: string | null;
};
export type ReleaseRuntimeChange = {
  minecraft_version_before: Evidence<string>;
  minecraft_version_after: Evidence<string>;
  loader_before: Evidence<string>;
  loader_after: Evidence<string>;
};
export type CommitDifference = {
  before: Evidence<string>;
  after: Evidence<string>;
  comparable: boolean;
  diagnostic: string | null;
};
export type ReleaseComparison = {
  before_release_id: string;
  after_release_id: string;
  entries: ReleaseComparisonEntry[];
  runtime: ReleaseRuntimeChange | null;
  notes_before: string | null;
  notes_after: string | null;
  changelog_artifacts_before: ChangelogArtifact[];
  changelog_artifacts_after: ChangelogArtifact[];
  commits: CommitDifference;
  diagnostic: string | null;
};
export type GitHubEnrichmentStatus = "deferred" | "not_requested" | "unavailable";
export type GitHubEnrichment = {
  status: GitHubEnrichmentStatus;
  repository: string | null;
  imported_fields: string[];
  diagnostic: string | null;
};

export type ChangelogSourceKind = "discovery_candidates" | "applied_operation" | "release_workspace_evidence";
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
export type ModrinthProjectIdentity = { modpack_id: string; slug: string | null; title: string | null };
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
  versions: ChangelogVersionResult[];
  pagination_complete: boolean;
  page_count: number;
};
export type ChangelogContentStatus = "available" | "empty" | "unavailable";
export type ChangelogVersionResult = {
  version: ModrinthVersionIdentity;
  response_position: number;
  is_current: boolean;
  published_at: string | null;
  created_at: string | null;
  changelog: string | null;
  retrieval: ChangelogRetrievalStatus;
  content_status: ChangelogContentStatus;
  included: boolean;
  diagnostic: CommandError | null;
};
export type ChangelogGenerationRequest = {
  modpack_id: string;
  snapshot_id: string | null;
  release_workspace_id?: string | null;
  introduction: string | null;
  offline: boolean;
  source: ChangelogSourceKind;
  request_fingerprint: string;
};
export type ChangelogStage = "proposed" | "final";
export type ChangelogProgress = {
  attempt_id: string;
  completed: number;
  total: number;
  message: string;
  cancellable: boolean;
};
export type ChangelogArtifact = {
  id: string;
  modpack_id: string;
  snapshot_id: string | null;
  release_workspace_id: string | null;
  stage: ChangelogStage;
  source_capture_fingerprint: string | null;
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
  selected_version_ids: string[];
  created_at: string;
  is_current: boolean;
  frozen: boolean;
  archived_at: string | null;
};
export type ChangelogRevisionRequest = {
  artifact_id: string;
  prior_revision_id: string | null;
  content: string;
  introduction: string | null;
  selected_version_ids?: string[];
};
export type ChangelogSelectionRevisionRequest = {
  workspace_id: string;
  artifact_id: string;
  prior_revision_id: string | null;
  selected_version_ids: string[];
};
export type ChangelogRevisionArchiveRequest = {
  revision_id: string;
  archived: boolean;
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
export type ModpackFingerprint = {
  root: string;
  entries: FingerprintEntry[];
  complete: boolean;
  diagnostic: string | null;
};
export type FingerprintComparison = {
  before: ModpackFingerprint;
  after: ModpackFingerprint;
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
  modpack_fingerprint: string;
  warning_category: string;
  recovery_state: string;
  acknowledged_at: string;
};
export type OperationAttempt = {
  id: string;
  modpack_id: string;
  workspace_id: string | null;
  snapshot_id: string | null;
  predecessor_id: string | null;
  kind: OperationKind;
  status: OperationStatus;
  outcome: OperationOutcome | null;
  recovery: RecoveryObservation;
  process: ProcessEvidence | null;
  before_fingerprint: ModpackFingerprint | null;
  after_fingerprint: ModpackFingerprint | null;
  verification: OperationVerification | null;
  error: CommandError | null;
  created_at: string;
  finished_at: string | null;
};
export type PinOperationRequest = {
  modpack_id: string;
  workspace_id: string | null;
  entry_id: string;
};
export type ApplyOperationRequest = {
  operation_id: string;
  workspace_id: string;
  snapshot_id: string | null;
  candidate_ids: string[];
  predecessor_id?: string | null;
};
export type ApplyOperationSummary = {
  selected: number;
  skipped: number;
  deferred: number;
  blocked: number;
  pinned: number;
  uncertain: number;
};
export type ApplyOperationReport = {
  snapshot_id: string | null;
  summary: ApplyOperationSummary;
  attempts: OperationAttempt[];
  outcome: OperationOutcome;
};
export type SnapshotLifecycle = "draft" | "reviewable" | "closed" | "cancelled" | "stale";
export type SnapshotDecision = "undecided" | "selected" | "skipped" | "blocked" | "deferred";
export type SnapshotNoteScope = "modpack" | "snapshot" | "candidate";
export type SnapshotRecord = {
  id: string;
  modpack_id: string;
  predecessor_id: string | null;
  lifecycle: SnapshotLifecycle;
  outcome: DiscoveryOutcomeKind;
  label: string | null;
  created_at: string;
  updated_at: string;
  closed_at: string | null;
  result: DiscoveryResult;
  baseline_capture: ReleaseCapture | null;
  candidates: SnapshotCandidateRecord[];
  decisions: SnapshotDecisionRecord[];
  notes: SnapshotNoteRecord[];
  rechecks: SnapshotRecheckRecord[];
};
export type SnapshotCandidateRecord = { id: string; candidate: UpdateCandidate; observed_at: string };
export type SnapshotDecisionRecord = { id: number; candidate_id: string; decision: SnapshotDecision; note: string | null; recorded_at: string };
export type SnapshotNoteRecord = { id: number; modpack_id: string; snapshot_id: string | null; candidate_id: string | null; scope: SnapshotNoteScope; note: string; is_current: boolean; recorded_at: string };
export type SnapshotRecheckRecord = { id: number; comparable: boolean; unchanged: boolean; differences: string[]; checked_at: string };
export type DiscoveryProgressKind = "starting" | "running" | "prompt_detected" | "cancelling" | "finished";
export type DiscoveryProgress = {
  modpack_id: string;
  kind: DiscoveryProgressKind;
  message: string;
  output_bytes: number;
  cancellable: boolean;
};
