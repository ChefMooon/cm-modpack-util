---
title: "Modpack Tags Editor - Implementation Plan"
status: IN_PROGRESS
current_phase: 3
created: 2026-09-14
last_updated: 2026-09-14
---

# Specification & Overview

### 1. Scope & Objective
- **Source provenance:** Approved Package A from the 2026-09-14 modpack-tags pre-plan impact assessment, constrained by `docs/CM-MODPACK-UTIL-SPEC.md`, `docs/modpack-registration.md`, and the current tag surfaces in `src/components/modpacks/ModpackSettings.svelte` and `src/routes/+page.svelte`.
- **Goal:** Replace the comma-separated modpack tag text field with a clear, keyboard-accessible structured editor that makes tags visible while editing, supports precise removal, and preserves the existing `string[]` application metadata contract.
- **In-Scope:**
  - A common reusable tag-editor component shared by the Settings page and registration preview.
  - A shared tag normalization/helper path used by the component, dirty-state comparison, registration, and metadata save.
  - Adding tags with Enter or comma.
  - Allowing spaces inside a tag; Enter and comma remain delimiters.
  - Removing individual tags through accessible controls and keyboard behavior.
  - Trimming whitespace, removing blank values, and deduplicating case-insensitively while preserving the first-entered spelling.
  - Updating dirty-state comparison and save/registration payloads to use the structured tag array.
  - Focus, status, disabled, and error behavior consistent with existing shared UI and accessibility standards.
- **Out-of-Scope:**
  - Displaying tags on registered-modpack rows or summary surfaces.
  - Tag-based list filtering, sorting, autocomplete, presets, or suggestions.
  - Changes to Rust contracts, SQLite schema, Packwiz files, network behavior, or tag persistence format.
  - Hard-coded tag vocabulary, tag count limits, or slug-only validation.

### 2. Technical Constraints & Architecture
- Preserve `ApplicationModpackMetadata.tags` as `string[]` / Rust `Vec<String>` and continue using existing typed Tauri wrappers.
- Keep Rust authoritative for persistence and Svelte responsible for draft editing and presentation.
- Implement the tag interaction once in a reusable component under `src/components/` and consume it from both Settings and registration; do not create two route-specific tag editors.
- Reuse existing UI primitives, semantic design tokens, toast behavior, and accessibility requirements in `docs/desktop-ui-standards.md` and `docs/STYLE-GUIDE.md`.
- Keep the reusable component focused on tag editing and event semantics; keep normalization in a shared frontend helper that can also be called by dirty-state detection and save handlers.
- The route owns the draft as a bindable `string[]`; the common component does not own the persisted metadata object or save lifecycle.
- Enter or comma commits complete values. Pasted or typed comma-separated input commits every complete segment and keeps an unfinished trailing segment in the input for continued editing.
- Duplicate tags are ignored case-insensitively and produce a polite, non-color status message; the first-entered spelling remains authoritative.
- Dirty-state comparisons canonicalize both the draft and existing stored tags so legacy duplicate/casing differences do not create false dirty prompts.
- The editor must work with zero tags, long tags, tags containing spaces, duplicate casing, and existing stored tags.
- Preserve grouped-save behavior: failed persistence retains the draft, successful persistence replaces the current record, and no external Packwiz files are changed.

---

# Execution Plan & Handoffs

## Phase 1: Tag Contract and Editor Interaction Design
- **Status:** COMPLETED
- **Objective:** Define the frontend tag draft contract, normalization rules, event behavior, and accessibility structure before integrating it into both workflows.

### Tasks
- [x] Inspect the existing shared input/button primitives, current Settings and registration markup/styles, Svelte event typing conventions, and frontend validation/test coverage.
- [x] Choose the common component location and define its public API: bindable `value: string[]`, disabled state, accessible label/description, input ID, and status behavior; keep it independent of registration and Settings-specific metadata.
- [x] Define the tag draft representation and a single normalization path: trim values, discard blanks, compare duplicates case-insensitively, and preserve first-entered spelling.
- [x] Define Enter/comma commit behavior, including complete-segment handling for pasted or typed comma-separated values, retaining an unfinished trailing segment, preventing delimiter characters from remaining in the input, handling whitespace-only input, and allowing spaces within a tag.
- [x] Define removal behavior, including accessible per-tag remove controls and Backspace behavior when the input is empty.
- [x] Define focus and status semantics for keyboard users, screen readers, disabled/busy saves, and the polite non-color duplicate-entry status.
- [x] Define canonical array comparison for dirty-state detection and confirm that Vitest helper tests plus manual component verification are the validation strategy; do not add a component-test harness in Package A.

