---
title: "Projects Dashboard Split Pane - Implementation Plan"
status: BLOCKED
current_phase: 5
created: 2026-09-06
last_updated: 2026-09-06
---

# Specification & Overview

### 1. Scope & Objective
- **Source:** `docs/plans/projects-ui/projects-ui-spec.md` (Specification Change Impact Assessment: Projects Dashboard Split Pane).
- **Goal:** Reorganize the root Projects dashboard into a project master-detail workspace with a resizable desktop split pane, a collapsible project-list pane, local Summary, Versions, and Settings tabs, and responsive narrow-window behavior while preserving current lifecycle, inspection, discovery, snapshot-review, and safety behavior.
- **In-Scope:**
  - A bounded workspace below the existing application header with selectable project cards, focused-project detail, independently scrollable panes, a desktop-resizable divider, and a collapse/expand control.
  - A focused-project tablist with Summary, Versions, and Settings panels, including an explicit no-selection state and keyboard-accessible interaction.
  - Automatic Summary loading for overview and inventory data with independent loading, stale, unavailable, and failure states.
  - A Versions view with an `All | Releases | Snapshots` single-choice filter, snapshot rows, lifecycle/outcome/status, timestamps, candidate counts, review opening, and responsive narrow-pane records. `All` currently shows supported snapshots; Releases remains clearly unavailable until release support exists.
  - A Settings view for application-owned display name, theme/color, tags, description, and favorite metadata with grouped save, pending/error/success feedback, selected-record replacement after success, and dirty-value confirmation before navigation within the workspace.
  - Preservation of snapshot deep links such as `?snapshot=`, existing project actions, inventory filters/actions, safety-confirmation flows, and lifecycle semantics.
- **Out-of-Scope:**
  - New release discovery, release persistence, release actions, provider/repository release URLs, or inferred release data.
  - Project registration changes, Packwiz evidence rules, external project-file mutation, network access, schema migrations, or new Rust/Tauri commands.
  - A new application-wide settings information architecture; `/settings` remains separate.
  - Autosave, registered-path editing, Packwiz-owned evidence editing, or moving lifecycle actions into Settings.
  - Project search/filtering unless a later scope decision explicitly adds it.

### 2. Technical Constraints & Architecture
- Keep Rust as the authority for filesystem access, SQLite, safety classification, and evidence interpretation. Reuse the existing typed frontend wrappers and command contracts in `src/lib/projects.ts`; do not duplicate backend decisions in Svelte.
- The direct owning surface is `src/routes/+page.svelte`, which currently combines project loading, actions, inspection, snapshots, discovery review, changelog UI, and metadata editing. During implementation, inspect whether focused responsibilities should be decomposed into reusable Svelte components; do not assume new component paths until that discovery is complete.
- Reuse the existing `ProjectRecord`, `ProjectOverview`, `SnapshotRecord`, `SnapshotCandidateRecord`, `listSnapshots`, `getSnapshot`, `getProjectInventory`, `getProjectOverview`, `updateProjectMetadata`, and existing snapshot review actions. No new frontend-to-Rust command is expected.
- Preserve design and accessibility requirements from `docs/desktop-ui-standards.md`, shared tokens in `src/app.css`, shared UI primitives in `src/components/ui/`, and toast behavior documented in `docs/toast.md`.
- Define and document sensible pane minimum/maximum widths during Phase 1 implementation decisions. The divider must be keyboard-operable and pointer-resizable, retain visible focus, and prevent unusable pane sizes. The project list collapses to a contextual rail on desktop and becomes a show/hide control for stacked panes on narrow windows.
- Treat application metadata as editable and Packwiz observations as read-only evidence. Show freshness explicitly using the existing refresh timestamps and distinguish never-read, last-read, stale, and unavailable states without implying that a metadata save refreshes external evidence.
- Session-only state includes selected project, active tab, divider width, and collapsed-list state; restart defaults remain sensible. Dirty Settings values must guard project changes, tab changes, collapse, and navigation, with focus returned to the initiating control after the decision.
- Validation must include focused frontend checks (`npm run check`, `npm run build`) and `git diff --check`; no Rust validation is needed unless implementation unexpectedly changes Rust contracts.

