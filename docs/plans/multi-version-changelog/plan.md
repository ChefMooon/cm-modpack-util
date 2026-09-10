## Plan: Multi-Version Changelog Review

Extend the Rust-owned Modrinth generation pipeline to retain every compatible release between the installed and updated versions, expose those found changelogs in a selectable review section, and persist selection changes as immutable proposed revisions. Reuse the existing `changelog-progress` event path and typed Tauri wrappers, while hardening Modrinth pagination, caching, retry, rate-limit, and request-budget behavior. This plan also includes the release-owned candidate evidence and snapshot-independent artifact persistence required for detached/current-project workspaces.

**Steps**

### Phase 1: Contracts and behavior
1. Add release-owned candidate evidence and snapshot-independent artifact persistence for detached/current-project workspaces before extending workspace generation. Preserve snapshot provenance when present, allow a nullable artifact snapshot association when the release owns its evidence, and keep proposed versus final changelog ownership distinct.
2. Add a version-level changelog contract alongside `ChangelogEntryResult` that carries the Modrinth version identity, response position, publication/creation timestamps, normalized changelog text, retrieval status, empty-content classification, and inclusion state. Normalize provider content in Rust by trimming surrounding whitespace for classification and rendering; classify `None`, empty strings, and whitespace-only strings as an explicit empty/unavailable-content state without treating them as a provider failure. Keep ordering evidence in Rust contracts; do not make Svelte infer version ordering.
3. Define the relationship between installed version, updated version, and intermediate versions. The Rust service will identify unique current and target matches from the provider response, use documented Modrinth response order as the primary sequence, retain timestamps as consistency evidence, and include compatible versions strictly between the endpoints. If timestamps contradict response order, endpoints are ambiguous, pagination is incomplete, or the sequence cannot be proven, preserve the diagnostic and do not silently invent intermediate entries.
4. Extend revision/selection contracts so a user’s checked/unchecked version IDs can be persisted as an append-only proposed revision. The original generated artifact remains immutable; a selection action creates a new revision with validated ownership, selected provider version IDs, rendered Markdown, and the prior revision relationship. Keep manual text editing and selection-derived rendering distinct so later manual edits are not overwritten unexpectedly.
5. Mirror all new Rust wire types and requests in `src/lib/domain.ts`, add typed wrappers in `src/lib/modpacks.ts`, and define structured errors/diagnostics for ambiguous ordering, unavailable intermediate versions, empty changelog content, provider budget exhaustion, and rate-limit retry exhaustion. Empty content is a warning/evidence state, not a generation exception by itself.

### Phase 2: Modrinth retrieval and scalable generation
6. Refactor `src-tauri/src/changelog.rs` so a Modrinth result retains the complete compatible version list rather than selecting only the target. Reuse the existing exact-file, exact-version, loader/game-version, and normalized matching precedence; apply it separately to current and target endpoints; preserve evidence for every included version, including versions with no usable changelog text.
7. Update `src-tauri/src/changelog_transport.rs` to make version-list retrieval explicit about `limit`/`offset` pagination, response size limits, and request context. Page only as needed, cap total pages/versions per project, deduplicate repeated provider versions, and stop once the current/target boundaries and required intermediates are known when the provider ordering makes that safe. Avoid one request per changelog version: Modrinth’s version-list response already contains the changelog field, so pacing applies between project requests.
8. Add a generation context that owns per-attempt cancellation, project/query deduplication, pacing, retry/backoff, progress, and hard alpha budgets of 100 provider requests per generation, 10 pages per project, and 500 provider versions per project. Retain the current three-second inter-project delay, honor `Retry-After` for HTTP 429, add bounded retries with exponential backoff for transient 429/5xx/timeouts, and return visible partial/rate-limited diagnostics when limits are exhausted. Update the User-Agent/version and include request-shape metadata in cache keys.
9. Expand cache handling in `src-tauri/src/db/mod.rs` and `src-tauri/src/db/schema.sql` around normalized project/query version sets with page/completeness metadata, loader/game-version context, retrieval time, and association evidence. Use a 24-hour TTL for online generation; only explicit offline generation may use expired cache, and it must mark the evidence stale. Preserve the alpha schema-reset boundary because additive/incompatible local databases are allowed to require a documented reset.
10. Generate the initial artifact with all proven non-empty intermediate changelogs included by default, in provider release order, while retaining all found versions and their inclusion defaults in the artifact evidence. Never render empty or whitespace-only changelog text, headings, separators, or padding into Markdown. Retain empty-content versions as visible evidence with a warning and an excluded/non-selectable inclusion state; if every found changelog is empty, persist the artifact and a clear warning rather than failing or creating blank output. Keep unresolved, missing, ambiguous, partial, offline, rate-limited, and empty-content states explicit. Reuse `ChangelogProgress`, but emit progress for project/version-page work and final persistence with the generated attempt ID consistently; add cancellation checks between pages, requests, retries, and pacing delays.

