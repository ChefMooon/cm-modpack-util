---
title: "Separate Production and Development Databases"
status: COMPLETED
current_phase: 3 / 3
created: 2026-09-23
last_updated: 2026-09-23
---

# Overall Plan Completion Status

* **Final State:** COMPLETED
* **Total Phases Completed:** 3 / 3
* **Summary of Outcome:** Debug-assertion builds use an isolated development database; release builds retain the existing production database. Paths and reset safety are documented, and all planned checks passed.
* **Final Validation:** `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` passed; `cargo test --manifest-path src-tauri/Cargo.toml` passed (127 tests); `npm run tauri build` produced the Windows NSIS bundle and updater signature; `git diff --check` passed. Git reported an informational CRLF-to-LF normalization warning for `docs/data-management.md`.

# Specification & Overview

### 1. Scope & Objective
- **Goal:** Prevent development runs from reading or changing the production application database, so development work and testing cannot affect production application data.
- **In-Scope:** Route development desktop runs to a distinct SQLite database location; preserve the existing database location for production builds; create the development database through the existing schema initialization behavior; document both locations and safe reset boundaries; add focused Rust coverage for the path policy.
- **Out-of-Scope:** Database schema changes or migrations, automatic copying/import of production data, changes to user-facing data transfer, Packwiz project files, frontend changes, and broader environment/profile management.

### 2. Technical Constraints & Architecture
- `src-tauri/src/lib.rs` currently obtains Tauri's application data directory and opens `cm-modpack-util.sqlite` there for application startup. The Tauri identifier in `src-tauri/tauri.conf.json` is `com.cm.modpackutil`; the `npm run tauri dev` and `npm run tauri build` commands are documented in `README.md`.
- `src-tauri/src/db/mod.rs::initialize` initializes an empty database from `schema.sql`, rejects unsupported existing schemas, and does not migrate or replace existing data. Keep this common database initializer and schema policy unchanged.
- **User decision:** Keep the current shared database location and its contents as production data. Development starts with a separate, newly initialized database; do not copy the existing database.
- **Planning assumption:** Production must continue using the current `app_data_dir/cm-modpack-util.sqlite` location so existing installation data remains in place. Development gets a dedicated location that cannot resolve to that same database or its SQLite sidecars.
- Rust remains responsible for directory creation, path selection, database opening, and startup errors. Path selection must not fall back silently to the production database if development storage cannot be prepared.
- **Risks:** Incorrect debug/release classification could send a production run to development storage or expose production data to development. A reset instruction that omits the development-specific path or SQLite WAL/SHM files could also mislead developers. Address these with focused path-policy tests and explicit documentation.
- **Dependencies:** The development-versus-production build boundary supported by the current Tauri setup must be confirmed while implementing the path policy. If other build profiles are supported, establish their mapping before changing startup behavior.
- **Open decisions:** None. The existing database remains production data and development starts fresh, as selected by the user.

---

# Execution Plan & Handoffs

## Phase 1: Isolate the Development Database Path
- **Status:** COMPLETED
- **Objective:** Make development desktop startup open a distinct application-owned SQLite database while production continues opening the existing database location.

### Tasks
- [x] In `src-tauri/src/lib.rs`, confirm the reliable Tauri/Rust build-mode signal for the documented development and production commands, then use it to select separate database paths. Preserve the production path exactly; select a dedicated development location and create any required directory before database initialization.
- [x] Keep `db::initialize` as the sole database initialization path for both environments. Do not introduce schema forks, automatic migration, copying, import, or production-database fallback behavior.
- [x] Add focused Rust unit coverage alongside the path-selection logic. Verify that development and production resolve to distinct paths, production resolves to the current path, and path selection does not open or modify a real user database.

### Verification & Acceptance Criteria
*All criteria must pass before advancing to the handoff report.*
- [x] **Automated Checks:** `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` and `cargo test --manifest-path src-tauri/Cargo.toml`.
- [x] **Functional Assertions:** Development and production resolve to separate SQLite files; production retains `app_data_dir/cm-modpack-util.sqlite`; development uses its own location and initializes through the existing schema; no path-selection failure silently opens production storage.

### Plan Compliance Checklist
*Verify each item against the actual diff before claiming this phase complete. Do not mark the phase COMPLETED if any item fails.*
- [x] **Required Files:** `src-tauri/src/lib.rs` contains the startup path selection and focused tests, or the tests are placed in an existing Rust test location without adding unrelated files.
- [x] **Boundaries:** Do not modify `src-tauri/src/db/schema.sql`, alter schema compatibility behavior, change the production database path, or touch frontend and Packwiz project files.
- [x] **Legacy Code Removed:** Replace the single unconditional startup path selection; retain the existing production database filename and location as the production branch.
- [x] **Acceptance Checks:** Confirm both Rust commands above ran and the functional path assertions passed; do not treat compilation alone as proof of isolation.

### Phase 1 Handoff & Verification Report
*Filled out by the executing agent upon phase completion.*
- **Compliance Check:** PASSED — `src-tauri/src/lib.rs` contains the startup path selector and pure path-policy test; schema, production path, frontend, and Packwiz files were unchanged, and the unconditional startup path was replaced.
- **Verification Result:** PASSED
- **Execution Proof / Logs:**
```bash
  $ cargo fmt --manifest-path src-tauri/Cargo.toml -- --check -> passed
  $ cargo test --manifest-path src-tauri/Cargo.toml -> 127 tests passed, 0 failed
```
- **Artifacts Created/Modified:**
* `src-tauri/src/lib.rs` - Routes debug-assertion builds to an isolated development subdirectory and tests both path results.

