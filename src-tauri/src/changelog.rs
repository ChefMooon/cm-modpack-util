use crate::changelog_transport::{HttpModrinthTransport, ModrinthTransport, ModrinthVersionQuery};
use crate::db;
use crate::domain::{
    ChangelogArtifact, ChangelogEntryResult, ChangelogGenerationRequest, ChangelogGenerationStatus,
    ChangelogRetrievalStatus, ChangelogSourceKind, CommandError, Evidence, LocalEntryIdentity,
    ModrinthProjectIdentity, ModrinthVersionIdentity, ProviderMatchConfidence,
    ProviderMatchEvidence, SnapshotLifecycle, SnapshotRecord, VersionAssociationEvidence,
};
use serde_json::Value;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;
use tauri::State;
use tauri::{AppHandle, Emitter};

const LOOKUP_DELAY: Duration = Duration::from_secs(3);

#[derive(Default)]
pub struct ChangelogRuntime {
    cancel_requested: AtomicBool,
}

#[cfg(test)]
mod cancellation_tests {
    use super::ChangelogRuntime;

    #[test]
    fn cancellation_can_be_requested_and_reset_for_a_new_attempt() {
        let runtime = ChangelogRuntime::default();
        runtime.begin();
        assert!(!runtime.is_cancelled());
        runtime.cancel();
        assert!(runtime.is_cancelled());
        runtime.begin();
        assert!(!runtime.is_cancelled());
    }
}

#[cfg(test)]
mod provider_tests {
    use super::*;

    struct FixtureTransport {
        response: String,
    }

    impl ModrinthTransport for FixtureTransport {
        fn get_versions(
            &self,
            _project: &str,
            _query: &ModrinthVersionQuery,
        ) -> Result<String, CommandError> {
            Ok(self.response.clone())
        }
    }

    fn candidate(provider: crate::domain::InventoryProvider) -> crate::domain::UpdateCandidate {
        crate::domain::UpdateCandidate {
            identity: Evidence::Observed("Example Mod".into()),
            current_version: Evidence::Observed("1.0.0".into()),
            available_version: Evidence::Observed("1.1.0".into()),
            local_path: Evidence::Observed("mods/example.pw.toml".into()),
            provider: Evidence::Observed(provider),
            side: Evidence::Unknown,
            pin: Evidence::Unknown,
            source_url: Evidence::Unknown,
            page_link: Some(crate::domain::TrustedPageLink {
                url: "https://modrinth.com/mod/example".into(),
                provider: crate::domain::InventoryProvider::Modrinth,
            }),
            severity: Evidence::Unknown,
            version_change: crate::domain::VersionChangeKind::Minor,
            output_evidence: "Example Mod: 1.0.0 -> 1.1.0".into(),
        }
    }

    #[test]
    fn exact_version_list_match_is_confirmed() {
        let transport = FixtureTransport {
            response: r#"[{"id":"version-id","project_id":"project-id","version_number":"1.1.0","changelog":"Fixed a bug"}]"#.into(),
        };
        let result = fetch_modrinth(
            &transport,
            &candidate(crate::domain::InventoryProvider::Modrinth),
        );
        assert_eq!(result.retrieval, ChangelogRetrievalStatus::Retrieved);
        assert_eq!(
            result.match_evidence.confidence,
            ProviderMatchConfidence::Exact
        );
        assert_eq!(result.changelog.as_deref(), Some("Fixed a bug"));
    }

    #[test]
    fn packwiz_jar_target_matches_modrinth_loader_version() {
        let transport = FixtureTransport {
            response: r#"[{"id":"version-id","project_id":"project-id","version_number":"neoforge-1.21.1-0.4.14","loaders":["neoforge"],"game_versions":["1.21.1"],"changelog":"Fixed a bug"}]"#.into(),
        };
        let mut candidate = candidate(crate::domain::InventoryProvider::Modrinth);
        candidate.available_version =
            Evidence::Observed("ubesdelight-neoforge-1.21.1-0.4.14.jar".into());
        let result = fetch_modrinth(&transport, &candidate);
        assert_eq!(result.retrieval, ChangelogRetrievalStatus::Retrieved);
        assert_eq!(
            result.match_evidence.confidence,
            ProviderMatchConfidence::High
        );
        assert_eq!(
            result
                .match_evidence
                .version
                .as_ref()
                .and_then(|version| version.version_number.as_deref()),
            Some("neoforge-1.21.1-0.4.14")
        );
    }

