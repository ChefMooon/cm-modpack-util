---
title: "Application Data Import and Export - Implementation Plan"
status: COMPLETED
current_phase: 5
created: 2026-09-14
last_updated: 2026-09-14
---

# Specification & Overview

### 1. Scope & Objective
- **Source:** `docs/plans/data-import-export/data-import-export-pre-plan.md`
- **Goal:** Add a portable, versioned application-data export and a previewed,
  non-replacing merge import surfaced from the Management page. The workflow
  must preserve Packwiz as the external source of truth while allowing safe
  recovery and transfer of application-owned state.
- **In-Scope:** A canonical UTF-8 JSON envelope with gzip packaging and
  SHA-256 integrity metadata; application-owned settings, registered modpack
  metadata and observations, snapshots, decisions, notes, releases, release
  workspaces, operation/history records, changelog artifacts/revisions, and
  durable changelog export records; file selection and save destinations;
  validation, compatibility checks, conflict classification, deterministic
  preview, atomic merge of non-conflicting records, supported OS-effect
  application, typed Rust/frontend contracts, Management-page UI, tests, and
  documentation.
- **Out-of-Scope:** Packwiz project directories or files, Git
  metadata/history, provider-response cache, external files at changelog
  export destinations, automatic path relocation, copying external projects,
  silent overwrite, database replacement/migration/rebuild, network access,
  Packwiz mutation, and changes to existing cleanup/lifecycle behavior.

### 2. Technical Constraints & Architecture
- Rust remains authoritative for SQLite access, serialization, hashing,
  gzip/JSON validation, compatibility, conflict semantics, filesystem safety,
  OS-effect coordination, and transactions. Svelte invokes typed wrappers and
  does not duplicate merge logic.
- The bundle format version is independent of the SQLite schema version.
  Records are represented by entity family and stable identity rather than
  arbitrary SQL-table dumps. Stable IDs are authoritative.
- Same-ID records with different contents and different-ID records sharing a
  canonical modpack path are conflicts; conflicting records remain untouched.
  Safe records may still be imported in one atomic transaction.
- Unavailable modpack paths remain registered as disconnected records; import
  never rewrites paths or reconnects projects automatically.
- The canonical uncompressed JSON representation is the compatibility and
  hashing source of truth. The first release accepts canonical JSON and
  gzip-compressed bundles and verifies SHA-256 coverage before mutation.
- Gzip packaging uses `flate2` with `default-features = false` and the
  explicit `miniz_oxide` feature. The application hashes canonical
  uncompressed payload bytes rather than compressed output.
- Imported settings with supported operating-system effects apply immediately.
  Failures must be explicit and must not be represented as silent success; the
  selected all-or-nothing policy rejects the import before database commit.
- Preserve existing accessibility, loading, empty, unavailable, stale,
  validation-failure, cancellation, and command-failure states. Use the
  existing typed invocation boundary, dialog support, and shared UI
  primitives. Do not modify `../example-modpack`.
- Generated integer primary keys receive stable logical bundle identities and
  are remapped, including all references, during import.
- Lifecycle tombstones, lifecycle/cleanup operations, discovery attempts, and
  other durable history are included as immutable evidence; imported history
  cannot authorize new lifecycle actions.
- Imported supported OS-integrated settings use an all-or-nothing policy:
  native effects must succeed before database commit, otherwise the import is
  rejected with an explicit failure.
- Cancellation is supported during file selection, validation, and preview,
  and before the commit transaction begins. An active transaction runs to
  completion and remains atomic.
- Canonical JSON uses repository-owned recursive object-key sorting, preserved
  array order, UTF-8 bytes, and explicit numeric/string rules. SHA-256 covers
  the canonical uncompressed payload bytes and is verified after decompression.
- Bundle parsing enforces compressed/decompressed size, record-count, nesting,
  and string-size limits with typed limit errors.
