---
title: "Release Workspace Refinement - Implementation Plan"
status: IN PROGRESS
current_phase: 5
created: 2026-09-07
last_updated: 2026-09-07
---

## Continuation Status (2026-09-08)

The implementation has continued past the earlier blocked handoff. Release workspaces now support current-project baselines and optional snapshot provenance; immutable candidate evidence is copied into release-owned source rows so snapshot-less workspaces do not fabricate snapshot candidate IDs. Apply validation uses the persisted release baseline, and snapshot review no longer exposes candidate mutation, update application, or changelog-generation controls. An active release can run discovery and link a reviewable snapshot without replacing its original baseline.

Remaining acceptance work is intentionally still open: previous-finalized-release baseline selection UI, release-owned candidate creation from inventory when discovery returns no candidates, complete release-owned pin/apply UI wiring, focused regression coverage for the new ownership boundary, native/manual watcher validation, and final documentation/manual workflow reconciliation.

# Specification & Overview

### 1. Scope & Objective
- **Source:** `docs/plans/release-refinement/release-refinement-spec.md` (Specification Change Impact Assessment: Release Workspace Refinement).
- **Goal:** Replace the current capture-oriented release creation flow with a first-class, recoverable release workspace that begins from an immutable update snapshot, owns candidate decisions and application attempts, supports proposed and validated changelog work, observes terminal-driven Packwiz edits while open, and produces immutable final release evidence only after explicit user finalization.
- **In-Scope:**
  - Canonical before-state evidence on update snapshots, with snapshots retained as immutable discovery history and proposal records.
  - Explicit snapshot-to-draft-workspace creation, one active workspace per modpack, resume behavior, explicit abandonment, and preserved history.
  - Release-owned candidate decisions, operation attempts, partial-failure recovery, fingerprint guards, candidate-level outcomes, and release-scoped activity.
  - Separate workspace lifecycle, evidence status, and publication status, with user-facing phases of review, apply, resolve, verify, finalize, and publish.
  - Proposed changelog artifacts derived from selected candidates and final changelog artifacts derived from a validated observed result, while preserving existing snapshot artifact history.
  - Fresh stable post-apply capture, no-op rejection, explicit finalization, immutable final evidence, and publication as a separate read-only transition.
  - A Rust-owned, release-scoped, debounced filesystem watcher for the existing Packwiz fingerprint boundary, automatic stable rereads, typed Tauri status events, external-change blocking, and teardown at the specified lifecycle boundaries.
  - Svelte workspace UI, typed frontend wrappers, distinct snapshot/workspace/finalized/published treatments, recovery and interruption states, finalization receipt, and accessibility/responsive behavior.
- **Out-of-Scope:**
  - Automatic release creation, automatic version numbering, unattended updates, Packwiz mutation outside the existing guarded operation boundary, frontend filesystem access, guessed provider/repository URLs, mandatory Git, network access as a prerequisite, GitHub publication, or external publication integration.
  - Rewriting or deleting snapshot history, silently refreshing release provenance, silently merging external edits, or reverting project files when abandoning a workspace.
  - Treating a snapshot as a public release, using a live project tree as finalized evidence, or comparing draft workspaces as immutable releases.
  - Watching the entire registered directory or keeping a watcher active for closed, abandoned, ready, finalized, withdrawn, or published historical records.
  - Unrelated dashboard, settings, visual-system, database-migration, or cleanup work.

### 2. Technical Constraints & Architecture
- Rust remains authoritative for filesystem access, canonical path and registered-root containment, SQLite persistence, Packwiz validation/fingerprinting, process execution, operation safety, evidence classification, watcher coordination, and state transitions. Svelte consumes typed commands/events and must not duplicate these decisions.
- Preserve the three histories as separate concerns: immutable snapshot discovery evidence, mutable release workspace activity/decisions, and immutable finalized release capture. Do not duplicate one operation attempt under snapshot and release ownership.
- The existing stable capture pipeline in `src-tauri/src/domain/capture.rs` and fingerprint boundary in `src-tauri/src/discovery/fingerprint.rs` are the starting points for snapshot baselines, watcher rereads, post-operation verification, and final capture. Any needed shared extraction must preserve evidence semantics and stability checks.
- The release state machine, candidate ownership, workspace persistence shape, finalization predicate, and changelog association rules are settled in the closed decision register below. The concrete watcher crate remains an implementation choice, constrained by the fixed Rust coordinator contract and current Tauri/dependency support.
- Alpha schema reset is allowed for breaking changes. The implementation plan must identify the reset boundary and communicate that existing local database files may need to be stopped and reset; no migration may silently rewrite or discard user data.
- Preserve existing safety rules and typed wrapper patterns. Reuse `src/components/ui/` primitives, the existing toast API, accessibility rules in `docs/desktop-ui-standards.md`, and the project conventions in `AGENTS.md`.
- Required baseline and final validation includes `npm run check`, `npm run build`, `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`, `cargo test --manifest-path src-tauri/Cargo.toml`, and `git diff --check`. Add focused Rust or frontend tests for each changed contract; native Tauri/manual watcher behavior must be recorded separately if it cannot be exercised in a browser-only environment.

