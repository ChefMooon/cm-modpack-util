---
title: "Modpack Tag Organization - Future Implementation Plan"
status: DRAFT
current_phase: 1
created: 2026-09-14
last_updated: 2026-09-14
---

# Specification & Overview

### 1. Scope & Objective
- **Source provenance:** Deferred Package B from the 2026-09-14 modpack-tags pre-plan impact assessment, following the approved Package A plan in `docs/plans/modpack-tags-package-a-plan.md` and constrained by `docs/CM-MODPACK-UTIL-SPEC.md`, `docs/ROADMAP.md`, and `docs/STYLE-GUIDE.md`.
- **Goal:** Make saved modpack tags useful for scanning and organization by displaying them in the registered-modpack list and providing explicit tag-based filtering.
- **In-Scope:**
  - Displaying saved tags on registered modpack rows with compact, accessible overflow behavior.
  - Adding a registered-modpack tag filter.
  - Defining and implementing multi-tag matching semantics.
  - Handling zero-tag, no-match, archived, disconnected, and long-tag states.
  - Preserving Favorite as a separate first-class field rather than treating it as a tag.
- **Out-of-Scope:**
  - Replacing the Package A structured editor.
  - Packwiz or provider tag integration.
  - Network-backed suggestions or remote taxonomy.
  - Hard-coded required tag vocabularies.
  - Click-to-filter interactions unless explicitly approved during Phase 1.

### 2. Technical Constraints & Architecture
- Package A must be complete first so tags have deterministic trimming and case-insensitive duplicate behavior.
- Preserve `ApplicationModpackMetadata.tags` as `string[]`; no database migration or Rust command is expected from the current evidence.
- Keep list filtering in the frontend list state and preserve the existing archived visibility behavior.
- Follow `docs/desktop-ui-standards.md`, `docs/STYLE-GUIDE.md`, shared semantic tokens, visible focus, keyboard access, and color-independent meaning.
- Tags must remain visually distinct from Packwiz evidence, lifecycle status, theme, validation, and Favorite.

---

# Execution Plan & Handoffs

## Phase 1: Organization Semantics and Information Design
- **Status:** NOT_STARTED
- **Objective:** Close the product decisions required for useful, predictable tag organization before UI implementation.

### Tasks
- [ ] Verify Package A normalization is complete and identify the authoritative normalized tag source.
- [ ] Decide whether selecting multiple tags means match-any or match-all; if both are supported, define the control and default.
- [ ] Decide whether filtering combines with archived visibility and any future text search, and define empty-state wording.
- [ ] Define row-level tag placement, maximum visible tags, overflow disclosure, and behavior at narrow widths.
- [ ] Decide whether tags are passive display only or whether clicking a tag may activate a filter; defer nested interaction until explicitly approved.

### Verification & Acceptance Criteria
- [ ] **Automated Checks:** Run the Package A frontend checks before beginning organization work.
- [ ] **Functional Assertions:** The decisions cover empty tags, duplicate casing, long values, zero results, archived records, disconnected records, and keyboard behavior.

### Plan Compliance Checklist
- [ ] **Required Files:** Inspect `src/components/modpacks/ModpackList.svelte`, `src/routes/+page.svelte`, Package A tag helper/editor files, relevant UI standards, and current list state.
- [ ] **Boundaries:** No implementation of filtering or display begins until matching and interaction semantics are recorded.
- [ ] **Legacy Code Removed:** None in this decision phase; confirm Package A's comma-separated paths remain removed.
- [ ] **Acceptance Checks:** Decisions are recorded in this plan before Phase 2.

### Phase 1 Handoff & Verification Report
- **Compliance Check:** PENDING
- **Verification Result:** PENDING
- **Execution Proof & Logs:** Pending
- **Artifacts Created/Modified:** Pending
- **Decisions & Deviations:** Pending
- **Next Phase Context:** Pending

---

