use serde::{Deserialize, Serialize};

use super::{CommandError, Evidence, InventoryProvider};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ChangelogSourceKind {
    DiscoveryCandidates,
    AppliedOperation,
    ReleaseWorkspaceEvidence,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ChangelogGenerationStatus {
    Pending,
    Running,
    Complete,
    Partial,
    Failed,
    Cancelled,
    Indeterminate,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ChangelogStage {
    Proposed,
    Final,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProviderMatchConfidence {
    Exact,
    High,
    Ambiguous,
    Unresolved,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LocalEntryIdentity {
    pub entry_id: String,
    pub metadata_path: String,
    pub local_provider: InventoryProvider,
    pub local_identity: Evidence<String>,
    pub local_version: Evidence<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModrinthProjectIdentity {
    pub modpack_id: String,
    pub slug: Option<String>,
    pub title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModrinthVersionIdentity {
    pub version_id: String,
    pub version_number: Option<String>,
    pub game_versions: Vec<String>,
    pub loaders: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VersionAssociationEvidence {
    pub local_version: Evidence<String>,
    pub provider_version: Option<String>,
    pub matched_exactly: bool,
    pub details: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderMatchEvidence {
    pub confidence: ProviderMatchConfidence,
    pub project: Option<ModrinthProjectIdentity>,
    pub version: Option<ModrinthVersionIdentity>,
    pub association: Option<VersionAssociationEvidence>,
    pub candidates: Vec<String>,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ChangelogRetrievalStatus {
    NotRequested,
    Cached,
    CachedStale,
    Retrieved,
    Missing,
    Ambiguous,
    Unresolved,
    Failed,
    OfflineUnavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChangelogEntryResult {
    pub local: LocalEntryIdentity,
    pub match_evidence: ProviderMatchEvidence,
    pub retrieval: ChangelogRetrievalStatus,
    pub changelog: Option<String>,
    pub diagnostic: Option<CommandError>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChangelogCacheKey {
    pub provider: String,
    pub endpoint: String,
    pub request_shape: String,
    pub project_id: String,
    pub version_id: String,
    pub game_version: Option<String>,
    pub loader: Option<String>,
    pub requested_fields: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChangelogCacheRecord {
    pub id: String,
    pub key: ChangelogCacheKey,
    pub raw_response: String,
    pub association: ProviderMatchEvidence,
    pub retrieved_at: String,
    pub request_context: String,
    pub response_context: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChangelogGenerationRequest {
    pub modpack_id: String,
    pub snapshot_id: String,
    #[serde(default)]
    pub release_workspace_id: Option<String>,
    pub introduction: Option<String>,
    pub offline: bool,
    pub source: ChangelogSourceKind,
    pub request_fingerprint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChangelogArtifact {
    pub id: String,
    pub modpack_id: String,
    pub snapshot_id: String,
    pub release_workspace_id: Option<String>,
    pub stage: ChangelogStage,
    pub source_capture_fingerprint: Option<String>,
    pub attempt_id: String,
    pub status: ChangelogGenerationStatus,
    pub introduction: Option<String>,
    pub content: String,
    pub entries: Vec<ChangelogEntryResult>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChangelogRevision {
    pub id: String,
    pub artifact_id: String,
    pub prior_revision_id: Option<String>,
    pub content: String,
    pub introduction: Option<String>,
    pub created_at: String,
    pub is_current: bool,
    pub frozen: bool,
    pub archived_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ChangelogExportStatus {
    Exported,
    Unavailable,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChangelogExport {
    pub id: String,
    pub artifact_id: String,
    pub revision_id: Option<String>,
    pub destination: String,
    pub format: String,
    pub content: String,
    pub content_hash: String,
    pub status: ChangelogExportStatus,
    pub exported_at: String,
    pub diagnostic: Option<CommandError>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChangelogProgress {
    pub attempt_id: String,
    pub completed: u32,
    pub total: u32,
    pub message: String,
    pub cancellable: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_provider_and_source_states() {
        let entry = ChangelogEntryResult {
            local: LocalEntryIdentity {
                entry_id: "entry-1".into(),
                metadata_path: "mods/example.pw.toml".into(),
                local_provider: InventoryProvider::Curseforge,
                local_identity: Evidence::Observed("example".into()),
                local_version: Evidence::Observed("1.0".into()),
            },
            match_evidence: ProviderMatchEvidence {
                confidence: ProviderMatchConfidence::Ambiguous,
                project: None,
                version: None,
                association: None,
                candidates: vec!["a".into(), "b".into()],
                reason: "Multiple projects matched".into(),
            },
            retrieval: ChangelogRetrievalStatus::Ambiguous,
            changelog: None,
            diagnostic: None,
        };
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("curseforge"));
        assert!(json.contains("ambiguous"));
    }

    #[test]
    fn serializes_generation_statuses_as_wire_values() {
        assert_eq!(
            serde_json::to_string(&ChangelogGenerationStatus::Indeterminate).unwrap(),
            "\"indeterminate\""
        );
        assert_eq!(
            serde_json::to_string(&ChangelogRetrievalStatus::CachedStale).unwrap(),
            "\"cached_stale\""
        );
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChangelogRevisionRequest {
    pub artifact_id: String,
    pub prior_revision_id: Option<String>,
    pub content: String,
    pub introduction: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChangelogRevisionArchiveRequest {
    pub revision_id: String,
    pub archived: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChangelogExportRequest {
    pub artifact_id: String,
    pub revision_id: Option<String>,
    pub destination: String,
    pub content: String,
}
