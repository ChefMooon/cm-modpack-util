# Specification Change Impact Assessment: Projects Dashboard Split Pane

## Decision Summary
- Overall disposition: hand off to planning agent
- Confidence: medium-high
- Scope assessed: Reorganize the root Projects dashboard into a project master-detail workspace with a resizable desktop split pane, a collapsible project-list pane, Summary, Versions, and Settings tabs, while preserving current project lifecycle, inspection, discovery, and safety behavior.
- The request is beneficial and implementable as a frontend information-architecture change. Existing Rust commands and TypeScript contracts already support project metadata updates and durable snapshot reads. The implementation plan should treat Releases as a deliberate placeholder state, not as a new release data source.

## Proposed Changes
### C1: Project master-detail layout and collapsible project list
- Intended outcome: Keep the project list visible beside the focused project's details, while allowing users to collapse the list when they need more room for the detail view.
- In scope: Project cards in a left pane, focused-project detail pane, desktop vertical divider that can be resized, a clearly labeled toggle button to collapse/expand the list, selected-card styling, loading/error/empty states, and responsive stacking on narrow windows.
- Collapse behavior: On desktop, collapse the list to a compact rail that retains an accessible expand control and enough selected-project context to avoid losing orientation. On narrow windows, use the toggle as a show/hide control for the stacked project list rather than forcing a two-column layout. The selected project and active detail state must survive collapse.
- Wireframe structure: The application header remains outside this feature scope. Below it, the project workspace is a bounded two-pane region with a vertical divider; the left pane contains selectable project cards and the right pane contains the focused project's detail surface.
- Layout behavior: The project list and detail content should have independent scrolling within the workspace so a long project list does not push the detail tabs out of view. The divider and panes need stable minimum dimensions and must not resize the surrounding page unpredictably.
- Out of scope: Changing project registration, Packwiz evidence rules, lifecycle semantics, or external project files.
- Dependencies: Existing `ProjectRecord` list and current `inspectProject` loading path; C2-C5 depend on the focused project state from this layout.
- Open assumptions: Selecting a card should focus it and load its details; action buttons such as Check for updates, Refresh, Archive, Disconnect, and Reconnect remain available in the project context without being silently removed.
- Resolved behavior: Summary data loads automatically when a project is selected. Selected project, active tab, divider width, and collapsed-list state persist for the current app session only and reset to sensible defaults after restart.

### C2: Project detail tabs
- Intended outcome: Give the focused project a predictable top-level navigation model with `Summary`, `Versions`, and `Settings` tabs.
- In scope: A keyboard-accessible tablist, active tab state, tab panels, an explicit no-focused-project state, and preservation of the selected tab while the focused project remains selected.
- Wireframe structure: Tabs are local to the right detail pane, visually attached to its top edge, and operate on the focused project only. They do not span across or replace the left project-list pane.
- Out of scope: A new application-wide settings information architecture; the existing `/settings` route remains separate.
- Dependencies: C1 focused-project state; C3 moves snapshot UI into Versions; C4 moves metadata editing into Settings.
- Open assumptions: Summary is the default tab when a project is newly selected unless a deep-linked snapshot requires opening Versions.
- Resolved behavior: Existing `?snapshot=` links select the snapshot's project, activate Versions, and open the existing snapshot review workflow.

### C3: Versions tab with unified filterable table
- Intended outcome: Move snapshot-related UI into a single Versions tab with one unified data table and a segmented `All | Releases | Snapshots` filter above it.
- In scope: Snapshot rows, lifecycle/outcome/status, timestamps, candidate counts or other useful snapshot summary fields, row selection/open-review behavior, loading/empty/error states, and segmented-filter semantics.
- Out of scope: Implementing release discovery, release persistence, release actions, or guessed provider/repository release data.
- Dependencies: Existing `listSnapshots(projectId)` and `getSnapshot(id)` commands and `SnapshotRecord`/`SnapshotCandidateRecord` contracts.
- Open assumptions: Until Releases exists, `All` shows supported snapshot records and `Releases` shows a clear unavailable/empty state explaining that releases are not implemented. The table should not imply that snapshots and releases are already the same record type.
- Resolved behavior: Activating a snapshot row selects its project if necessary, switches to Versions, and opens the existing review workflow. On narrow panes, rows reflow into compact labeled records rather than requiring horizontal scrolling.

