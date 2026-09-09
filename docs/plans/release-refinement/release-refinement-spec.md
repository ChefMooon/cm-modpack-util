# Specification Change Impact Assessment: Release Workspace Refinement

## Decision Summary
- Overall disposition: hand off to the planning agent
- Confidence: medium
- Scope assessed: the relationship between update snapshots, release workspaces, applied updates, changelog generation, and final captured release state.

The earlier promotion-only model is superseded by the user's clarified workflow. The recommended architecture is:

1. An update check creates a snapshot containing proposed candidates and before-state evidence.
2. The user explicitly creates a draft release workspace from that snapshot.
3. The release workspace owns candidate selection, update application attempts, recovery, changelog drafts/finals, and release metadata.
4. The release starts from the snapshot before-state and ends with a fresh validated post-apply capture.
5. The snapshot remains visible as discovery history, while only one active release workspace may use it.
6. The user explicitly finalizes the release after reviewing the validated result and changelog. Publication status remains separate from finalization.
7. While a draft or working release workspace is open, the Rust backend watches Packwiz-relevant project files and automatically refreshes observed evidence when terminal edits occur. The watcher stops when the workspace closes, is abandoned, becomes ready, or is published.
8. The release workspace presents a small user-facing phase model: review, apply, resolve, verify, finalize, and publish. Backend state may be more granular, but the frontend always shows the current phase, blocking reason, evidence freshness, and primary next action.
9. Workspace lifecycle, evidence status, and publication status are separate concerns. A release may be verified and finalized while remaining unpublished, and a partially applied release remains recoverable without being treated as published.
10. Closing the application or leaving the workspace does not abandon it. Abandonment is explicit, preserves history, and reopens the source snapshot for a new workspace without reverting project files.
11. External edits are shown as an actionable workspace interruption. Automatic refresh preserves release decisions and provenance, but the workspace remains blocked until the user reviews and resolves the observed change.

This is a larger alpha redesign than the previous assessment. It is worthwhile because it makes the release the place where a public version is assembled, rather than a record created after an unrelated update workflow has already ended.

The intended user journey is:

1. **Review**: inspect the immutable snapshot baseline and choose candidate updates.
2. **Apply**: confirm the selected, skipped, deferred, and blocked candidates, then apply selected changes only when the live fingerprint matches the snapshot baseline.
3. **Resolve**: handle failed operations, partial success, recovery requirements, or external edits at candidate level.
4. **Verify**: inspect a fresh stable post-operation capture and the observed before/after result.
5. **Finalize**: review the release receipt and final changelog, then explicitly freeze the release evidence.
6. **Publish**: optionally mark the finalized release as published; publication is separate from finalization and makes the release read-only.

The UI should use these phases as the primary mental model. The backend may retain detailed states such as `applying`, `recovery_required`, and `ready_to_finalize`, but those states should map to a clear phase, blocking message, and next action rather than appearing as an unexplained state list.

## Proposed Changes
### C1: Reposition snapshots as update proposals and discovery history
- Intended outcome: an update check answers which mods may need changes and preserves the starting evidence for later release work.
- In scope: snapshot candidates, before-state fingerprint/capture, discovery lifecycle, candidate evidence, and navigation into a release workspace.
- Out of scope: making a snapshot a public release, hiding snapshots after release creation, or making snapshot decisions permanently final.
- Dependencies: existing discovery result, snapshot, candidate, recheck, and operation contracts.
- Open assumptions: the current discovery result can be extended to retain a canonical before-state capture; current operation records can remain associated with the release workspace rather than being silently duplicated.

### C2: Create a draft release workspace from a snapshot
- Intended outcome: the user can start a named release while reviewing update candidates, before any selected update is applied.
- In scope: explicit snapshot-to-release action, one active release workspace per snapshot, release draft metadata, candidate selection state, and clear workspace status.
- Out of scope: automatic release creation, automatic version numbering, external publication, or requiring Git.
- Dependencies: C1 before-state evidence, snapshot ownership, release lifecycle contracts, and the existing release tables.
- Open assumptions: creating the workspace does not consume or hide the snapshot; it establishes an explicit association and an active-workspace guard. Closing the application or leaving the workspace preserves the active draft; only an explicit abandon action ends it.