    #[test]
    fn normalized_version_match_is_high_confidence() {
        let transport = FixtureTransport {
            response: r#"[{"id":"version-id","project_id":"project-id","version_number":"0.4.14","changelog":"Fixed a bug"}]"#.into(),
        };
        let mut candidate = candidate(crate::domain::InventoryProvider::Modrinth);
        candidate.available_version = Evidence::Observed("ubesdelight-0.4.14.jar".into());
        let result = fetch_modrinth(&transport, &candidate);
        assert_eq!(result.retrieval, ChangelogRetrievalStatus::Retrieved);
        assert_eq!(
            result.match_evidence.confidence,
            ProviderMatchConfidence::High
        );
    }

    #[test]
    fn exact_provider_file_match_overrides_version_number_shape() {
        let transport = FixtureTransport {
            response: r#"[{"id":"version-id","project_id":"project-id","version_number":"release-2026","files":[{"filename":"example-neoforge-1.21.1-0.4.14.jar"}],"changelog":"Fixed a bug"}]"#.into(),
        };
        let mut candidate = candidate(crate::domain::InventoryProvider::Modrinth);
        candidate.available_version =
            Evidence::Observed("example-neoforge-1.21.1-0.4.14.jar".into());
        let result = fetch_modrinth(&transport, &candidate);
        assert_eq!(result.retrieval, ChangelogRetrievalStatus::Retrieved);
        assert_eq!(
            result.match_evidence.confidence,
            ProviderMatchConfidence::Exact
        );
    }

    #[test]
    fn normalized_version_matches_remain_ambiguous_across_releases() {
        let transport = FixtureTransport {
            response: r#"[{"id":"one","version_number":"0.4.14","loaders":["fabric"]},{"id":"two","version_number":"neoforge-1.21.1-0.4.14","loaders":["neoforge"]}]"#.into(),
        };
        let mut candidate = candidate(crate::domain::InventoryProvider::Modrinth);
        candidate.available_version = Evidence::Observed("ubesdelight-0.4.14.jar".into());
        let result = fetch_modrinth(&transport, &candidate);
        assert_eq!(result.retrieval, ChangelogRetrievalStatus::Ambiguous);
        assert_eq!(
            result.match_evidence.confidence,
            ProviderMatchConfidence::Ambiguous
        );
    }

    #[test]
    fn duplicate_exact_versions_remain_ambiguous() {
        let transport = FixtureTransport {
            response:
                r#"[{"id":"one","version_number":"1.1.0"},{"id":"two","version_number":"1.1.0"}]"#
                    .into(),
        };
        let result = fetch_modrinth(
            &transport,
            &candidate(crate::domain::InventoryProvider::Modrinth),
        );
        assert_eq!(result.retrieval, ChangelogRetrievalStatus::Ambiguous);
        assert_eq!(
            result.match_evidence.confidence,
            ProviderMatchConfidence::Ambiguous
        );
    }

    #[test]
    fn curseforge_candidate_is_not_guessed_as_modrinth() {
        let transport = FixtureTransport {
            response: "{}".into(),
        };
        let result = fetch_modrinth(
            &transport,
            &candidate(crate::domain::InventoryProvider::Curseforge),
        );
        assert_eq!(result.retrieval, ChangelogRetrievalStatus::Unresolved);
        assert_eq!(
            result.match_evidence.confidence,
            ProviderMatchConfidence::Unresolved
        );
    }
}

impl ChangelogRuntime {
    fn begin(&self) {
        self.cancel_requested.store(false, Ordering::Release);
    }

    fn cancel(&self) {
        self.cancel_requested.store(true, Ordering::Release);
    }

    fn is_cancelled(&self) -> bool {
        self.cancel_requested.load(Ordering::Acquire)
    }
}

fn emit_progress(app: &AppHandle, progress: crate::domain::ChangelogProgress) {
    let _ = app.emit("changelog-progress", progress);
}