### Phase 3: Selection review UI
11. Add a new found-changelog section/card in `src/components/modpacks/ProposedChangelogPanel.svelte`, positioned after the generated artifact/revision list and before the proposed revision editor. Group records by mod, show installed-to-updated boundaries and each intermediate provider version, changelog availability/status, release date/order evidence, and a checkbox for each selectable changelog. Render empty-content versions as retained evidence with a descriptive warning such as `No changelog text was published for this version; it will not be added to the generated Markdown.` Do not render an empty preview row as changelog content.
12. Add accessible `Add all` and `Remove all` controls using the existing `Button` primitive, plus per-mod select-all behavior if the grouped layout makes it useful. Define bulk actions over selectable non-empty versions only, so `Add all` never adds empty content and `Remove all` never creates blank output. Show the empty-content warning in the same add/remove management section, keep empty-content checkboxes disabled and excluded from selection counts, and explain the disabled state through visible text and accessible labeling. Keep selection controls disabled for frozen/archived revisions, unresolved or unproven provider versions, empty-content versions, active generation, and command failures; ensure keyboard labels, focus, semantic status, reduced motion, and color-independent meaning follow `docs/desktop-ui-standards.md`.
13. Wire selection changes through the route state in `src/routes/+page.svelte` and workspace route flow: derive selected non-empty version IDs from the loaded artifact/revision, call the typed selection-revision command, replace the current revision only after persistence succeeds, refresh selected draft/content, and leave the original generated artifact and revision history visible. Reject empty-content IDs server-side even if a stale or modified client submits them. Keep generation, revision creation, and workspace-pointer selection as separate Rust commands, but make each ownership-validated, idempotent, and recoverable. Handle concurrent generation, stale selection responses, empty results, all-removed, all-selected, partial generation, and manual-draft conflict states explicitly.
14. When the draft contains unsaved manual edits, require explicit confirmation before selection regenerates Markdown. Preserve the dirty text in revision history, then create a new selection-derived revision; never silently overwrite the draft.
15. Pass the new selection/progress data through `src/components/modpacks/ReleaseReviewModal.svelte` and the snapshot review surface as applicable. Keep workspace and snapshot command ownership explicit and do not add direct provider/database calls to components.
16. Add an in-panel generation progress indicator for workspace generation using `ChangelogProgress` rather than relying only on the button spinner: show determinate completed/total work where available, the current message, rate-limit/backoff state, and a cancellation action when the Rust command is cancellable. Preserve the existing snapshot progress behavior and normalize attempt-ID handling so progress cannot leak between attempts.

### Phase 4: Tests, documentation, and UX/API hardening
17. Add focused Rust tests for detached workspace evidence, current/target matching, multiple intermediate versions such as 1.4.4 -> 1.4.6 -> 1.4.8, hybrid provider ordering, incompatible loader/game-version exclusions, ambiguous boundaries, missing changelogs, `None`/empty/whitespace-only normalization, omission of empty Markdown sections, pagination/deduplication, cache TTL/offline reuse, fixed budgets, 429 Retry-After/backoff, cancellation during delay, and partial persistence.
18. Add persistence/contract tests for nullable snapshot artifact round trips, version-level artifact evidence, empty-content warnings and excluded states, selected IDs that cannot contain empty-content versions, immutable original artifacts, append-only selection revisions, separate-command recovery, ownership/frozen-revision rejection, and reload after restart. Do not add a frontend test runner in this phase; use `npm run check` and `npm run build` for typed validation plus the manual interaction workflow below.
19. Update `docs/packwiz-commands.md` or the changelog/provider documentation, `README.md`/`docs/ROADMAP.md` only where the supported boundary changes, and the relevant v0.1.0 plan/handoff with detached workspace, selection, caching, pagination, budget, TTL, and rate-limit behavior. Document the alpha database reset requirement if schema changes are incompatible.
20. Run focused validation first, then `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`, focused and full `cargo test --manifest-path src-tauri/Cargo.toml`, `npm run check`, `npm run build`, and `git diff --check`. Perform a desktop manual check for detached and snapshot-backed multi-version proposals, add/remove all, dirty-draft confirmation, reload persistence, generation progress/cancel, rate-limit messaging, recovery after separate-command failure, and finalization using a selected proposed revision.