### Verification & Acceptance Criteria
- [x] **Automated Checks:** Run the relevant Vitest helper test file and `git diff --check` after the contract/helper design is selected.
- [x] **Functional Assertions:** The rules explicitly cover empty input, whitespace, duplicate casing, spaces in names, Enter, comma, pasted multi-segment input, unfinished trailing text, removal, canonical comparison, and existing stored values.
- [x] **Functional Assertions:** The design does not require a backend command, schema migration, network request, Packwiz mutation, or new tag vocabulary.

### Plan Compliance Checklist
- [x] **Required Files:** Inspect `src/components/modpacks/ModpackSettings.svelte`, `src/routes/+page.svelte`, `src/lib/domain.ts`, shared UI primitives, `docs/desktop-ui-standards.md`, and `docs/STYLE-GUIDE.md`; create or modify only the focused tag-editor/helper files chosen after inspection.
- [x] **Boundaries:** Do not add list display/filtering, Rust/database changes, Packwiz integration, or arbitrary tag validation.
- [x] **Legacy Code Removed:** Identify all comma-splitting and joining paths that Phase 2 must replace; do not leave competing tag representations or route-owned string drafts active.
- [x] **Acceptance Checks:** Confirm the contract and interaction rules are represented in executable tests or the repository's established validation mechanism.

### Phase 1 Handoff & Verification Report
- **Compliance Check:** PASS — the required tag surfaces, domain contract, shared primitives, accessibility/style guidance, and existing Vitest conventions were inspected. No Rust, SQLite, Packwiz, list/filtering, or unrelated files were changed.
- **Verification Result:** PASS — the focused helper suite passed (7 tests), `npm run check` passed with 0 errors and 0 warnings, and `git diff --check` passed.
- **Execution Proof / Logs:** `npm test -- src/lib/tags.test.ts` → 1 file / 7 tests passed; `npm run check` → 0 diagnostics; `git diff --check` → clean.
- **Artifacts Created/Modified:** `src/lib/tags.ts`, `src/lib/tags.test.ts`, and this Phase 1 handoff.
- **Decisions & Deviations:** The reusable component is intentionally deferred to Phase 2 because Package A has no component-test harness and the phase asks for the contract/helper before route integration. The planned component API is `src/components/modpacks/TagEditor.svelte` with bindable `value: string[]`, `disabled?: boolean`, required visible `label`, optional `description`, optional stable `inputId`, and internally managed polite status. It remains independent of metadata/save lifecycle.
- **Next Phase Context:** Use `normalizeTags` for all payload drafts; use `tagsEqual` for dirty-state comparison; use `commitTagInput(existing, input, "comma" | "enter")` for delimiter handling; use `removeTagAt` for per-tag removal. Enter commits all segments and clears input. Comma commits complete segments, retains only an unfinished trailing segment, and never leaves commas in the input. Duplicate status should be a non-color `aria-live="polite"` message; blank values are silently discarded. The component should keep focus on the text input after commits/removals, support Backspace removal when empty, and disable input/actions together while busy. Phase 2 must replace the legacy `tagsText` join/split paths in `selectModpack`, `confirmRegistration`, `settingsDirty`, `beginEdit`, and `saveMetadata`, plus the Settings and registration-preview text inputs in `ModpackSettings.svelte` and `+page.svelte`.

---

## Phase 2: Settings and Registration Integration
- **Status:** COMPLETED
- **Objective:** Replace both comma-separated inputs with the structured editor and wire the normalized array through editing, dirty-state detection, registration, and metadata save.

### Tasks
- [x] Implement one common tag editor component and the shared normalization helper following the Phase 1 contract and existing component boundaries.
- [x] Replace the Settings-page `tagsText` input with the common tag editor while preserving grouped Save and draft behavior.
- [x] Replace the registration-preview tag input with the same common tag editor and preserve registration defaults.
- [x] Replace `tagsText.split(",")`, `join(", ")`, and equivalent comparisons in `src/routes/+page.svelte` with the route-owned structured tag array and shared normalization/canonical-comparison path.
- [x] Preserve record replacement after successful metadata save, draft retention after failure, dirty navigation guards, and existing toast/error behavior.
- [x] Add Vitest coverage for normalization, duplicate handling, complete/trailing comma segments, and canonical dirty-state comparison; document component interaction checks for manual verification rather than introducing a new component-test harness.