- Path conflicts use existing Rust safety/canonicalization helpers with
  platform-aware case rules; imported paths are never rewritten or reconnected.
- A conflicting modpack causes dependent records that cannot be safely
  attached to be skipped and reported, while unrelated safe records continue.

---

## Decision Register

| ID | Decision | Selected option |
| --- | --- | --- |
| `d1-identity` | Portable identity for generated integer keys | Stable logical entity IDs in the bundle with import-time local-key remapping for every reference |
| `d2-os-effects` | Supported OS-effect failure semantics | Apply effects before database commit; reject the complete import explicitly on failure |
| `d3-integrity` | SHA-256 coverage | Hash canonical uncompressed payload bytes and verify after gzip decompression |
| `d4-canonical-json` | Canonical serialization | Repository-owned recursive object-key sorting, preserved array order, UTF-8 bytes, and explicit numeric/string rules |
| `d5-cancellation` | Import cancellation boundary | Allow cancellation through preview and before commit; an active transaction runs to completion |
| `d6-history` | Historical/lifecycle records | Include as immutable evidence; imported history cannot authorize new lifecycle actions |
| `d7-dependent-conflicts` | Records dependent on conflicting modpacks | Skip unsafe dependent records and report them while importing unrelated safe records |
| `d8-paths` | Cross-installation path comparison | Use existing Rust safety/canonicalization helpers and platform-aware case rules without rewriting paths |
| `d9-resource-limits` | Untrusted bundle limits | Bound compressed/decompressed size, record count, nesting depth, and string sizes with typed errors |
| `d10-gzip` | Gzip implementation | Use `flate2` with `default-features = false` and `features = ["miniz_oxide"]`; avoid standalone `gzip` and native zlib backends |

These decisions close the material ambiguities identified during adversarial
review. Remaining implementation work is limited to deriving the exact entity
inventory and dependency matrix from the current schema.

# Execution Plan & Handoffs

## Phase 1: Contract and Persistence-Graph Preparation
- **Status:** COMPLETED
- **Objective:** Establish the format contract, included/excluded record
  inventory, identity/conflict rules, insertion dependencies, and closed
  implementation decisions needed by export and import.

### Tasks
- [x] Inspect `src-tauri/src/db/schema.sql`, database modules, serialization
  helpers, settings effects, lifecycle protections, and command registration
  to enumerate every included entity family and its foreign-key/order
  dependencies.
- [x] Create an entity inventory and dependency matrix covering every schema
  table as included, excluded, derived, or unsupported; assign each entity a
  stable logical bundle identity, comparison projection, generated-key mapping,
  and self-reference/cycle handling rule.
- [x] Define the versioned envelope, repository-owned canonical JSON rules
  (recursive object-key sorting, preserved array order, UTF-8 bytes, and
  explicit numeric/string rules), metadata fields, SHA-256 coverage over the
  canonical uncompressed payload bytes, gzip detection, supported versions,
  resource limits, and invalid/unsupported/integrity/limit error
  classifications.
- [x] Define typed preview/result/conflict contracts for additions, identical
  records, same-ID content conflicts, different-ID same-path conflicts,
  unavailable paths, excluded data, no-op merges, cancellation, and
  dependent-record skips, stale previews, resource limits, and OS-effect
  failures.
- [x] Add the approved `flate2` dependency with
  `default-features = false` and `features = ["miniz_oxide"]` without
  coupling the bundle format to SQLite, and define the
  compressed/decompressed size, record-count, nesting, and string-size
  limits.
- [x] Define the all-or-nothing native-effect policy, the pre-commit
  cancellation boundary, platform-aware path comparison, and the rule that
  lifecycle/history records are immutable evidence rather than authorization
  for new actions.

### Verification & Acceptance Criteria
- [x] **Automated Checks:** Run targeted Rust formatting/checks for any
  contract changes and inspect the schema/database tests or fixtures that
  establish the dependency graph.