### C4: Summary tab for inspection information
- Intended outcome: Move the current inspect/overview/inventory information into Summary so the selected project's evidence is visible in its primary detail view.
- In scope: Project overview metrics, validation evidence, inventory table and its existing filters/actions, recent activity, re-read behavior, and unavailable/stale/error states.
- Out of scope: Changing the meaning of read-only Packwiz evidence, pin/unpin safety flows, provider links, or inventory filtering rules.
- Dependencies: C1 selection and existing `getProjectInventory`/`getProjectOverview` calls.
- Resolved behavior: Summary automatically loads overview and inventory data when a project is selected, with independent loading and failure states. Planning may optimize caching or invalidation, but must not restore an extra Inspect step.

### C5: Settings tab with saved project metadata
- Intended outcome: Let users edit application-owned project information in context and save it with a clear, explicit action.
- In scope: Move the current edit form fields into Settings, including display name, theme/color, tags, description, and favorite; validation and disabled/pending Save state; success toast; inline or banner error; and protection against losing unsaved changes.
- Out of scope: Editing Packwiz-owned evidence, changing the registered filesystem path, or moving lifecycle actions into Settings in this first pass. Archive, disconnect, restore, and reconnect remain separate project actions.
- Dependencies: Existing `updateProjectMetadata(id, application)` wrapper and `update_project_metadata` Tauri command; C1 selected project state.
- Open assumptions: Save is grouped form submission, not autosave. The saved record must replace the selected project record so the card and Summary update immediately.
- Resolved behavior: Leaving Settings with dirty values prompts before switching project, changing tabs, collapsing the list, or navigating away. Auto-save is out of scope.

## Clarifications
- Asked and answered:
	- Narrow windows should stack the project list and detail panes; the desktop divider should not force unusable horizontal scrolling.
	- `All` should show snapshots for now; `Releases` should be present but clearly unavailable or empty until implemented.
	- Settings should replace the current metadata edit modal for metadata only. Lifecycle actions should remain outside Settings.
- Still needed:
	- Exact minimum/maximum pane widths.
	- Whether `All` should later merge releases and snapshots chronologically or group them by type when releases are implemented.
- Assumptions used for this assessment:
	- This is an alpha desktop application and breaking frontend layout changes are acceptable when current workflows remain available.
	- Existing command contracts are authoritative; no network or Packwiz mutation is introduced by this UI change.

## Current-State Evidence
- The root dashboard at [`src/routes/+page.svelte`](../../../../src/routes/+page.svelte) currently owns project loading, project actions, inspection, snapshots, discovery review, changelog UI, and the edit modal in one route. This is the direct owning surface for the requested dashboard change.
- The current project list renders each project as a `.project-row` with status, Packwiz evidence, Inspect, update, edit, refresh, archive, disconnect, and reconnect actions. Inspection is currently rendered below the entire list in an `.inspection` section rather than beside a selected project.
- The provided wireframe shows the intended composition: the existing application header above a bounded workspace, a left project-list pane, a vertical divider, and pane-local tabs above the right detail pane. The top header/project control in the wireframe is not part of this feature request and should remain owned by the existing application header.
- `inspectProject(project)` sets `inspected`, then loads `getProjectInventory(project.id)` and `getProjectOverview(project.id)` in parallel. This provides the existing Summary data path and already has loading and unavailable handling.
- Snapshot UI is currently split between `checkProjectForUpdates`/`openSnapshot` state and a full-width `.discovery-panel`. Snapshot records are loaded through `listSnapshots` and `getSnapshot`; `SnapshotRecord` includes lifecycle, outcome, timestamps, candidates, decisions, notes, and rechecks in [`src/lib/domain.ts`](../../../../src/lib/domain.ts).
- The frontend wrapper [`src/lib/projects.ts`](../../../../src/lib/projects.ts) already exposes `listSnapshots`, `getSnapshot`, `updateProjectMetadata`, and existing snapshot review actions. No new frontend-to-Rust command is required for the stated first pass.
- Project application metadata is typed as `ApplicationProjectMetadata` with display name, icon, theme, tags, favorite, description, and lifecycle. The current edit modal already binds the requested metadata fields and calls `updateProjectMetadata` through `saveMetadata`.
- The backend registers `list_snapshots`, `get_snapshot`, and `update_project_metadata` in [`src-tauri/src/lib.rs`](../../../../src-tauri/src/lib.rs), with persistence implemented in [`src-tauri/src/db/mod.rs`](../../../../src-tauri/src/db/mod.rs). The request does not require a schema migration.
- Existing UI standards in [`docs/desktop-ui-standards.md`](../../../../docs/desktop-ui-standards.md) require keyboard access, visible focus, accessible names, stable layouts, explicit grouped-save behavior, pending/failure handling, and responsive minimum-window behavior. These requirements directly apply to tabs, the divider, the segmented control, the collapse toggle, and Settings.
- The current edit modal already shows a saving state and success toast, using the global toast API documented in [`docs/toast.md`](../../../../docs/toast.md). The new Settings surface should preserve that feedback pattern and add unsaved-change protection.
- The current `ProjectRecord` exposes application metadata, canonical path, lifecycle, validation, Packwiz name/author/version/format/index observations, and `last_opened_at`/`last_refreshed_at`. The project card currently shows only a subset, so the new focused-project header/Summary should make identity, lifecycle, path, metadata ownership, and freshness more visible.
- `ProjectOverview` additionally exposes Minecraft version, loader, inventory counts, Git state, known update count, validation, and recent activity. These should be prioritized into a concise Summary hierarchy rather than presented as one undifferentiated evidence block.
- Freshness must be explicit: show the last successful read/refresh time and a stale or unavailable state when current evidence cannot be trusted. Provide a clear Re-read action without implying that application metadata or external Packwiz files were modified.
- Unknown: The repository does not currently define a release/version record type or release command. Release semantics, source, ordering, and persistence must remain explicitly deferred rather than inferred from Packwiz version fields.