**Relevant files**
- `src-tauri/src/changelog.rs` — generation orchestration, current/target matching, version-range selection, content rendering, progress, cancellation, and cache writes.
- `src-tauri/src/changelog_transport.rs` — Modrinth version-list query parameters, pagination, response bounds, Retry-After handling, retry/backoff, and request diagnostics.
- `src-tauri/src/domain/changelog.rs` and `src-tauri/src/domain/mod.rs` — version-level result, selection/revision request, progress, and diagnostic contracts.
- `src-tauri/src/db/mod.rs` — cache lookup/write, artifact loading, revision persistence, selection revision ownership/frozen checks, and reload round trips.
- `src-tauri/src/db/schema.sql` — release-owned candidate evidence, nullable/snapshot-independent artifact persistence, normalized version-set cache, and selection persistence; follow the alpha reset policy.
- `src/lib/domain.ts` and `src/lib/modpacks.ts` — TypeScript wire contracts and typed Tauri commands/listeners.
- `src/routes/+page.svelte` — detached/snapshot-backed workspace generation state, progress subscription, selection actions, revision refresh, recovery handling, and error handling.
- `src/components/modpacks/ProposedChangelogPanel.svelte` — new found-changelogs selection section, add/remove-all controls, progress display, and responsive/accessibility treatment.
- `src/components/modpacks/ReleaseReviewModal.svelte` and `src/components/modpacks/SnapshotReviewModal.svelte` — pass the new props and preserve distinct snapshot/workspace ownership while keeping the review experience consistent.
- `docs/desktop-ui-standards.md`, `docs/packwiz-commands.md`, `README.md`, `docs/ROADMAP.md`, and the relevant v0.1.0 plan — UI, provider, scope, and alpha database-reset documentation.

**Verification**
1. Rust unit/integration fixtures prove the full intermediate range is returned and ordered for a current version followed by multiple newer Modrinth releases, with no extra request per version; empty and whitespace-only provider changelogs remain in evidence but produce no Markdown content.
2. Mock transport assertions prove pagination is bounded, compatible versions are deduplicated, exact cache hits avoid network calls, the three-second inter-project delay is honored, Retry-After/backoff is bounded, and 429/timeout/budget exhaustion remains visible as partial or failed evidence.
3. Persistence tests prove all found version records and selection choices survive reload, empty-content records remain visible but excluded, selection revisions cannot contain empty-content IDs, selection revisions are append-only, frozen revisions cannot change, and the original generated artifact remains unchanged.
4. Frontend type/build checks plus the manual desktop workflow verify checkboxes, `Add all`, `Remove all`, empty-content warnings and disabled controls, no blank Markdown spacing, empty/partial/unavailable states, progress/cancel, stale/error handling, and the explicit dirty-draft confirmation behavior; no automated interaction runner is introduced in this phase.
5. Run `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`, `cargo test --manifest-path src-tauri/Cargo.toml`, `npm run check`, `npm run build`, and `git diff --check`; record any unavailable live Modrinth or native desktop checks as residual risk.
6. Manual desktop workflow: generate a multi-version proposal, verify every proven non-empty intermediate changelog is initially selected, verify empty-content versions show a warning and cannot be selected or added by bulk actions, verify no blank headings or padding appear in the Markdown, remove/re-add individual versions and all selectable versions, reload the workspace, save/edit a proposed revision, and finalize only the selected revision.