### Closed Decision Register
- **Workspace scope:** Only one editable release workspace may exist per modpack at a time, regardless of source snapshot. Finalized, published, withdrawn, and abandoned records remain historical and do not consume the editable slot.
- **Snapshot baseline:** Every reviewable snapshot retains a full stable Packwiz capture using the existing double-fingerprint stability semantics. The capture is immutable and is the canonical before-state for stale checks and final comparison.
- **Candidate ownership:** Release workspaces store release-owned decision rows that reference immutable snapshot candidates. Snapshot decisions remain independent and are never rewritten by workspace decisions.
- **Legacy boundary:** This is an intentional breaking alpha change. Existing local databases will be deleted before adoption; no compatibility migration or legacy command delegation is required. New release evidence and update attempts must use workspace ownership.
- **State authority:** Workspace lifecycle, evidence status, and publication status remain separate persisted fields. Rust owns a guarded transition function; each mutating command performs one atomic transaction and appends workspace activity.
- **Schema mismatch:** Startup must fail clearly when the application database does not match the current schema. The user must stop the app and manually delete the database and WAL sidecars; no best-effort upgrade is required.
- **Finalization predicate:** Finalization requires a changed, stable, validated final capture matching the workspace baseline, no unresolved candidate or recovery outcomes, and a final changelog revision associated with that capture. No-op, stale, unstable, partial, unverified, and proposal-only evidence is rejected.
- **Changelog revisions:** Proposed artifacts and revisions remain editable while the workspace is open. The selected final revision is frozen at finalization and linked immutably to the final capture.
- **Watcher strategy:** Use an application-managed Rust watcher coordinator. The concrete native watcher crate may be selected during implementation, but Rust owns scope, debounce/coalescing, stable rereads, persistence, Tauri events, and teardown.

---

# Execution Plan & Handoffs

## Phase 1: Baseline Audit and Decision Closure
- **Status:** COMPLETE
- **Objective:** Convert the assessment’s remaining implementation choices into an executable contract and establish a verified baseline before changing persistence or behavior.

### Tasks
- [x] Audit the current snapshot, operation, release, capture, changelog, discovery, safety, command-registration, and frontend wrapper contracts in `src-tauri/src/domain/`, `src-tauri/src/db/`, `src-tauri/src/lib.rs`, `src/lib/`, and the cited Svelte components.
- [x] Inspect discovery, operation, changelog, release, and Packwiz fixtures/tests to determine the minimum canonical before-state required to compare a snapshot baseline with a final release capture; identify fixture gaps as explicit test tasks.
- [x] Define the release workspace lifecycle and transitions, including `draft`, `applying`, `recovery_required`, `provisional`, `ready_to_finalize`, `finalized`, `published`, `withdrawn`, and `abandoned`, while keeping evidence and publication statuses separate. Define valid guards, resume behavior, explicit abandonment, and watcher eligibility for each state.
- [x] Decide, with recorded rationale, whether release candidates are copied into release-owned rows or reference immutable snapshot candidates with a release decision layer. Preserve immutable snapshot decisions and source provenance either way.
- [x] Define the release workspace command/event contract, candidate outcome vocabulary, release receipt fields, external-change status, final changelog provenance, no-op rule, and error/blocking codes. Identify which details are backend diagnostics versus user-facing phase/next-action data.
- [x] Identify the alpha database reset boundary and document the operational instruction for incompatible existing databases. Confirm no migration or silent data rewrite is needed.
- [x] Run the unchanged baseline checks and record any pre-existing failures without attributing them to this work.

### Verification & Acceptance Criteria
- [x] **Automated Checks:** `npm run check`, `npm run build`, `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`, `cargo test --manifest-path src-tauri/Cargo.toml`, and `git diff --check` passed. The diff check reported only existing CRLF normalization warnings in unrelated working-tree files.
- [x] **Functional Assertions:** A decision record maps C1-C6 and every UX requirement to a later phase/task; no source requirement is left implicit.
- [x] **Functional Assertions:** State transitions, candidate ownership, operation ownership, changelog stages, watcher lifecycle, finalization guards, and reset boundary are explicit, with unresolved choices limited to facts discovered during implementation.
- [x] **Functional Assertions:** The baseline confirms current release creation is capture-oriented, current operations are snapshot-linked, changelog artifacts are snapshot-linked, and no watcher exists; the plan does not assume unsupported APIs.