fn source_allowed(snapshot: &SnapshotRecord, source: &ChangelogSourceKind) -> bool {
    matches!(
        (&snapshot.lifecycle, source),
        (
            SnapshotLifecycle::Reviewable,
            ChangelogSourceKind::DiscoveryCandidates
        ) | (
            SnapshotLifecycle::Closed,
            ChangelogSourceKind::AppliedOperation
        ) | (
            SnapshotLifecycle::Reviewable,
            ChangelogSourceKind::ReleaseWorkspaceEvidence
        ) | (
            SnapshotLifecycle::Closed,
            ChangelogSourceKind::ReleaseWorkspaceEvidence
        )
    )
}

fn external_observation_notes(
    database: &db::Database,
    workspace_id: &str,
) -> Result<Vec<String>, CommandError> {
    let connection = database
        .0
        .lock()
        .map_err(|_| CommandError::new("database_unavailable", "Database is unavailable"))?;
    let mut statement = connection
        .prepare("SELECT observed_at, changed_scope_json, overlapped_operation FROM release_workspace_observations WHERE workspace_id = ?1 ORDER BY id")
        .map_err(|error| CommandError::new("database_read_failed", "External observations could not be read").with_details(error.to_string()))?;
    let result = statement
        .query_map([workspace_id], |row| {
            let observed_at: String = row.get(0)?;
            let scope_json: String = row.get(1)?;
            let overlapped: bool = row.get(2)?;
            let scope: Vec<String> =
                serde_json::from_str(&scope_json).map_err(|_| rusqlite::Error::InvalidQuery)?;
            Ok(format!(
                "{}{}: {}",
                observed_at,
                if overlapped {
                    " (overlapped an app operation)"
                } else {
                    ""
                },
                scope.join(", ")
            ))
        })
        .map_err(|error| {
            CommandError::new(
                "database_read_failed",
                "External observations could not be read",
            )
            .with_details(error.to_string())
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| {
            CommandError::new(
                "database_record_invalid",
                "An external observation is invalid",
            )
            .with_details(error.to_string())
        });
    result
}

fn unresolved(candidate: &crate::domain::UpdateCandidate, offline: bool) -> ChangelogEntryResult {
    let local = LocalEntryIdentity {
        entry_id: match &candidate.identity {
            Evidence::Observed(value) => value.clone(),
            _ => candidate.output_evidence.clone(),
        },
        metadata_path: match &candidate.local_path {
            Evidence::Observed(value) => value.clone(),
            _ => String::new(),
        },
        local_provider: match &candidate.provider {
            Evidence::Observed(value) => value.clone(),
            _ => crate::domain::InventoryProvider::Unknown,
        },
        local_identity: candidate.identity.clone(),
        local_version: candidate.current_version.clone(),
    };
    let status = if offline {
        ChangelogRetrievalStatus::OfflineUnavailable
    } else {
        ChangelogRetrievalStatus::Unresolved
    };
    ChangelogEntryResult {
        local,
        match_evidence: ProviderMatchEvidence {
            confidence: ProviderMatchConfidence::Unresolved,
            project: None,
            version: None,
            association: None,
            candidates: Vec::new(),
            reason: "A strong Modrinth project and version association was not available".into(),
        },
        retrieval: status,
        changelog: None,
        diagnostic: None,
    }
}

fn modrinth_project_slug(candidate: &crate::domain::UpdateCandidate) -> Option<&str> {
    candidate
        .page_link
        .as_ref()
        .map(|link| link.url.trim_end_matches('/').rsplit('/').next())
        .flatten()
        .or_else(|| match &candidate.source_url {
            Evidence::Observed(url) => url.trim_end_matches('/').rsplit('/').next(),
            _ => None,
        })
}

fn fetch_modrinth<T: ModrinthTransport>(
    transport: &T,
    candidate: &crate::domain::UpdateCandidate,
) -> ChangelogEntryResult {
    let mut result = unresolved(candidate, false);
    let Evidence::Observed(provider) = &candidate.provider else {
        return result;
    };
    if !matches!(provider, crate::domain::InventoryProvider::Modrinth) {
        result.match_evidence.reason = "The local download provider is not Modrinth".into();
        return result;
    }
    let Some(url) = candidate
        .page_link
        .as_ref()
        .map(|link| link.url.as_str())
        .or_else(|| match &candidate.source_url {
            Evidence::Observed(value) => Some(value.as_str()),
            _ => None,
        })
    else {
        return result;
    };
    let Some(slug) = url.trim_end_matches('/').rsplit('/').next() else {
        return result;
    };
    if slug.is_empty() || url.contains("?") {
        return result;
    }
    let Evidence::Observed(target) = &candidate.available_version else {
        return result;
    };
    eprintln!(
        "[changelog] resolving identity local={} provider={provider:?} page_url={url} slug={slug} current_version={:?} target={target}",
        evidence_debug(&candidate.identity),
        evidence_debug(&candidate.current_version),
    );
    let hints = local_version_hints(target);
    let raw = match transport.get_versions(
        slug,
        &ModrinthVersionQuery {
            loader: hints.loader.clone(),
            game_version: hints.game_version.clone(),
        },
    ) {
        Ok(raw) => raw,
        Err(error) if error.code == "provider_version_missing" => {
            eprintln!("[changelog] provider version missing slug={slug} requested_target={target}");
            result.retrieval = ChangelogRetrievalStatus::Missing;
            result.match_evidence.reason =
                "Modrinth does not have the requested local version".into();
            return result;
        }
        Err(error) => {
            eprintln!(
                "[changelog] provider request failed slug={slug} requested_target={target} code={} message={}",
                error.code,
                error.message
            );
            result.retrieval = ChangelogRetrievalStatus::Failed;
            result.diagnostic = Some(error);
            return result;
        }
    };
    let version: Value = match serde_json::from_str(&raw) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("[changelog] provider response JSON parse failed slug={slug} error={error}");
            result.retrieval = ChangelogRetrievalStatus::Failed;
            result.diagnostic = Some(
                CommandError::new("provider_response_invalid", "Modrinth response was invalid")
                    .with_details(error.to_string()),
            );
            return result;
        }
    };
    let version = if version.is_array() {
        let versions = version
            .as_array()
            .expect("array value should have an array representation");
        eprintln!(
            "[changelog] version candidates slug={slug} requested_target={target} count={} values={}",
            versions.len(),
            version_candidates_debug(versions)
        );
        let selection = select_modrinth_version(versions, target, &hints);
        if selection.matches.len() != 1 {
            eprintln!(
                "[changelog] exact version match count={} slug={slug} requested_target={target}",
                selection.matches.len()
            );
            result.retrieval = ChangelogRetrievalStatus::Ambiguous;
            result.match_evidence.confidence = ProviderMatchConfidence::Ambiguous;
            result.match_evidence.candidates = selection
                .matches
                .iter()
                .map(|value| value_string(value, "id"))
                .collect();
            result.match_evidence.reason = if selection.matches.is_empty() {
                "Modrinth returned no version matching the normalized local version".into()
            } else {
                "Multiple Modrinth versions matched the normalized local version".into()
            };
            return result;
        }
        selection.matches[0]
    } else {
        eprintln!(
            "[changelog] provider returned single object slug={slug} requested_target={target} id={} version_number={}",
            value_string(&version, "id"),
            value_string(&version, "version_number")
        );
        &version
    };
    let provider_version = version.get("version_number").and_then(Value::as_str);
    let exact_file_match = version_has_file(version, target);
    if !exact_file_match
        && !provider_version_matches(target, provider_version)
        && version.get("id").and_then(Value::as_str) != Some(target)
    {
        eprintln!(
            "[changelog] selected provider version did not match slug={slug} requested_target={target} id={} version_number={}",
            value_string(version, "id"),
            value_string(version, "version_number")
        );
        result.retrieval = ChangelogRetrievalStatus::Ambiguous;
        result.match_evidence.confidence = ProviderMatchConfidence::Ambiguous;
        result.match_evidence.reason =
            "Modrinth returned a different version number than requested".into();
        return result;
    }
    let project_id = version
        .get("project_id")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let version_id = version
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let changelog = version
        .get("changelog")
        .and_then(Value::as_str)
        .map(str::to_owned);
    let exact_provider_match = provider_version == Some(target)
        || version.get("id").and_then(Value::as_str) == Some(target);
    let confidence = if exact_file_match || exact_provider_match {
        ProviderMatchConfidence::Exact
    } else {
        ProviderMatchConfidence::High
    };
    result.match_evidence = ProviderMatchEvidence {
        confidence,
        project: Some(ModrinthProjectIdentity {
            modpack_id: project_id.into(),
            slug: Some(slug.into()),
            title: None,
        }),
        version: Some(ModrinthVersionIdentity {
            version_id: version_id.into(),
            version_number: provider_version.map(str::to_owned),
            game_versions: string_array(version, "game_versions"),
            loaders: string_array(version, "loaders"),
        }),
        association: Some(VersionAssociationEvidence {
            local_version: candidate.current_version.clone(),
            provider_version: provider_version.map(str::to_owned),
            matched_exactly: exact_file_match || exact_provider_match,
            details: vec![if exact_file_match {
                "Exact Modrinth file-name match".into()
            } else if exact_provider_match {
                "Exact Modrinth version match".into()
            } else {
                "Unique normalized mod-version match".into()
            }],
        }),
        candidates: Vec::new(),
        reason: "Project slug and provider version were obtained from local Modrinth evidence"
            .into(),
    };
    result.retrieval = if changelog.is_some() {
        ChangelogRetrievalStatus::Retrieved
    } else {
        ChangelogRetrievalStatus::Missing
    };
    result.changelog = changelog;
    result
}