### C3: Move update decisions and operations into the release workspace
- Intended outcome: the release records the exact selected, skipped, deferred, blocked, applied, failed, and retried changes that make up the proposed public version.
- In scope: user selection inside the release workspace, apply/recovery operations, partial failure handling, post-operation validation, and release-scoped activity.
- Out of scope: changing Packwiz safety rules, adding unattended updates, or allowing the frontend to execute Packwiz directly.
- Dependencies: existing operation safety boundary, snapshot before-state, registered-path authorization, and release lifecycle.
- Open assumptions: snapshot candidates are proposals copied or referenced into the release workspace, while the snapshot's original discovery evidence remains immutable. The apply confirmation must summarize selected, skipped, deferred, blocked, pinned, and uncertain candidates before execution.

### C4: Add draft and final changelog work to the release
- Intended outcome: users can prepare a changelog while assembling the release, preview proposed changes, and produce a final changelog from the validated result.
- In scope: draft changelog from selected candidates, final changelog after validated application, editable release-owned revisions, and links to source snapshot evidence.
- Out of scope: deleting existing snapshot changelog artifacts, making network access mandatory, or treating a draft changelog as proof that updates were applied.
- Dependencies: existing changelog provider evidence, artifact/revision/export contracts, selected candidate decisions, and final release capture.
- Open assumptions: preview and final artifacts must be visibly distinguished; the final artifact should be generated from observed changes, not only proposed candidates. A proposed changelog must be labeled as proposal content, while a final changelog must identify the validated release capture it represents.

### C5: Finalize a release from a validated result
- Intended outcome: a release becomes ready for publication only after explicit user review of the applied state and changelog.
- In scope: fresh post-apply capture, release finalization transition, no-op rejection for finalized releases, provisional/recovery states, and immutable final evidence.
- Out of scope: requiring a Git commit/tag, automatic publication, or using current files as a substitute for the captured result after finalization.
- Dependencies: C3 operation verification, C4 final changelog, capture stability checks, and guarded release status transitions.
- Open assumptions: a release with failed or partial application remains recoverable as draft/provisional; it is not discarded and cannot be finalized until evidence is valid. Finalization presents a release receipt containing the baseline, selected/applied/skipped changes, fingerprint status, external-change status, final changelog revision, and no-op result before requiring confirmation.

### C6: Observe terminal-driven project edits during release work
- Intended outcome: users may run Packwiz commands in a separate terminal while the release workspace is open, and the application reflects those changes without requiring a manual refresh.
- In scope: a release-scoped Rust filesystem watcher, debounced events for Packwiz-relevant files and directories, automatic re-read of validated evidence, external-change status, and watcher teardown when the workspace closes or is abandoned.
- Out of scope: watching the entire project indiscriminately, watching after the release workspace closes or is published, treating external edits as app-owned operations, or silently rewriting release decisions and snapshot provenance.
- Dependencies: registered-root containment, the existing fingerprint and validation readers, release workspace lifecycle, operation coordinator, and Tauri event/command transport.
- Open assumptions: automatic refresh preserves selected/skipped/deferred decisions, reports the observed state change, and marks the workspace as externally changed until the user resolves it. The workspace displays changed scope and resulting evidence differences, not only an activity log entry. An external edit overlapping an app Packwiz operation creates a recovery-required state rather than being merged silently.

## Clarifications
- Asked and answered:
  - A release workspace is created at the start of review.
  - The release owns candidate decisions, application attempts, and release work while the snapshot remains discovery history.
  - Changelog work supports both a proposed/draft view and a final post-application view.
  - The snapshot remains visible, but only one active release may use it.
  - Failed or partial application leaves a recoverable draft/provisional release.
  - Finalization is explicit after a validated result; successful application does not automatically publish.
  - The release starts from the snapshot before-state.
  - The user selects candidates in the release workspace.
  - A no-op release cannot be finalized.
  - Git is not required for finalization.
  - Abandoning a draft preserves its history and reopens the source snapshot for a new release workspace.
  - Applying updates requires the live project fingerprint to match the snapshot before-state; otherwise the release is blocked and a new snapshot is required.
  - New update checks may create independent snapshots while another release workspace is active; they do not replace that workspace's source snapshot.
  - The release workspace watches Packwiz-relevant files only while open; it stops immediately when closed or abandoned.
  - A published release is historical and read-only; opening it never starts a project watcher, and any watcher stops before publication completes.
  - External edits automatically refresh observed evidence while preserving release decisions and snapshot provenance.
  - An external edit during an app-owned Packwiz operation is recorded and requires recovery rather than being merged or ignored.