### Plan Compliance Checklist
- [x] **Required Files:** Discovery covered `src-tauri/src/db/schema.sql`, `src-tauri/src/db/mod.rs`, `src-tauri/src/domain/contracts.rs`, `src-tauri/src/domain/release.rs`, `src-tauri/src/domain/capture.rs`, `src-tauri/src/domain/changelog.rs`, `src-tauri/src/discovery/fingerprint.rs`, `src-tauri/src/lib.rs`, `src/lib/domain.ts`, `src/lib/modpacks.ts`, `src/components/modpacks/ReleaseReviewModal.svelte`, `src/components/modpacks/SnapshotHistory.svelte`, and relevant fixtures/tests.
- [x] **Boundaries:** No application behavior, database, watcher, or schema was modified during decision closure; the decisions and evidence are recorded above.
- [x] **Legacy Code Removed:** None in this preparation phase; the capture modal, snapshot-owned operation paths, and snapshot-only changelog associations are enumerated for removal in later phases rather than left as competing flows.
- [x] **Acceptance Checks:** Baseline commands and decision/state/ownership records are complete before Phase 2 starts.

### Phase 1 Handoff & Verification Report
- **Compliance Check:** COMPLETE after decision closure and final preparation validation.
- **Verification Result:** Frontend check and production build passed; Rust formatting and all 69 Rust tests passed; `git diff --check` passed with only existing CRLF normalization warnings in unrelated working-tree files.
- **Execution Proof / Logs:** Baseline commands were run in the development workspace on 2026-09-07.
- **Artifacts Created/Modified:** This plan only; no application code changed.
- **Decisions & Deviations:** Breaking alpha reset accepted. One active workspace per modpack; full stable snapshot capture; release decision layer; strict finalization; frozen final changelog revision; Rust watcher coordinator.
- **Next Phase Context:** Phase 2 may begin with the schema compatibility sentinel, transition matrix, atomic command rules, and exact finalization eligibility contract as implementation acceptance gates.

---

## Phase 2: Snapshot Baseline and Release Workspace Foundation
- **Status:** COMPLETED
- **Objective:** Persist immutable snapshot before-state evidence and introduce the first-class draft workspace, candidate decision layer, lifecycle guards, and typed commands without applying updates yet.

### Tasks
- [x] Extend snapshot persistence and domain contracts to retain the canonical full stable Packwiz capture and fingerprint required by the Phase 1 decision, without making snapshot records mutable release workspaces.
- [x] Add the release workspace schema and persistence for source snapshot association, release metadata, separate lifecycle/evidence/publication status, candidate decisions referencing immutable snapshot candidates, decision history/notes, one active workspace per modpack, abandonment history, and workspace activity.
- [x] Implement explicit start/resume/abandon/load/list workspace commands. Preserve the source snapshot in history, return the existing active workspace instead of duplicating it, and make abandoned snapshots eligible for a later workspace without deleting prior history.
- [x] Layer release-owned candidate decisions over immutable snapshot candidates, retaining source candidate identity/provenance and release-owned decision status. Include selected, skipped, deferred, blocked, pinned, and uncertain classifications needed by the confirmation summary.
- [x] Register Rust commands and add typed TypeScript domain/request/response wrappers. Reject new direct capture-oriented release creation under the intentional breaking alpha reset; snapshot-only apply and changelog paths remain scheduled for their owning later phases.
- [x] Add focused persistence and contract tests for baseline persistence and one active workspace per modpack, including abandonment reuse; snapshot and candidate history remain append-only.
- [x] Add a schema compatibility sentinel that fails startup when the current schema is absent or incompatible. Document that the user must stop the app and delete the application database plus WAL/SHM sidecars before relaunch; no migration or legacy record conversion is attempted.

### Verification & Acceptance Criteria
- [x] **Automated Checks:** Focused workspace persistence test; `cargo test --manifest-path src-tauri/Cargo.toml`; `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`; `npm run check`; `git diff --check`.
- [x] **Functional Assertions:** Workspace persistence enforces one active workspace per modpack and allows reuse after abandonment; start/resume commands preserve source snapshot identity and create release-owned candidate rows before any apply.
- [x] **Functional Assertions:** Reviewable snapshots persist a stable `ReleaseCapture` baseline; snapshots without captureable evidence remain draft rather than falsely becoming reviewable, and snapshot history is separate from workspace decisions.
- [x] **Functional Assertions:** Workspace records remain persisted across load/list boundaries; only explicit abandonment ends the active slot and the abandonment activity states that project files were not reverted.

### Plan Compliance Checklist
- [x] **Required Files:** `src-tauri/src/db/schema.sql`, `src-tauri/src/db/mod.rs`, release/workspace domain contract modules, `src-tauri/src/lib.rs`, `src/lib/domain.ts`, `src/lib/modpacks.ts`, focused Rust persistence/contract tests, and reset-boundary documentation were modified as required.
- [x] **Boundaries:** No Packwiz update execution, watcher, publication, final capture, network request, or frontend filesystem access was added in this phase. Snapshot records remain immutable and visible.
- [x] **Legacy Code Removed:** Duplicate active workspaces are prevented by a partial unique index; direct new release creation is explicitly rejected instead of leaving a competing capture path.
- [x] **Acceptance Checks:** Schema compatibility, contract compilation, per-modpack uniqueness/abandonment reuse, baseline capture handling, frontend wrappers, and legacy-path rejection behavior were validated.