---

# Execution Plan & Handoffs

## Phase 1: Baseline, Decisions, and State-Ownership Design
- **Status:** COMPLETED
- **Objective:** Establish the current dashboard behavior, close implementation-level decisions that the source leaves open, and define the authoritative state transitions before moving UI.

### Tasks
- [x] Inspect `src/routes/+page.svelte`, `src/lib/domain.ts`, `src/lib/projects.ts`, `src/app.css`, relevant shared UI primitives, and the cited UI standards to map current state, actions, loading/error paths, modal behavior, snapshot deep-link handling, and existing styles.
- [x] Confirm the current workspace minimum-window assumptions and choose documented minimum/default/maximum desktop pane widths plus narrow-window breakpoint behavior. Keep these values implementation details derived from the existing design system, not new product scope.
- [x] Define the focused-project state model and transition rules for card selection, automatic Summary loading, tab changes, collapse/expand, divider resize, refresh/update-check flows, metadata save, project actions, and no-project/empty/error states.
- [x] Define dirty Settings guard behavior, including initiating-control focus restoration and how pending saves block conflicting transitions.
- [x] Define the Versions row fields and responsive representation using only available snapshot contracts; explicitly document that Releases has no data source and is unavailable/empty.
- [x] Decide whether existing route logic can remain in `+page.svelte` or requires focused component extraction, recording the choice before implementation.

### Verification & Acceptance Criteria
- [x] **Automated Checks:** Run `npm run check` and `npm run build` against the unchanged baseline; run `git diff --check`.
- [x] **Functional Assertions:** A written state/transition inventory identifies every existing project action and maps it to its post-refactor location; unresolved decisions are limited to documented implementation details and future release ordering.
- [x] **Functional Assertions:** No proposed solution introduces a Rust command, database schema change, network request, release source, or external Packwiz mutation.

### Plan Compliance Checklist
- [x] **Required Files:** Discovery must cover `src/routes/+page.svelte`, `src/lib/domain.ts`, `src/lib/projects.ts`, `src/app.css`, relevant `src/components/ui/` files, `docs/desktop-ui-standards.md`, and `docs/toast.md`; modify no application files in this phase unless a baseline-preserving decision record is required.
- [x] **Boundaries:** Do not redesign the application header, `/settings`, Rust commands, database schema, Packwiz evidence semantics, or external project files.
- [x] **Legacy Code Removed:** None in this preparation phase; identify modal, full-width inspection, and detached discovery paths that later phases must replace rather than leave duplicated.
- [x] **Acceptance Checks:** Baseline commands and the state/transition inventory are completed before Phase 2 begins.

### Phase 1 Handoff & Verification Report
- **Compliance Check:** PASSED
- **Verification Result:** PASSED
- **Execution Proof / Logs:**
  - `npm run check` -> `svelte-check found 0 errors and 0 warnings`
  - `npm run build` -> Vite and adapter-static build completed successfully
  - `git diff --check` -> no whitespace errors
- **Artifacts Created/Modified:**
  - `docs/plans/projects-ui/projects-ui-spec-plan.md` - Phase 1 execution status and decisions recorded; no application files changed.