**Decisions**
- Default inclusion: all proven found changelogs are checked initially.
- Empty changelog handling: `None`, empty, and whitespace-only provider content is retained as release/version evidence with a visible warning, excluded from generated Markdown and selection counts, and rejected by selection persistence. Empty content alone does not fail generation; an artifact with no non-empty entries remains durable with an explicit warning.
- Selection persistence: checkbox choices are durable through append-only proposed revisions; generated artifacts remain immutable.
- Ordering: documented Modrinth response order is primary; publication/creation timestamps are retained as consistency evidence. Contradictions, ties, or incomplete pagination are ambiguous and are not silently selected. Semantic parsing is not used as the sole source of truth.
- Pacing: retain the existing three-second delay between project requests, add bounded Retry-After-aware retries/backoff, and do not sleep between versions already returned in one response.
- Scope: support Modrinth only, while supporting both snapshot-backed and detached/current-project release workspaces. No CurseForge changelog API, background lookups, Packwiz mutation, guessed provider identity, or unattended generation.
- Pagination and scaling: use normalized project/query version-set caching, bounded page retrieval, exact cache reuse, project-level deduplication, and explicit partial/rate-limit states. Hard alpha caps are 100 provider requests per generation, 10 pages per project, and 500 provider versions per project. A potentially large result should not be loaded or rendered without a configured cap.
- Cache freshness: online generation uses a 24-hour TTL and refreshes entries beyond it; explicit offline generation may use expired entries but marks them stale. Expired cache is not silently treated as fresh online evidence. The 24-hour window balances daily release-review freshness with the local-first workflow and existing provider pacing.
- Persistence sequencing: generation, revision creation, and workspace-pointer selection remain separate commands. Each validates ownership, is idempotent where possible, and records/reports recoverable intermediate states.
- Manual draft conflicts: changing selection with unsaved Markdown requires explicit confirmation, preserves the dirty text in history, and then creates a new selection-derived revision.
- Frontend verification boundary: no new frontend test runner in this phase; `npm run check`, `npm run build`, and the specified manual desktop workflow are the committed validation boundary.

**Further Considerations**
1. The main UX improvement beyond the requested section is to make the selected range explicit per mod, for example “1.4.4 -> 1.4.8, including 1.4.6,” with a compact “why this version is included” evidence disclosure. This prevents users from mistaking an intermediate changelog for a duplicate or unrelated release.
2. Show a small generation summary beside progress: projects requested, provider versions found, changelogs selected, cached hits, and unresolved entries. This gives the user useful scale/rate-limit feedback without exposing implementation details.
3. Keep manual Markdown editing and checkbox selection visibly separate. If the user edits the draft after selection, show that the draft contains manual changes and require an explicit confirmation before a later selection change regenerates the Markdown, preventing accidental loss.
4. API review: the current one-request-per-mod plus three-second delay is acceptable for small packs because changelogs arrive in the version-list response, but it does not yet scale predictably for large packs or long histories. Pagination, project deduplication, exact cache reuse, request budgets, Retry-After handling, and a shared client rate limiter are higher-value than simply increasing the fixed pause. The plan should measure request count and total generation time in fixtures before choosing a longer fixed delay.
5. Empty changelog UX: distinguish `no provider text` from `provider lookup unavailable`, `ambiguous match`, and `request failure`. Preserve a concise reason and the provider version identity so users can inspect the source release, but avoid offering a control that can only create empty output. Consider a trusted provider-page link as an optional inspection action when one already exists; do not construct a guessed URL.

## Handoff: Partial Implementation, 2026-09-10

### Current status

The implementation is partially complete and the worktree contains focused changelog changes in these files:

- `src-tauri/src/changelog.rs`
- `src-tauri/src/changelog_transport.rs`
- `src-tauri/src/db/mod.rs`
- `src-tauri/src/db/schema.sql`
- `src-tauri/src/domain/changelog.rs`
- `src-tauri/src/domain/mod.rs`
- `src-tauri/src/lib.rs`
- `src/lib/domain.ts`
- `src/lib/modpacks.ts`
- `src/routes/+page.svelte`
- `src/components/modpacks/ProposedChangelogPanel.svelte`
- `src/components/modpacks/ReleaseReviewModal.svelte`

Do not discard unrelated worktree changes. Inspect the current files before editing because formatter or user changes may have modified the implementation since this handoff was written.

### Implemented