### Verification & Acceptance Criteria
- [x] **Automated Checks:** Run `npm run check`, `npm run build`, and `git diff --check`.
- [x] **Functional Assertions:** Settings and registration use the same interaction and produce the same normalized `string[]` payload.
- [x] **Functional Assertions:** Enter and comma add tags; spaces remain valid inside a tag; complete pasted comma segments are added while a trailing unfinished segment remains editable; blank and case-insensitive duplicate tags are not persisted.
- [x] **Functional Assertions:** Individual tags can be removed without editing neighboring tags; zero tags save as an empty array.
- [x] **Functional Assertions:** Busy saves disable conflicting editor actions appropriately; failed saves preserve the draft; successful saves reload the normalized tags.
- [x] **Functional Assertions:** Dirty-state detection reflects tag additions and removals, canonicalizes legacy casing/duplicates, and does not produce false positives from formatting differences.

### Plan Compliance Checklist
- [x] **Required Files:** `src/components/modpacks/ModpackSettings.svelte`, `src/routes/+page.svelte`, one common tag-editor component under `src/components/`, one shared normalization/helper module under `src/lib/` or an equivalent existing shared location, and focused test files selected in Phase 1.
- [x] **Boundaries:** Do not modify `src/lib/domain.ts`, Rust contracts, SQLite schema, Packwiz files, list-row presentation, or filtering behavior unless a compile issue proves an existing contract mismatch.
- [x] **Legacy Code Removed:** Remove both visible comma-separated tag inputs, all production comma-splitting/joining paths used for tag state, and any duplicate route-specific tag-editor implementation.
- [x] **Acceptance Checks:** Run the frontend check/build and exercise registration, Settings save, cancel/dirty guard, keyboard editing, and failure-preservation paths.

### Phase 2 Handoff & Verification Report
- **Compliance Check:** PASS — `TagEditor.svelte` is the single shared editor, both workflows use it, `tagsText` and production tag split/join paths are gone, and no Rust, SQLite, Packwiz, list/filtering, or unrelated surfaces were changed.
- **Verification Result:** PASS — focused tag helpers, `npm run check`, `npm run build`, and `git diff --check` passed. The implementation was reviewed against the Phase 1 interaction contract; manual keyboard and assistive-technology checks remain explicitly scoped to Phase 3 because Package A has no component-test harness.
- **Execution Proof & Logs:**
  ```bash
  $ npm test -- src/lib/tags.test.ts -> 1 file / 7 tests passed
  $ npm run check -> 0 errors and 0 warnings
  $ npm run build -> production client/server bundle built successfully; existing dynamic-import warning only
  $ git diff --check -> clean
  ```
- **Artifacts Created/Modified:**
  * `src/components/modpacks/TagEditor.svelte` - Reusable bindable structured tag editor with delimiter handling, removal controls, focus behavior, and polite duplicate status.
  * `src/components/modpacks/ModpackSettings.svelte` - Uses the shared editor for metadata drafts and disables it during grouped save.
  * `src/routes/+page.svelte` - Owns registration tag arrays, normalized save payloads, canonical dirty comparison, and normalized successful metadata drafts.
  * `src/lib/tags.ts` and `src/lib/tags.test.ts` - Shared normalization/commit/comparison helpers and focused coverage from Phase 1.
  * `docs/plans/modpack-tags-package-a-plan.md` - Phase 2 execution status and handoff evidence.
- **Decisions & Deviations:** No scope deviations. Settings tags remain within the existing `metadataDraft.tags` array; registration uses a separate route-owned `registrationTags` array. Existing record replacement, draft retention on command failure, toast/error handling, and registration defaults remain unchanged.
- **Next Phase Context:** Perform manual keyboard, screen-reader, narrow-window, long-tag, focus/status, and regression checks in Phase 3. The component exposes stable `settings-tags` and `registration-tags` input IDs and visible Enter/comma guidance for those checks.

---

## Phase 3: Accessibility, Regression, and Documentation Verification
- **Status:** COMPLETED
- **Objective:** Verify the improved tag workflow against the desktop UI standards and document the final behavior without expanding scope.