* **Decisions & Deviations:** No deviations. `cfg!(debug_assertions)` distinguishes the documented Tauri dev build from the release bundle; no custom Cargo profiles are declared in the inspected manifest.
* **Next Phase Context:** Development uses `app_data_dir/development/cm-modpack-util.sqlite`; production remains at `app_data_dir/cm-modpack-util.sqlite`. Both use the same initializer and schema. Directory creation errors propagate rather than falling back.

---

## Phase 2: Document Environment Storage and Reset Boundaries
- **Status:** COMPLETED
- **Objective:** Make it clear which database development and production use and how to reset development storage without risking production data.

### Tasks
- [x] Update `docs/data-management.md` to describe the production database path, the development database path selected in Phase 1, and the build modes that use each.
- [x] Update the development reset guidance to identify the correct database and its `-wal` and `-shm` sidecars, explicitly warn against deleting production files when resetting development data, and retain the existing no-migration/no-silent-replacement policy.

### Verification & Acceptance Criteria
- [x] **Automated Checks:** Review the rendered Markdown and run `git diff --check`.
- [x] **Functional Assertions:** A developer can identify which database a development run uses, reset development storage without deleting production data, and understand that no existing data is copied or migrated.

### Plan Compliance Checklist
*Verify each item against the actual diff before claiming this phase complete. Do not mark the phase COMPLETED if any item fails.*
- [x] **Required Files:** `docs/data-management.md` documents the actual Phase 1 paths and reset procedure.
- [x] **Boundaries:** Do not modify README/product scope or create a competing database-policy document; `docs/data-management.md` remains the source for database reset guidance.
- [x] **Legacy Code Removed:** Replace any reset wording that implies development and production share one undifferentiated database location; preserve accurate existing alpha reset and incompatibility guidance.
- [x] **Acceptance Checks:** Confirm documentation matches the implemented path policy and `git diff --check` passes.

### Phase 2 Handoff & Verification Report
- **Compliance Check:** PASSED — `docs/data-management.md` describes the two paths and build modes, reset instructions target only the development subdirectory and sidecars, and existing alpha schema/reset constraints remain. No competing document or out-of-scope files were changed.
- **Verification Result:** PASSED
- **Execution Proof / Logs:**
```bash
  $ render docs/data-management.md with marked and assert path/reset text -> Markdown rendered; database paths and reset warning are present
  $ git diff --check -> passed (Git reported its CRLF-to-LF normalization warning)
```
- **Artifacts Created/Modified:**
* `docs/data-management.md` - Documents profile-specific database locations and safe development reset boundaries.

* **Decisions & Deviations:** No deviations.
* **Next Phase Context:** Validate implementation and documentation together; debug-assertion builds use the development subdirectory and release builds use the existing production location.

---

## Phase 3: Validate Isolation and Final Handoff
- **Status:** COMPLETED
- **Objective:** Verify the complete path policy, preserve production compatibility, and hand off evidence that development storage is isolated.

### Tasks
- [x] Review the final diff to confirm startup chooses the intended path for both documented run modes and that database initialization and error reporting remain explicit.
- [x] Run the focused Rust formatting and test checks; build the production desktop bundle with `npm run tauri build` to verify the release configuration still compiles.
- [x] Review the data-management documentation against the final implementation and record test/build results and any deviations in this phase handoff.

### Verification & Acceptance Criteria
- [x] **Automated Checks:** `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`, `cargo test --manifest-path src-tauri/Cargo.toml`, `npm run tauri build`, and `git diff --check`.
- [x] **Functional Assertions:** Automated tests prove the development and production path mapping without accessing real user data; release startup remains configured for the existing production database location; documentation does not direct developers to reset production storage.

### Plan Compliance Checklist
*Verify each item against the actual diff before claiming this phase complete. Do not mark the phase COMPLETED if any item fails.*
- [x] **Required Files:** The only planned implementation changes are `src-tauri/src/lib.rs` (startup policy and tests) and `docs/data-management.md`; no separate test file was needed.
- [x] **Boundaries:** No production database or user application data was opened, reset, copied, or modified during validation; no unrelated schema, frontend, or Packwiz changes were included.
- [x] **Legacy Code Removed:** There is no remaining unconditional startup path that causes development and production to share the same database file.
- [x] **Acceptance Checks:** All listed checks ran successfully and functional path assertions were verified.

### Phase 3 Handoff & Verification Report
- **Compliance Check:** PASSED — final implementation changes are limited to the planned Rust startup path and database documentation; production path and initializer are preserved, debug and release paths are distinct, and no real user database or Packwiz files were touched.
- **Verification Result:** PASSED
- **Execution Proof / Logs:**
```bash
  $ cargo fmt --manifest-path src-tauri/Cargo.toml -- --check -> passed
  $ cargo test --manifest-path src-tauri/Cargo.toml -> 127 tests passed, 0 failed
  $ npm run tauri build -> release Windows NSIS bundle and updater signature created
  $ git diff --check -> passed (Git reported its CRLF-to-LF normalization warning)
```
- **Artifacts Created/Modified:**
* `src-tauri/src/lib.rs` - Isolates development database path and adds path-policy coverage.
* `docs/data-management.md` - Documents build-mode paths and safe development resets.
* `src-tauri/target/release/bundle/nsis/CM-Modpack-Util_0.0.17_x64-setup.exe` - Generated validation artifact (build output, not a source change).

* **Decisions & Deviations:** No deviations or unresolved issues.
* **Next Phase Context:** Implementation complete; hand off this plan and the recorded evidence to Helm Implementation Final Validation.