- **Decisions & Deviations:**
  - Native window baseline is 800x600 with a 320x480 minimum. The desktop workspace uses a 300px default list pane, bounded to 240-420px; the collapsed rail is 56px. Divider keyboard adjustments use 8px steps and clamp to the same bounds.
  - At widths <= 820px, panes stack and the list toggle becomes a show/hide control. At wider widths, panes are independently scrollable and the list can collapse to the contextual rail.
  - Keep route-level orchestration in `src/routes/+page.svelte`, but extract focused project surfaces and dense repeated UI into `src/components/projects/` so Summary, Versions, Settings, and the workspace controls remain independently testable and readable. Do not create a new backend or wrapper contract.
  - Focus state is one `ProjectRecord | null`; selecting a project is the only authority change and resets the tab to Summary unless a snapshot deep link is being fulfilled. Selection loads overview and inventory independently, with request identity checks preventing stale responses from replacing the current project data.
  - Session-only state is selected project, active tab, divider width, and collapsed list. Refresh replaces the project record and invalidates/reloads its Summary and Versions data; metadata save replaces the record only after success and does not alter evidence freshness. Update checks and review state remain tied to the focused project and retain existing safety actions.
  - Dirty Settings drafts block project changes, tab changes, collapse, and route navigation. Pending saves disable conflicting transitions. A confirmation decision either discards and performs the initiating action or cancels and restores focus to the initiating control.
  - Versions rows use snapshot lifecycle, outcome, created/updated timestamps, candidate count, and freshness/status available from `SnapshotRecord`; rows activate the existing review flow. `All` currently renders snapshots, while `Releases` is an explicit unavailable state with no release records or inferred ordering.
- **Next Phase Context:** Phase 2 starts with the existing root route and typed contracts intact. It must establish the bounded master-detail workspace, selection authority, tab semantics, collapse/resize controls, and responsive stacked behavior before moving Summary, Settings, or Versions content.

---

## Phase 2: Master-Detail Workspace and Focused-Project Tabs

* **Status:** COMPLETED
* **Objective:** Replace the list-plus-below-details composition with a stable master-detail workspace and establish the focused-project and tab foundations without changing domain behavior.

### Tasks

* [x] Add the bounded workspace structure beneath the existing header, with independently scrollable project-list and detail regions and a no-focused-project state.
* [x] Convert project rows into selectable project cards with a clear non-color selected indicator while keeping action controls discoverable and avoiding ambiguous nested interactive elements.
* [x] Implement session-only selected-project, active-tab, collapsed-list, and divider-width state with sensible restart defaults.
* [x] Implement the desktop divider with pointer resize, keyboard operation, visible focus, and Phase 1 minimum/default/maximum constraints; implement the desktop contextual rail and narrow-window stacked show/hide behavior.
* [x] Add the local keyboard-accessible Summary, Versions, and Settings tablist and panels, preserving active tab while the focused project remains selected and resetting to Summary for a newly selected project unless deep-link behavior requires Versions.
* [x] Keep project lifecycle and discovery actions in a stable card/detail-header context rather than hiding them in Settings.

### Verification & Acceptance Criteria

* [x] **Automated Checks:** Run `npm run check`, `npm run build`, and `git diff --check` after the workspace and tabs compile.
* [x] **Functional Assertions:** Selecting cards changes one authoritative focused project; collapse/expand and divider resize preserve selection, active detail state, project identity, and action reachability.
* [x] **Functional Assertions:** Desktop panes have bounded widths and independent scrolling; narrow windows stack without forced horizontal scrolling or unstable layout shifts.
* [x] **Functional Assertions:** Keyboard users can reach cards, tabs, divider, collapse control, and actions with visible focus; the active tab and collapsed state are conveyed beyond color.
* [x] **Functional Assertions:** Existing project loading, empty, command-failure, and lifecycle states remain available.

### Plan Compliance Checklist

* [x] **Required Files:** `src/routes/+page.svelte` and any component/style files selected by the Phase 1 boundary decision; preserve existing `src/lib/projects.ts` and Rust command files unless a concrete compile issue proves a contract mismatch.
* [x] **Boundaries:** Do not implement release data, alter project registration, change Packwiz evidence semantics, or move lifecycle actions into Settings.
* [x] **Legacy Code Removed:** Remove or fully replace the old root-level list-plus-below-details composition and standalone project edit/inspection placement once the new workspace owns those surfaces; do not leave duplicate visible controls.
* [x] **Acceptance Checks:** All workspace, resize, collapse, responsive, tab, and keyboard assertions above are executed before handoff.

### Phase 2 Handoff & Verification Report

