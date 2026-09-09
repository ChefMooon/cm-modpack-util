use super::{
    ChangelogArtifact, Evidence, GitWorkingTreeState, InventoryEntry, InventoryProvider,
    InventorySide, ModpackFingerprint, UpdateCandidate, ValidationResult,
};
use serde::{Deserialize, Serialize};

pub const RELEASE_CAPTURE_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReleaseWorkspaceLifecycle {
    Draft,
    Applying,
    RecoveryRequired,
    Provisional,
    ReadyToFinalize,
    Finalized,
    Published,
    Withdrawn,
    Abandoned,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReleaseWorkspaceEvidenceStatus {
    Baseline,
    Stale,
    Unavailable,
    Unverified,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReleaseWorkspacePhase {
    Review,
    Apply,
    Resolve,
    Verify,
    Finalize,
    Publish,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReleaseWorkspaceEvidenceFreshness {
    Current,
    Stale,
    Unavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReleaseBaselineOrigin {
    CurrentProject,
    Snapshot,
    FinalizedRelease,
    DetachedSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReleaseCandidateDecision {
    Undecided,
    Selected,
    Skipped,
    Deferred,
    Blocked,
    Pinned,
    Uncertain,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReleaseCandidateOutcome {
    Applied,
    FailedBeforeChange,
    ChangedButUnverified,
    Skipped,
    Blocked,
    Retryable,
    Pinned,
    Deferred,
    Uncertain,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReleaseWorkspaceCandidate {
    pub id: i64,
    pub source_candidate_id: String,
    pub candidate: UpdateCandidate,
    pub decision: ReleaseCandidateDecision,
    pub note: Option<String>,
    pub recorded_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReleaseWorkspaceActivity {
    pub id: i64,
    pub event_type: String,
    pub occurred_at: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReleaseWorkspaceObservation {
    pub workspace_id: String,
    pub changed_scope: Vec<String>,
    pub evidence_freshness: ReleaseWorkspaceEvidenceFreshness,
    pub blocking_reason: Option<String>,
    pub overlapped_operation: bool,
    pub fingerprint: Option<ModpackFingerprint>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReleaseWorkspaceObservationRecord {
    pub id: i64,
    pub observed_at: String,
    pub changed_scope: Vec<String>,
    pub evidence_freshness: ReleaseWorkspaceEvidenceFreshness,
    pub blocking_reason: Option<String>,
    pub overlapped_operation: bool,
    pub fingerprint: Option<ModpackFingerprint>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReleaseWorkspaceEvidence {
    pub workspace: ReleaseWorkspace,
    pub inventory: Vec<InventoryEntry>,
    pub observations: Vec<ReleaseWorkspaceObservationRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReleaseWorkspace {
    pub id: String,
    pub modpack_id: String,
    pub source_snapshot_id: Option<String>,
    pub baseline_origin: ReleaseBaselineOrigin,
    pub baseline_release_id: Option<String>,
    pub baseline_capture: Option<ReleaseCapture>,
    pub metadata: ReleaseMetadata,
    pub lifecycle: ReleaseWorkspaceLifecycle,
    pub evidence_status: ReleaseWorkspaceEvidenceStatus,
    pub publication_status: ReleasePublicationStatus,
    pub final_capture: Option<ReleaseCapture>,
    pub final_changelog_revision_id: Option<String>,
    pub finalization_receipt: Option<ReleaseReceipt>,
    pub phase: ReleaseWorkspacePhase,
    pub blocking_reason: Option<String>,
    pub evidence_freshness: ReleaseWorkspaceEvidenceFreshness,
    pub primary_next_action: String,
    pub created_at: String,
    pub updated_at: String,
    pub abandoned_at: Option<String>,
    pub candidates: Vec<ReleaseWorkspaceCandidate>,
    pub activity: Vec<ReleaseWorkspaceActivity>,
}

pub fn project_workspace_state(
    lifecycle: &ReleaseWorkspaceLifecycle,
    evidence_status: &ReleaseWorkspaceEvidenceStatus,
    candidates: &[ReleaseWorkspaceCandidate],
) -> (
    ReleaseWorkspacePhase,
    Option<String>,
    ReleaseWorkspaceEvidenceFreshness,
    String,
) {
    let freshness = match evidence_status {
        ReleaseWorkspaceEvidenceStatus::Baseline => ReleaseWorkspaceEvidenceFreshness::Current,
        ReleaseWorkspaceEvidenceStatus::Stale => ReleaseWorkspaceEvidenceFreshness::Stale,
        ReleaseWorkspaceEvidenceStatus::Unavailable
        | ReleaseWorkspaceEvidenceStatus::Unverified => {
            ReleaseWorkspaceEvidenceFreshness::Unavailable
        }
    };
    let unresolved = candidates.iter().any(|candidate| {
        matches!(
            candidate.decision,
            ReleaseCandidateDecision::Blocked | ReleaseCandidateDecision::Uncertain
        )
    });
    match lifecycle {
        ReleaseWorkspaceLifecycle::Draft => (
            ReleaseWorkspacePhase::Review,
            if unresolved {
                Some("candidate_review_required".to_string())
            } else {
                None
            },
            freshness,
            "review_candidates".to_string(),
        ),
        ReleaseWorkspaceLifecycle::Applying => (
            ReleaseWorkspacePhase::Apply,
            None,
            freshness,
            "wait_for_apply".to_string(),
        ),
        ReleaseWorkspaceLifecycle::RecoveryRequired => (
            ReleaseWorkspacePhase::Resolve,
            Some("recovery_required".to_string()),
            freshness,
            "resolve_recovery".to_string(),
        ),
        ReleaseWorkspaceLifecycle::Provisional => (
            ReleaseWorkspacePhase::Verify,
            Some("verification_required".to_string()),
            freshness,
            "verify_result".to_string(),
        ),
        ReleaseWorkspaceLifecycle::ReadyToFinalize => (
            ReleaseWorkspacePhase::Finalize,
            None,
            freshness,
            "finalize_release".to_string(),
        ),
        ReleaseWorkspaceLifecycle::Finalized
        | ReleaseWorkspaceLifecycle::Published
        | ReleaseWorkspaceLifecycle::Withdrawn => (
            ReleaseWorkspacePhase::Publish,
            None,
            freshness,
            "view_release".to_string(),
        ),
        ReleaseWorkspaceLifecycle::Abandoned => (
            ReleaseWorkspacePhase::Review,
            Some("workspace_abandoned".to_string()),
            freshness,
            "start_new_workspace".to_string(),
        ),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StartReleaseWorkspaceRequest {
    pub modpack_id: String,
    pub snapshot_id: Option<String>,
    pub baseline_release_id: Option<String>,
    pub metadata: ReleaseMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReleaseWorkspaceDecisionRequest {
    pub workspace_id: String,
    pub source_candidate_id: String,
    pub decision: ReleaseCandidateDecision,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FinalizeReleaseWorkspaceRequest {
    pub workspace_id: String,
    pub final_changelog_revision_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PublishReleaseWorkspaceRequest {
    pub workspace_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WithdrawReleaseWorkspaceRequest {
    pub workspace_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RebaseReleaseWorkspaceRequest {
    pub workspace_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReleasePublicationStatus {
    Draft,
    Provisional,
    Published,
    Withdrawn,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReleaseMetadata {
    pub name: String,
    pub version: Option<String>,
    pub description: Option<String>,
    pub notes: Option<String>,
    pub publication_status: ReleasePublicationStatus,
}

pub fn validate_publication_transition(
    current: &ReleasePublicationStatus,
    next: &ReleasePublicationStatus,
    fresh_capture: bool,
) -> Result<(), super::CommandError> {
    if matches!(
        (current, next),
        (
            ReleasePublicationStatus::Provisional,
            ReleasePublicationStatus::Published
        )
    ) && !fresh_capture
    {
        return Err(super::CommandError::new(
            "fresh_capture_required",
            "A provisional release requires a fresh validated capture before publication",
        ));
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReleaseEligibilitySource {
    ValidatedCurrentState,
    ReviewableSnapshot,
    CompletedOperation,
    PartialOperation,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReleaseEligibility {
    pub eligible: bool,
    pub provisional: bool,
    pub source: Option<ReleaseEligibilitySource>,
    pub validation: Vec<ValidationResult>,
    pub diagnostic: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CapturedRuntime {
    pub minecraft_version: Evidence<String>,
    pub loader: Evidence<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CapturedEntry {
    pub local_id: String,
    pub metadata_path: String,
    pub name: Evidence<String>,
    pub version: Evidence<String>,
    pub provider: Evidence<InventoryProvider>,
    pub side: Evidence<InventorySide>,
    pub source_url: Evidence<String>,
    pub pin: Evidence<bool>,
    pub capture_provenance: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CapturedPackwizState {
    pub schema_version: u32,
    pub pack_name: Evidence<String>,
    pub pack_version: Evidence<String>,
    pub pack_format: Evidence<String>,
    pub index_file: Evidence<String>,
    pub runtime: CapturedRuntime,
    pub entries: Vec<CapturedEntry>,
    pub source_fingerprint: ModpackFingerprint,
    pub pack_author: Evidence<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GitProvenance {
    pub repository_root: Option<String>,
    pub branch: Evidence<String>,
    pub commit: Evidence<String>,
    pub tag: Evidence<String>,
    pub remote: Evidence<String>,
    pub working_tree: GitWorkingTreeState,
    pub merge_or_rebase_in_progress: bool,
    pub history_available: bool,
    pub diagnostic: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReleaseCapture {
    pub schema_version: u32,
    pub captured_at: String,
    pub capture_fingerprint: String,
    pub eligibility: ReleaseEligibility,
    pub state: CapturedPackwizState,
    pub provenance: GitProvenance,
    pub snapshot_id: Option<String>,
    pub changelog_artifact_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReleaseReceipt {
    pub workspace_id: String,
    pub source_snapshot_id: Option<String>,
    pub baseline_fingerprint: String,
    pub final_capture_fingerprint: String,
    pub changed: bool,
    pub stable: bool,
    pub validated: bool,
    pub final_changelog_revision_id: String,
    pub finalized_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SelectReleaseWorkspaceChangelogRequest {
    pub workspace_id: String,
    pub changelog_revision_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CreateReleaseWorkspaceChangelogRequest {
    pub workspace_id: String,
    pub introduction: Option<String>,
}

pub fn validate_finalization(
    lifecycle: &ReleaseWorkspaceLifecycle,
    baseline_capture_fingerprint: &str,
    final_capture: &ReleaseCapture,
    final_changelog_revision_id: Option<&str>,
    unresolved_candidates: bool,
) -> Result<(), super::CommandError> {
    if !matches!(
        lifecycle,
        ReleaseWorkspaceLifecycle::ReadyToFinalize | ReleaseWorkspaceLifecycle::Provisional
    ) {
        return Err(super::CommandError::new(
            "invalid_finalization_state",
            "Workspace is not ready to finalize",
        ));
    }
    if final_capture.state.source_fingerprint.root.is_empty()
        || final_capture.capture_fingerprint.is_empty()
        || !final_capture.eligibility.eligible
        || final_capture.eligibility.provisional
    {
        return Err(super::CommandError::new(
            "final_capture_unverified",
            "Final capture is not validated",
        ));
    }
    if final_capture.capture_fingerprint == baseline_capture_fingerprint {
        return Err(super::CommandError::new(
            "final_capture_mismatch",
            "Final capture does not match the release baseline",
        ));
    }
    if unresolved_candidates {
        return Err(super::CommandError::new(
            "unresolved_candidates",
            "Candidate recovery outcomes must be resolved before finalization",
        ));
    }
    if final_changelog_revision_id.is_none() {
        return Err(super::CommandError::new(
            "final_changelog_required",
            "A final changelog revision is required",
        ));
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReleaseRecord {
    pub id: String,
    pub modpack_id: String,
    pub metadata: ReleaseMetadata,
    pub capture: ReleaseCapture,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReleaseCreateRequest {
    pub modpack_id: String,
    pub metadata: ReleaseMetadata,
    pub snapshot_id: Option<String>,
    pub changelog_artifact_ids: Vec<String>,
    pub provisional: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReleaseUpdateRequest {
    pub release_id: String,
    pub metadata: ReleaseMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReleaseComparisonRequest {
    pub before_release_id: String,
    pub after_release_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReleaseStateChange {
    Added,
    Removed,
    VersionChanged,
    ProviderChanged,
    SourceChanged,
    SideChanged,
    Unchanged,
    Ambiguous,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReleaseComparisonEntry {
    pub identity: String,
    pub change: ReleaseStateChange,
    pub before: Option<CapturedEntry>,
    pub after: Option<CapturedEntry>,
    pub diagnostic: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReleaseRuntimeChange {
    pub minecraft_version_before: Evidence<String>,
    pub minecraft_version_after: Evidence<String>,
    pub loader_before: Evidence<String>,
    pub loader_after: Evidence<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommitDifference {
    pub before: Evidence<String>,
    pub after: Evidence<String>,
    pub comparable: bool,
    pub diagnostic: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReleaseComparison {
    pub before_release_id: String,
    pub after_release_id: String,
    pub entries: Vec<ReleaseComparisonEntry>,
    pub runtime: Option<ReleaseRuntimeChange>,
    pub notes_before: Option<String>,
    pub notes_after: Option<String>,
    pub changelog_artifacts_before: Vec<ChangelogArtifact>,
    pub changelog_artifacts_after: Vec<ChangelogArtifact>,
    pub commits: CommitDifference,
    pub diagnostic: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GitHubEnrichmentStatus {
    Deferred,
    NotRequested,
    Unavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GitHubEnrichment {
    pub status: GitHubEnrichmentStatus,
    pub repository: Option<String>,
    pub imported_fields: Vec<String>,
    pub diagnostic: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Evidence, FingerprintEntry, SnapshotLifecycle};

    fn capture() -> ReleaseCapture {
        ReleaseCapture {
            schema_version: RELEASE_CAPTURE_SCHEMA_VERSION,
            captured_at: "2026-09-07T00:00:00Z".into(),
            capture_fingerprint: "sha256:test".into(),
            eligibility: ReleaseEligibility {
                eligible: true,
                provisional: false,
                source: Some(ReleaseEligibilitySource::ValidatedCurrentState),
                validation: Vec::new(),
                diagnostic: None,
            },
            state: CapturedPackwizState {
                schema_version: RELEASE_CAPTURE_SCHEMA_VERSION,
                pack_name: Evidence::Observed("example".into()),
                pack_version: Evidence::Unavailable,
                pack_format: Evidence::Observed("packwiz:1.1.0".into()),
                index_file: Evidence::Observed("index.toml".into()),
                runtime: CapturedRuntime {
                    minecraft_version: Evidence::Observed("1.20.1".into()),
                    loader: Evidence::Observed("fabric 0.16".into()),
                },
                entries: Vec::new(),
                source_fingerprint: ModpackFingerprint {
                    root: "/packs/example".into(),
                    entries: vec![FingerprintEntry {
                        relative_path: "pack.toml".into(),
                        kind: "file".into(),
                        size: Some(12),
                        modified_ns: None,
                        content_hash: Some("hash".into()),
                    }],
                    complete: true,
                    diagnostic: None,
                },
                pack_author: Evidence::Unavailable,
            },
            provenance: GitProvenance {
                repository_root: None,
                branch: Evidence::Unavailable,
                commit: Evidence::Unavailable,
                tag: Evidence::Unavailable,
                remote: Evidence::Unavailable,
                working_tree: GitWorkingTreeState::NotRepository,
                merge_or_rebase_in_progress: false,
                history_available: false,
                diagnostic: None,
            },
            snapshot_id: None,
            changelog_artifact_ids: Vec::new(),
        }
    }

    #[test]
    fn release_capture_serializes_versioned_unavailable_provenance() {
        let json = serde_json::to_string(&capture()).unwrap();
        assert!(json.contains("schema_version"));
        assert!(json.contains("not_repository"));
        assert!(json.contains("unavailable"));
    }

    #[test]
    fn publication_status_is_separate_from_snapshot_lifecycle() {
        assert_eq!(
            serde_json::to_string(&ReleasePublicationStatus::Provisional).unwrap(),
            "\"provisional\""
        );
        assert_eq!(
            serde_json::to_string(&SnapshotLifecycle::Reviewable).unwrap(),
            "\"reviewable\""
        );
    }

    #[test]
    fn candidate_outcomes_preserve_recovery_distinctions() {
        assert_eq!(
            serde_json::to_string(&ReleaseCandidateOutcome::FailedBeforeChange).unwrap(),
            "\"failed_before_change\""
        );
        assert_eq!(
            serde_json::to_string(&ReleaseCandidateOutcome::ChangedButUnverified).unwrap(),
            "\"changed_but_unverified\""
        );
    }

    #[test]
    fn provisional_publication_requires_fresh_capture() {
        let error = validate_publication_transition(
            &ReleasePublicationStatus::Provisional,
            &ReleasePublicationStatus::Published,
            false,
        )
        .unwrap_err();
        assert_eq!(error.code, "fresh_capture_required");
        assert!(validate_publication_transition(
            &ReleasePublicationStatus::Provisional,
            &ReleasePublicationStatus::Published,
            true,
        )
        .is_ok());
    }

    #[test]
    fn finalization_rejects_noop_capture() {
        let error = validate_finalization(
            &ReleaseWorkspaceLifecycle::ReadyToFinalize,
            "sha256:test",
            &capture(),
            Some("revision-1"),
            false,
        )
        .unwrap_err();
        assert_eq!(error.code, "final_capture_mismatch");
    }

    #[test]
    fn finalization_rejects_unresolved_candidates() {
        let mut final_capture = capture();
        final_capture.capture_fingerprint = "sha256:changed".into();
        let error = validate_finalization(
            &ReleaseWorkspaceLifecycle::ReadyToFinalize,
            "sha256:baseline",
            &final_capture,
            Some("revision-1"),
            true,
        )
        .unwrap_err();
        assert_eq!(error.code, "unresolved_candidates");
    }

    #[test]
    fn workspace_projection_maps_recovery_and_finalize_actions() {
        let candidates = vec![ReleaseWorkspaceCandidate {
            id: 1,
            source_candidate_id: "candidate-1".into(),
            candidate: UpdateCandidate {
                identity: Evidence::Unknown,
                current_version: Evidence::Unknown,
                available_version: Evidence::Unknown,
                local_path: Evidence::Unknown,
                provider: Evidence::Unknown,
                side: Evidence::Unknown,
                pin: Evidence::Unknown,
                source_url: Evidence::Unknown,
                page_link: None,
                severity: Evidence::Unknown,
                version_change: crate::domain::VersionChangeKind::Unknown,
                output_evidence: String::new(),
            },
            decision: ReleaseCandidateDecision::Selected,
            note: None,
            recorded_at: "0".into(),
        }];
        let projection = project_workspace_state(
            &ReleaseWorkspaceLifecycle::RecoveryRequired,
            &ReleaseWorkspaceEvidenceStatus::Stale,
            &candidates,
        );
        assert_eq!(projection.0, ReleaseWorkspacePhase::Resolve);
        assert_eq!(projection.1.as_deref(), Some("recovery_required"));
        assert_eq!(projection.2, ReleaseWorkspaceEvidenceFreshness::Stale);
        assert_eq!(projection.3, "resolve_recovery");

        let projection = project_workspace_state(
            &ReleaseWorkspaceLifecycle::ReadyToFinalize,
            &ReleaseWorkspaceEvidenceStatus::Baseline,
            &candidates,
        );
        assert_eq!(projection.0, ReleaseWorkspacePhase::Finalize);
        assert_eq!(projection.1, None);
        assert_eq!(projection.3, "finalize_release");
    }
}
