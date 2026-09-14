# Pre-Plan Impact Assessment: Application Data Import and Export

## Decision Summary

- **Overall disposition:** Decision-complete and ready for implementation planning.
- **Handoff status:** Retain this resolved pre-plan as the implementation-planning input; implementation has not started.
- **Confidence:** Medium-high.
- **Scope assessed:** Portable application-owned data exposed through the Management page.

The feature will use a versioned JSON data bundle with SHA-256 integrity metadata
and gzip-compressed bundles in the first release. Import will preview and merge
safe records rather than replace the local database. Stable record IDs are
authoritative; conflicting records remain untouched while unrelated records may
be imported atomically.

## Proposed Changes

### C1: Versioned application-data export

- **Intended outcome:** Let users save durable application state to a portable file.
- **In scope:** Settings, registered modpack metadata and observations, snapshots,
  decisions, notes, releases, release workspaces, operation/history records,
  changelog artifacts/revisions, and durable changelog export records.
- **Out of scope:** Packwiz project directories, Packwiz files, Git
  metadata/history, provider-response cache, and external files written to
  changelog export destinations.
- **Dependencies:** A stable export envelope, format-version compatibility rules,
  SHA-256 integrity metadata, native save-dialog support, and Rust-owned database
  access.

### C2: Previewed merge import

- **Intended outcome:** Import saved data without replacing the existing local
  database, while showing conflicts before any write.
- **In scope:** File selection, gzip/JSON detection, validation, integrity
  verification, compatibility checks, conflict classification, preview, and
  transactional merge.
- **Out of scope:** Automatic path relocation, copying external Packwiz
  projects, silent overwrite, or database migration/rebuild.
- **Dependencies:** C1's versioned envelope and deterministic record identity
  and conflict rules.

### C3: Modular Management-page UI

- **Intended outcome:** Add discoverable import/export controls to
  `src/routes/management/+page.svelte` without making the route a large
  orchestration component.
- **In scope:** A reusable data-transfer panel, status/loading/error states,
  export destination selection, import file selection, preview modal, conflict
  summaries, and confirmation.
- **Out of scope:** Reworking existing cleanup/lifecycle flows or placing
  database/merge logic in Svelte.
- **Dependencies:** Typed frontend wrappers and Rust commands from C1 and C2.

## Resolved Clarifications

- The bundle contains application-owned data only; it is not a Packwiz project
  backup.
- Import is previewed and merged rather than a database replacement.
- Durable changelog export records are included.
- Provider-response cache is excluded as disposable and potentially stale.
- External export destination files are not copied or modified.
- Same-ID records with differing contents are reported as conflicts and are not
  overwritten.
- Stable IDs are authoritative. A different-ID record with the same canonical
  modpack path is reported as a conflict rather than merged automatically.
- Non-conflicting records may be imported from a bundle containing conflicts.
- The non-conflicting portion is committed in one atomic transaction; conflicts
  remain untouched.
- The first release supports canonical UTF-8 JSON and gzip-compressed bundles.
- Imported settings that have supported operating-system effects, such as
  launch-at-login, apply those effects immediately. Failures must be reported
  explicitly rather than silently producing a partial success.

## Current-State Evidence

- `src/routes/management/+page.svelte` currently owns lifecycle previews,
  cleanup, storage reporting, loading, and error/status presentation. It has no
  import/export controls and is already a dense route-level coordinator.
- `src/lib/modpacks.ts` centralizes typed Tauri wrappers and uses
  `@tauri-apps/plugin-dialog` for directory selection and changelog export
  destinations. Data-transfer wrappers should follow this boundary.
- `src/lib/settings.ts` exposes settings through typed
  `loadSettings`, `saveSetting`, and `resetStoredSettings` helpers.
- `src-tauri/src/db/schema.sql` contains a broad relational graph covering
  settings, modpacks, observations, snapshots, candidates, operations,
  changelogs, releases, release workspaces, lifecycle records, and cleanup
  records.
- `src-tauri/src/db/mod.rs` owns SQLite access, transaction handling, JSON
  serialization, and typed command errors. Existing durable evidence is stored
  as JSON inside relational records.
- `src-tauri/src/lib.rs` centrally registers Tauri commands.
- `src-tauri/src/db/lifecycle.rs` protects registered Packwiz directories, Git
  metadata/history, and user export destination files.
