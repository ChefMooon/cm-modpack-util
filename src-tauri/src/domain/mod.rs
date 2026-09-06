mod contracts;
pub mod inventory;
pub mod validation;

pub use contracts::{
    ActivityEventType, ActivityRecord, ApplicationProjectMetadata, CancellationState,
    CandidateSeverity, CommandError, CompatibilityEvidence, CompatibilityProfile,
    CompatibilityStatus, DiscoveryDiagnostics, DiscoveryOutcomeKind, DiscoveryProgress,
    DiscoveryProgressKind, DiscoveryResult, Evidence, FingerprintComparison, FingerprintEntry,
    GitStatusObservation, GitWorkingTreeState, InventoryCounts, InventoryEntry, InventoryProvider,
    InventorySide, Observation, ObservationFreshness, OperationStatus, PackwizObservations,
    ProcessEvidence, ProjectFingerprint, ProjectLifecycle, ProjectOverview, ProjectRecord,
    ProjectReference, PromptState, RefreshResult, RegistrationPreview, SnapshotCandidateRecord,
    SnapshotDecision, SnapshotDecisionRecord, SnapshotLifecycle, SnapshotNoteRecord,
    SnapshotNoteScope, SnapshotRecheckRecord, SnapshotRecord, TrustedPageLink, UpdateCandidate,
    ValidationResult, ValidationSeverity, VersionChangeKind, VersionEvidence,
};