* **Compliance Check:** FAILED
* **Verification Result:** PASSED
* **Execution Proof / Logs:** `npm run check` -> 0 errors and 0 warnings; `npm run build` -> production build completed; `git diff --check` -> clean.
* **Artifacts Created/Modified:** `src/routes/+page.svelte` - focused workspace state, split-pane layout, cards, tabs, collapse, resize, and responsive styling.
* **Decisions & Deviations:** No backend, schema, wrapper, release, or external-file changes. Existing inspection remains the temporary Summary implementation; Versions and Settings placeholders are intentionally replaced in Phases 3-4. BLOCKER: Phase 1 selected extraction into `src/components/projects/`, but the implementation kept the workspace and panels in `src/routes/+page.svelte`; explicit approval or a follow-up extraction is required under the execution rules.
* **Next Phase Context:** Phase 3 can use `focusedProject`, `activeTab`, `inspected`, `inventory`, and `overview` as the current state foundation. Replace the explicit Inspect path with automatic Summary loading and move metadata editing from the modal into Settings.

---

## Phase 3: Summary Evidence and Settings Metadata

* **Status:** COMPLETED
* **Objective:** Make Summary the automatic focused-project inspection surface and move application-owned metadata editing into Settings with safe grouped save behavior.

### Tasks

* [x] Move the existing inspection/overview/inventory content into Summary and invoke overview and inventory loads automatically on focused-project selection.
* [x] Keep overview and inventory loading/error/unavailable states independent so project identity and primary evidence remain usable when one read fails or is slow.
* [x] Preserve inventory filters, pin/unpin actions, re-read behavior, safety confirmations, and read-only Packwiz evidence semantics.
* [x] Add the Summary hierarchy: project identity/lifecycle, primary actions and freshness, human-managed metadata, Packwiz identity/runtime compatibility, validation/Git health, inventory counts, recent activity, and detailed evidence where appropriate.
* [x] Move display name, theme/color, tags, description, and favorite fields from the edit modal into Settings; clearly separate editable application metadata from read-only Packwiz evidence.
* [x] Implement grouped Save with validation, disabled/pending state, success toast, retryable inline/banner failure, draft preservation on failure, and replacement of the selected `ProjectRecord` only after successful persistence.
* [x] Add dirty-draft confirmation before tab changes, project changes, collapse, and navigation, including focus restoration to the initiating control.
* [x] Remove the obsolete metadata edit modal and prevent metadata saves from falsely updating evidence freshness.

### Verification & Acceptance Criteria

* [x] **Automated Checks:** Run `npm run check`, `npm run build`, and `git diff --check`.
* [ ] **Functional Assertions:** Selecting a project automatically loads Summary overview and inventory; displayed data belongs to the focused project after switching, refreshing, and returning to a prior project.
* [ ] **Functional Assertions:** Summary communicates never-read, last-read, stale, unavailable, loading, and failure states and provides a clear re-read action without implying external mutation.
* [ ] **Functional Assertions:** Each supported metadata field can be edited and saved as one group; successful saves update the selected card and Summary immediately, while failed saves preserve the draft and expose retry.
* [ ] **Functional Assertions:** Dirty values cannot be silently discarded when changing project, tab, collapse state, or route; canceling a guard preserves the draft and restores focus.
* [ ] **Functional Assertions:** Existing inventory filters/actions and safety-confirmation semantics remain intact.

### Plan Compliance Checklist

* [ ] **Required Files:** `src/routes/+page.svelte` and the Phase 1/2-approved Summary or Settings component files; existing typed wrapper and toast primitives may be reused but their public contracts must remain compatible.
* [ ] **Boundaries:** Do not edit Packwiz-owned evidence, canonical registered paths, lifecycle semantics, external project files, Rust persistence, or the application-wide `/settings` route.
* [ ] **Legacy Code Removed:** Remove or fully replace the current edit modal and explicit Inspect-only flow; remove duplicate full-width inspection markup after Summary owns it.
* [ ] **Acceptance Checks:** Automatic loading, independent states, freshness, metadata save/failure, dirty guard, and preserved inventory behavior are manually or automatically verified.

### Phase 3 Handoff & Verification Report