- Still needed before implementation planning:
  - Exact release workspace state machine, including draft, applying, recovery-required, provisional, ready-to-finalize, finalized, published, withdrawn, and abandoned states.
  - Whether creating a release copies snapshot candidates into release-owned rows or references immutable snapshot candidates with release decisions layered on top.
  - Whether the implementation stores release candidate rows separately or layers release decisions over immutable snapshot candidate rows; the behavior is decided, but the normalization is still an implementation choice.
- Superseded decision:
  - The prior assessment recommended promoting a snapshot after-state directly into a release. That no longer matches the desired workflow because the release now owns the application and changelog assembly. Snapshot after-state promotion should not be the primary creation path.
- Assumptions used for this assessment:
  - Packwiz files remain authoritative for current external state.
  - Rust remains the authority for filesystem access, validation, operation safety, persistence, and state capture.
  - Alpha database reset is acceptable for breaking schema changes.
  - A native Rust watcher or equivalent OS-backed watcher is acceptable; the exact crate and event transport remain planning decisions.

## User Experience Requirements
- Snapshots, active release workspaces, finalized releases, and published releases must use distinct labels, actions, and visual status treatments. They may appear in one history view, but they must not look interchangeable.
- The primary snapshot action is `Start release from this snapshot`. If an active workspace already uses the snapshot, the UI opens or resumes that workspace rather than creating a duplicate or silently replacing it.
- Closing the application, navigating away, or closing a workspace panel does not abandon the release. Abandonment is an explicit action that preserves history and states that project files will not be reverted.
- Every active workspace shows its current user-facing phase, blocking reason, evidence freshness, and primary next action. Detailed backend states remain available in diagnostics or activity history.
- Before applying updates, the workspace presents a confirmation summary of selected, skipped, deferred, blocked, pinned, and uncertain candidates, along with the snapshot baseline match status.
- Recovery is visible at candidate level. The workspace distinguishes applied, failed-before-change, changed-but-unverified, skipped, blocked, and retryable candidates, and offers the smallest appropriate next action.
- External edits appear as a persistent actionable status, not only as an activity entry. The UI shows the changed scope and observed evidence differences while preserving release decisions and snapshot provenance.
- Proposed and final changelogs are visibly distinct. Proposed content is derived from selected candidates and is never presented as evidence that changes were applied. Final content identifies the validated release capture it represents.
- Finalization presents a release receipt containing the snapshot baseline, selected/applied/skipped changes, fingerprint and validation status, external-change status, final changelog revision, and no-op result before requiring confirmation.
- Published releases are historical and read-only. Opening one never starts a project watcher, and any watcher stops before publication completes.