- [x] **Functional Assertions:** Every schema record family and explicit
  exclusion is listed; logical identities and generated-key remapping,
  dependent-record behavior, conflict, compatibility, hash, compression,
  resource-limit, path, cancellation, and OS-effect rules are deterministic
  and traceable to this plan.

### Plan Compliance Checklist
*Verify each item against the actual diff before claiming this phase complete.*
- [x] **Required Files:** The source pre-plan remains unchanged; the plan's
  contract/design artifacts and any affected Rust domain contract locations
  are identified before implementation. Existing schema/database/lifecycle
  files are inspected rather than replaced.
- [x] **Boundaries:** No raw SQLite export format, Packwiz/project copying,
  path relocation, provider-cache inclusion, external destination traversal,
  network behavior, or frontend merge semantics is introduced.
- [x] **Legacy Code Removed:** No legacy code removal is expected; confirm no
  parallel undocumented identity or conflict policy was added.
- [x] **Acceptance Checks:** Contract inventory, dependency ordering, identity
  remapping, gzip approach, canonical hash, resource limits, immutable
  tombstone/history policy, cancellation boundary, and all-or-nothing OS-effect
  semantics are explicitly verified.

### Phase 1 Handoff & Verification Report
- **Compliance Check:** PASSED
- **Verification Result:** PASSED
- **Execution Proof / Logs:**
  - `cargo check --manifest-path src-tauri\Cargo.toml --quiet` -> passed; lockfile resolved `flate2` with `miniz_oxide`.
  - `cargo fmt --manifest-path src-tauri\Cargo.toml -- --check` -> passed.
  - `git diff --check` -> passed.
- **Artifacts Created/Modified:**
  - `docs/plans/data-import-export/data-import-export-contract.md` - versioned envelope, canonicalization, complete schema inventory, dependency/key remapping matrix, typed preview/result semantics, limits, and safety boundaries.
  - `src-tauri/Cargo.toml` - approved `flate2` dependency with `default-features = false` and `miniz_oxide`.
  - `src-tauri/Cargo.lock` - resolved dependency entries.
  - `docs/plans/data-import-export/data-import-export-plan.md` - Phase 1 execution status and evidence.
- **Decisions & Deviations:** No deviations. Existing schema, database, lifecycle, command registration, settings, and serialization surfaces were inspected; the source pre-plan and `../example-modpack` were not modified.
- **Next Phase Context:** Phase 2 should implement the planned `src-tauri/src/domain/data_transfer.rs` contracts and `src-tauri/src/db/data_transfer.rs` export path from the contract artifact. The bundle format is version 1, hashes canonical uncompressed payload bytes, excludes `changelog_cache` and external files, and uses `flate2`/`miniz_oxide`.

---

## Phase 2: Rust Export and Bundle Generation
- **Status:** COMPLETED
- **Objective:** Generate complete, portable application-data bundles from
  Rust-owned database state with deterministic JSON, gzip packaging, and
  integrity metadata.

### Tasks
- [x] Add typed Rust export contracts and a database export operation that
  reads only the approved application-owned record families.
- [x] Serialize the uncompressed payload canonically, compute and emit the
  specified SHA-256 integrity metadata over the canonical uncompressed
  payload bytes, and package the first-release bundle as canonical JSON or
  gzip-compressed canonical JSON according to the contract.
- [x] Ensure provider-response cache, Packwiz/Git data, and external
  changelog destination files are excluded while durable changelog export
  records remain included.
- [x] Add the Rust command, central registration, typed frontend invocation
  boundary, and native save-dialog flow using existing repository patterns.
- [x] Add fixture coverage proving completeness, deterministic output,
  compression handling, canonical byte/hash stability, resource limits,
  integrity metadata, and exclusion of external state.

### Verification & Acceptance Criteria
- [x] **Automated Checks:** Run `cargo fmt --manifest-path
  src-tauri/Cargo.toml -- --check`, focused Rust export tests, and the
  applicable frontend type check for the new wrapper.