## Impact Findings
### C1: Project master-detail layout and collapsible project list
- Classification: beneficial with conditions
- Positive impact: Keeps project identity and project switching visible while details are reviewed; makes the focused project explicit; creates a stable home for the tabs; and lets users reclaim detail-pane width without losing the current selection.
- Negative impact or unintended consequence: A draggable divider or collapse toggle can create inaccessible or unusably narrow panes, pointer-only interaction, layout shifts, and poor mobile behavior. A collapsed rail that hides all identity can make the detail view disorienting. Moving action controls may make existing lifecycle actions harder to discover.
- Affected surfaces: `src/routes/+page.svelte`, project card markup, inspection/discovery placement, responsive CSS, collapse/resize state, and likely decomposition into focused Svelte components because the current route already handles several responsibilities.
- Dependencies and interactions: C2-C5 need a single authoritative selected project. Snapshot deep links and update-check panels must not become detached from the selected project when cards are changed or the list is collapsed.
- Confidence and rationale: High confidence in value; the current list-plus-below-details structure is directly visible in the route. Conditions are needed for keyboard resize, minimum widths, a focusable labeled toggle, mobile stacking, and preserving action discoverability.
- Discriminating check: At the minimum supported desktop window and a narrow viewport, select multiple projects, resize the divider to both limits, collapse and expand the list, tab through all controls, and verify no content, selection, identity, or action becomes unreachable.
- Required information check: The collapsed rail and focused-project header must retain enough identity to orient the user, including project name and lifecycle/status; the expanded Summary must expose path, Packwiz identity, key runtime versions, counts, validation, Git state, and freshness.
- Required layout check: The workspace remains a bounded split region beneath the application header; the project list and detail pane scroll independently; project cards remain selectable without nesting conflicting interactive controls; and the detail pane stays usable while the list grows.
- Recommendation: proceed

### C2: Project detail tabs
- Classification: beneficial with conditions
- Positive impact: Separates read-only evidence, version history/reviews, and editable metadata into clear user-intent areas; reduces the current dashboard's long vertical scan.
- Negative impact or unintended consequence: Tabs can hide active workflows, reset state unexpectedly, or make snapshot review feel disconnected from the project. A tablist that relies on color or lacks keyboard semantics would regress accessibility.
- Affected surfaces: Root dashboard markup/state, URL/deep-link handling for `?snapshot=`, focus management, and potentially component boundaries.
- Dependencies and interactions: Must coordinate with C3 snapshot review navigation and C4 loading behavior. The selected tab should not silently change while a save, update check, or review action is running.
- Confidence and rationale: High confidence if Summary is the default and the active tab is exposed semantically with native or equivalent accessible tab patterns.
- Discriminating check: Keyboard-only navigation across tabs and a direct `?snapshot=` link should land on the correct project and Versions context without losing the existing review workflow.
- Required interaction check: Switching tabs, collapsing the list, and changing projects must preserve session state unless dirty Settings values trigger the explicit discard confirmation.
- Required placement check: Tabs remain visually and semantically associated with the right detail pane, and selecting a tab does not change or hide the left project-list context.
- Recommendation: proceed