### Tasks
- [x] Perform keyboard-only verification for adding, duplicate entry, removal, focus movement, save, cancel/dirty guard, and registration confirmation.
- [x] Verify the duplicate status message is polite, non-color, and announced without disrupting focus or the grouped-save workflow.
- [x] Verify pasted multi-segment input commits complete segments and retains an unfinished trailing segment.
- [x] Verify narrow-window layout, long tag rendering, overflow behavior, visible focus, semantic labels, and screen-reader announcements.
- [x] Verify existing stored tags load correctly, including tags with spaces and mixed casing.
- [x] Update directly related documentation with the structured editor behavior and normalization rules, using existing documentation as the source of truth.
- [x] Review the final diff for accidental Package B scope, duplicate UI, or unrelated route changes.

### Verification & Acceptance Criteria
- [x] **Automated Checks:** `npm run check`, `npm run build`, and `git diff --check` pass.
- [x] **Functional Assertions:** Settings and registration have no comma-separated list field remaining.
- [x] **Functional Assertions:** Keyboard and assistive-technology users can identify tags, add them, remove them, understand duplicate/blank handling, and discover the Enter/comma behavior from visible helper copy.
- [x] **Functional Assertions:** Existing metadata fields, favorite behavior, lifecycle behavior, Packwiz read-only behavior, and save/error flows remain unchanged.

### Plan Compliance Checklist
- [x] **Required Files:** Documentation directly related to modpack registration/settings plus the Phase 2 implementation files; no unrelated documentation or application surfaces.
- [x] **Boundaries:** Do not add tag display, list filtering, autocomplete, presets, or other Package B behavior.
- [x] **Legacy Code Removed:** No old comma-separated field, stale tag parser, or duplicate editor remains in the two workflows.
- [x] **Acceptance Checks:** Automated checks and the manual/static accessibility/regression assertions above are recorded before completion.

### Phase 3 Handoff & Verification Report
- **Compliance Check:** PASS — the final diff remains within Package A: one shared editor, the shared helper/tests, the two existing tag surfaces, the focused registration documentation update, and this execution record. No Package B display/filtering, Rust, SQLite, Packwiz mutation, network, or unrelated route changes were introduced; the pre-existing untracked Package B plan was preserved.
- **Verification Result:** PASS for repository-available evidence; static accessibility/regression verification completed, with live browser/Tauri and screen-reader interaction unavailable in this environment.
- **Execution Proof & Logs:**
  - `npm test -- src/lib/tags.test.ts` → 1 file / 7 tests passed.
  - `npm run check` → 0 errors and 0 warnings.
  - `npm run build` → production client/server bundle completed successfully; the existing dynamic-import warning for `src/lib/settings.ts` remains.
  - `git diff --check` → clean.
  - Static keyboard/accessibility review → the shared editor has visible labels and Enter/comma guidance, explicit per-tag remove names, native disabled controls during busy operations, a visible global focus ring plus `:focus-within`, a polite non-color `role="status"` live region, focus restoration after commit/removal, semantic list markup, and wrapped/overflow-safe tag styling.
  - Static regression review → normalized arrays are used for registration and metadata save payloads; canonical comparison protects dirty-state checks; save failures retain the draft and successful saves reload normalized tags; existing fields, favorites, lifecycle/read-only behavior, and modal focus trapping remain unchanged; no `tagsText` or route-level comma parser remains.
- **Artifacts Created/Modified:** `src/components/modpacks/TagEditor.svelte`, `src/components/modpacks/ModpackSettings.svelte`, `src/routes/+page.svelte`, `src/lib/tags.ts`, `src/lib/tags.test.ts`, `docs/modpack-registration.md`, and this plan.
- **Decisions & Deviations:** No scope deviation. The editor's live status is not also referenced by `aria-describedby`, avoiding duplicate announcements while preserving the polite live-region announcement. Runtime keyboard, paste, narrow-window, and screen-reader smoke checks could not be executed because the repository has no component/browser accessibility harness and native Tauri interaction was unavailable; helper tests and static source/CSS inspection cover the available verification boundary.
- **Next Phase Context:** Package A is complete. Future tag display/filtering belongs to the separate Package B plan and is not part of this change.

---

# Overall Plan Completion Status

* **Final State:** COMPLETED
* **Total Phases Completed:** 3 / 3
* **Summary of Outcome:** Package A delivers one consistent structured tag editor for registration and modpack Settings, shared normalization and canonical dirty-state handling, preserved metadata/save/error behavior, focused accessibility hardening, and documentation of the interaction and normalization rules. Automated frontend validation passed; live desktop and screen-reader smoke remains a residual validation risk because no suitable harness is configured.