fn evidence_debug<T: std::fmt::Debug>(evidence: &Evidence<T>) -> String {
    format!("{evidence:?}")
}

fn value_string(value: &Value, field: &str) -> String {
    value
        .get(field)
        .and_then(Value::as_str)
        .unwrap_or("<missing>")
        .to_string()
}

#[derive(Debug, Default)]
struct LocalVersionHints {
    normalized_version: Option<String>,
    loader: Option<String>,
    game_version: Option<String>,
}

fn local_version_hints(target: &str) -> LocalVersionHints {
    let filename = target.trim().to_ascii_lowercase();
    let base = filename.strip_suffix(".jar").unwrap_or(&filename);
    let loaders = ["neoforge", "fabric", "quilt", "forge"];
    for loader in loaders {
        if let Some((_, suffix)) = base.split_once(&format!("-{loader}-")) {
            let mut parts = suffix.splitn(2, '-');
            let first = parts.next().unwrap_or_default();
            let remainder = parts.next().unwrap_or_default();
            if !remainder.is_empty() {
                return LocalVersionHints {
                    normalized_version: Some(remainder.to_string()),
                    loader: Some(loader.to_string()),
                    game_version: Some(first.to_string()),
                };
            }
        }
    }
    let normalized_version = base
        .rsplit('-')
        .find(|part| {
            part.chars()
                .next()
                .is_some_and(|value| value.is_ascii_digit())
        })
        .map(str::to_string);
    LocalVersionHints {
        normalized_version,
        ..LocalVersionHints::default()
    }
}

