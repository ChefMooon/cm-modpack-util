use crate::changelog_transport::{
    HttpModrinthTransport, ModrinthTransport, ModrinthVersionQuery, RequestControl,
};
use crate::db;
use crate::domain::{
    ChangelogArtifact, ChangelogContentStatus, ChangelogEntryResult, ChangelogGenerationRequest,
    ChangelogGenerationStatus, ChangelogRetrievalStatus, ChangelogSourceKind,
    ChangelogVersionResult, CommandError, Evidence, LocalEntryIdentity, ModrinthProjectIdentity,
    ModrinthVersionIdentity, ProviderMatchConfidence, ProviderMatchEvidence, SnapshotLifecycle,
    SnapshotRecord, UpdateCandidate, VersionAssociationEvidence,
};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tauri::State;
use tauri::{AppHandle, Emitter};

const LOOKUP_DELAY: Duration = Duration::from_secs(3);
const MAX_PROJECT_PAGES: u32 = 10;
const MAX_PROVIDER_VERSIONS: usize = 500;
const MAX_PROVIDER_REQUESTS: usize = 100;

pub struct ChangelogRuntime {
    active: Mutex<Option<Arc<GenerationContext>>>,
}

impl Default for ChangelogRuntime {
    fn default() -> Self {
        Self {
            active: Mutex::new(None),
        }
    }
}

struct GenerationContext {
    attempt_id: String,
    total: AtomicU32,
    completed: AtomicU32,
    requests: AtomicUsize,
    cancelled: AtomicBool,
}

impl GenerationContext {
    fn new(attempt_id: String, total: u32) -> Self {
        Self {
            attempt_id,
            total: AtomicU32::new(total),
            completed: AtomicU32::new(0),
            requests: AtomicUsize::new(0),
            cancelled: AtomicBool::new(false),
        }
    }

    fn progress(
        &self,
        completed: u32,
        message: String,
        cancellable: bool,
    ) -> crate::domain::ChangelogProgress {
        self.completed.store(completed, Ordering::Release);
        crate::domain::ChangelogProgress {
            attempt_id: self.attempt_id.clone(),
            completed,
            total: self.total.load(Ordering::Acquire),
            message,
            cancellable,
        }
    }
}

#[cfg(test)]
mod cancellation_tests {
    use super::ChangelogRuntime;
    use crate::changelog_transport::RequestControl;
    use std::sync::atomic::Ordering;

    #[test]
    fn cancellation_can_be_requested_and_reset_for_a_new_attempt() {
        let runtime = ChangelogRuntime::default();
        let first = runtime.begin("first".into(), 1);
        assert!(!first.is_cancelled());
        runtime.cancel();
        assert!(first.is_cancelled());
        let second = runtime.begin("second".into(), 1);
        assert!(!second.is_cancelled());
        assert!(first.is_cancelled());
    }

    #[test]
    fn request_budget_is_owned_by_one_attempt() {
        let runtime = ChangelogRuntime::default();
        let context = runtime.begin("attempt".into(), 1);
        assert!(context.before_request().is_ok());
        assert_eq!(context.requests.load(Ordering::Acquire), 1);
    }

    #[test]
    fn changelog_version_label_uses_release_suffix() {
        assert_eq!(
            super::changelog_version_label(Some("neoforge-1.21.1-1.4.7")),
            "1.4.7"
        );
        assert_eq!(
            super::changelog_version_label(Some("1.4.7-beta")),
            "1.4.7-beta"
        );
        assert_eq!(super::changelog_version_label(Some("1.4.7")), "1.4.7");
        assert_eq!(super::changelog_version_label(None), "Provider release");
    }
}