## Phase 2: Tag Display and Filtering
- **Status:** NOT_STARTED
- **Objective:** Add compact tag visibility and a keyboard-accessible filter without disrupting current modpack selection or lifecycle controls.

### Tasks
- [ ] Add accessible tag chips or labels to registered modpack rows with an intentional overflow treatment.
- [ ] Add tag filter state and controls to the registered-modpack list.
- [ ] Implement the approved matching semantics and combine them with archived visibility predictably.
- [ ] Add zero-match and no-tag states that explain how to clear the filter.
- [ ] Preserve modpack row selection, quick actions, lifecycle controls, responsive layout, and visible focus.

### Verification & Acceptance Criteria
- [ ] **Automated Checks:** Run `npm run check`, `npm run build`, and `git diff --check`.
- [ ] **Functional Assertions:** Rows show zero, one, several, and overflowing tags without unstable layout or inaccessible hidden content.
- [ ] **Functional Assertions:** Filtering returns correct results for single and multiple tags, mixed casing, archived/disconnected records, and no matches.
- [ ] **Functional Assertions:** Clearing the filter restores the correct active/archived list and does not change the focused modpack.

### Plan Compliance Checklist
- [ ] **Required Files:** `src/components/modpacks/ModpackList.svelte`, `src/routes/+page.svelte`, and only focused shared/helper files needed by the approved design.
- [ ] **Boundaries:** Do not change persistence, Packwiz evidence, registration semantics, Favorite semantics, or Package A editor behavior.
- [ ] **Legacy Code Removed:** Remove any temporary placeholder filter/display implementation if the final design replaces it; do not leave duplicate controls.
- [ ] **Acceptance Checks:** Automated and responsive/accessibility assertions are run before handoff.

### Phase 2 Handoff & Verification Report
- **Compliance Check:** PENDING
- **Verification Result:** PENDING
- **Execution Proof & Logs:** Pending
- **Artifacts Created/Modified:** Pending
- **Decisions & Deviations:** Pending
- **Next Phase Context:** Pending

---

## Phase 3: Regression and Future-Scope Review
- **Status:** NOT_STARTED
- **Objective:** Confirm that tag organization improves scanning without turning tags into a second lifecycle, favorite, or Packwiz state system.

### Tasks
- [ ] Verify keyboard and assistive-technology use of the filter and any tag disclosure.
- [ ] Verify list density and responsive behavior with realistic tag counts and long labels.
- [ ] Verify active, archived, and disconnected lifecycle behavior remains unchanged.
- [ ] Review whether autocomplete, presets, or click-to-filter should remain deferred as separate future scope.

### Verification & Acceptance Criteria
- [ ] **Automated Checks:** `npm run check`, `npm run build`, and `git diff --check` pass.
- [ ] **Functional Assertions:** Tags are clearly application-owned, Favorite remains independent, and no Packwiz evidence is implied.
- [ ] **Functional Assertions:** The final behavior matches the Phase 1 organization decisions and does not silently introduce Package C features.

### Plan Compliance Checklist
- [ ] **Required Files:** Phase 2 implementation files and directly related documentation only.
- [ ] **Boundaries:** No autocomplete, preset taxonomy, network suggestion, Packwiz mutation, or provider integration.
- [ ] **Legacy Code Removed:** No duplicate tag display/filter controls or abandoned semantics remain.
- [ ] **Acceptance Checks:** Automated, responsive, accessibility, and lifecycle regression checks are recorded.

### Phase 3 Handoff & Verification Report
- **Compliance Check:** PENDING
- **Verification Result:** PENDING
- **Execution Proof & Logs:** Pending
- **Artifacts Created/Modified:** Pending
- **Decisions & Deviations:** Pending
- **Next Phase Context:** Pending

---

# Overall Plan Completion Status

* **Final State:** IN_PROGRESS
* **Total Phases Completed:** 0 / 3
* **Summary of Outcome:** Future work will expose saved tags in the modpack list and support explicit tag-based filtering after Package A normalization and editor work are complete.