fn normalized_provider_version(version: &str, hints: &LocalVersionHints) -> Option<String> {
    let value = version.trim().to_ascii_lowercase();
    if let Some(loader) = &hints.loader {
        if let Some(suffix) = value.strip_prefix(&format!("{loader}-")) {
            let mut parts = suffix.splitn(2, '-');
            parts.next();
            return parts.next().map(str::to_string);
        }
    }
    value
        .rsplit('-')
        .find(|part| {
            part.chars()
                .next()
                .is_some_and(|value| value.is_ascii_digit())
        })
        .map(str::to_string)
}

fn version_has_file(version: &Value, target: &str) -> bool {
    let target = target.trim().to_ascii_lowercase();
    version
        .get("files")
        .and_then(Value::as_array)
        .is_some_and(|files| {
            files.iter().any(|file| {
                file.get("filename")
                    .and_then(Value::as_str)
                    .map(|filename| filename.eq_ignore_ascii_case(&target))
                    .unwrap_or(false)
            })
        })
}

fn string_array(value: &Value, field: &str) -> Vec<String> {
    value
        .get(field)
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

struct VersionSelection<'a> {
    matches: Vec<&'a Value>,
}

fn select_modrinth_version<'a>(
    versions: &'a [Value],
    target: &str,
    hints: &LocalVersionHints,
) -> VersionSelection<'a> {
    let exact_files = versions
        .iter()
        .filter(|value| version_has_file(value, target))
        .collect::<Vec<_>>();
    if !exact_files.is_empty() {
        return VersionSelection {
            matches: exact_files,
        };
    }
    let matches = versions
        .iter()
        .filter(|value| {
            let compatible_loader = hints.loader.as_ref().is_none_or(|loader| {
                string_array(value, "loaders")
                    .iter()
                    .any(|candidate| candidate.eq_ignore_ascii_case(loader))
            });
            let compatible_game = hints.game_version.as_ref().is_none_or(|game_version| {
                string_array(value, "game_versions")
                    .iter()
                    .any(|candidate| candidate.eq_ignore_ascii_case(game_version))
            });
            let provider_version = value.get("version_number").and_then(Value::as_str);
            let normalized_match = provider_version
                .and_then(|version| normalized_provider_version(version, hints))
                .zip(hints.normalized_version.as_ref())
                .is_some_and(|(provider, local)| provider == *local);
            let id_match = value.get("id").and_then(Value::as_str) == Some(target);
            compatible_loader && compatible_game && (normalized_match || id_match)
        })
        .collect();
    VersionSelection { matches }
}