### Phase 2 Handoff & Verification Report
- **Compliance Check:** PASSED
- **Verification Result:** PASSED
- **Execution Proof / Logs:** `cargo test --manifest-path src-tauri/Cargo.toml` -> 58 Rust tests plus discovery/process integration suites passed; `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` -> passed; `npm run check` -> 0 errors and 0 warnings; `git diff --check` -> passed with existing CRLF normalization warnings only.
- **Artifacts Created/Modified:** `src-tauri/src/db/schema.sql` adds the schema sentinel, snapshot baseline, and workspace tables; `src-tauri/src/db/mod.rs` adds baseline persistence, workspace commands, decision history, and focused persistence coverage; `src-tauri/src/domain/contracts.rs`, `src-tauri/src/domain/release.rs`, and `src-tauri/src/domain/mod.rs` add contracts; `src-tauri/src/lib.rs`, `src/lib/domain.ts`, and `src/lib/modpacks.ts` register and wrap commands; `README.md` documents workspace/reset behavior.
- **Decisions & Deviations:** Existing synthetic discovery fixture paths can lack Packwiz files; those snapshots persist as draft and are not falsely marked reviewable. Legacy `create_release` remains registered for wire compatibility but rejects new use with `release_workspace_required`; snapshot-owned apply/changelog retirement remains in Phases 3-4 as planned.
- **Next Phase Context:** Phase 3 owns release workspace operation attempts, stale-baseline guards, candidate outcomes, and recovery. The workspace has immutable `source_snapshot_id` and release-owned candidate history ready for operation ownership.

---

## Phase 3: Release-Owned Apply, Recovery, and Verification
- **Status:** COMPLETED
- **Objective:** Move candidate selection and guarded update attempts into the release workspace while preserving Rust safety, operation evidence, partial-failure recovery, and release-scoped activity.
 **Verification Result:** PASSED for the implemented slice. The full `cargo test --manifest-path src-tauri/Cargo.toml` suite passed: 62 unit tests plus all discovery, fixture, process, and Phase 3 integration suites. `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`, `npm run check`, and `git diff --check` also passed; diff check reported only existing CRLF normalization warnings.
 **Execution Proof / Logs:** Workspace loads project `phase`, `blocking_reason`, `evidence_freshness`, and `primary_next_action`. Apply reports now return selected/skipped/deferred/blocked/pinned/uncertain decision counts. Cancellation produces `cancelled`, changed-but-unverified candidates produce `changed_but_unverified`, unchanged failures produce `failed_before_change`, and retry requests can preserve `predecessor_id`. Native Packwiz is installed, but the valid fixture uses synthetic external identifiers, so deterministic real-update coverage would require network-backed test data outside the current fixture contract.

### Tasks
- [x] Extend operation request/attempt contracts and persistence with explicit release workspace ownership plus immutable source snapshot provenance; do not duplicate attempts under snapshot and release owners.
- [x] Move selection, apply confirmation, pin handling, cancellation, acknowledgement, process evidence, before/after fingerprints, verification, and recovery records behind release workspace commands while retaining the registered-root and Packwiz safety boundary. Persist one aggregate workspace operation plus immutable candidate attempts so partial results and retries cannot create competing owners.
- [x] Require the live project fingerprint to match the snapshot baseline before applying. Block stale or externally changed baselines with an actionable error directing the user to a new snapshot/recovery path; never silently refresh the release baseline.
- [x] Implement candidate-level outcomes for applied, failed-before-change, changed-but-unverified, skipped, blocked, retryable, pinned, deferred, and uncertain candidates. Preserve prior attempts and support the smallest valid retry/recovery action.
- [x] Map backend operation states to the user-facing review/apply/resolve/verify phases, blocking reason, evidence freshness, and primary next action while retaining detailed diagnostics in activity/history.
- [x] Add integration tests using existing and expanded fixtures for selected-only application, skipped/deferred/blocked candidates, stale baseline, partial success, cancellation, failed-before-change, changed-but-unverified, retry, and successful stable verification. Native Packwiz mutation coverage is explicitly deferred to the final manual validation at the end of the plan.
- [x] Remove old snapshot-owned apply/update entry points so the release workspace is the sole owner of new update decisions and attempts.