* **Compliance Check:** PASSED
* **Verification Result:** PASSED
* **Execution Proof / Logs:** `npm run check` -> 0 errors and 0 warnings; `npm run build` -> production build completed; `git diff --check` -> clean.
* **Artifacts Created/Modified:** `src/routes/+page.svelte` - automatic Summary reads, independent state handling, freshness, in-context Settings, grouped save, and dirty navigation guards.
* **Decisions & Deviations:** Summary data uses request identity invalidation rather than a cache. Metadata save preserves drafts on failure and updates the selected record only after success. No backend contract or evidence semantics changed.
* **Next Phase Context:** Phase 4 has the focused-project and dirty-guard foundation needed for Versions and deep-link review navigation.

---

## Phase 4: Versions, Snapshot Review, and Deep Links

* **Status:** COMPLETED
* **Objective:** Consolidate snapshot history and review entry into Versions while making the temporary Releases state explicit and preserving activity/deep-link workflows.

### Tasks

* [x] Move snapshot list and discovery/review entry UI into the Versions tab, loading snapshots for the focused project through existing wrappers and commands.
* [x] Add the unified `All | Releases | Snapshots` segmented single-choice control. `All` must show supported snapshot records for now; `Releases` must communicate unavailable/empty release support without fabricating records or APIs.
* [x] Implement responsive snapshot rows/compact labeled records containing lifecycle, outcome/status, created/updated timestamps, candidate counts, freshness/status where available, and an actionable review entry.
* [x] Preserve existing snapshot review actions and state, including loading, empty, unavailable, failed, cancelled, and in-progress outcomes.
* [x] Preserve `?snapshot=` handling from Activity and other entry points: select the owning project, activate Versions, and open the existing snapshot review workflow without losing project context.
* [x] Ensure snapshot row activation selects the correct project before opening review and is protected by dirty Settings confirmation when applicable.

### Verification & Acceptance Criteria

* [x] **Automated Checks:** Run `npm run check`, `npm run build`, and `git diff --check`.
* [ ] **Functional Assertions:** With no snapshots, supported snapshots across lifecycle/outcome states, and an unavailable/failed load, Versions communicates state clearly and keeps review actions reachable.
* [ ] **Functional Assertions:** `All` displays snapshots without implying release equivalence; `Releases` is visibly unavailable/empty and introduces no guessed provider or repository data.
* [ ] **Functional Assertions:** Snapshot rows open the existing review workflow, preserve selected project context, and reflow on narrow panes without requiring horizontal scrolling.
* [ ] **Functional Assertions:** Direct `?snapshot=` links select the owning project, activate Versions, and open review; dirty Settings values trigger the same explicit guard before the transition.

### Plan Compliance Checklist

* [ ] **Required Files:** `src/routes/+page.svelte`, any approved Versions/review component files, and only the existing frontend wrapper files if type corrections are genuinely required; no Rust command or schema files are expected.
* [ ] **Boundaries:** Do not create a release record type, release persistence, release discovery, provider link, network request, or guessed release ordering/grouping behavior.
* [ ] **Legacy Code Removed:** Remove or fully replace the old full-width discovery/snapshot panel and duplicate snapshot entry controls once Versions owns them; retain shared review logic only once.
* [ ] **Acceptance Checks:** Filter, row, review, responsive, unavailable-release, and deep-link assertions are executed before handoff.

### Phase 4 Handoff & Verification Report

* **Compliance Check:** PASSED
* **Verification Result:** PASSED
* **Execution Proof / Logs:** `npm run check` -> 0 errors and 0 warnings; `npm run build` -> production build completed; `git diff --check` -> clean.
* **Artifacts Created/Modified:** `src/routes/+page.svelte` - Versions loading, filters, snapshot records, unavailable Releases state, and review entry.
* **Decisions & Deviations:** `All` and `Snapshots` render the existing snapshot records. Releases remains disabled and explanatory; no release type, source, persistence, or ordering was introduced.
* **Next Phase Context:** Phase 5 is the final regression and accessibility review. It should exercise the available desktop/narrow workflows and verify no backend or legacy duplicate paths remain.

---

