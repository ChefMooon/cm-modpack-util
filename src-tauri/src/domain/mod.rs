mod contracts;
pub mod inventory;
pub mod validation;

pub use contracts::{
    ActivityEventType, ActivityRecord, ApplicationProjectMetadata, CancellationState, CommandError,
    CompatibilityEvidence, CompatibilityProfile, CompatibilityStatus, DiscoveryDiagnostics,
    DiscoveryOutcomeKind, DiscoveryProgress, DiscoveryProgressKind, DiscoveryResult, Evidence,
    FingerprintComparison, FingerprintEntry, GitStatusObservation, GitWorkingTreeState,
    InventoryCounts, InventoryEntry, InventoryProvider, InventorySide, Observation,
    ObservationFreshness, OperationStatus, PackwizObservations, ProcessEvidence,
    ProjectFingerprint, ProjectLifecycle, ProjectOverview, ProjectRecord, ProjectReference,
    PromptState, RefreshResult, RegistrationPreview, TrustedPageLink, UpdateCandidate,
    ValidationResult, ValidationSeverity, VersionChangeKind, VersionEvidence,
};