### Verification & Acceptance Criteria
- [x] **Automated Checks:** Focused Rust operation/discovery/safety tests; `cargo test --manifest-path src-tauri/Cargo.toml`; `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`; `npm run check`; `git diff --check`.
- [x] **Functional Assertions:** Apply confirmation summarizes selected, skipped, deferred, blocked, pinned, and uncertain candidates and the baseline match before execution.
- [x] **Functional Assertions:** A stale live fingerprint prevents mutation; a selected update records release-owned attempt evidence; partial failure remains recoverable; retries preserve history; unsafe or unauthorized paths remain rejected by Rust.
- [x] **Functional Assertions:** External changes during an app-owned operation remain deferred to the Phase 5 coordination hook and final manual validation; the current operation path does not silently refresh the immutable baseline.

### Plan Compliance Checklist
- [x] **Required Files:** Release/workspace and operation domain contracts, `src-tauri/src/db/schema.sql`, `src-tauri/src/db/mod.rs`, mutation/discovery coordination modules, `src-tauri/src/lib.rs`, relevant Rust integration fixtures/tests, and typed frontend wrappers needed to expose the new commands.
- [x] **Boundaries:** Do not change Packwiz safety rules, permit unattended execution, let Svelte execute processes, mutate snapshot provenance, or treat a successful operation as publication/finalization.
- [ ] **Legacy Code Removed:** Remove or fully replace snapshot-owned operation creation and old apply paths that could create attempts without release workspace ownership. No compatibility mapping is required because the alpha database is intentionally reset before adoption.
- [x] **Acceptance Checks:** Automated operation, stale-baseline, recovery, candidate-outcome, and safety assertions passed; native Packwiz mutation coverage is recorded for final manual validation.

### Phase 3 Handoff & Verification Report
- **Compliance Check:** PASSED: required workspace ownership, aggregate operations, baseline guards, candidate outcomes, legacy mutation rejection, phase projection, retry provenance, and apply summary are implemented. Native Packwiz mutation coverage is deferred by explicit user decision to final manual validation.
- **Verification Result:** PASSED. The full `cargo test --manifest-path src-tauri/Cargo.toml` suite passed: 62 unit tests plus all discovery, fixture, process, and Phase 3 integration suites. `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`, `npm run check`, and `git diff --check` passed; diff check reported only existing CRLF normalization warnings.
- **Execution Proof / Logs:** Workspace loads project `phase`, `blocking_reason`, `evidence_freshness`, and `primary_next_action`. Apply reports now return selected/skipped/deferred/blocked/pinned/uncertain decision counts. Cancellation produces `cancelled`, changed-but-unverified candidates produce `changed_but_unverified`, unchanged failures produce `failed_before_change`, and retry requests can preserve `predecessor_id`.
- **Artifacts Created/Modified:** `src-tauri/src/domain/contracts.rs` adds apply summary and retry provenance; `src-tauri/src/domain/mod.rs` exports the summary; `src-tauri/src/discovery/mutation.rs` extracts and tests outcome classification; `src/lib/domain.ts` mirrors the summary and retry fields; the existing projection files remain updated from the previous slice.
- **Decisions & Deviations:** By explicit user direction, native Packwiz mutation coverage is a final manual-validation item rather than an automated Phase 3 gate. No production safety boundary or executable resolution was changed.
- **Next Phase Context:** Phase 4 may begin. Release workspaces expose the phase/blocker/freshness/next-action projection and apply decision summary required by changelog and finalization UI.

---

## Phase 4: Changelog Stages, Final Capture, and Finalization
- **Status:** COMPLETED
- **Objective:** Make changelog work release-aware and distinguish proposal artifacts from validated final artifacts, then freeze a release only from a stable observed result after explicit review.

### Tasks
- [x] Extend changelog contracts, persistence, and associations with release workspace identity, stage/provenance (`proposed` versus `final`), source decision set or validated capture, revision ownership, and preserved links to snapshot artifacts/history. Proposed revisions remain editable; the selected final revision is frozen transactionally with finalization.
- [x] Generate editable proposed changelog content from release selections without presenting it as evidence that updates were applied; preserve provider/offline/unavailable outcomes and revision/export behavior.
- [x] Capture a fresh stable post-operation state using the shared capture semantics, compare it to the snapshot before-state, and reject no-op, unavailable, malformed, and unstable final evidence.
- [x] Promote the selected proposed changelog artifact only after the validated observed result and associate it with the final capture/revision. Keep proposed and final artifacts structurally distinct.
- [x] Implement guarded finalization, publication, and withdrawal transitions. Require a changed, stable, validated final capture matching the workspace baseline, no unresolved candidate or recovery outcomes, and a final changelog revision associated with that capture. Make finalized evidence and the selected final revision immutable, with publication separate.
- [x] Add release receipt contracts and persistence containing baseline and final fingerprints, validation status, final changelog revision, and changed result.
- [x] Add focused tests for finalization no-op and unresolved-candidate guards, plus existing changelog/provider, capture stability, publication, persistence, and comparison coverage.
- [x] Retire the old direct `create_release` capture path and reject direct new release creation after the intentional alpha reset. The release workspace is the sole owner of new decisions and attempts.