fn provider_version_matches(local_target: &str, provider_version: Option<&str>) -> bool {
    let hints = local_version_hints(local_target);
    let Some(provider_version) = provider_version else {
        return false;
    };
    provider_version == local_target
        || normalized_provider_version(provider_version, &hints)
            .zip(hints.normalized_version)
            .is_some_and(|(provider, local)| provider == local)
}

fn version_candidates_debug(values: &[Value]) -> String {
    values
        .iter()
        .map(|value| {
            format!(
                "{}:{}",
                value_string(value, "id"),
                value_string(value, "version_number")
            )
        })
        .collect::<Vec<_>>()
        .join(",")
}

fn cached_modrinth(
    database: &db::Database,
    candidate: &crate::domain::UpdateCandidate,
) -> Option<ChangelogEntryResult> {
    let Some(slug) = modrinth_project_slug(candidate) else {
        return None;
    };
    let Evidence::Observed(target) = &candidate.available_version else {
        return None;
    };
    let Evidence::Observed(current_version) = &candidate.current_version else {
        return None;
    };
    let (raw, association) =
        db::cached_changelog_response_for_local_version(database, slug, current_version).ok()??;
    if !matches!(
        association.confidence,
        ProviderMatchConfidence::Exact | ProviderMatchConfidence::High
    ) || !provider_version_matches(
        target,
        association
            .version
            .as_ref()
            .and_then(|version| version.version_number.as_deref()),
    ) || association.association.is_none()
    {
        return None;
    }
    let value: Value = serde_json::from_str(&raw).ok()?;
    let version = if value.is_array() {
        value.as_array()?.iter().find(|value| {
            provider_version_matches(target, value.get("version_number").and_then(Value::as_str))
                || value.get("id").and_then(Value::as_str) == Some(target)
        })?
    } else {
        &value
    };
    let changelog = version
        .get("changelog")
        .and_then(Value::as_str)
        .map(str::to_owned);
    let mut result = unresolved(candidate, false);
    result.match_evidence = association;
    result.retrieval = if changelog.is_some() {
        ChangelogRetrievalStatus::Cached
    } else {
        ChangelogRetrievalStatus::Missing
    };
    result.changelog = changelog;
    Some(result)
}