#[cfg(test)]
mod provider_tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct FixtureTransport {
        response: String,
    }

    struct PagedFixtureTransport {
        calls: AtomicUsize,
    }

    struct CappedFixtureTransport;

    impl ModrinthTransport for CappedFixtureTransport {
        fn get_versions(
            &self,
            _project: &str,
            query: &ModrinthVersionQuery,
        ) -> Result<String, CommandError> {
            let mut versions = (0..crate::changelog_transport::MAX_PAGE_SIZE)
                .map(|index| {
                    format!(
                        r#"{{"id":"v{}","version_number":"1.0.{}","changelog":"Change"}}"#,
                        query.offset + index,
                        query.offset + index
                    )
                })
                .collect::<Vec<_>>();
            if query.offset == 0 {
                versions[0] =
                    r#"{"id":"current","version_number":"1.0.0","changelog":"Current"}"#.into();
                versions[1] =
                    r#"{"id":"target","version_number":"1.1.0","changelog":"Target"}"#.into();
            }
            Ok(format!("[{}]", versions.join(",")))
        }
    }

    impl ModrinthTransport for PagedFixtureTransport {
        fn get_versions(
            &self,
            _project: &str,
            query: &ModrinthVersionQuery,
        ) -> Result<String, CommandError> {
            self.calls.fetch_add(1, Ordering::AcqRel);
            if query.offset == 0 {
                Ok(format!(
                    "[{}]",
                    (0..crate::changelog_transport::MAX_PAGE_SIZE)
                        .map(|index| format!(r#"{{"id":"v{index}","version_number":"{index}.0"}}"#))
                        .collect::<Vec<_>>()
                        .join(",")
                ))
            } else {
                Ok(r#"[{"id":"target","version_number":"1.1.0","changelog":"Target"}]"#.into())
            }
        }
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

    #[test]
    fn retains_intermediate_versions_and_excludes_empty_content() {
        let transport = FixtureTransport {
            response: r#"[
                {"id":"current","version_number":"1.0.0","changelog":" Current "},
                {"id":"middle","version_number":"1.0.5","changelog":"\n  "},
                {"id":"target","version_number":"1.1.0","changelog":" Target "}
            ]"#
            .into(),
        };
        let result = fetch_modrinth(
            &transport,
            &candidate(crate::domain::InventoryProvider::Modrinth),
        );
        assert_eq!(result.versions.len(), 3);
        assert_eq!(result.versions[1].changelog, None);
        assert!(!result.versions[1].included);
        assert_eq!(result.versions[2].changelog.as_deref(), Some("Target"));
    }

    #[test]
    fn contradictory_provider_dates_keep_evidence_but_block_default_selection() {
        let transport = FixtureTransport {
            response: r#"[
                {"id":"target","version_number":"1.1.0","date_published":"2026-01-01T00:00:00Z","date_created":"2026-01-01T00:00:00Z","changelog":"Target"},
                {"id":"middle","version_number":"1.0.5","date_published":"2026-02-01T00:00:00Z","date_created":"2026-02-01T00:00:00Z","changelog":"Middle"},
                {"id":"current","version_number":"1.0.0","date_published":"2026-03-01T00:00:00Z","date_created":"2026-03-01T00:00:00Z","changelog":"Current"}
            ]"#
            .into(),
        };
        let result = fetch_modrinth(
            &transport,
            &candidate(crate::domain::InventoryProvider::Modrinth),
        );
        assert_eq!(
            result.diagnostic.as_ref().map(|error| error.code.as_str()),
            Some("provider_order_ambiguous")
        );
        assert!(result.versions.iter().all(|version| !version.included));
        assert_eq!(result.versions[1].changelog.as_deref(), Some("Middle"));
    }

    #[test]
    fn pagination_requests_next_page_until_a_short_page() {
        let transport = PagedFixtureTransport {
            calls: AtomicUsize::new(0),
        };
        let result = fetch_modrinth(
            &transport,
            &candidate(crate::domain::InventoryProvider::Modrinth),
        );
        assert_eq!(transport.calls.load(Ordering::Acquire), 2);
        assert_eq!(result.retrieval, ChangelogRetrievalStatus::Retrieved);
    }

    #[test]
    fn pagination_cap_is_incomplete_and_does_not_select_intermediates() {
        let result = fetch_modrinth(
            &CappedFixtureTransport,
            &candidate(crate::domain::InventoryProvider::Modrinth),
        );
        assert_eq!(
            result.page_count as usize * crate::changelog_transport::MAX_PAGE_SIZE as usize,
            MAX_PROVIDER_VERSIONS
        );
        assert!(!result.pagination_complete);
        assert_eq!(
            result.diagnostic.as_ref().map(|error| error.code.as_str()),
            Some("provider_pagination_incomplete")
        );
        assert!(result.versions.iter().all(|version| !version.included));
    }

    #[test]
    fn cancelled_context_stops_before_the_next_page_request() {
        let transport = PagedFixtureTransport {
            calls: AtomicUsize::new(0),
        };
        let context = GenerationContext::new("cancelled".into(), 1);
        context.cancelled.store(true, Ordering::Release);
        let result = fetch_modrinth_with_context(
            &transport,
            &candidate(crate::domain::InventoryProvider::Modrinth),
            &context,
        );
        assert_eq!(transport.calls.load(Ordering::Acquire), 0);
        assert_eq!(result.retrieval, ChangelogRetrievalStatus::Failed);
        assert_eq!(
            result.diagnostic.as_ref().map(|error| error.code.as_str()),
            Some("changelog_cancelled")
        );
    }
}

impl ChangelogRuntime {
    fn begin(&self, attempt_id: String, total: u32) -> Arc<GenerationContext> {
        let context = Arc::new(GenerationContext::new(attempt_id, total));
        if let Ok(mut active) = self.active.lock() {
            if let Some(previous) = active.replace(context.clone()) {
                previous.cancelled.store(true, Ordering::Release);
            }
        }
        context
    }

    fn cancel(&self) {
        if let Ok(active) = self.active.lock() {
            if let Some(context) = active.as_ref() {
                context.cancelled.store(true, Ordering::Release);
            }
        }
    }
}

impl RequestControl for GenerationContext {
    fn before_request(&self) -> Result<(), CommandError> {
        if self.is_cancelled() {
            return Err(CommandError::new(
                "changelog_cancelled",
                "Changelog generation was cancelled",
            ));
        }
        let request = self.requests.fetch_add(1, Ordering::AcqRel) + 1;
        if request > MAX_PROVIDER_REQUESTS {
            return Err(CommandError::new(
                "provider_budget_exhausted",
                "The changelog provider request budget was exhausted",
            ));
        }
        Ok(())
    }

    fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }

    fn wait(&self, duration: Duration) {
        let mut remaining = duration;
        while remaining > Duration::ZERO && !self.is_cancelled() {
            let interval = remaining.min(Duration::from_millis(100));
            thread::sleep(interval);
            remaining -= interval;
        }
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
        versions: Vec::new(),
        pagination_complete: false,
        page_count: 0,
        raw_response: None,
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

#[cfg(test)]
fn fetch_modrinth<T: ModrinthTransport>(
    transport: &T,
    candidate: &crate::domain::UpdateCandidate,
) -> ChangelogEntryResult {
    let context = GenerationContext::new("test-attempt".into(), 1);
    fetch_modrinth_with_context(transport, candidate, &context)
}

fn fetch_modrinth_with_context<T: ModrinthTransport>(
    transport: &T,
    candidate: &crate::domain::UpdateCandidate,
    context: &GenerationContext,
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
    let mut all_versions = Vec::new();
    let mut page_count = 0;
    let mut pagination_complete = false;
    let mut raw = None;
    for page in 0..MAX_PROJECT_PAGES {
        let page_raw = match transport.get_versions_with_control(
            slug,
            &ModrinthVersionQuery {
                loader: hints.loader.clone(),
                game_version: hints.game_version.clone(),
                limit: crate::changelog_transport::MAX_PAGE_SIZE,
                offset: page * crate::changelog_transport::MAX_PAGE_SIZE,
            },
            context,
        ) {
            Ok(raw) => raw,
            Err(error) if error.code == "provider_version_missing" => {
                eprintln!(
                    "[changelog] provider version missing slug={slug} requested_target={target}"
                );
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
        let page_value: Value = match serde_json::from_str(&page_raw) {
            Ok(value) => value,
            Err(error) => {
                result.retrieval = ChangelogRetrievalStatus::Failed;
                result.diagnostic = Some(
                    CommandError::new("provider_response_invalid", "Modrinth response was invalid")
                        .with_details(error.to_string()),
                );
                return result;
            }
        };
        let Some(page_versions) = page_value.as_array() else {
            raw = Some(page_raw);
            break;
        };
        page_count += 1;
        all_versions.extend(
            page_versions
                .iter()
                .take(MAX_PROVIDER_VERSIONS.saturating_sub(all_versions.len()))
                .cloned(),
        );
        raw = Some(serde_json::to_string(&all_versions).unwrap_or(page_raw));
        if page_versions.len() < crate::changelog_transport::MAX_PAGE_SIZE as usize {
            pagination_complete = true;
            break;
        }
        if all_versions.len() >= MAX_PROVIDER_VERSIONS || page + 1 == MAX_PROJECT_PAGES {
            break;
        }
    }
    let raw = raw.unwrap_or_else(|| "[]".into());
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
    let mut found_versions = Vec::new();
    let version = if version.is_array() {
        let raw_versions = version
            .as_array()
            .expect("array value should have an array representation");
        let versions = deduplicate_provider_versions(raw_versions);
        eprintln!(
            "[changelog] version candidates slug={slug} requested_target={target} count={} values={}",
            versions.len(),
            version_candidates_debug(&versions)
        );
        let selection = select_modrinth_version(&versions, target, &hints);
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
        let current_selection = match &candidate.current_version {
            Evidence::Observed(current) => {
                select_modrinth_version(&versions, current, &local_version_hints(current))
            }
            _ => VersionSelection {
                matches: Vec::new(),
            },
        };
        let selected_id = selection.matches[0].get("id").and_then(Value::as_str);
        let target_index = versions
            .iter()
            .position(|value| value.get("id").and_then(Value::as_str) == selected_id);
        let current_index = current_selection.matches.first().and_then(|value| {
            let current_id = value.get("id").and_then(Value::as_str);
            versions
                .iter()
                .position(|candidate| candidate.get("id").and_then(Value::as_str) == current_id)
        });
        let range = current_index
            .zip(target_index)
            .map(|(current, target)| (current.min(target), current.max(target)));
        let ordering_proven = provider_order_is_proven(&versions);
        for (position, value) in versions.iter().enumerate() {
            let Some((start, end)) = range else { break };
            if position < start || position > end {
                continue;
            }
            let provider_version_id = value_string(value, "id");
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
            if !compatible_loader || !compatible_game || provider_version_id == "<missing>" {
                continue;
            }
            let changelog = normalized_changelog(value.get("changelog").and_then(Value::as_str));
            let content_status = if changelog.is_some() {
                ChangelogContentStatus::Available
            } else {
                ChangelogContentStatus::Empty
            };
            let is_current = current_index == Some(position);
            found_versions.push(ChangelogVersionResult {
                version: version_identity(value),
                response_position: position as u32,
                is_current,
                published_at: value
                    .get("date_published")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                created_at: value
                    .get("date_created")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                changelog,
                retrieval: if content_status == ChangelogContentStatus::Available {
                    ChangelogRetrievalStatus::Retrieved
                } else {
                    ChangelogRetrievalStatus::Missing
                },
                content_status,
                included: pagination_complete
                    && ordering_proven
                    && !is_current
                    && content_status == ChangelogContentStatus::Available,
                diagnostic: None,
            });
        }
        if !ordering_proven {
            result.diagnostic = Some(CommandError::new(
                "provider_order_ambiguous",
                "Modrinth publication dates contradict or tie the documented response order; intermediate releases were retained but not selected by default",
            ));
        }
        selection.matches[0].clone()
    } else {
        eprintln!(
            "[changelog] provider returned single object slug={slug} requested_target={target} id={} version_number={}",
            value_string(&version, "id"),
            value_string(&version, "version_number")
        );
        version.clone()
    };
    let provider_version = version.get("version_number").and_then(Value::as_str);
    let exact_file_match = version_has_file(&version, target);
    if !exact_file_match
        && !provider_version_matches(target, provider_version)
        && version.get("id").and_then(Value::as_str) != Some(target)
    {
        eprintln!(
            "[changelog] selected provider version did not match slug={slug} requested_target={target} id={} version_number={}",
            value_string(&version, "id"),
            value_string(&version, "version_number")
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
    let changelog = normalized_changelog(version.get("changelog").and_then(Value::as_str));
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
            game_versions: string_array(&version, "game_versions"),
            loaders: string_array(&version, "loaders"),
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
    result.pagination_complete = pagination_complete;
    result.page_count = page_count;
    result.raw_response = Some(raw.clone());
    result.pagination_complete = pagination_complete;
    result.page_count = page_count;
    if !pagination_complete {
        result.diagnostic = Some(CommandError::new(
            "provider_pagination_incomplete",
            "Modrinth version history reached the pagination safety cap; intermediate releases were retained but not selected by default",
        ));
    }
    result.versions = if found_versions.is_empty() {
        vec![ChangelogVersionResult {
            version: version_identity(&version),
            response_position: 0,
            is_current: false,
            published_at: version
                .get("date_published")
                .and_then(Value::as_str)
                .map(str::to_owned),
            created_at: version
                .get("date_created")
                .and_then(Value::as_str)
                .map(str::to_owned),
            changelog: normalized_changelog(version.get("changelog").and_then(Value::as_str)),
            retrieval: result.retrieval.clone(),
            content_status: if version
                .get("changelog")
                .and_then(Value::as_str)
                .and_then(|text| (!text.trim().is_empty()).then_some(()))
                .is_some()
            {
                ChangelogContentStatus::Available
            } else {
                ChangelogContentStatus::Empty
            },
            included: pagination_complete && result.changelog.is_some(),
            diagnostic: None,
        }]
    } else {
        found_versions
    };
    result
}

fn changelog_version_label(version_number: Option<&str>) -> &str {
    let Some(version_number) = version_number else {
        return "Provider release";
    };

    version_number
        .match_indices('-')
        .rev()
        .find_map(|(index, _)| {
            let suffix = &version_number[index + 1..];
            suffix
                .chars()
                .next()
                .filter(char::is_ascii_digit)
                .map(|_| suffix)
        })
        .unwrap_or(version_number)
}

fn normalized_changelog(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

fn version_identity(value: &Value) -> ModrinthVersionIdentity {
    ModrinthVersionIdentity {
        version_id: value_string(value, "id"),
        version_number: value
            .get("version_number")
            .and_then(Value::as_str)
            .map(str::to_owned),
        game_versions: string_array(value, "game_versions"),
        loaders: string_array(value, "loaders"),
    }
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

fn deduplicate_provider_versions(values: &[Value]) -> Vec<Value> {
    let mut seen = HashSet::new();
    values
        .iter()
        .filter(|value| seen.insert(value_string(value, "id")))
        .cloned()
        .collect()
}

fn provider_order_is_proven(values: &[Value]) -> bool {
    values.windows(2).all(|window| {
        ["date_published", "date_created"].iter().all(|field| {
            let Some(previous) = window[0].get(*field).and_then(Value::as_str) else {
                return true;
            };
            let Some(next) = window[1].get(*field).and_then(Value::as_str) else {
                return true;
            };
            previous > next
        })
    })
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

struct CachedVersionTransport {
    response: String,
}

impl ModrinthTransport for CachedVersionTransport {
    fn get_versions(
        &self,
        _project: &str,
        _query: &ModrinthVersionQuery,
    ) -> Result<String, CommandError> {
        Ok(self.response.clone())
    }
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
    let hints = local_version_hints(target);
    let (raw, association, stale, complete, page_count) =
        db::cached_changelog_response_for_local_version_with_query(
            database,
            slug,
            current_version,
            hints.loader.as_deref(),
            hints.game_version.as_deref(),
        )
        .ok()??;
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
    let context = GenerationContext::new("cached-attempt".into(), 1);
    let mut result = fetch_modrinth_with_context(
        &CachedVersionTransport {
            response: raw.clone(),
        },
        candidate,
        &context,
    );
    result.match_evidence = association;
    result.retrieval = if result.changelog.is_some() {
        if stale {
            ChangelogRetrievalStatus::CachedStale
        } else {
            ChangelogRetrievalStatus::Cached
        }
    } else {
        ChangelogRetrievalStatus::Missing
    };
    result.pagination_complete = complete;
    result.page_count = page_count;
    result.raw_response = Some(raw);
    if !complete {
        result.diagnostic = Some(CommandError::new(
            "provider_pagination_incomplete",
            "Cached Modrinth history is incomplete; intermediate releases were retained but not selected by default",
        ));
        for version in &mut result.versions {
            version.included = false;
        }
    }
    Some(result)
}

#[tauri::command]
pub fn generate_changelog(
    app: AppHandle,
    database: State<'_, db::Database>,
    runtime: State<'_, ChangelogRuntime>,
    request: ChangelogGenerationRequest,
) -> Result<ChangelogArtifact, CommandError> {
    let candidates: Vec<UpdateCandidate> = if let Some(snapshot_id) = request.snapshot_id.as_deref()
    {
        let snapshot = db::load_snapshot(&database, snapshot_id)?;
        if snapshot.modpack_id != request.modpack_id || !source_allowed(&snapshot, &request.source)
        {
            return Err(CommandError::new(
                "changelog_source_unavailable",
                "The selected snapshot cannot generate a changelog",
            ));
        }
        snapshot
            .candidates
            .into_iter()
            .map(|record| record.candidate)
            .collect()
    } else if let Some(workspace_id) = request.release_workspace_id.as_deref() {
        let workspace = db::load_release_workspace_inner(&database, workspace_id)?;
        if workspace.modpack_id != request.modpack_id
            || !matches!(
                request.source,
                ChangelogSourceKind::ReleaseWorkspaceEvidence
            )
        {
            return Err(CommandError::new(
                "changelog_source_unavailable",
                "The release workspace cannot generate a changelog",
            ));
        }
        workspace
            .candidates
            .into_iter()
            .map(|record| record.candidate)
            .collect()
    } else {
        return Err(CommandError::new(
            "changelog_source_unavailable",
            "Changelog candidates are unavailable",
        ));
    };
    let total = candidates.len() as u32;
    let attempt_id = db::new_changelog_attempt_id();
    let context = runtime.begin(attempt_id.clone(), total);
    let mut entries = Vec::with_capacity(candidates.len());
    let mut requested_projects = HashSet::new();
    let mut fetched_entries: HashMap<String, ChangelogEntryResult> = HashMap::new();
    let transport = if request.offline {
        None
    } else {
        Some(HttpModrinthTransport::new()?)
    };
    for (index, candidate) in candidates.iter().enumerate() {
        if context.is_cancelled() {
            break;
        }
        let number = index as u32 + 1;
        emit_progress(
            &app,
            context.progress(
                index as u32,
                format!("Looking up changelog {number} of {total}"),
                true,
            ),
        );
        let request_key = modrinth_project_slug(candidate).map(|slug| {
            let hints = match &candidate.available_version {
                Evidence::Observed(version) => local_version_hints(version),
                _ => LocalVersionHints::default(),
            };
            format!(
                "{slug}|{:?}|{:?}|{:?}|{:?}",
                hints.loader,
                hints.game_version,
                candidate.current_version,
                candidate.available_version
            )
        });
        let is_new_provider_request = request_key
            .as_ref()
            .is_some_and(|key| requested_projects.insert(key.clone()));
        let result = if request.offline {
            cached_modrinth(&database, candidate).unwrap_or_else(|| unresolved(candidate, true))
        } else {
            let result = if let Some(cached) = fetched_entries.get(
                request_key
                    .as_ref()
                    .expect("Modrinth candidates have a request key"),
            ) {
                cached.clone()
            } else {
                let fetched = match cached_modrinth(&database, candidate) {
                    Some(cached) if cached.retrieval == ChangelogRetrievalStatus::Cached => cached,
                    _ => fetch_modrinth_with_context(
                        transport.as_ref().expect("online transport should exist"),
                        candidate,
                        &context,
                    ),
                };
                fetched_entries.insert(
                    request_key
                        .as_ref()
                        .expect("Modrinth candidates have a request key")
                        .clone(),
                    fetched.clone(),
                );
                fetched
            };
            if let (
                Some(_project),
                Some(version),
                Evidence::Observed(_url),
                Evidence::Observed(target),
            ) = (
                &result.match_evidence.project,
                &result.match_evidence.version,
                &candidate.source_url,
                &candidate.available_version,
            ) {
                let _ = db::cache_changelog_response_with_metadata(
                    &database,
                    &crate::domain::ChangelogCacheKey {
                        provider: "modrinth".into(),
                        endpoint: "project_versions".into(),
                        request_shape: format!(
                            "limit={};offset=0;pages={};complete={}",
                            crate::changelog_transport::MAX_PAGE_SIZE,
                            result.page_count,
                            result.pagination_complete
                        ),
                        project_id: modrinth_project_slug(candidate).unwrap_or_default().into(),
                        version_id: version.version_id.clone(),
                        game_version: local_version_hints(target).game_version,
                        loader: local_version_hints(target).loader,
                        requested_fields: vec!["changelog".into()],
                    },
                    result.raw_response.as_deref().unwrap_or("[]"),
                    &result.match_evidence,
                    result.page_count,
                    result.pagination_complete,
                );
            }
            result
        };
        entries.push(result);
        emit_progress(
            &app,
            context.progress(
                number,
                format!("Completed changelog lookup {number} of {total}"),
                true,
            ),
        );
        if !request.offline && is_new_provider_request && number < total {
            context.wait(LOOKUP_DELAY);
        }
    }
    let confirmed = entries
        .iter()
        .filter(|entry| {
            matches!(
                entry.retrieval,
                ChangelogRetrievalStatus::Retrieved
                    | ChangelogRetrievalStatus::Cached
                    | ChangelogRetrievalStatus::CachedStale
                    | ChangelogRetrievalStatus::Missing
            )
        })
        .count();
    let status = if context.is_cancelled() {
        ChangelogGenerationStatus::Cancelled
    } else if confirmed == entries.len() {
        ChangelogGenerationStatus::Complete
    } else if confirmed > 0 {
        ChangelogGenerationStatus::Partial
    } else {
        ChangelogGenerationStatus::Failed
    };
    let mut content = request
        .introduction
        .as_deref()
        .map(str::trim)
        .filter(|value: &&str| !value.is_empty())
        .unwrap_or_default()
        .to_string();
    if !content.is_empty() {
        content.push_str("\n\n");
    }
    for entry in &entries {
        let name: &str = match &entry.local.local_identity {
            Evidence::Observed(value) => value.as_str(),
            _ => "Unknown mod",
        };
        let versions = entry
            .versions
            .iter()
            .filter(|version| version.included && version.changelog.is_some())
            .collect::<Vec<_>>();
        if versions.is_empty() {
            if entry.versions.is_empty() {
                if let Some(changelog) = entry
                    .changelog
                    .as_deref()
                    .map(str::trim)
                    .filter(|value: &&str| !value.is_empty())
                {
                    content.push_str(&format!("## {name}\n\n{changelog}\n\n"));
                }
            }
            continue;
        }
        content.push_str(&format!("## {name}\n\n"));
        for version in versions {
            let label = changelog_version_label(version.version.version_number.as_deref());
            content.push_str(&format!(
                "### {label}\n\n{}\n\n",
                version.changelog.as_deref().unwrap_or_default()
            ));
        }
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
        content.push_str("## Externally observed changes\n\n");
        content.push_str("These changes were observed in the modpack files outside CM Modpack Util and are not labeled as app-applied updates.\n\n");
        for observation in observations {
            content.push_str(&format!("- {observation}\n"));
        }
        content.push('\n');
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
        context.progress(total, "Saving changelog artifact".into(), true),
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
pub fn create_changelog_selection_revision(
    database: State<'_, db::Database>,
    request: crate::domain::ChangelogSelectionRevisionRequest,
) -> Result<crate::domain::ChangelogRevision, CommandError> {
    db::persist_changelog_selection_revision(&database, &request)
}

#[tauri::command]
pub fn archive_changelog_revision(
    database: State<'_, db::Database>,
    request: crate::domain::ChangelogRevisionArchiveRequest,
) -> Result<crate::domain::ChangelogRevision, CommandError> {
    db::set_changelog_revision_archived(&database, &request)
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