- Added `ChangelogVersionResult` and `ChangelogContentStatus` Rust/TypeScript wire contracts.
- Retained compatible provider versions between the current and target matches using provider response order, including response position and publication/creation timestamps.
- Normalized changelog text with trimming and retained empty-content evidence without rendering it.
- Added bounded Modrinth page requests using `limit` and `offset`, with a ten-page/project and 500-version/project cap.
- Added bounded HTTP retry behavior for transient 429/5xx responses, including numeric `Retry-After` handling and exponential backoff.
- Added a 100-provider-request generation cap with visible per-entry budget diagnostics.
- Made artifact and attempt snapshot associations nullable for detached workspaces.
- Allowed release workspace generation from workspace-owned candidates when no snapshot is linked.
- Added `selected_version_ids_json` to changelog revisions and persisted selection IDs across reloads.
- Added `create_changelog_selection_revision`, including artifact/workspace ownership validation and rejection of unknown, unavailable, or empty-content IDs.
- Added the found-changelog review section with individual selection and `Add all` / `Remove all` controls.
- Added dirty-draft confirmation before selection-derived Markdown replaces the draft.
- Corrected generation progress events to use the generation attempt ID consistently.
- Changed all-empty provider results to remain durable evidence rather than automatically becoming generation failures.

### Remaining implementation work

Complete these in order, validating each slice before proceeding:

1. **Generation context and request ownership**
	- Replace the current per-candidate loop state with a context that owns attempt ID, cancellation, request count, deduplicated project/query requests, inter-project pacing, retry/backoff state, and progress.
	- Deduplicate candidates with the same Modrinth project and equivalent loader/game-version query. Reuse one fetched version set to produce local entry-specific associations.
	- Check cancellation between page requests, retries, backoff sleeps, and the three-second project delay. Mark progress as cancellable while generation is active.
	- Keep the hard caps at 100 provider requests per generation, 10 pages per project, and 500 provider versions per project. Emit partial/rate-limited diagnostics rather than silently dropping work.

2. **Ordering and completeness diagnostics**
	- Distinguish a complete final page from a page-cap stop. Do not select intermediates by default when the current/target range cannot be proven complete.
	- Detect contradictory publication/creation timestamps and preserve an ambiguity diagnostic while retaining raw evidence.
	- Deduplicate versions by provider ID before range selection, not only after collecting evidence.
	- Add explicit unavailable/intermediate-boundary diagnostics to the Rust and TypeScript contracts where needed.

3. **Cache semantics**
	- Extend `changelog_cache` with normalized project/query version-set metadata, page count, completeness, loader/game-version context, and retrieval time as required by the plan.
	- Include pagination/request shape in cache keys and cache the complete version response rather than only the selected target version.
	- Implement the 24-hour online TTL. Online generation must refresh expired records; explicit offline generation may use them only with `CachedStale` evidence.
	- Ensure exact cache hits avoid provider requests and add tests for fresh, expired-online, and expired-offline behavior.
	- The schema change is intentionally alpha-reset compatible. Existing local databases with the old changelog schema may need to be stopped and reset; do not silently migrate or delete user data.

4. **Revision and workspace correctness**
	- Verify selection revisions against the active workspace lifecycle, frozen/archived state, and modpack ownership in the same command boundary.
	- Preserve manual draft revisions in history before a confirmed selection replacement, and make selection requests idempotent where possible.
	- Ensure snapshot-backed and detached workspaces both load artifacts, revisions, and selected IDs after restart.

5. **Frontend progress and review surfaces**
	- Pass selection/progress state through `SnapshotReviewModal.svelte` where snapshot changelog generation is exposed.
	- Add an in-panel workspace generation progress display showing completed/total work, current message, retry/rate-limit state, and cancellation.
	- Disable selection controls for frozen/archived revisions, active generation, command failure, unresolved/unproven entries, and stale selection responses.
	- Show installed-to-updated boundaries and response-order/date evidence per mod. Keep empty-content rows visible as disabled evidence and excluded from counts.