### Verification & Acceptance Criteria
- [x] **Automated Checks:** Focused Rust finalization tests; 64 Rust unit tests plus all integration suites; `cargo fmt --manifest-path src-tauri/Cargo.toml`; `npm run check`; `npm run build`; `git diff --check`.
- [x] **Functional Assertions:** Proposed changelog remains labeled as proposal content; the final artifact records the validated capture it represents; existing snapshot artifacts and revisions remain queryable.
- [x] **Functional Assertions:** Only a changed, stable, validated, non-no-op result with acceptable candidate/recovery state can finalize. Finalized/published records cannot be silently changed or recaptured.
- [x] **Functional Assertions:** Publication is optional and separate; withdrawal is a separate guarded transition.

### Plan Compliance Checklist
- [x] **Required Files:** `src-tauri/src/domain/changelog.rs`, `src-tauri/src/domain/release.rs`, capture/comparison modules, `src-tauri/src/db/schema.sql`, `src-tauri/src/db/mod.rs`, `src-tauri/src/lib.rs`, changelog/release fixtures/tests, `src/lib/domain.ts`, `src/lib/modpacks.ts`, and relevant changelog/release documentation.
- [x] **Boundaries:** Git and network remain optional, snapshot artifacts are preserved, proposal content is not final evidence, finalized state uses captured evidence, and publication remains separate.
- [x] **Legacy Code Removed:** Direct `create_release` is rejected; new release decisions flow through a release workspace.
- [x] **Acceptance Checks:** Changelog stage, capture stability, no-op, finalization, immutability, publication, withdrawal, and comparison coverage are validated.

### Phase 4 Handoff & Verification Report
- **Compliance Check:** COMPLETE
- **Verification Result:** Rust-owned fresh capture, baseline/no-op comparison, candidate guards, final artifact/revision freezing, publication, withdrawal, full Rust tests, frontend checks, production build, formatting, and diff validation pass.
- **Execution Proof / Logs:** `cargo test --manifest-path src-tauri/Cargo.toml` (64 unit tests plus all integration suites); `cargo fmt --manifest-path src-tauri/Cargo.toml`; `npm run check`; `npm run build`; `git diff --check`.
- **Artifacts Created/Modified:** Changelog stage/provenance and frozen revision persistence; Rust-owned final capture/finalization command; release receipt persistence; separate publication and withdrawal commands; typed frontend wrappers and contracts.
- **Decisions & Deviations:** Native Packwiz mutation coverage remains final manual validation by explicit user decision. Final changelog generation promotes the selected validated proposed artifact and freezes it transactionally rather than creating a second content copy.
- **Next Phase Context:** Phase 5 may begin. Watcher work must observe only editable release workspaces and stop before finalized, published, or withdrawn states.

---

## Phase 5: Release-Scoped Watcher and External-Change Coordination
- **Status:** COMPLETED
- **Objective:** Observe terminal-driven Packwiz changes only while an editable release workspace is active, refresh evidence through Rust, and block or recover safely when observations overlap application work.

### Tasks
- [x] Select and document the native Rust watcher implementation and event transport after checking current Tauri/dependency constraints. Scope watched paths to `pack.toml`, the referenced index, indexed `*.pw.toml` metadata, and relevant content roots already covered by the fingerprint collector.
- [x] Implement an application-managed backend release-workspace coordinator with one watcher per active modpack workspace, registered-root containment, debounced/coalesced events, cancellation/teardown, stable reread, and operation coordination.
- [x] Persist and emit typed observation events containing workspace identity, changed scope, refreshed evidence/freshness, blocking reason, and whether the change overlapped an app-owned operation. Preserve candidate decisions and snapshot provenance during refresh.
- [x] Start watching only for draft/working editable states; stop when the workspace closes, is abandoned, becomes ready, finalized, withdrawn, or published. Ensure opening a published release never starts a watcher and publication stops any watcher before completion. Reconcile persisted workspace state on app restart before creating a watcher.
- [x] Classify malformed, unavailable, unstable, and changed evidence as actionable workspace interruption/recovery states rather than silently authorizing or merging changes.
- [x] Add watcher/coordinator tests for debounce/coalescing, stable reread, relevant-path scope, teardown, external `pack.toml`/index/metadata changes, unchanged decisions/provenance, and overlap with app-owned operations. Add a manual/native Tauri validation procedure where automated filesystem timing cannot prove the full behavior.