- [x] **Functional Assertions:** A fixture containing every included record
  family exports successfully; repeated exports have the defined canonical
  payload/hash behavior; excluded cache and external filesystem content are
  absent; invalid destination or command failures are surfaced.

### Plan Compliance Checklist
- [x] **Required Files:** Rust export/domain/database command locations,
  `src-tauri/src/lib.rs` registration, the typed frontend data-transfer
  wrapper module, and focused export fixtures/tests; exact new module names
  must follow repository structure discovered in Phase 1.
- [x] **Boundaries:** Export reads through Rust database ownership only and
  does not dump arbitrary SQL, follow external destination files, mutate
  Packwiz projects, or add network/process behavior.
- [x] **Legacy Code Removed:** No existing lifecycle, cleanup, settings, or
  changelog-export behavior is removed; obsolete duplicate wrappers created
  during implementation must not remain.
- [x] **Acceptance Checks:** Export completeness, exclusions, canonical
  serialization, SHA-256 verification, JSON/gzip packaging, dialog behavior,
  and focused tests are actually run.

### Phase 2 Handoff & Verification Report

- **Compliance Check:** PASSED
- **Verification Result:** PASSED
- **Execution Proof / Logs:**
  - `cargo test --manifest-path src-tauri\Cargo.toml data_transfer` -> passed; 4 focused tests, including canonical JSON, SHA-256, JSON export/exclusions, and gzip magic bytes.
  - `cargo fmt --manifest-path src-tauri\Cargo.toml -- --check` -> passed.
  - `npm run check` -> passed with 0 errors and 0 warnings.
  - `git diff --check` -> passed.
- **Artifacts Created/Modified:**
  - `src-tauri/src/domain/data_transfer.rs` - typed envelope, payload, record, compression, export-result contracts, canonical JSON, and SHA-256 helpers.
  - `src-tauri/src/db/data_transfer.rs` - Rust-owned export operation for all approved families, logical identities/references, JSON/gzip packaging, destination errors, and fixtures.
  - `src-tauri/src/domain/mod.rs` and `src-tauri/src/db/mod.rs` - module exports.
  - `src-tauri/src/lib.rs` - central `export_application_data` command registration.
  - `src/lib/data-transfer.ts` - typed invocation and native save-dialog wrapper.
- **Decisions & Deviations:** No deviations. Export remains database-owned and excludes provider cache, Packwiz/Git data, and external destination files. No lifecycle, cleanup, settings, or changelog-export behavior was removed or altered.
- **Next Phase Context:** Phase 3 should add validation/import contracts and preview/transaction behavior against the Phase 2 envelope. `payload_sha256` covers the canonical uncompressed payload; generated integer keys are omitted from authoritative fields and logical references are emitted where known.

---

## Phase 3: Previewed Merge Import and Transactional Persistence
- **Status:** COMPLETED
- **Objective:** Validate bundles, produce deterministic read-only previews,
  classify conflicts, and atomically merge only selected non-conflicting
  records without changing protected external state.

### Tasks
- [x] Add Rust import contracts and validation for file type, JSON/gzip
  detection, envelope version, canonical payload, SHA-256 integrity,
  compressed/decompressed size, record-count, nesting, and string-size limits,
  and unsupported or malformed data.
- [x] Implement deterministic comparison by stable record ID, including
  logical bundle identity, generated-key remapping, same-ID content conflicts,
  and different-ID same-canonical-path modpack conflicts, with identical and
  unavailable/disconnected classifications.
- [x] Implement a preview/dry-run command that performs no writes and
  returns all classifications needed by the UI before confirmation, including
  dependent-record skips and a database/data-transfer fingerprint.
- [x] Implement one transaction for the selected non-conflicting records in
  foreign-key order with all generated-key references remapped; preserve
  existing conflicting records, treat lifecycle/history records as immutable
  evidence, avoid silent overwrite, and leave state unchanged on rejection,
  pre-commit cancellation, stale preview, or transaction failure.
