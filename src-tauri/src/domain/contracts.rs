use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Observation<T> {
    Observed(T),
    Unavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProjectLifecycle {
    Active,
    Maintenance,
    Archived,
    Disconnected,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApplicationProjectMetadata {
    pub display_name: String,
    pub icon: Option<String>,
    pub theme: Option<String>,
    pub tags: Vec<String>,
    pub favorite: bool,
    pub description: Option<String>,
    pub lifecycle: ProjectLifecycle,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PackwizObservations {
    pub name: Observation<String>,
    pub author: Observation<String>,
    pub version: Observation<String>,
    pub pack_format: Observation<String>,
    pub index_file: Observation<String>,
    pub index_hash_format: Observation<String>,
    pub index_hash: Observation<String>,
    pub declared_versions: Vec<(String, String)>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RegistrationPreview {
    pub canonical_path: String,
    pub application_defaults: ApplicationProjectMetadata,
    pub packwiz: PackwizObservations,
    pub validation: Vec<ValidationResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectRecord {
    pub id: String,
    pub canonical_path: String,
    pub application: ApplicationProjectMetadata,
    pub packwiz: PackwizObservations,
    pub validation: Vec<ValidationResult>,
    pub created_at: String,
    pub updated_at: String,
    pub last_opened_at: Option<String>,
    pub last_refreshed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Evidence<T> {
    Observed(T),
    Unknown,
    Unavailable,
    Malformed { message: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InventoryProvider {
    Modrinth,
    Curseforge,
    Unsupported,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InventorySide {
    Client,
    Server,
    Both,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CandidateSeverity {
    Critical,
    Warning,
    Informational,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TrustedPageLink {
    pub url: String,
    pub provider: InventoryProvider,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InventoryEntry {
    pub local_id: String,
    pub metadata_path: String,
    pub name: Evidence<String>,
    pub version: Evidence<String>,
    pub provider: Evidence<InventoryProvider>,
    pub side: Evidence<InventorySide>,
    pub pin: Evidence<bool>,
    pub source_url: Evidence<String>,
    pub page_link: Option<TrustedPageLink>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct InventoryCounts {
    pub total: u32,
    pub provider_modrinth: u32,
    pub provider_curseforge: u32,
    pub provider_unsupported: u32,
    pub provider_unknown: u32,
    pub side_client: u32,
    pub side_server: u32,
    pub side_both: u32,
    pub side_unknown: u32,
    pub pinned: u32,
    pub unpinned: u32,
    pub pin_unknown: u32,
    pub malformed: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GitWorkingTreeState {
    NotRepository,
    Clean,
    Dirty,
    Conflicted,
    Unavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GitStatusObservation {
    pub state: GitWorkingTreeState,
    pub repository_root: Option<String>,
    pub merge_or_rebase_in_progress: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectOverview {
    pub validation: Vec<ValidationResult>,
    pub minecraft_version: Evidence<String>,
    pub loader: Evidence<String>,
    pub inventory_counts: InventoryCounts,
    pub git: GitStatusObservation,
    pub known_update_count: Option<u32>,
    pub activity: Vec<ActivityRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ActivityEventType {
    Registered,
    Opened,
    RefreshSucceeded,
    RefreshFailed,
    MetadataChanged,
    LifecycleChanged,
    SnapshotCreated,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ActivityRecord {
    pub event_type: ActivityEventType,
    pub occurred_at: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ObservationFreshness {
    Current,
    Stale,
    Unavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RefreshResult {
    pub status: OperationStatus,
    pub freshness: ObservationFreshness,
    pub observed_at: Option<String>,
    pub inventory: Option<Vec<InventoryEntry>>,
    pub overview: Option<ProjectOverview>,
    pub error: Option<CommandError>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CompatibilityStatus {
    Supported,
    Unsupported,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum VersionEvidence {
    Observed(String),
    Unavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CompatibilityProfile {
    pub id: String,
    pub executable_name: String,
    pub version: VersionEvidence,
    pub command: Vec<String>,
    pub prompt_patterns: Vec<String>,
    pub cancellation_response: String,
    pub cancellation_markers: Vec<String>,
    pub no_update_markers: Vec<String>,
    pub expected_exit_codes: Vec<i32>,
    pub max_output_bytes: u64,
    pub timeout_seconds: u64,
    pub supported_platforms: Vec<String>,
    pub known_limitations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CompatibilityEvidence {
    pub status: CompatibilityStatus,
    pub profile: Option<CompatibilityProfile>,
    pub executable_path: Option<String>,
    pub version_output: Option<String>,
    pub diagnostic: Option<CommandError>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PromptState {
    NotSeen,
    ExpectedSeen,
    NoUpdates,
    Missing,
    Ambiguous,
    Unexpected,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CancellationState {
    NotAttempted,
    SentN,
    Confirmed,
    WriteFailed,
    PrematureExit,
    Terminated,
    UserCancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProcessEvidence {
    pub executable: String,
    pub arguments: Vec<String>,
    pub working_directory: String,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub output_truncated: bool,
    pub prompt: PromptState,
    pub cancellation: CancellationState,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FingerprintEntry {
    pub relative_path: String,
    pub kind: String,
    pub size: Option<u64>,
    pub modified_ns: Option<u128>,
    pub content_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectFingerprint {
    pub root: String,
    pub entries: Vec<FingerprintEntry>,
    pub complete: bool,
    pub diagnostic: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FingerprintComparison {
    pub before: ProjectFingerprint,
    pub after: ProjectFingerprint,
    pub unchanged: bool,
    pub comparable: bool,
    pub differences: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VersionChangeKind {
    Major,
    Minor,
    Bugfix,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UpdateCandidate {
    pub identity: Evidence<String>,
    pub current_version: Evidence<String>,
    pub available_version: Evidence<String>,
    pub local_path: Evidence<String>,
    pub provider: Evidence<InventoryProvider>,
    pub side: Evidence<InventorySide>,
    pub pin: Evidence<bool>,
    pub source_url: Evidence<String>,
    pub page_link: Option<TrustedPageLink>,
    pub severity: Evidence<CandidateSeverity>,
    pub version_change: VersionChangeKind,
    pub output_evidence: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DiscoveryOutcomeKind {
    Normal,
    Unsupported,
    Unsafe,
    Indeterminate,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiscoveryDiagnostics {
    pub compatibility: CompatibilityEvidence,
    pub process: Option<ProcessEvidence>,
    pub fingerprint: Option<FingerprintComparison>,
    pub messages: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiscoveryResult {
    pub status: OperationStatus,
    pub outcome: DiscoveryOutcomeKind,
    pub candidates: Vec<UpdateCandidate>,
    pub diagnostics: DiscoveryDiagnostics,
    pub error: Option<CommandError>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OperationKind {
    Apply,
    Pin,
    Unpin,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OperationOutcome {
    Complete,
    Partial,
    Failed,
    Cancelled,
    Unsafe,
    Indeterminate,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OperationVerification {
    pub comparable: bool,
    pub intended_state: String,
    pub observed_state: String,
    pub differences: Vec<String>,
    pub verified: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RecoveryObservation {
    pub git: GitStatusObservation,
    pub recovery_available: bool,
    pub warning_category: Option<String>,
    pub diagnostic: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RecoveryAcknowledgement {
    pub operation_id: String,
    pub project_fingerprint: String,
    pub warning_category: String,
    pub recovery_state: String,
    pub acknowledged_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OperationAttempt {
    pub id: String,
    pub project_id: String,
    pub snapshot_id: Option<String>,
    pub predecessor_id: Option<String>,
    pub kind: OperationKind,
    pub status: OperationStatus,
    pub outcome: Option<OperationOutcome>,
    pub recovery: RecoveryObservation,
    pub process: Option<ProcessEvidence>,
    pub before_fingerprint: Option<ProjectFingerprint>,
    pub after_fingerprint: Option<ProjectFingerprint>,
    pub verification: Option<OperationVerification>,
    pub error: Option<CommandError>,
    pub created_at: String,
    pub finished_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PinOperationRequest {
    pub project_id: String,
    pub entry_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApplyOperationRequest {
    pub operation_id: String,
    pub snapshot_id: String,
    pub candidate_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApplyOperationReport {
    pub snapshot_id: String,
    pub attempts: Vec<OperationAttempt>,
    pub outcome: OperationOutcome,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SnapshotLifecycle {
    Draft,
    Reviewable,
    Closed,
    Cancelled,
    Stale,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SnapshotDecision {
    Undecided,
    Selected,
    Skipped,
    Blocked,
    Deferred,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SnapshotNoteScope {
    Project,
    Snapshot,
    Candidate,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SnapshotCandidateRecord {
    pub id: String,
    pub candidate: UpdateCandidate,
    pub observed_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SnapshotDecisionRecord {
    pub id: i64,
    pub candidate_id: String,
    pub decision: SnapshotDecision,
    pub note: Option<String>,
    pub recorded_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SnapshotNoteRecord {
    pub id: i64,
    pub project_id: String,
    pub snapshot_id: Option<String>,
    pub candidate_id: Option<String>,
    pub scope: SnapshotNoteScope,
    pub note: String,
    pub is_current: bool,
    pub recorded_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SnapshotRecheckRecord {
    pub id: i64,
    pub comparable: bool,
    pub unchanged: bool,
    pub differences: Vec<String>,
    pub checked_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SnapshotRecord {
    pub id: String,
    pub project_id: String,
    pub predecessor_id: Option<String>,
    pub lifecycle: SnapshotLifecycle,
    pub outcome: DiscoveryOutcomeKind,
    pub label: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub closed_at: Option<String>,
    pub result: DiscoveryResult,
    pub candidates: Vec<SnapshotCandidateRecord>,
    pub decisions: Vec<SnapshotDecisionRecord>,
    pub notes: Vec<SnapshotNoteRecord>,
    pub rechecks: Vec<SnapshotRecheckRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DiscoveryProgressKind {
    Starting,
    Running,
    PromptDetected,
    Cancelling,
    Finished,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiscoveryProgress {
    pub project_id: String,
    pub kind: DiscoveryProgressKind,
    pub message: String,
    pub output_bytes: u64,
    pub cancellable: bool,
}

impl ValidationResult {
    pub fn info(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            valid: true,
            severity: ValidationSeverity::Info,
            code: code.into(),
            message: message.into(),
        }
    }

    pub fn warning(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            valid: true,
            severity: ValidationSeverity::Warning,
            code: code.into(),
            message: message.into(),
        }
    }

    pub fn error(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            valid: false,
            severity: ValidationSeverity::Error,
            code: code.into(),
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ProjectReference {
    pub id: String,
    pub name: String,
    pub root_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OperationStatus {
    Pending,
    Running,
    Succeeded,
    Failed,
    Cancelled,
    Unavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ValidationSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ValidationResult {
    pub valid: bool,
    pub severity: ValidationSeverity,
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommandError {
    pub code: String,
    pub message: String,
    pub details: Option<String>,
}

impl CommandError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            details: None,
        }
    }

    pub fn with_details(mut self, details: impl Into<String>) -> Self {
        self.details = Some(details.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ApplicationProjectMetadata, CommandError, Evidence, InventoryCounts, InventoryProvider,
        Observation, OperationKind, OperationOutcome, OperationStatus, ProjectLifecycle,
        SnapshotDecision, SnapshotLifecycle, SnapshotNoteScope, ValidationSeverity,
    };

    #[test]
    fn contracts_serialize_stable_wire_values() {
        assert_eq!(
            serde_json::to_string(&OperationStatus::Unavailable).unwrap(),
            "\"unavailable\""
        );
        assert_eq!(
            serde_json::to_string(&ValidationSeverity::Warning).unwrap(),
            "\"warning\""
        );
        assert_eq!(
            serde_json::to_string(&SnapshotDecision::Deferred).unwrap(),
            "\"deferred\""
        );
        assert_eq!(
            serde_json::to_string(&SnapshotLifecycle::Stale).unwrap(),
            "\"stale\""
        );
        assert_eq!(
            serde_json::to_string(&SnapshotNoteScope::Candidate).unwrap(),
            "\"candidate\""
        );
        assert_eq!(
            serde_json::to_string(&OperationKind::Unpin).unwrap(),
            "\"unpin\""
        );
        assert_eq!(
            serde_json::to_string(&OperationOutcome::Indeterminate).unwrap(),
            "\"indeterminate\""
        );
        assert_eq!(
            serde_json::to_string(&CommandError::new(
                "not_registered",
                "Project is not registered"
            ))
            .unwrap(),
            r#"{"code":"not_registered","message":"Project is not registered","details":null}"#
        );
    }

    #[test]
    fn application_and_external_values_are_distinguishable() {
        let application = ApplicationProjectMetadata {
            display_name: "My pack".to_string(),
            icon: None,
            theme: Some("ember".to_string()),
            tags: vec!["survival".to_string()],
            favorite: true,
            description: None,
            lifecycle: ProjectLifecycle::Active,
        };
        assert_eq!(application.lifecycle, ProjectLifecycle::Active);
        assert_eq!(Observation::<String>::Unavailable, Observation::Unavailable);
        assert_eq!(
            serde_json::to_string(&Observation::Unavailable::<String>).unwrap(),
            "\"unavailable\""
        );
    }

    #[test]
    fn inventory_contract_preserves_unknown_states() {
        let counts = InventoryCounts {
            total: 2,
            provider_modrinth: 1,
            provider_unknown: 1,
            ..InventoryCounts::default()
        };
        assert_eq!(counts.total, 2);
        assert_eq!(Evidence::Unknown::<InventoryProvider>, Evidence::Unknown);
    }
}