#[tauri::command]
pub fn generate_changelog(
    app: AppHandle,
    database: State<'_, db::Database>,
    runtime: State<'_, ChangelogRuntime>,
    request: ChangelogGenerationRequest,
) -> Result<ChangelogArtifact, CommandError> {
    runtime.begin();
    let snapshot = db::load_snapshot(&database, &request.snapshot_id)?;
    if snapshot.modpack_id != request.modpack_id
        || !source_allowed(&snapshot, &request.source)
        || (matches!(
            request.source,
            ChangelogSourceKind::ReleaseWorkspaceEvidence
        ) && request.release_workspace_id.is_none())
    {
        return Err(CommandError::new(
            "changelog_source_unavailable",
            "The selected snapshot cannot generate a changelog",
        ));
    }
    let total = snapshot.candidates.len() as u32;
    let attempt_id = db::new_changelog_attempt_id();
    let mut entries = Vec::with_capacity(snapshot.candidates.len());
    let transport = if request.offline {
        None
    } else {
        Some(HttpModrinthTransport::new()?)
    };
    for (index, record) in snapshot.candidates.iter().enumerate() {
        if runtime.is_cancelled() {
            break;
        }
        let number = index as u32 + 1;
        emit_progress(
            &app,
            crate::domain::ChangelogProgress {
                attempt_id: attempt_id.clone(),
                completed: index as u32,
                total,
                message: format!("Looking up changelog {number} of {total}"),
                cancellable: false,
            },
        );
        let result = if request.offline {
            cached_modrinth(&database, &record.candidate)
                .unwrap_or_else(|| unresolved(&record.candidate, true))
        } else {
            let result = fetch_modrinth(
                transport.as_ref().expect("online transport should exist"),
                &record.candidate,
            );
            if let (
                Some(project),
                Some(version),
                Evidence::Observed(_url),
                Evidence::Observed(_target),
            ) = (
                &result.match_evidence.project,
                &result.match_evidence.version,
                &record.candidate.source_url,
                &record.candidate.available_version,
            ) {
                let _ = db::cache_changelog_response(
                    &database,
                    &crate::domain::ChangelogCacheKey {
                        provider: "modrinth".into(),
                        endpoint: "project_versions".into(),
                        request_shape: "version_changelog".into(),
                        project_id: modrinth_project_slug(&record.candidate).unwrap_or_default().into(),
                        version_id: version.version_id.clone(),
                        game_version: None,
                        loader: None,
                        requested_fields: vec!["changelog".into()],
                    },
                    &serde_json::json!({"id": version.version_id, "project_id": project.modpack_id, "version_number": version.version_number, "changelog": result.changelog}).to_string(),
                    &result.match_evidence,
                );
            }
            result
        };
        entries.push(result);
        emit_progress(
            &app,
            crate::domain::ChangelogProgress {
                attempt_id: request.request_fingerprint.clone(),
                completed: number,
                total,
                message: format!("Completed changelog lookup {number} of {total}"),
                cancellable: false,
            },
        );
        if !request.offline && number < total {
            let mut remaining = LOOKUP_DELAY;
            while remaining > Duration::ZERO && !runtime.is_cancelled() {
                let interval = remaining.min(Duration::from_millis(100));
                thread::sleep(interval);
                remaining -= interval;
            }
        }
    }
    let confirmed = entries
        .iter()
        .filter(|entry| {
            matches!(
                entry.retrieval,
                ChangelogRetrievalStatus::Retrieved | ChangelogRetrievalStatus::Cached
            )
        })
        .count();
    let status = if runtime.is_cancelled() {
        ChangelogGenerationStatus::Cancelled
    } else if confirmed == entries.len() {
        ChangelogGenerationStatus::Complete
    } else if confirmed > 0 {
        ChangelogGenerationStatus::Partial
    } else {
        ChangelogGenerationStatus::Failed
    };
    let mut content = request.introduction.clone().unwrap_or_default();
    if !content.is_empty() {
        content.push_str("\n\n");
    }
    for entry in &entries {
        let name = match &entry.local.local_identity {
            Evidence::Observed(value) => value.as_str(),
            _ => "Unknown mod",
        };
        content.push_str(&format!("## {name}\n\n"));
        content.push_str(
            entry
                .changelog
                .as_deref()
                .unwrap_or("Changelog unavailable or not confirmed."),
        );
        content.push_str("\n\n");
    }
    if matches!(
        request.source,
        ChangelogSourceKind::ReleaseWorkspaceEvidence
    ) {
        let observations = external_observation_notes(
            &database,
            request
                .release_workspace_id
                .as_deref()
                .expect("validated workspace id"),
        )?;
        if !observations.is_empty() {
            content.push_str("## Externally observed changes\n\n");
            content.push_str("These changes were observed in the modpack files outside CM Modpack Util and are not labeled as app-applied updates.\n\n");
            for observation in observations {
                content.push_str(&format!("- {observation}\n"));
            }
            content.push('\n');
        }
    }
    let artifact = ChangelogArtifact {
        id: format!("artifact-{attempt_id}"),
        modpack_id: request.modpack_id.clone(),
        snapshot_id: request.snapshot_id.clone(),
        release_workspace_id: request.release_workspace_id.clone(),
        stage: crate::domain::ChangelogStage::Proposed,
        source_capture_fingerprint: None,
        attempt_id: attempt_id.clone(),
        status,
        introduction: request.introduction.clone(),
        content,
        entries,
        created_at: String::new(),
        updated_at: String::new(),
    };
    emit_progress(
        &app,
        crate::domain::ChangelogProgress {
            attempt_id: request.request_fingerprint.clone(),
            completed: total,
            total,
            message: "Saving changelog artifact".into(),
            cancellable: false,
        },
    );
    db::persist_changelog_artifact(&database, &request, &artifact)?;
    emit_progress(
        &app,
        crate::domain::ChangelogProgress {
            attempt_id,
            completed: total,
            total,
            message: "Changelog generation complete".into(),
            cancellable: false,
        },
    );
    Ok(artifact)
}