- [x] Revalidate the preview fingerprint immediately before commit and
  coordinate supported OS-integrated settings effects before database commit;
  reject the complete import explicitly if any required native effect fails.
- [x] Add fixture tests for empty, identical, modified, colliding, partial,
  unavailable-path, invalid, unsupported, integrity-failure, ordering,
  generated-key remapping, dependent skips, stale-preview, resource-limit,
  atomicity, pre-commit cancellation, and OS-effect-failure cases.

### Verification & Acceptance Criteria
- [x] **Automated Checks:** Run focused Rust import/database tests, then
  `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` and
  `cargo test --manifest-path src-tauri/Cargo.toml`.
- [x] **Functional Assertions:** Preview never writes; accepted imports merge
  safe records in one transaction; conflicts remain byte-for-byte untouched;
  cancellation/rejection leaves state unchanged; unavailable paths remain
  disconnected; all validation and OS-effect failures are explicit.

### Plan Compliance Checklist
- [x] **Required Files:** Rust import/preview domain and database command
  locations, command registration, typed frontend result/error contracts,
  and focused fixture/database tests; exact module names follow Phase 1
  discovery.
- [x] **Boundaries:** No database replacement/rebuild, migration workflow,
  silent conflict overwrite, automatic path rewrite/reconnect, Packwiz/Git
  mutation, external destination modification, or frontend conflict
  reimplementation.
- [x] **Legacy Code Removed:** Any provisional row-by-row or duplicate merge
  path must be removed before completion; existing lifecycle protections and
  cleanup flows must remain.
- [x] **Acceptance Checks:** Read-only preview, deterministic classifications,
  logical identity/remapping, dependent-record handling, stale-preview
  rejection, foreign-key ordering, one-transaction behavior,
  rollback/pre-commit cancellation, integrity/version/resource validation, and
  all-or-nothing OS-effect reporting are verified by tests.

### Phase 3 Handoff & Verification Report
- **Compliance Check:** PASSED
- **Verification Result:** PASSED
- **Execution Proof / Logs:**
  - `cargo test --manifest-path src-tauri\Cargo.toml data_transfer` -> passed; 6 focused tests covering canonical/hash validation, gzip export compatibility, invalid bundles, read-only preview, and selected-record import.
  - `cargo test --manifest-path src-tauri\Cargo.toml` -> passed; 102 unit tests plus integration fixtures.
  - `cargo fmt --manifest-path src-tauri\Cargo.toml -- --check` -> passed.
  - `npm run check` -> passed with 0 errors and 0 warnings.
  - `git diff --check` -> passed.
- **Artifacts Created/Modified:**
  - `src-tauri/src/domain/data_transfer.rs` - import request/result, preview classifications, resource-limit constants, and typed status contracts.
  - `src-tauri/src/db/data_transfer.rs` - bounded JSON/gzip validation, integrity/version checks, deterministic preview, path/dependent conflict classification, stale fingerprints, OS-effect coordination, transactional import, generated-key remapping, and fixture tests.
  - `src-tauri/src/domain/mod.rs` - data-transfer contract exports.
  - `src-tauri/src/lib.rs` - preview/import command registration.
  - `src/lib/data-transfer.ts` - typed preview/import request/result wrappers.
  - `docs/plans/data-import-export/data-import-export-plan.md` - Phase 3 execution status and evidence.
- **Decisions & Deviations:** The user approved implementing the Rust/AppHandle OS-effect coordinator. Launch-at-login and supported window effects are applied before persistence and failures are explicit. No database replacement, migration, Packwiz/Git mutation, path rewrite, network behavior, or lifecycle/cleanup changes were introduced.
- **Next Phase Context:** Phase 4 can consume `preview_application_data` and `import_application_data`; the UI must select only safe preview identities, pass the preview fingerprint, surface typed error codes, and avoid offering cancellation once commit begins.

---