6. **Tests and documentation**
	- Add Rust fixture tests for 1.4.4 -> 1.4.6 -> 1.4.8 ordering, hybrid ordering, duplicate IDs, incompatible loader/game versions, ambiguous endpoints, missing/empty/whitespace changelogs, pagination caps, request deduplication, cache TTL/offline reuse, retry/rate limits, cancellation, budget exhaustion, and partial persistence.
	- Add database round-trip tests for nullable snapshot IDs, version evidence, selected IDs, immutable artifacts, append-only revisions, ownership rejection, frozen revisions, and reload after restart.
	- Update provider/cache behavior documentation and the relevant v0.1.0 handoff or roadmap entry. Preserve the existing README alpha database-reset guidance.
	- Perform the manual desktop workflow in the plan for both snapshot-backed and detached workspaces, including reload, dirty draft confirmation, add/remove all, cancellation, rate-limit messaging, recovery, and finalization.

### Known risks for the next agent

- The current `ModrinthTransport` trait returns raw page JSON only; retry metadata is implemented in the HTTP transport, so fixture transports need to model page sequences and failures explicitly before transport behavior can be fully tested.
- The current cache lookup does not expose retrieval age, so it cannot yet distinguish fresh cached evidence from stale offline evidence.
- The current selection UI derives artifact defaults when a revision has an empty selected-ID list. This is acceptable for generated revisions only if empty selection revisions are represented distinctly; verify the all-removed case so it does not fall back to defaults.
- The current frontend progress listener primarily gates snapshot progress. Workspace generation needs an attempt-scoped state path before progress can be considered complete.
- No live Modrinth or native desktop manual check has been completed in this handoff.

### Validation evidence

The following checks passed after the current implementation slice:

```text
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml
npm run check
npm run build
git diff --check
```

The existing Rust provider tests include coverage for exact matching, file-name precedence, normalized matching, ambiguity, non-Modrinth boundaries, and intermediate version retention with whitespace-only content. They do not replace the broader Phase 4 test matrix above.

### Suggested starting point

Start by reading `generate_changelog`, `fetch_modrinth`, `HttpModrinthTransport::get_versions`, `cached_changelog_response_for_local_version`, and the revision load/persist functions in `src-tauri/src/db/mod.rs`. Then add a fixture transport that records `(project, loader, game_version, limit, offset)` calls. That fixture will provide the cheapest discriminating check for request deduplication, bounded pagination, cache reuse, cancellation, and rate-limit behavior before changing the UI again.

## Handoff: Remaining Work, 2026-09-10

### Current state

The previous partial implementation has been extended and validated. Do not discard unrelated worktree changes. Inspect the current files before editing because the repository may contain additional user or formatter changes.

The latest validated changes are concentrated in:

- `src-tauri/src/changelog.rs`
- `src-tauri/src/db/mod.rs`
- `src-tauri/src/db/schema.sql`
- `src/components/modpacks/ProposedChangelogPanel.svelte`
- `src/components/modpacks/ReleaseReviewModal.svelte`
- `src/components/modpacks/SnapshotReviewModal.svelte`
- `src/routes/+page.svelte`

### Completed in this slice

- Added generation-local reuse of fetched changelog results for equivalent project/query and local endpoint requests.
- Corrected provider budget accounting so reused generation results do not consume the 100-request budget.
- Added cache metadata columns for normalized query context, page count, completeness, loader, and game version.
- Added cache-age evaluation using the 24-hour TTL.
- Fresh cache evidence is reused during online generation; stale cache evidence is marked `CachedStale` for offline generation and causes online generation to refresh.
- Added release-workspace ownership and terminal-lifecycle validation before selection revisions are created.
- Added workspace changelog progress with completed/total work, current status text, and cancellation.
- Added snapshot changelog progress and cancellation feedback to the snapshot review modal.
- Preserved the existing empty-content exclusion, add/remove-all controls, dirty-draft confirmation, append-only revision behavior, and nullable snapshot ownership changes.

### Validation evidence

All of the following passed after the latest edits:

```text
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml
npm run check
npm run build
git diff --check
```

The focused changelog suite also passed with 14 tests and no warnings.

### Remaining implementation work

Complete these in order, adding focused tests before widening the implementation:

1. **Make generation context authoritative**
	- Replace the remaining loop-local coordination with an explicit attempt context owning cancellation, provider request count, deduplicated request state, pacing, retry state, and progress.
	- Pass cancellation into pagination and transport retry/backoff handling. Check it between page requests, retries, backoff sleeps, and the three-second inter-project delay.
	- Ensure progress events use the actual attempt ID from the beginning of generation. The workspace UI currently uses an empty attempt ID while preparing, so prevent cross-attempt event leakage when multiple surfaces are active.
	- Preserve visible partial/rate-limited diagnostics when a request, page, or generation budget is exhausted.

