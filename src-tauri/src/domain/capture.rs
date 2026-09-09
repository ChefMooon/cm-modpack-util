use super::{
    inventory, validation, CapturedEntry, CapturedPackwizState, CapturedRuntime, CommandError,
    Evidence, ReleaseCapture, ReleaseEligibility, RELEASE_CAPTURE_SCHEMA_VERSION,
};
use crate::discovery::fingerprint;
use sha2::{Digest, Sha256};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn capture(
    root: &Path,
    eligibility: ReleaseEligibility,
    snapshot_id: Option<String>,
    changelog_artifact_ids: Vec<String>,
) -> Result<ReleaseCapture, CommandError> {
    let before = fingerprint::collect(root).map_err(|error| {
        CommandError::new(
            "capture_unavailable",
            "Release source fingerprint could not be collected",
        )
        .with_details(error)
    })?;
    let preview = validation::preview(&root.to_string_lossy())?;
    let entries = inventory::read_inventory(root)?;
    let after = fingerprint::collect(root).map_err(|error| {
        CommandError::new(
            "capture_unavailable",
            "Release source fingerprint could not be verified",
        )
        .with_details(error)
    })?;
    require_stable(&before, &after)?;

    let runtime = CapturedRuntime {
        minecraft_version: declared_version(&preview.packwiz.declared_versions, "minecraft"),
        loader: preview
            .packwiz
            .declared_versions
            .iter()
            .find(|(key, _)| key != "minecraft")
            .map(|(key, value)| Evidence::Observed(format!("{key} {value}")))
            .unwrap_or(Evidence::Unavailable),
    };
    let captured_entries = entries
        .into_iter()
        .map(|entry| CapturedEntry {
            local_id: entry.local_id,
            metadata_path: entry.metadata_path,
            name: entry.name,
            version: entry.version,
            provider: entry.provider,
            side: entry.side,
            source_url: entry.source_url,
            pin: entry.pin,
            capture_provenance: "validated_packwiz_read".to_string(),
        })
        .collect();
    let state = CapturedPackwizState {
        schema_version: RELEASE_CAPTURE_SCHEMA_VERSION,
        pack_name: observation_to_evidence(preview.packwiz.name),
        pack_author: observation_to_evidence(preview.packwiz.author),
        pack_version: observation_to_evidence(preview.packwiz.version),
        pack_format: observation_to_evidence(preview.packwiz.pack_format),
        index_file: observation_to_evidence(preview.packwiz.index_file),
        runtime,
        entries: captured_entries,
        source_fingerprint: after,
    };
    let capture_fingerprint = fingerprint_state(&state)?;
    Ok(ReleaseCapture {
        schema_version: RELEASE_CAPTURE_SCHEMA_VERSION,
        captured_at: timestamp(),
        capture_fingerprint,
        eligibility,
        state,
        provenance: super::provenance::observe(root),
        snapshot_id,
        changelog_artifact_ids,
    })
}

pub fn require_stable(
    before: &super::ModpackFingerprint,
    after: &super::ModpackFingerprint,
) -> Result<(), CommandError> {
    let stability = fingerprint::compare(before.clone(), after.clone());
    if stability.unchanged {
        Ok(())
    } else {
        Err(CommandError::new(
            "capture_unstable",
            "Packwiz evidence changed while the release was being captured",
        )
        .with_details(stability.differences.join("; ")))
    }
}

fn declared_version(declared: &[(String, String)], key: &str) -> Evidence<String> {
    declared
        .iter()
        .find(|(declared_key, _)| declared_key == key)
        .map(|(_, value)| Evidence::Observed(value.clone()))
        .unwrap_or(Evidence::Unavailable)
}

fn observation_to_evidence(value: super::Observation<String>) -> Evidence<String> {
    match value {
        super::Observation::Observed(value) => Evidence::Observed(value),
        super::Observation::Unavailable => Evidence::Unavailable,
    }
}

fn fingerprint_state(state: &CapturedPackwizState) -> Result<String, CommandError> {
    let json = serde_json::to_vec(state).map_err(|error| {
        CommandError::new(
            "capture_serialization_failed",
            "Captured release state could not be serialized",
        )
        .with_details(error.to_string())
    })?;
    let mut hasher = Sha256::new();
    hasher.update(json);
    Ok(format!("sha256:{:x}", hasher.finalize()))
}

fn timestamp() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default();
    format!("{seconds}")
}

#[cfg(test)]
mod tests {
    use super::{capture, require_stable};
    use crate::domain::{
        FingerprintEntry, ModpackFingerprint, ReleaseEligibility, ReleaseEligibilitySource,
    };
    use std::path::PathBuf;

    fn fixture(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/packwiz")
            .join(name)
    }

    #[test]
    fn valid_fixture_captures_packwiz_and_entry_evidence() {
        let result = capture(
            &fixture("valid"),
            ReleaseEligibility {
                eligible: true,
                provisional: false,
                source: Some(ReleaseEligibilitySource::ValidatedCurrentState),
                validation: Vec::new(),
                diagnostic: None,
            },
            None,
            Vec::new(),
        )
        .unwrap();
        assert_eq!(result.schema_version, 1);
        assert!(!result.capture_fingerprint.is_empty());
        assert_eq!(result.state.entries.len(), 1);
    }

    #[test]
    fn changed_fingerprint_is_rejected_as_retryable_instability() {
        let before = ModpackFingerprint {
            root: "root".into(),
            entries: vec![FingerprintEntry {
                relative_path: "pack.toml".into(),
                kind: "file".into(),
                size: Some(1),
                modified_ns: None,
                content_hash: Some("before".into()),
            }],
            complete: true,
            diagnostic: None,
        };
        let mut after = before.clone();
        after.entries[0].content_hash = Some("after".into());
        let error = require_stable(&before, &after).unwrap_err();
        assert_eq!(error.code, "capture_unstable");
    }
}