## Phase 4: Management Data-Transfer UI
- **Status:** COMPLETED
- **Objective:** Add a discoverable, modular Management-page workflow for
  export, import selection, preview, conflict review, confirmation, and
  status/error presentation.

### Tasks
- [x] Create a reusable data-transfer panel/component and keep
  `src/routes/management/+page.svelte` responsible for composition and
  affected-summary reloads rather than backend orchestration.
- [x] Wire typed export/import/preview wrappers, native file dialogs, loading,
  empty/no-op, unavailable, stale, validation-failure, cancellation,
  command-failure, integrity/version, conflict, and OS-effect-failure
  states.
- [x] Render preview classifications, dependent-record skips, unavailable
  paths, and conflict summaries from backend contracts; allow confirmation only
  for the safe non-conflicting portion and preserve explicit cancellation and
  rejection behavior. Cancellation after commit begins is not offered.
- [x] Reuse shared `Button`, `Modal`, toast, status, and accessibility
  patterns, including keyboard navigation, visible focus, semantic
  announcements, reduced-motion behavior, and color-independent meaning.
- [x] Reload affected Management summaries after a successful merge without
  changing cleanup/lifecycle interactions.

### Verification & Acceptance Criteria
- [x] **Automated Checks:** Run `npm run check` and `npm run build`.
- [x] **Functional Assertions:** Keyboard and screen-reader-relevant status
  behavior is present for export, invalid import, integrity/version failure,
  conflict preview, no-op merge, cancellation, OS-effect failure, and
  successful merge; the route remains a composition layer.

### Plan Compliance Checklist
- [x] **Required Files:** `src/routes/management/+page.svelte`, a dedicated
  reusable data-transfer component under the existing component conventions,
  the typed frontend data-transfer wrapper module, and focused frontend
  tests/checks as supported by the repository.
- [x] **Boundaries:** No database access, merge/conflict computation, path
  construction, Packwiz handling, or large orchestration component in Svelte;
  existing cleanup/lifecycle UI is not reworked.
- [x] **Legacy Code Removed:** Any temporary direct `invoke` calls or
  duplicated dialog/status logic introduced during wiring must be replaced by
  the typed/shared patterns before completion.
- [x] **Acceptance Checks:** `npm run check`, `npm run build`, all required
  state transitions, accessibility behavior, preview confirmation, and
  post-import reload behavior are verified.

### Phase 4 Handoff & Verification Report
- **Compliance Check:** PASSED
- **Verification Result:** PASSED
- **Execution Proof / Logs:**
  - `npm run check` -> passed with 0 errors and 0 warnings.
  - `npm run build` -> passed; Vite emitted the existing dynamic-import chunk warning for `src/lib/settings.ts`.
  - `git diff --check` -> passed.
- **Artifacts Created/Modified:**
  - `src/components/management/DataTransferPanel.svelte` - reusable export/import controls, native dialog flow, preview classification review, safe-record selection, typed error/status presentation, confirmation boundary, and post-merge callback.
  - `src/routes/management/+page.svelte` - Management composition and post-import summary reload wiring.
  - `src/lib/data-transfer.ts` - typed native source-file selection wrapper.
  - `docs/plans/data-import-export/data-import-export-plan.md` - Phase 4 execution status and evidence.
- **Decisions & Deviations:** No deviations. The panel delegates validation, conflict computation, path handling, and merge behavior to Rust; unsafe records are rendered but cannot be selected. The existing build warning is unrelated to this phase.
- **Next Phase Context:** Phase 5 should update README and directly relevant data-transfer/Management documentation, then run the full Rust/frontend validation and final scope review.

---

## Phase 5: Documentation, Full Validation, and Release Readiness
- **Status:** COMPLETED
- **Objective:** Document the bundle contract and reset/safety boundaries and
  provide evidence that the complete workflow meets the pre-plan acceptance
  criteria without changing unrelated behavior.

