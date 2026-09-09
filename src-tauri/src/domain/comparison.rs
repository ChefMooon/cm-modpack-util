use super::{
    CapturedEntry, CommitDifference, Evidence, ReleaseComparison, ReleaseComparisonEntry,
    ReleaseRecord, ReleaseRuntimeChange, ReleaseStateChange,
};
use std::collections::{HashMap, HashSet};

pub fn compare(before: &ReleaseRecord, after: &ReleaseRecord) -> ReleaseComparison {
    let mut entries = Vec::new();
    let mut matched_before = HashSet::new();
    let mut matched_after = HashSet::new();
    let before_by_local = before
        .capture
        .state
        .entries
        .iter()
        .map(|entry| (entry.local_id.clone(), entry))
        .collect::<HashMap<_, _>>();
    let after_by_local = after
        .capture
        .state
        .entries
        .iter()
        .map(|entry| (entry.local_id.clone(), entry))
        .collect::<HashMap<_, _>>();

    for (identity, before_entry) in &before_by_local {
        if let Some(after_entry) = after_by_local.get(identity) {
            matched_before.insert(identity.clone());
            matched_after.insert(identity.clone());
            entries.push(compare_entry(
                identity.clone(),
                Some(before_entry),
                Some(after_entry),
            ));
        }
    }

    let unmatched_before = before
        .capture
        .state
        .entries
        .iter()
        .filter(|entry| !matched_before.contains(&entry.local_id))
        .collect::<Vec<_>>();
    let unmatched_after = after
        .capture
        .state
        .entries
        .iter()
        .filter(|entry| !matched_after.contains(&entry.local_id))
        .collect::<Vec<_>>();
    for entry in &unmatched_before {
        let identity = provider_identity(entry);
        let matches = unmatched_after
            .iter()
            .filter(|candidate| provider_identity(candidate) == identity)
            .collect::<Vec<_>>();
        entries.push(if matches.len() == 1 {
            compare_entry(identity, Some(entry), Some(matches[0]))
        } else if matches.is_empty() {
            compare_entry(entry.local_id.clone(), Some(entry), None)
        } else {
            ReleaseComparisonEntry {
                identity,
                change: ReleaseStateChange::Ambiguous,
                before: Some((*entry).clone()),
                after: None,
                diagnostic: Some("Provider identity matches multiple captured entries".into()),
            }
        });
    }
    for entry in &unmatched_after {
        if !unmatched_before
            .iter()
            .any(|before_entry| provider_identity(before_entry) == provider_identity(entry))
        {
            entries.push(compare_entry(entry.local_id.clone(), None, Some(entry)));
        }
    }
    entries.sort_by(|left, right| left.identity.cmp(&right.identity));

    let runtime = if before.capture.state.runtime != after.capture.state.runtime {
        Some(ReleaseRuntimeChange {
            minecraft_version_before: before.capture.state.runtime.minecraft_version.clone(),
            minecraft_version_after: after.capture.state.runtime.minecraft_version.clone(),
            loader_before: before.capture.state.runtime.loader.clone(),
            loader_after: after.capture.state.runtime.loader.clone(),
        })
    } else {
        None
    };
    let commits = commit_difference(before, after);
    ReleaseComparison {
        before_release_id: before.id.clone(),
        after_release_id: after.id.clone(),
        entries,
        runtime,
        notes_before: before.metadata.notes.clone(),
        notes_after: after.metadata.notes.clone(),
        changelog_artifacts_before: Vec::new(),
        changelog_artifacts_after: Vec::new(),
        commits,
        diagnostic: (before.modpack_id != after.modpack_id)
            .then(|| "Releases belong to different modpacks".into()),
    }
}