## Current-State Evidence
- The product specification currently describes snapshots as update sessions and says a completed snapshot may become the basis for a release. The clarified workflow expands this into a release workspace that begins before application. See [CM-MODPACK-UTIL-SPEC.md](../../CM-MODPACK-UTIL-SPEC.md#update-snapshots).
- `persist_discovery_result` currently creates snapshots from discovery results and stores candidates plus `result_json`; it does not yet store a complete canonical before-state capture suitable for a release workspace baseline. See [db/mod.rs](../../../src-tauri/src/db/mod.rs) and [schema.sql](../../../src-tauri/src/db/schema.sql).
- Snapshot contracts currently model lifecycle, candidates, decisions, notes, rechecks, and operation relationships. They do not yet model release-owned candidate decisions or a release workspace state machine. See [contracts.rs](../../../src-tauri/src/domain/contracts.rs).
- Existing operation contracts and persistence already retain before/after fingerprints, verification, process evidence, outcomes, and recovery observations. These are useful foundations for moving operation ownership under a release workspace. See [contracts.rs](../../../src-tauri/src/domain/contracts.rs) and [db/mod.rs](../../../src-tauri/src/db/mod.rs).
- The current release model captures validated state at explicit release creation and treats captured release state as immutable. Its `snapshot_id` is an optional association, not a workspace origin or operation owner. See [release.rs](../../../src-tauri/src/domain/release.rs) and [capture.rs](../../../src-tauri/src/domain/capture.rs).
- The current release creation UI allows an optional snapshot link, while release comparison is release-to-release. It does not expose a release workspace for candidate selection, application, recovery, or draft/final changelog work. See [ReleaseReviewModal.svelte](../../../src/components/modpacks/ReleaseReviewModal.svelte).
- The current changelog workflow is explicitly snapshot-oriented and supports durable artifacts, revisions, and exports. The release refinement should add release context without destroying that source history. See [CM-MODPACK-UTIL-SPEC.md](../../CM-MODPACK-UTIL-SPEC.md#changelog-generation).
- The Versions view already presents snapshots and releases together and is a likely entry point for starting a release workspace from a snapshot. See [SnapshotHistory.svelte](../../../src/components/modpacks/SnapshotHistory.svelte).
- The current schema already has a unique `release_snapshots.snapshot_id` relationship, but that constraint alone is insufficient: it does not distinguish an active draft workspace from a finalized release, nor does it prevent stale or abandoned workspace ambiguity.
- The repository has no current filesystem watcher or project-file event contract. Evidence refresh is currently explicit through `refresh_modpack`, and the release surface is a route-owned modal. See [db/mod.rs](../../../src-tauri/src/db/mod.rs), [modpacks.ts](../../../src/lib/modpacks.ts), and [ReleaseReviewModal.svelte](../../../src/components/modpacks/ReleaseReviewModal.svelte).
- The existing fingerprint collector already defines the relevant Packwiz evidence boundary: `pack.toml`, the referenced index, indexed metadata, and relevant content roots. This is a better watch scope than the entire registered directory. See [fingerprint.rs](../../../src-tauri/src/discovery/fingerprint.rs).
- The repository has no current filesystem watcher or project-file event contract. Evidence refresh is currently explicit through `refresh_modpack`, and the release surface is a route-owned modal. A watcher must therefore be attached only to an editable release workspace, never to a published release detail view.

## Impact Findings
### C1: Reposition snapshots as update proposals and discovery history
- Classification: beneficial with conditions
- Positive impact: matches the user's mental model of an update check as a proposal list and preserves snapshots as useful records even when no release is made.
- Negative impact or unintended consequence: snapshots lose their role as the sole owner of update decisions and must retain enough immutable evidence to anchor a later release without becoming mutable release workspaces themselves.
- Affected surfaces: snapshot contracts, discovery persistence, before-state capture, rechecks, snapshot UI, and fixtures.
- Dependencies and interactions: C2 and C3 depend on a stable snapshot baseline; C4 can use snapshot changelog artifacts as proposal inputs.
- Confidence and rationale: medium. Existing fingerprints and operation evidence provide building blocks, but canonical before-state coverage must be verified.
- Discriminating check: inspect discovery and operation fixtures to prove that a snapshot can retain the full before-state required to compare with a final release capture.
- Recommendation: revise

### C2: Create a draft release workspace from a snapshot
- Classification: beneficial with conditions
- Positive impact: gives the user a natural place to assemble a public version and prevents release creation from being a detached metadata form.
- Negative impact or unintended consequence: workspace ownership, abandonment, and duplicate active releases need explicit rules; otherwise candidates and operations can become ambiguous.
- Affected surfaces: release domain contracts, schema, commands, typed wrappers, Versions UI, release workspace components, and persistence tests.
- Dependencies and interactions: requires C1 and must precede moving operation ownership.
- Confidence and rationale: high for product fit, medium for schema shape.
- Discriminating check: create a draft from a reviewable snapshot, verify the snapshot remains visible, verify a second active workspace is rejected, and verify an abandoned draft has a defined recovery path.
- Recommendation: proceed with conditions

### C3: Move update decisions and operations into the release workspace
- Classification: beneficial with conditions
- Positive impact: makes the release record explain exactly what was selected, changed, skipped, retried, or blocked, which is essential for trustworthy release notes and recovery.
- Negative impact or unintended consequence: operation records currently point to snapshots, so ownership must be moved, generalized, or related to both without duplicating history. A failed operation must not strand the project or silently reset decisions.
- Affected surfaces: operation request/record contracts, persistence foreign keys, mutation commands, snapshot review UI, release workspace UI, and safety tests.
- Dependencies and interactions: must preserve the current Rust-owned safety boundary and before/after verification; C5 depends on its final capture.
- Confidence and rationale: medium. The current operation model is strong, but changing its ownership is a cross-cutting alpha migration.
- Discriminating check: run a selected-update fixture through draft creation, partial failure, retry, and success; verify every attempt and decision remains queryable from the release workspace and no unsafe operation is enabled.
- Recommendation: proceed with conditions

### C4: Add draft and final changelog work to the release
- Classification: beneficial with conditions
- Positive impact: places release context, selected changes, and final observed changes together while retaining the original snapshot artifact history.
- Negative impact or unintended consequence: proposed candidates can differ from applied changes, so a draft changelog must never be presented as final. Provider failures and offline states remain possible during either stage.
- Affected surfaces: changelog requests and associations, artifact ownership, release workspace UI, export/revision navigation, and provider/offline tests.
- Dependencies and interactions: C3 supplies selection/application state; C5 supplies final observed state.
- Confidence and rationale: high for the two-stage user need, medium for artifact relationship details.
- Discriminating check: generate a draft from selected candidates, apply only a subset, generate final content, and verify the final artifact reflects observed changes while the draft remains preserved.
- Recommendation: proceed with conditions

### C5: Finalize a release from a validated result
- Classification: beneficial with conditions
- Positive impact: preserves explicit human review, keeps publication status distinct, and makes partial or failed application recoverable rather than falsely publishable.
- Negative impact or unintended consequence: introduces more lifecycle states and requires clear handling for no-op, stale, disconnected, and provisional releases.
- Affected surfaces: release lifecycle contracts, capture pipeline, persistence guards, finalization command, release UI, and integration tests.
- Dependencies and interactions: depends on C3 and C4; release-to-release comparison should use only finalized immutable captures, while draft workspaces may show preview deltas.
- Confidence and rationale: high for safety and product clarity.
- Discriminating check: attempt finalization after a no-op, partial failure, stale fingerprint, and successful validated apply; only the last case should transition to ready/finalized.
- Recommendation: proceed with conditions

### C6: Observe terminal-driven project edits during release work
- Classification: beneficial with conditions
- Positive impact: supports the real workflow of using a terminal for Packwiz commands while keeping the release workspace visibly current. It reduces stale UI and lets users see externally added, removed, or changed mods before finalization.
- Negative impact or unintended consequence: filesystem events are noisy, duplicated, and sometimes arrive before writes are complete. Automatic refresh can race with app-owned operations, overwrite neither decisions nor provenance, and must not imply that an external edit has been validated or approved.
- Affected surfaces: Rust watcher/service lifecycle, Tauri event contracts, fingerprint and validation reads, release workspace state, operation coordination, Svelte event handling, and watcher fixtures/tests.
- Dependencies and interactions: C3 and C5 must define how external changes block application/finalization; C6 must reuse the registered-root safety boundary and the existing relevant-file fingerprint scope.
- Confidence and rationale: high for feasibility, medium for the event/lifecycle design because no watcher exists today. The release-open scope is a useful containment boundary, but the watcher must be backend-owned rather than a frontend filesystem workaround.
- Discriminating check: open a release workspace, edit `pack.toml` or indexed metadata from a separate process, wait through event debounce, and verify the release receives refreshed evidence while selected decisions and snapshot provenance remain unchanged. Repeat during an app-owned operation and verify recovery is required.
- Lifecycle check: verify watching starts only for a draft or working release, stops when the workspace becomes ready or published, and does not restart when a published release is opened for historical review.
- Recommendation: proceed with conditions

## Cross-Change Considerations
- The architecture should distinguish three histories: snapshot discovery history, release workspace activity, and finalized release evidence. Do not collapse them into one mutable record.
- A snapshot should remain immutable after creation. Release decisions may begin from snapshot candidates, but changing a release selection must not rewrite the original discovery result.
- The likely data model needs a release workspace or release-work-item layer between `releases` and `release_snapshots`, plus release-scoped candidate decisions and operation associations. Exact normalization should be decided during planning after the existing operation contracts are audited.
- Before-state capture belongs to the snapshot. Final after-state capture belongs to the release after a verified operation. A release comparison must compare the finalized release capture with another finalized release capture, not a live working tree.
- Snapshot candidate comparison and release final-state comparison answer different questions. They should have separate contracts and UI labels.
- Changelog drafts should be derived from proposed selections, while final changelogs should be derived from validated observed changes. Both may reference the same snapshot but must remain distinguishable.
- One active release workspace per snapshot is a useful alpha constraint. Abandoned drafts need an explicit close/abandon operation so the snapshot can become eligible for another workspace without deleting history.
- Applying from a release workspace must verify the live project's fingerprint against the snapshot before-state. A mismatch blocks the operation and directs the user to create a new snapshot; it must not silently refresh release decisions.
- New snapshots remain independent while a release is active. They may later seed another release workspace, but cannot replace the active release's source snapshot.
- A release-scoped watcher should observe `pack.toml`, the referenced index, indexed `*.pw.toml` metadata, and relevant Packwiz content roots already covered by the fingerprint boundary. It should not watch the entire directory or continue after the workspace closes.
- Automatic refresh must be treated as observation, not authorization. It should preserve release decisions and source snapshot provenance, expose an external-change marker, and require explicit recovery when edits overlap an app-owned Packwiz operation.
- Watcher events need debouncing/coalescing and a final stable read. A raw filesystem event is not sufficient evidence that Packwiz state is complete or parseable.
- The watcher must not become a second filesystem authority in Svelte. Rust should canonicalize and contain paths, perform validation/fingerprinting, classify unavailable or malformed evidence, and emit typed status/events to the frontend.
- Publication is a hard watcher boundary: a published release retains its historical evidence but never observes or records project changes made after publication. Later edits belong to a new snapshot or release workspace.
- No-op releases should remain drafts for notes or planning but cannot be finalized. Git, GitHub, and external publication remain optional.
- The prior release comparison implementation and capture contracts are reusable for final release evidence, but the current release creation command and modal are too narrow to serve as the workspace without decomposition.

## Architecture Refinements
- Introduce a first-class release workspace boundary. The current release record is an immutable capture-oriented record, while the new workspace owns mutable candidate decisions, operations, recovery, draft changelog work, and metadata until finalization.
- Keep workspace lifecycle, evidence status, and publication status separate. Do not extend publication status to carry `applying`, `recovery_required`, `verified`, or `finalized` states.
- Give operations explicit release workspace ownership while retaining the source snapshot association as immutable provenance. Do not duplicate one operation attempt under both snapshot and release ownership.
- Store release candidate decisions separately from immutable snapshot decisions. A release decision should retain its source candidate, decision history, notes, application status, and final observed outcome.
- Reuse the existing stable capture pipeline for snapshot baselines, watcher refreshes, post-operation verification, and final release captures. Avoid creating separate readers with subtly different evidence semantics.
- Make watcher ownership explicit in a backend release-workspace coordinator. It must provide one watcher per active workspace, debounced and coalesced events, stable rereads, operation coordination, explicit teardown, and typed Tauri events containing workspace identity, changed scope, evidence status, and blocking reason.
- Treat the final release capture as immutable evidence. Release comparison must compare finalized captures rather than a live working tree or an automatically refreshed workspace observation.
- Preserve existing snapshot changelog artifacts and revision history while adding release context and stage/provenance metadata. Release-owned changelog artifacts must identify whether they are proposed or final and which capture or decision set produced them.
- Use the alpha schema-reset policy deliberately: breaking schema changes are acceptable, but the plan must specify the reset boundary and must not silently rewrite or discard existing local data.

## Handoff Options
1. **Continue specification assessment**: decide only the watcher timing and UX details that would change implementation scope, such as whether automatic refresh is visible as a status event, a refreshed inventory panel, or both.
2. **Hand off to the planning agent**: plan an alpha-breaking redesign in phases: snapshot baseline capture, release workspace schema/contracts, operation ownership migration, draft/final changelog flow, release-scoped watcher and external-change recovery, finalization and immutable capture, then UI and regression validation.

Recommended next step: hand off to planning. The product direction, user-facing phases, lifecycle guardrails, recovery behavior, and terminal-edit observation behavior are sufficiently clear; candidate-row normalization, exact backend state names, workspace-versus-final-release persistence shape, watcher crate choice, changelog finalization requirement, and event payload shape should be resolved against existing contracts during plan phase 1.

## Quality Gate
- The revised request was decomposed into six changes covering snapshot role, release workspace creation, operation ownership, changelog timing, finalization, and release-scoped observation of terminal-driven edits.
- Multiple clarification rounds were used for lifecycle, evidence, workspace ownership, changelog timing, failure recovery, baseline, candidate selection, and no-op behavior.
- The earlier promotion-only recommendation is explicitly marked superseded.
- Current-state claims are tied to snapshot, operation, release, changelog, schema, and UI surfaces.
- Benefits, risks, dependencies, interactions, and discriminating checks are explicit.
- User-facing release phases, resume and abandonment semantics, candidate-level recovery, external-change presentation, changelog stage labeling, and finalization review requirements are explicit.
- Architecture refinements distinguish workspace lifecycle, evidence status, and publication status, and define ownership for operations, candidate decisions, captures, changelog artifacts, and watcher coordination.
- Remaining implementation-level choices are visible before planning, including candidate-row normalization, exact backend state names, workspace persistence shape, watcher implementation, debounce behavior, changelog finalization policy, and event payload shape.
- No application code was modified; only this assessment document was revised.