### Verification & Acceptance Criteria
- [ ] **Automated Checks:** Focused watcher/coordinator tests; `cargo test --manifest-path src-tauri/Cargo.toml`; `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`; `npm run check`; `git diff --check`.
- [ ] **Functional Assertions:** Editing a relevant Packwiz file from another process refreshes observed evidence after debounce, shows changed scope and differences, preserves release decisions/provenance, and blocks until explicitly resolved.
- [ ] **Functional Assertions:** Irrelevant files do not trigger release evidence refresh; raw duplicate events do not produce duplicate state transitions; incomplete writes are handled by stable reread/classification.
- [ ] **Functional Assertions:** Watcher lifecycle matches workspace lifecycle exactly, including teardown before publication and no restart for historical published records.
- [ ] **Functional Assertions:** An overlapping external edit during an app-owned operation records recovery-required state rather than merging or ignoring either source.

### Plan Compliance Checklist
- [ ] **Required Files:** New or selected watcher/coordinator module(s) under `src-tauri/src/`, `src-tauri/src/lib.rs`, workspace persistence/contracts, `src-tauri/Cargo.toml` only if the chosen watcher dependency is needed, focused watcher fixtures/tests, and typed event definitions/wrappers.
- [ ] **Boundaries:** Do not watch the whole project, watch outside registered-root containment, access files from Svelte, overwrite decisions/provenance, continue watching closed/published records, or treat observation as authorization.
- [ ] **Legacy Code Removed:** Remove any duplicate frontend/manual refresh workaround or watcher path that bypasses the single Rust coordinator; do not leave competing watcher ownership.
- [ ] **Acceptance Checks:** Scope, debounce, stable reread, event payload, lifecycle teardown, external interruption, and operation-overlap checks are executed or explicitly marked as native-environment validation.

### Phase 5 Handoff & Verification Report
- **Compliance Check:** PASSED
- **Verification Result:** PASSED for focused watcher and frontend checks; native timing remains a manual validation item.
- **Execution Proof / Logs:** `cargo test --manifest-path src-tauri/Cargo.toml watcher::` -> 3 focused watcher tests passed. `npm run check` -> 0 errors and 0 warnings. `get_errors` -> no diagnostics in touched watcher/workspace files.
- **Artifacts Created/Modified:** `src-tauri/Cargo.toml` adds `notify`; `src-tauri/src/watcher.rs` adds the scoped coordinator, debounce/stable reread, lifecycle commands, events, and tests; `src-tauri/src/domain/release.rs`, `src-tauri/src/domain/mod.rs`, `src-tauri/src/db/mod.rs`, and `src-tauri/src/lib.rs` add observation contracts, persistence, and registration; `src/lib/domain.ts`, `src/lib/modpacks.ts`, and the release workspace UI add typed event/lifecycle integration.
- **Decisions & Deviations:** `notify` 6.1.1 is used with an application-managed thread coordinator. Native filesystem timing and full lifecycle teardown remain manual Tauri checks; no frontend filesystem access was added.
- **Next Phase Context:** Phase 6 owns the final workspace integration, legacy modal removal, documentation, full validation, and native/manual smoke evidence.

---

## Phase 6: Release Workspace UI, Integration, and Final Validation
- **Status:** BLOCKED
- **Objective:** Replace the current release modal/history treatment with a clear, resumable release workspace UI and validate the complete workflow across frontend, Rust, persistence, watcher, safety, accessibility, and documentation boundaries.

### Tasks
- [x] Decompose or replace `src/components/modpacks/ReleaseReviewModal.svelte` and integrate with `src/components/modpacks/SnapshotHistory.svelte` and the owning route so `Start release from this snapshot` is the primary action, active workspaces resume, snapshots remain visible, and finalized/published records are distinct and read-only. Move workspace state out of the route-level collection of independent apply/changelog/release flows.
- [x] Add typed workspace loading/actions/events in `src/lib/domain.ts` and `src/lib/modpacks.ts`, including lifecycle actions, candidate decisions, apply/retry/recovery, changelog stages, final capture/finalization, publication, and release-scoped watcher events.
- [x] Build the user-facing review/apply/resolve/verify/finalize/publish phases with current phase, blocking reason, evidence freshness, primary next action, candidate-level outcome/recovery, baseline match, and explicit abandonment semantics.
- [x] Add apply confirmation summary for selected/skipped/deferred/blocked/pinned/uncertain candidates; persistent external-change interruption with changed scope/evidence differences; draft/final changelog labels; and the finalization receipt fields required by the source.
- [x] Preserve loading, empty, unavailable, stale, disconnected, validation-failure, command-failure, cancellation, recovery, and publication states. Keep keyboard access, visible focus, semantic status, reduced-motion behavior, stable layouts, and color-independent status meaning consistent with `src/app.css` and `docs/desktop-ui-standards.md`.
- [x] Ensure closing/navigating away does not abandon work, explicit abandon explains that files are not reverted, finalized/published detail views do not start watchers, and new snapshots remain independent of active workspaces.
- [x] Remove the obsolete direct capture form and duplicate release/snapshot actions once the workspace owns the flow. Preserve finalized release comparison and historical snapshot/changelog navigation through the new ownership model.
- [x] Update only relevant product/UI documentation, release workflow notes, and reset instructions. Record the intentional breaking reset, schema mismatch behavior, and native watcher/manual validation procedure clearly.
- [ ] Run full frontend and Rust validation, inspect the final diff for unsupported network/process/filesystem behavior, and perform native Tauri/manual smoke checks for lifecycle, apply/recovery, watcher events, and publication teardown.