fn compare_entry(
    identity: String,
    before: Option<&CapturedEntry>,
    after: Option<&CapturedEntry>,
) -> ReleaseComparisonEntry {
    let change = match (before, after) {
        (None, Some(_)) => ReleaseStateChange::Added,
        (Some(_), None) => ReleaseStateChange::Removed,
        (Some(before), Some(after)) if before.version != after.version => {
            ReleaseStateChange::VersionChanged
        }
        (Some(before), Some(after)) if before.provider != after.provider => {
            ReleaseStateChange::ProviderChanged
        }
        (Some(before), Some(after)) if before.source_url != after.source_url => {
            ReleaseStateChange::SourceChanged
        }
        (Some(before), Some(after)) if before.side != after.side => ReleaseStateChange::SideChanged,
        (Some(_), Some(_)) => ReleaseStateChange::Unchanged,
        (None, None) => ReleaseStateChange::Ambiguous,
    };
    ReleaseComparisonEntry {
        identity,
        change,
        before: before.cloned(),
        after: after.cloned(),
        diagnostic: None,
    }
}

fn provider_identity(entry: &CapturedEntry) -> String {
    let provider = match &entry.provider {
        Evidence::Observed(provider) => format!("{provider:?}"),
        _ => "unknown".to_string(),
    };
    let name = match &entry.name {
        Evidence::Observed(name) => name.to_ascii_lowercase(),
        _ => entry.local_id.clone(),
    };
    format!("{provider}:{name}")
}

fn commit_difference(before: &ReleaseRecord, after: &ReleaseRecord) -> CommitDifference {
    let same_repository =
        before.capture.provenance.repository_root == after.capture.provenance.repository_root;
    let before_commit = before.capture.provenance.commit.clone();
    let after_commit = after.capture.provenance.commit.clone();
    CommitDifference {
        comparable: same_repository
            && matches!(before_commit, Evidence::Observed(_))
            && matches!(after_commit, Evidence::Observed(_)),
        before: before_commit,
        after: after_commit,
        diagnostic: (!same_repository).then(|| "Git repositories differ or are unavailable".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::compare;
    use crate::domain::*;

    fn entry(id: &str, version: &str) -> CapturedEntry {
        CapturedEntry {
            local_id: id.into(),
            metadata_path: format!("mods/{id}.pw.toml"),
            name: Evidence::Observed(id.into()),
            version: Evidence::Observed(version.into()),
            provider: Evidence::Observed(InventoryProvider::Modrinth),
            side: Evidence::Observed(InventorySide::Both),
            source_url: Evidence::Unavailable,
            pin: Evidence::Observed(false),
            capture_provenance: "test".into(),
        }
    }

    fn release(id: &str, entries: Vec<CapturedEntry>) -> ReleaseRecord {
        ReleaseRecord {
            id: id.into(),
            modpack_id: "pack".into(),
            metadata: ReleaseMetadata {
                name: id.into(),
                version: None,
                description: None,
                notes: None,
                publication_status: ReleasePublicationStatus::Draft,
            },
            capture: ReleaseCapture {
                schema_version: 1,
                captured_at: "now".into(),
                capture_fingerprint: id.into(),
                eligibility: ReleaseEligibility {
                    eligible: true,
                    provisional: false,
                    source: Some(ReleaseEligibilitySource::ValidatedCurrentState),
                    validation: Vec::new(),
                    diagnostic: None,
                },
                state: CapturedPackwizState {
                    schema_version: 1,
                    pack_name: Evidence::Observed("pack".into()),
                    pack_author: Evidence::Unavailable,
                    pack_version: Evidence::Unavailable,
                    pack_format: Evidence::Unavailable,
                    index_file: Evidence::Unavailable,
                    runtime: CapturedRuntime {
                        minecraft_version: Evidence::Observed("1.20.1".into()),
                        loader: Evidence::Observed("fabric".into()),
                    },
                    entries,
                    source_fingerprint: ModpackFingerprint {
                        root: "root".into(),
                        entries: Vec::new(),
                        complete: true,
                        diagnostic: None,
                    },
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
            },
            created_at: "now".into(),
            updated_at: "now".into(),
        }
    }

    #[test]
    fn comparison_uses_captured_entries_after_current_tree_changes() {
        let before = release("one", vec![entry("example", "1.0")]);
        let after = release("two", vec![entry("example", "2.0")]);
        let result = compare(&before, &after);
        assert_eq!(result.entries[0].change, ReleaseStateChange::VersionChanged);
        assert!(!result.commits.comparable);
    }
}