2. **Finish ordering and completeness correctness**
	- Track whether pagination ended on a complete final page or stopped at the ten-page/500-version cap.
	- Do not default-select intermediate versions when the current/target range is not proven complete.
	- Deduplicate provider versions by ID before matching and range selection, retaining the first response position and raw ordering evidence.
	- Detect contradictory publication/creation timestamps, ties, unresolved boundaries, incompatible loader/game-version records, and ambiguous endpoint matches. Preserve raw evidence while returning explicit diagnostics.
	- Add wire-contract fields only where the current `ChangelogVersionResult` and entry diagnostic fields cannot express these states clearly.

3. **Complete cache semantics and tests**
	- Cache the complete fetched version set, not only the selected target projection.
	- Include the exact pagination/request shape in the cache key and persist page count, completeness, normalized loader/game-version query, and retrieval time consistently.
	- Verify fresh online hits avoid provider calls, expired online entries refresh, and explicit offline generation can reuse expired entries only as `CachedStale`.
	- Add database tests using controlled timestamps or an injectable clock; do not rely on sleeping for 24 hours.
	- Remember that the schema change follows the alpha reset policy. Do not silently migrate or delete existing local databases.

4. **Harden revision/workspace persistence**
	- Add round-trip tests for snapshot-backed and detached artifacts, version evidence, selected IDs, immutable artifacts, append-only revisions, reload after restart, and all-removed selections.
	- Verify an empty `selected_version_ids` list remains an intentional empty selection rather than falling back to generated defaults.
	- Make selection revision requests idempotent where practical and reject stale prior revisions, archived revisions, frozen revisions, and workspace lifecycle changes at one command boundary.
	- Preserve manual draft revisions before confirmed selection replacement.

5. **Complete frontend review behavior**
	- Confirm `SnapshotReviewModal.svelte` exposes the same found-version selection workflow as the workspace review when snapshot changelog generation is directly available; the current snapshot surface has progress/cancel feedback but still uses its older artifact editor presentation.
	- Disable selection controls for frozen/archived revisions, active generation, failed commands, unresolved or unproven records, and stale responses.
	- Show installed-to-updated boundaries, response position/date evidence, empty-content rows, partial state, retry/rate-limit state, and recovery messaging.
	- Test both snapshot-backed and detached workspace flows manually, including reload, cancellation, dirty-draft confirmation, add/remove all, separate-command failure recovery, and finalization.

6. **Expand tests and documentation**
	- Add fixture transport coverage for pagination, request deduplication, duplicate IDs, incompatible records, ambiguous endpoints, cancellation, retry/429 behavior, rate-limit exhaustion, provider budgets, partial persistence, and cache TTL/offline reuse.
	- Add the remaining database round-trip and ownership/frozen-state tests described in Phase 4.
	- Update `docs/packwiz-commands.md` or a provider/cache document with the actual pagination, cache, retry, budget, stale-evidence, and reset behavior. Update the relevant v0.1.0 handoff or roadmap entry without duplicating README reset guidance.

### Known implementation risks

- `ModrinthTransport` still returns raw page JSON, so cancellation-aware retry metadata and deterministic fixture failures need a transport/context design before the full retry/cancellation matrix can be tested.
- Cache metadata currently records only the information available through the existing cache API; it is not yet a complete version-set cache contract.
- Generation-local reuse is conservative because the key includes local endpoints. A broader project/query cache must re-associate each candidate from the retained raw version set rather than cloning an entry result.
- The snapshot review surface has progress feedback but does not yet share the workspace found-changelog selection component.
- No live Modrinth request or native desktop manual workflow has been completed in this handoff.

### Suggested starting point

Start by extracting a testable `GenerationContext` around `generate_changelog` and a fixture transport that records `(project, loader, game_version, limit, offset)` calls plus scripted responses/errors. First prove cancellation, request counts, page-cap classification, and exact cache reuse in Rust tests. Then change the cache to retain complete version sets and refactor ordering diagnostics around that retained evidence before making further UI changes.