### C3: Versions tab with unified filterable table
- Classification: beneficial with conditions
- Positive impact: Gives snapshots a durable, scannable home and establishes a clean extension point for releases without fabricating release data today. A single table avoids separate, competing lists.
- Negative impact or unintended consequence: `All` can be misleading if it silently means snapshots only; table rows may not contain enough context to distinguish reviewable, stale, cancelled, or failed snapshots; dense tables can become unusable on narrow panes.
- Affected surfaces: Snapshot state/rendering in `+page.svelte`, snapshot loading/error states, existing discovery panel placement, and potentially a reusable table/segmented-control component.
- Dependencies and interactions: Existing snapshot review actions must remain reachable from a row; C2 tab state and C1 project selection control which snapshots are loaded. Releases must remain a placeholder with no guessed API.
- Confidence and rationale: Medium-high because the existing `SnapshotRecord` contract is sufficient for rows, but desired table columns and row interaction are not fully specified.
- Discriminating check: With no snapshots, one snapshot in each lifecycle, and a failed/unavailable load, verify the table communicates state, filter counts/results, and an actionable path to review without implying releases exist.
- Required row behavior: Activating a snapshot row must route to the existing review workflow, preserve the selected project, and present lifecycle, outcome, created time, candidate count, and freshness/status in the row or its responsive equivalent.
- Recommendation: revise then proceed

### C4: Summary tab for inspection information
- Classification: beneficial with conditions
- Positive impact: Makes current inspect information the natural first view for a focused project and keeps evidence, validation, inventory, and activity together.
- Negative impact or unintended consequence: Loading inventory on every card selection could be expensive and noisy; moving existing pin/unpin controls into a smaller pane may cause crowding; stale observation status could be lost if Summary only shows raw values.
- Affected surfaces: `inspectProject`, overview/inventory rendering, inventory filters and pin actions, refresh/re-read controls, and loading/error states.
- Dependencies and interactions: Must preserve the existing read-only boundary and safety-confirmation flow. Planning should define whether data is cached per project and when invalidated after refresh or apply operations.
- Confidence and rationale: High for information architecture; medium for fetch timing because current code fetches only after explicit Inspect.
- Discriminating check: Select a modpack, switch tabs, select another modpack, refresh/apply a change, and return to the first modpack; verify displayed evidence belongs to the focused modpack and reflects invalidation rules.
- Required loading behavior: Selection automatically loads Summary overview and inventory data, with independent loading/error states so a slow or unavailable inventory does not hide the project identity or overview evidence.
- Recommendation: revise then proceed

### C5: Settings tab with saved project metadata
- Classification: beneficial with conditions
- Positive impact: Makes editable information discoverable in the same project context, removes the extra edit-modal step, and reuses an existing typed save command.
- Negative impact or unintended consequence: Users may confuse application-owned metadata with Packwiz-owned evidence; unsaved changes can be lost during project switching or tab changes; a failed save can leave the card, form, and selected project inconsistent.
- Affected surfaces: `beginEdit`, `saveMetadata`, current edit modal fields, selected project state, project card labels, validation/error feedback, and toast usage.
- Dependencies and interactions: Must preserve Rust validation and update the in-memory `ProjectRecord` only after a successful command. Lifecycle actions remain separate and should not be hidden behind Settings.
- Confidence and rationale: High because the command, wrapper, types, and current form already exist. Conditions are grouped-form safeguards required by the UI standards.
- Discriminating check: Edit each supported field, collapse the list or switch tabs/projects with dirty values, submit while saving, force a command failure, and confirm no partial or silently discarded state is presented.
- Required save behavior: A successful save updates the selected project card and Summary immediately; a failed save preserves the draft and exposes a retryable error.
- Recommendation: proceed