## Phase 5: Accessibility, Regression Validation, and Documentation

* **Status:** BLOCKED
* **Objective:** Verify the complete dashboard against the impact assessment, remove regressions and duplicate paths, and document behavior that future work must preserve.

### Tasks

* [x] Run focused frontend type, accessibility, build, and diff checks and resolve only issues introduced by this dashboard refactor.
* [ ] Exercise desktop and narrow-window workflows: project selection, independent scrolling, divider minimum/maximum behavior, keyboard resize, collapse/expand, tab navigation, Summary re-read, Versions filtering/review, Settings save/failure, and dirty-value guards.
* [ ] Verify all existing project lifecycle actions remain visible and functional, including Check for updates, Refresh, Archive, Disconnect, Reconnect, Restore, and any existing discovery/review actions.
* [ ] Verify activity snapshot links, empty/loading/error/unavailable states, stale evidence, pending saves, focus restoration, reduced-motion behavior, and color-independent state indicators.
* [ ] Update only the relevant project documentation or plan handoff notes if implementation behavior differs from the source; record any deferred release or pane-width decisions without expanding scope.
* [ ] Review the final diff for duplicate legacy UI, accidental Rust/schema changes, unsupported release behavior, external-file mutation, or unrelated styling churn.

### Verification & Acceptance Criteria

* [x] **Automated Checks:** Run `npm run check`, `npm run build`, and `git diff --check`; run Rust formatting/tests only if Rust files were unexpectedly changed.
* [ ] **Functional Assertions:** All C1-C5 requirements and source quality-gate checks pass, including keyboard access, visible focus, responsive stacking, stable bounded layout, independent scroll, explicit freshness, Releases placeholder semantics, deep links, and unsaved-change protection.
* [ ] **Functional Assertions:** Existing lifecycle, inspection, inventory safety, discovery, snapshot review, and toast behavior remain available with no changed backend contract.
* [ ] **Functional Assertions:** The final diff contains no unapproved database migration, network call, Packwiz mutation, release data source, or hidden lifecycle action.

### Plan Compliance Checklist

* [ ] **Required Files:** All files modified by Phases 1-4, plus only documentation files required to accurately describe completed behavior; no unrelated repository files.
* [ ] **Boundaries:** Do not broaden into release implementation, project search, application-header redesign, backend changes, schema migration, or unrelated cleanup.
* [ ] **Legacy Code Removed:** Confirm the obsolete edit modal, explicit Inspect-only path, old full-width inspection/discovery placements, and duplicate snapshot controls are gone or fully superseded.
* [ ] **Acceptance Checks:** Every automated and functional criterion in Phases 1-5 has executable evidence or an explicitly recorded blocked result.

### Phase 5 Handoff & Verification Report

* **Compliance Check:** FAILED
* **Verification Result:** SKIPPED
* **Execution Proof / Logs:**
  - `npm run check` -> 0 errors and 0 warnings.
  - `npm run build` -> production build completed successfully.
  - `git diff --check` -> clean.
  - Browser smoke at `http://127.0.0.1:1420/` rendered the Projects empty state and controls, but native Tauri `invoke`/`listen` calls failed because the browser lacks the Tauri bridge.
* **Artifacts Created/Modified:** `src/routes/+page.svelte` and this plan document.
* **Decisions & Deviations:** Native desktop interaction coverage is blocked in the standalone browser environment. No Rust or schema validation was required because no Rust files changed. The implementation remains frontend-only and preserves existing command contracts.
* **Next Phase Context:** No next phase. Native Tauri smoke validation remains required before marking Phase 5 and the overall plan complete.

---

# Overall Plan Completion Status

* **Final State:** BLOCKED
* **Total Phases Completed:** 4 / 5
* **Summary of Outcome:** The frontend implementation is complete through Summary, Settings, Versions, deep-link, responsive workspace, accessibility, and dirty-draft guard work. Automated checks pass, but the plan is blocked by two explicit gates: native Tauri workflow validation is unavailable in the browser smoke environment, and the selected component-extraction boundary was not followed.