#[tauri::command]
pub fn cancel_changelog_generation(
    runtime: State<'_, ChangelogRuntime>,
) -> Result<(), CommandError> {
    runtime.cancel();
    Ok(())
}

#[tauri::command]
pub fn create_changelog_revision(
    database: State<'_, db::Database>,
    request: crate::domain::ChangelogRevisionRequest,
) -> Result<crate::domain::ChangelogRevision, CommandError> {
    db::persist_changelog_revision(&database, &request)
}

#[tauri::command]
pub fn get_changelog_artifact(
    database: State<'_, db::Database>,
    artifact_id: String,
) -> Result<crate::domain::ChangelogArtifact, CommandError> {
    db::load_changelog_artifact(&database, &artifact_id)
}

#[tauri::command]
pub fn list_changelog_artifacts(
    database: State<'_, db::Database>,
    modpack_id: String,
) -> Result<Vec<crate::domain::ChangelogArtifact>, CommandError> {
    db::list_changelog_artifacts(&database, &modpack_id)
}

#[tauri::command]
pub fn list_changelog_revisions(
    database: State<'_, db::Database>,
    artifact_id: String,
) -> Result<Vec<crate::domain::ChangelogRevision>, CommandError> {
    db::list_changelog_revisions(&database, &artifact_id)
}

#[tauri::command]
pub fn list_changelog_exports(
    database: State<'_, db::Database>,
    artifact_id: String,
) -> Result<Vec<crate::domain::ChangelogExport>, CommandError> {
    db::list_changelog_exports(&database, &artifact_id)
}

#[tauri::command]
pub fn export_changelog(
    database: State<'_, db::Database>,
    request: crate::domain::ChangelogExportRequest,
) -> Result<crate::domain::ChangelogExport, CommandError> {
    let result = std::fs::write(&request.destination, &request.content);
    match result {
        Ok(()) => db::persist_changelog_export(
            &database,
            &request,
            crate::domain::ChangelogExportStatus::Exported,
            None,
        ),
        Err(error) => {
            let diagnostic = CommandError::new(
                "export_unavailable",
                "Markdown could not be written to the selected destination",
            )
            .with_details(error.to_string());
            let _ = db::persist_changelog_export(
                &database,
                &request,
                crate::domain::ChangelogExportStatus::Unavailable,
                Some(diagnostic.clone()),
            );
            Ok(crate::domain::ChangelogExport {
                id: String::new(),
                artifact_id: request.artifact_id,
                revision_id: request.revision_id,
                destination: request.destination,
                format: "markdown".into(),
                content: request.content,
                content_hash: String::new(),
                status: crate::domain::ChangelogExportStatus::Unavailable,
                exported_at: String::new(),
                diagnostic: Some(diagnostic),
            })
        }
    }
}