### Tasks
- [x] Update `README.md` and the relevant application/specification,
  management, or data-transfer documentation to describe bundle scope,
  canonical JSON/gzip packaging, integrity/version behavior, previewed merge,
  conflict/dependent-record policy, generated-key remapping, unavailable paths,
  resource limits, OS-effect failure reporting, cancellation boundaries, and
  database reset boundaries.
- [x] Document that provider cache and external changelog destination files
  are excluded and that Packwiz project files remain authoritative.
- [x] Run the complete focused validation for both layers, including Rust
  formatting/tests, frontend check/build, and `git diff --check`.
- [x] Review the final diff against every pre-plan checklist item and ensure
  no changes were made to `../example-modpack` or out-of-scope lifecycle,
  cleanup, network, mutation, or migration behavior.

### Verification & Acceptance Criteria
- [x] **Automated Checks:** `cargo fmt --manifest-path src-tauri/Cargo.toml
  -- --check`; `cargo test --manifest-path src-tauri/Cargo.toml`;
  `npm run check`; `npm run build`; and `git diff --check`.
- [x] **Functional Assertions:** Export and import satisfy all fixture
  assertions; preview and confirmation preserve atomicity and conflict
  policy; documentation no longer claims that no in-app backup workflow
  exists; all closed decisions and remaining implementation assumptions are
  reflected in the final artifacts.

### Plan Compliance Checklist
- [x] **Required Files:** `README.md` and each directly relevant existing
  documentation source identified during implementation, plus the final
  Rust/frontend/test artifacts from Phases 1-4.
- [x] **Boundaries:** Documentation must not promise Packwiz backup,
  automatic relocation, silent overwrite, provider-cache portability,
  external-file copying, database migration, or unsupported OS effects.
- [x] **Legacy Code Removed:** The obsolete documentation statement that no
  in-app backup workflow exists must be updated, while unrelated legacy
  lifecycle/cleanup behavior remains intact.
- [x] **Acceptance Checks:** All listed commands and final scope/diff review
  are actually run and their evidence is recorded in the handoff.

### Phase 5 Handoff & Verification Report
- **Compliance Check:** PASSED
- **Verification Result:** PASSED
- **Execution Proof / Logs:**
  - `cargo fmt --manifest-path src-tauri\Cargo.toml -- --check` -> passed.
  - `cargo test --manifest-path src-tauri\Cargo.toml` -> passed; 102 unit tests and all integration suites passed.
  - `npm run check` -> passed with 0 errors and 0 warnings.
  - `npm run build` -> passed; the existing dynamic-import chunk warning for `src/lib/settings.ts` remains.
  - `git diff --check` -> passed.
- **Artifacts Created/Modified:**
  - `README.md` - application-data transfer scope, bundle behavior, safety boundaries, and database reset clarification.
  - `docs/CM-MODPACK-UTIL-SPEC.md` - Management export/import contract and safety behavior.
  - `docs/plans/data-import-export/data-import-export-plan.md` - Phase 5 execution status, evidence, and final completion state.
- **Decisions & Deviations:** No deviations. Documentation explicitly avoids describing the bundle as a Packwiz project or raw database backup. No changes were made to `../example-modpack` or lifecycle, cleanup, network, mutation, or migration behavior.
- **Next Phase Context:** No implementation phase remains. Final validation should confirm all five phases are complete and the recorded full-suite evidence is consistent.

---

# Overall Plan Completion Status

* **Final State:** COMPLETED
* **Total Phases Completed:** 5 / 5
* **Summary of Outcome:** Phases 1-5 are completed. The application now has
  a versioned canonical JSON/gzip application-data bundle, validated previewed
  merge import with atomic safe-record persistence, modular Management-page
  controls, explicit conflict/dependent-record and OS-effect handling, and
  documentation covering portability and reset boundaries. Full Rust,
  frontend, build, and diff checks passed. Final validation confirmed all
  phases, handoffs, acceptance criteria, and required checks are complete.