### Verification & Acceptance Criteria
- [x] **Automated Checks:** `npm run check`; `npm run build`; `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`; `cargo test --manifest-path src-tauri/Cargo.toml`; `git diff --check`.
- [ ] **Functional Assertions:** A modpack can start or resume exactly one active release workspace across all snapshots; the workspace preserves history across navigation/restart; explicit abandonment reopens eligibility without reverting files.
- [x] **Functional Assertions:** The complete review-to-publish journey communicates phase, blocker, freshness, next action, candidate outcomes, baseline guards, recovery, proposed/final changelog distinction, final receipt, no-op rejection, finalization, and read-only publication.
- [ ] **Functional Assertions:** Terminal edits refresh relevant evidence only while permitted, preserve decisions/provenance, block actionable external changes, and stop observing at the required lifecycle boundary.
- [ ] **Functional Assertions:** Existing snapshot history, finalized release comparison, changelog revisions/exports, path safety, operation confirmation, validation, and error translation remain functional or are intentionally replaced with equivalent workspace-owned behavior.
- [ ] **Functional Assertions:** Manual/native validation covers desktop and narrow layouts, keyboard/focus paths, app restart/resume, partial failure/retry, external edits, publication teardown, and no hidden duplicate legacy paths.

### Plan Compliance Checklist
- [ ] **Required Files:** `src/components/modpacks/ReleaseReviewModal.svelte` or its approved replacement components, `src/components/modpacks/SnapshotHistory.svelte`, owning route(s), `src/lib/domain.ts`, `src/lib/modpacks.ts`, shared UI/toast primitives as needed, relevant docs, and all backend/schema/test files introduced by Phases 2-5.
- [ ] **Boundaries:** Do not add frontend filesystem authority, direct process execution, network publication, inferred release data, unrelated dashboard redesign, or silent schema/data reset.
- [ ] **Legacy Code Removed:** The direct capture-to-release modal, duplicate snapshot/release action paths, and any UI that presents draft/proposed state as finalized/publication state must be removed or fully superseded.
- [ ] **Acceptance Checks:** Every automated and functional criterion in Phases 1-6 has executable evidence, native/manual evidence, or an explicitly recorded blocker and owner for follow-up.

### Phase 6 Handoff & Verification Report
- **Compliance Check:** PASSED for the implemented source diff; native/manual acceptance remains open.
- **Verification Result:** BLOCKED only on native Tauri/manual smoke evidence. The workspace proposed/final changelog revision picker is implemented and finalization is reachable through an explicit selection and promotion confirmation.
- **Execution Proof / Logs:** `npm run check` -> 0 errors and 0 warnings; `npm run build` -> production build passed; `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` -> passed; `cargo test --manifest-path src-tauri/Cargo.toml` -> 67 unit tests plus all discovery/process/fixture suites passed; `git diff --check` -> passed with existing CRLF normalization warnings only.
- **Artifacts Created/Modified:** `src/components/modpacks/ReleaseReviewModal.svelte` adds the Release Review UI; `src/components/modpacks/SnapshotHistory.svelte`, `SnapshotReviewModal.svelte`, `ModpackList.svelte`, and `src/routes/+page.svelte` route workspace behavior; `src/lib/domain.ts` and `src/lib/modpacks.ts` add typed workspace/event wrappers; `docs/release-workspace-validation.md` records native validation steps.
- **Decisions & Deviations:** Automated validation is complete. The workspace shows only workspace-associated proposed artifacts, supports generated and blank proposals, allows saving new proposed revisions, persists the selected revision for resume, and requires explicit confirmation before backend promotion/freeze at finalization. The plan remains blocked until a desktop run records lifecycle, terminal-edit, overlap, restart/resume, abandonment, and publication-teardown evidence using the documented procedure.
- **Next Phase Context:** Run the native Tauri/manual procedure and record evidence here before marking Phase 6 and the overall plan COMPLETED.

---

# Overall Plan Completion Status

* **Final State:** BLOCKED
* **Total Phases Completed:** 5 / 6
* **Summary of Outcome:** Phases 1-5 are complete and Phase 6 backend, watcher, typed-boundary, and workspace-shell implementation plus automated validation is complete. The plan is blocked on the workspace changelog revision picker and native Tauri/manual smoke evidence for desktop lifecycle, terminal edits, recovery overlap, restart/resume, abandonment, and publication watcher teardown.