- `README.md` documents that the application database contains settings,
  registered modpacks, observations, snapshots, operations, release workspaces,
  releases, changelog artifacts, provider cache, export observations, and
  management state. It also states that no in-app backup workflow currently
  exists.
- `docs/CM-MODPACK-UTIL-SPEC.md` establishes that Packwiz files remain the
  external source of truth and that the alpha database has no migration
  workflow.
- `src-tauri/Cargo.toml` already includes `serde`, `serde_json`, and `sha2`.
  Gzip support will require either an approved dependency or an existing
  platform/library approach; no compression dependency is currently present.

## Impact Findings

### C1: Versioned application-data export

- **Classification:** Beneficial with conditions.
- **Positive impact:** Provides a portable recovery and transfer mechanism
  without exposing or duplicating Packwiz files. A typed envelope can remain
  stable across internal SQLite schema changes and can clearly identify omitted
  disposable data.
- **Negative impact or unintended consequence:** Raw SQLite export would tightly
  couple the file format to the current schema, foreign-key order, and alpha
  reset policy. Exported absolute paths may be invalid on another machine.
- **Affected surfaces:** Rust export command/module, domain contracts, frontend
  invocation helper, native save dialog, Management UI, documentation, and
  tests.
- **Dependencies and interactions:** The format version must be independent of
  the SQLite schema version. Stable IDs and relationships must be preserved.
  External destination files must not be followed or copied.
- **Discriminating check:** Export a fixture database containing every included
  record family and verify that provider cache and external filesystem content
  are absent.
- **Recommendation:** Proceed with a typed JSON envelope wrapped in gzip for
  the first release. Define one canonical uncompressed JSON representation so
  hashing and compatibility checks are deterministic.

### C2: Previewed merge import

- **Classification:** Beneficial with conditions.
- **Positive impact:** Safer than replacing the local SQLite database and
  compatible with preserving existing records. Preview can identify additions,
  identical records, conflicts, unavailable paths, unsupported versions, and
  integrity failures before mutation.
- **Negative impact or unintended consequence:** The relational graph is
  substantial. A naïve row-by-row merge could violate foreign keys, duplicate
  history, or leave partial state.
- **Affected surfaces:** Rust import/preview commands, conflict/result domain
  types, transaction logic, database tests, frontend preview UI, and command
  error translation.
- **Dependencies and interactions:** Stable IDs are authoritative. Same-ID
  content differences and different-ID same-path modpacks are conflicts.
  Unavailable paths remain registered as disconnected; paths are never rewritten
  automatically.
- **Discriminating check:** Import a fixture bundle into an empty database and
  into databases containing identical, modified, and colliding records. Verify
  deterministic previews and unchanged state after cancellation or rejection.
- **Recommendation:** Proceed with a preview/dry-run command and one atomic
  transaction for all selected non-conflicting records. Never silently
  overwrite conflicts.

### C3: Modular Management-page UI

- **Classification:** Beneficial with conditions.
- **Positive impact:** The Management page is the existing home for
  application-owned data operations, so users will find the workflow there. A
  dedicated data-transfer component keeps file handling and conflict rendering
  out of the route.
- **Negative impact or unintended consequence:** Import has many states:
  invalid file, unsupported version, integrity failure, no-op merge, conflicts,
  unavailable paths, partial preview, OS-effect failure, and successful commit.
  Directly adding all orchestration to the route would increase complexity.
- **Affected surfaces:** `src/routes/management/+page.svelte`, a new reusable
  management component, a typed data-transfer frontend module, existing
  `Modal`/`Button` primitives, and accessibility/status styling.
- **Dependencies and interactions:** Svelte invokes typed Rust wrappers only.
  The backend supplies conflict classifications; the frontend does not
  duplicate merge semantics. Successful import reloads affected management
  summaries without changing cleanup/lifecycle behavior.
- **Discriminating check:** Exercise keyboard navigation and visible status
  announcements through export, invalid import, conflict preview, cancellation,
  OS-effect failure, and successful merge flows. Run `npm run check` and
  `npm run build`.
- **Recommendation:** Proceed with a dedicated data-transfer panel and
  frontend invocation module. Keep the route responsible for composition and
  reloads.

