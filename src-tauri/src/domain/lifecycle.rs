use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::CommandError;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleRecordType {
    Modpack,
    Snapshot,
    Operation,
    ReleaseWorkspace,
    Release,
    ChangelogArtifact,
    ChangelogRevision,
    ChangelogExport,
    ProviderCache,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleAction {
    Archive,
    Restore,
    Detach,
    PermanentDelete,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CleanupScope {
    ProviderCache,
    ObsoleteExports,
    ProviderCacheAndObsoleteExports,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleOperationStatus {
    Planned,
    Running,
    Succeeded,
    Partial,
    Blocked,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProtectedReference {
    pub source_type: LifecycleRecordType,
    pub source_id: String,
    pub relationship: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OrphanClassification {
    pub record_type: LifecycleRecordType,
    pub record_id: String,
    pub missing_parent_type: LifecycleRecordType,
    pub missing_parent_id: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UnavailableDestination {
    pub export_id: String,
    pub destination: String,
    pub status: String,
    pub diagnostic: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ImpactPreview {
    pub target_type: LifecycleRecordType,
    pub target_id: String,
    pub action: LifecycleAction,
    pub scope: Option<CleanupScope>,
    pub fingerprint: String,
    pub direct_references: Vec<ProtectedReference>,
    pub retained_evidence: Vec<String>,
    pub removable_records: Vec<String>,
    pub external_boundaries: Vec<String>,
    pub orphan: Option<OrphanClassification>,
    pub unavailable_destination: Option<UnavailableDestination>,
    pub active_operation: bool,
    pub blocked_reasons: Vec<String>,
    pub eligible: bool,
    pub requires_confirmation: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LifecycleRequest {
    pub target_type: LifecycleRecordType,
    pub target_id: String,
    pub action: LifecycleAction,
    pub scope: Option<CleanupScope>,
    pub preview_fingerprint: String,
    pub confirmation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LifecycleResult {
    pub target_type: LifecycleRecordType,
    pub target_id: String,
    pub action: LifecycleAction,
    pub status: LifecycleOperationStatus,
    pub tombstoned: bool,
    pub physically_deleted: bool,
    pub retained_references: Vec<ProtectedReference>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CleanupRequest {
    pub scope: CleanupScope,
    pub preview_fingerprint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CleanupCandidate {
    pub record_type: LifecycleRecordType,
    pub record_id: String,
    pub bytes: u64,
    pub protected: bool,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CleanupPlan {
    pub scope: CleanupScope,
    pub fingerprint: String,
    pub candidates: Vec<CleanupCandidate>,
    pub protected_count: u64,
    pub removable_bytes: u64,
    pub blocked_reasons: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CleanupResult {
    pub scope: CleanupScope,
    pub status: LifecycleOperationStatus,
    pub removed_count: u64,
    pub protected_count: u64,
    pub removed_bytes: u64,
    pub partial_failures: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StorageCategory {
    pub name: String,
    pub bytes: u64,
    pub removable: bool,
    pub protected: bool,
    pub availability: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StorageReport {
    pub categories: Vec<StorageCategory>,
    pub application_owned_bytes: u64,
    pub removable_bytes: u64,
    pub protected_bytes: u64,
    pub unavailable_categories: Vec<String>,
    pub external_boundaries: Vec<String>,
}

pub fn confirmation_phrase(record_id: &str) -> String {
    format!("DELETE {record_id}")
}

pub fn validate_confirmation(
    record_id: &str,
    confirmation: Option<&str>,
) -> Result<(), CommandError> {
    if confirmation == Some(confirmation_phrase(record_id).as_str()) {
        Ok(())
    } else {
        Err(CommandError::new(
            "confirmation_required",
            format!(
                "Type {} to permanently delete this record",
                confirmation_phrase(record_id)
            ),
        ))
    }
}

pub fn preview_fingerprint(
    target_type: &LifecycleRecordType,
    target_id: &str,
    action: &LifecycleAction,
    scope: Option<&CleanupScope>,
    references: &[ProtectedReference],
    removable_records: &[String],
    active_operation: bool,
) -> String {
    let payload = format!(
        "{target_type:?}|{target_id}|{action:?}|{scope:?}|{references:?}|{removable_records:?}|{active_operation}"
    );
    format!("sha256:{:x}", Sha256::digest(payload.as_bytes()))
}

pub fn action_is_supported(target_type: &LifecycleRecordType, action: &LifecycleAction) -> bool {
    match action {
        LifecycleAction::PermanentDelete => matches!(
            target_type,
            LifecycleRecordType::ProviderCache | LifecycleRecordType::ChangelogExport
        ),
        LifecycleAction::Detach => matches!(
            target_type,
            LifecycleRecordType::Snapshot
                | LifecycleRecordType::ReleaseWorkspace
                | LifecycleRecordType::Release
                | LifecycleRecordType::ChangelogArtifact
                | LifecycleRecordType::ChangelogRevision
                | LifecycleRecordType::ChangelogExport
        ),
        LifecycleAction::Archive | LifecycleAction::Restore => !matches!(
            target_type,
            LifecycleRecordType::Operation | LifecycleRecordType::ProviderCache
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn confirmation_is_target_specific() {
        assert!(validate_confirmation("release-1", Some("DELETE release-1")).is_ok());
        assert!(validate_confirmation("release-1", Some("DELETE Release 1")).is_err());
    }

    #[test]
    fn physical_delete_is_allowlisted() {
        assert!(action_is_supported(
            &LifecycleRecordType::ProviderCache,
            &LifecycleAction::PermanentDelete
        ));
        assert!(action_is_supported(
            &LifecycleRecordType::ChangelogExport,
            &LifecycleAction::PermanentDelete
        ));
        assert!(!action_is_supported(
            &LifecycleRecordType::Release,
            &LifecycleAction::PermanentDelete
        ));
    }
}