## Cross-Change Considerations
- Establish selected-project state and layout boundaries before moving panels. C2 should establish tab state, followed by C4 Summary and C5 Settings, with C3 Versions requiring the most product clarification around row actions and table columns.
- Keep project lifecycle actions visible in a stable project-card or detail-header action area. Moving them into a hidden Settings tab would conflict with the clarified scope and reduce discoverability.
- Preserve the existing `?snapshot=` deep link. A snapshot opened from Activity should select its project and show the relevant review in Versions, or provide an explicit transition from the Versions row to the existing review panel.
- Use the Summary hierarchy in this order: project identity and lifecycle; freshness and primary actions; human-managed metadata; Packwiz identity and runtime compatibility; validation and Git health; inventory counts and recent activity. Keep detailed inventory and raw safety evidence available below or behind an explicit detail surface.
- Show application-owned metadata separately from read-only evidence. Display name, description, tags, favorite, and theme belong to Settings; canonical path, Packwiz identity, observed versions, validation, Git state, and freshness are evidence/context.
- Show `last_refreshed_at` as the primary evidence freshness indicator and distinguish “never read,” “last read at…,” “stale,” and “unavailable.” Do not imply that a metadata save refreshes Packwiz evidence.
- Summary should load overview and inventory automatically on selection, but keep their loading and failure states independent. A project header must remain usable while inventory is loading or unavailable.
- Treat observations and metadata differently in copy and visual treatment: Packwiz evidence is read-only and can be unavailable/stale; application metadata is editable and saved explicitly.
- The divider should be a real keyboard-operable separator with visible focus, min/max widths, a sensible default, and a predictable reset/fallback. Do not rely on pointer dragging alone.
- Match the wireframe's containment: keep the application header outside the project workspace, place the vertical divider between the list and detail panes, and attach the tablist to the detail pane rather than treating it as page-level navigation.
- Give the list and detail pane separate scroll containers where needed. Avoid a layout in which inventory, review evidence, or Settings causes the project cards or detail tabs to scroll away as one undifferentiated page.
- The collapse control needs an accessible name that changes with state, a visible focus indicator, a tooltip only as supplemental help, and a non-color indication of whether the project list is expanded. It must not trap focus inside a collapsed pane or hide the only way to expand it.
- The segmented control should use a semantic single-choice pattern, expose the active choice beyond color, support keyboard navigation where appropriate, and show a disabled or unavailable Releases state without pretending that the filter has data.
- Define narrow-table behavior before implementation: Versions rows reflow into compact labeled records while preserving status, timestamps, counts, and actions. Do not let long paths, timestamps, or diagnostic text resize the entire pane unpredictably.
- Add project-card affordances for selection that work beyond the action buttons. The focused card needs a clear non-color indicator and should not make the entire card an ambiguous nested interactive element.
- Consider a project search/filter control if the registered project list can grow beyond a small number. It is a useful UX addition, but should be a separate scope decision rather than silently included in this refactor.
- Use a confirmation guard for dirty Settings values before project switching, tab changes, collapse, or navigation. The guard should return focus to the initiating control after the decision and must not discard a draft silently.

## Handoff Options
1. **Continue specification assessment**: Decide exact minimum/maximum pane widths and the future ordering/grouping semantics for mixed releases and snapshots. These choices refine implementation details but do not currently block the overall recommendation.
2. **Hand off to the planning agent**: Plan a frontend-focused refactor of `src/routes/+page.svelte` around selected-project, layout, collapse, and tab state; preserve existing `src/lib/projects.ts` and Rust command contracts; automatically load Summary data; add freshness and project-information hierarchy; add Versions, Settings, responsive rows, deep-link behavior, and dirty-form confirmation; move snapshot and inspection UI without changing safety semantics; and include focused validation for keyboard navigation, resize limits, collapse/expand, filters, deep links, save failure, freshness states, and unsaved changes.

## Quality Gate
- Every requested change is identified and mapped to an intended outcome: split pane/project cards and collapse toggle (C1), tabs (C2), Versions and filters (C3), Summary inspection (C4), and Settings save flow (C5).
- Material clarifications were asked and answered for mobile layout, temporary Releases behavior, and Settings scope.
- Current-state claims are grounded in the root dashboard, frontend wrappers/types, registered Tauri commands, database persistence, and UI standards.
- Benefits, risks, affected surfaces, dependencies, and cross-change interactions are explicit for each change.
- Each classification includes a cheap discriminating check.
- Remaining decisions are visible before planning, with a recommendation to hand off because they refine rather than block the scoped change.
- No application code was modified; only this specification document was created or updated.