## Decision Register

| ID | Decision | Selected option | Why it matters |
| --- | --- | --- | --- |
| `d1-conflict-policy` | Same-ID records with different contents | Block the conflicting record and report it; do not overwrite | Prevents silent data loss |
| `d2-cross-install-identity` | Different-ID records with the same canonical path | Stable ID is authoritative; report a conflict | Avoids unsafe cross-installation merges |
| `d3-partial-merge` | Bundle contains both conflicts and safe records | Import safe records atomically and leave conflicts untouched | Preserves useful partial recovery without partial writes |
| `d4-packaging` | First-release bundle packaging | Canonical JSON plus gzip compression | Supports portability and smaller files while retaining a canonical payload |
| `d5-os-settings` | Settings with OS-integrated effects | Import preferences and apply supported effects immediately; report failures | Prevents settings/database divergence |

## Cross-Change Considerations

- C1 must define the envelope before C2 can define compatibility and preview
  behavior.
- C2 should stabilize preview/result contracts before C3's final interaction
  design.
- Rust owns database reads, hashing, gzip/JSON validation, conflict
  classification, OS-effect coordination, and transactions.
- The bundle format must remain independent of the SQLite schema because the
  repository intentionally has no migration system.
- Foreign-key ordering and stable IDs are central; records should be grouped by
  entity family rather than exposed as arbitrary SQL tables.
- Import must never modify Packwiz directories or reconnect paths automatically.
- Provider cache remains excluded and should be documented as disposable local
  state.
- External changelog destination files remain untouched. Stored export records
  can be restored, but destination availability must be reported.
- Existing lifecycle protections and cleanup behavior must remain intact after
  import.
- Documentation should update the current statement that no in-app backup
  workflow exists once implementation is complete.

## Implementation-Ready Checklist

- [ ] Define the bundle envelope version, canonical JSON serialization, gzip
  detection, and SHA-256 coverage.
- [ ] Enumerate included entity families and explicitly exclude provider cache
  and external filesystem content.
- [ ] Define stable-ID comparison and different-ID same-path conflict records.
- [ ] Implement preview as read-only and import as one transaction for the
  selected non-conflicting records.
- [ ] Preserve unavailable modpack paths as disconnected records.
- [ ] Apply supported OS-integrated settings effects and surface failures.
- [ ] Add typed Rust domain contracts and frontend invocation wrappers.
- [ ] Add a modular data-transfer component to the Management page.
- [ ] Preserve loading, empty, unavailable, stale, validation-failure, and
  command-failure states.
- [ ] Add Rust fixture tests for export completeness, gzip/JSON validation,
  integrity failures, conflicts, foreign-key ordering, atomicity, and
  cancellation.
- [ ] Add frontend checks for preview, confirmation, errors, accessibility, and
  management reload behavior.
- [ ] Update README and related documentation with bundle scope and reset
  boundaries.

## Remaining Risks and Assumptions

- Gzip support adds a dependency or implementation surface that should be
  reviewed during planning; the canonical JSON payload remains the compatibility
  source of truth.
- The plan assumes stable IDs are globally meaningful within exported records.
  If future installations regenerate IDs, the bundle must report conflicts
  rather than infer identity from paths.
- OS-integrated settings may fail independently of database import. The product
  must report this explicitly and define whether the transaction is considered
  committed with a follow-up remediation state.
- The exact entity insertion order and handling of historical tombstones should
  be finalized in the implementation plan from the schema's foreign-key graph.

## Handoff Options

1. **Proceed to implementation planning:** Use this file as the source of truth
   for the implementation plan and preserve all resolved decisions.
2. **Return to planning for another revision:** Revisit gzip packaging,
   historical tombstone inclusion, or OS-effect failure semantics.
3. **Pause:** Retain this resolved decision register without implementation.

## Quality Gate

- Every requested change is identified and mapped to an outcome.
- Material scope, conflict, packaging, and OS-integration decisions are resolved.
- Current-state claims are tied to repository files, symbols, schema, and
  documented boundaries.
- Benefits, risks, dependencies, and interactions are explicit.
- Each recommendation includes a discriminating validation check.
- The implementation checklist names required contracts, UI boundaries, tests,
  and documentation updates.
- No application code was modified as part of this pre-planning work.
