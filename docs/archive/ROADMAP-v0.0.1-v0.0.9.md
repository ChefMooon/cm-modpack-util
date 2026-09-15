# Archived Roadmap: v0.0.1-v0.0.9

> This roadmap is historical. It records the original implementation sequence and is preserved for context. See [the current 0.1.0 readiness roadmap](../ROADMAP.md) for active release work.

# CM Modpack Util Roadmap

## Purpose

This roadmap ordered the initial implementation of CM Modpack Util into small, independently plannable milestones. It was a high-level delivery sequence, not a replacement for the product specification or the implementation plan for any individual version.

The sequence followed three boundaries:

- Packwiz files remain authoritative for the current modpack state.
- SQLite owns application metadata, decisions, cached data, and history.
- Rust owns filesystem access, Packwiz processes, network access, persistence, safety checks, and domain-oriented Tauri commands.

Each minor version was intended to have its own plan covering detailed scope, data changes, UI work, tests, and acceptance criteria.

## Release Sequence

### v0.0.1 - Productize the Existing Template (Complete)

Turn the initial desktop shell into the first CM Modpack Util application foundation. This milestone built on the existing starter instead of recreating its infrastructure.

#### Already Provided by the Template

- Tauri 2, SvelteKit, TypeScript, Vite, and Rust integration.
- A bundled SQLite connection initialized during Tauri startup.
- Settings persistence through typed `get_settings`, `set_setting`, delete, and reset commands.
- Window state restoration, tray behavior, autostart, and the opener plugin.
- Shared Button, Modal, Tooltip, and Toast UI primitives.
- A settings route with theme, reduced-motion, startup, and tray preferences.
- `npm run build` and `npm run check` validation scripts.

#### Remaining Foundation Work

- Rename the package, Rust crate, Tauri product metadata, window title, tray labels, identifiers, and visible template copy for CM Modpack Util.
- Connect the first application shell to Packwiz project operations instead of application shell and navigation placeholders for Projects, Activity, and Settings.
- Preserve and test the existing settings behavior while moving shared app state and Tauri invocation patterns into product-owned modules.
- Define the first domain contracts for projects, operations, validation results, structured errors, and operation status without implementing project registration yet.
- Establish the Rust boundary for canonical paths and registered-project scoping so later filesystem and Packwiz work cannot bypass it.
- Retain the template's no-migration database approach through the initial version. Product tables should continue to be initialized through `CREATE TABLE IF NOT EXISTS`; introducing a migration system is post-initial-release work and must be planned before the schema changes afterward.
- Add foundation tests for database initialization, settings persistence, command error translation, path-boundary rules, and the initial frontend routing/shell states.
- Keep network access, Packwiz execution, project registration, and modpack file mutation out of this milestone.

**Exit gate:** The renamed CM Modpack Util desktop app starts through `npm run tauri dev`, passes `npm run check` and `npm run build`, preserves the template settings workflows, exposes a stable product shell, and has tested domain/path/database boundaries ready for project registration in `v0.2`.

### v0.0.2 - Project Registration and Validation (Complete)

- Select a local directory and validate it without network access or mutation.
- Parse `pack.toml`, the referenced index, and supported mod metadata files.
- Show a registration preview containing Packwiz-declared values and validation results.
- Canonicalize project paths and reject duplicate registrations.
- Store application-owned project metadata such as display name, theme, tags, favorite state, description, and lifecycle status.
- Support opening, editing, archiving, restoring, and reconnecting projects without deleting external files; retain unavailable projects as disconnected records.
- Record refresh timestamps and derived project facts.

**Exit gate:** A valid project can be registered and reopened offline, invalid or inaccessible projects produce understandable results, and registration does not modify the selected directory.

### v0.0.3 - Inventory and Project Overview

- Read the current mod inventory from Packwiz metadata.
- Display local entry identity, name, version, file path, provider, side, pin state, source URL, and unknown values where evidence is missing or malformed.
- Treat `pin = true` in local metadata as pinned and an absent `pin` field as the current unpinned (`false`) Packwiz observation; keep malformed values explicit and use `packwiz pin` and `packwiz unpin` for changes in a later mutation milestone.
- Derive provider and client/server-side counts from current files.
- Add trustworthy **Open mod page** actions without making provider requests just to render the inventory.
- Show project validation, Minecraft version, loader information, Git status, mod counts, update counts when known, and recent application activity.
- Add refresh behavior that re-reads local evidence and clearly reports failures.

**Exit gate:** Users can inspect a project and its mod metadata without changing files, and the interface never presents inferred provider or side data as fact.

### v0.0.4 - Packwiz Compatibility and Safe Update Checks

- Identify and record the tested Packwiz compatibility profile.
- Implement observable Packwiz process execution in the registered directory.
- Run `packwiz update -a` as an interactive cancellation probe.
- Detect the expected prompt, send only `n`, capture output, and verify the expected cancellation result.
- Capture before and after project fingerprints and reject changed state.
- Parse update candidates only when prompt handling, cancellation, compatibility, and post-cancellation validation all succeed.
- Report unsupported, unsafe, and indeterminate outcomes distinctly from normal update-check results.

**Exit gate:** **Check for updates** never sends `y`, never claims authoritative candidates without all required evidence, and does not modify the project during a normal successful update check.

### v0.0.5 - Snapshot Review and Decision Tracking

- Create snapshots for update sessions, including review, cancelled, unsafe, and indeterminate outcomes.
- Persist candidate observations and the before-state fingerprint.
- Add selected, skipped, blocked, and deferred decisions.
- Persist project-level and snapshot decision notes.
- Show pins, existing notes, severity classification, provider identity, side, and compatibility warnings during review.
- Invalidate a review when relevant project files change externally.
- Preserve immutable operation history and make retries create new linked work.

**Exit gate:** A user can review and explain every candidate decision, close or cancel a session without changing Packwiz files, revisit the session later, and start a fresh linked retry when native freshness evidence requires it. Update application and pin mutation remain deferred to v0.0.6.

### v0.0.6 - Applying Updates and Pin Management

- Apply only a confirmed update set supported by the compatibility profile.
- Capture command output, errors, exit status, progress, and cancellation state.
- Block invalid projects, concurrent operations, active Git conflict states, stale reviews, and missing before-state evidence.
- Require acknowledgement when Git or another recovery path is unavailable.
- Re-read and validate Packwiz files after the operation.
- Classify results as successful, partially successful, failed, or cancelled.
- Implement exact-file interactive `packwiz pin` and `packwiz unpin` flows.
- Never use ambiguous `--yes` selection for pinning or unpinning.
- Record the resulting Packwiz state and preserve failed or partial snapshots.

**Current implementation boundary:** The audited Packwiz binary supports verified positional-slug pin/unpin operations and targeted update operations, delivered with immutable operation history, cancellation evidence, confirmation UI, and post-read verification. The all-target `packwiz update -a` form remains discovery-only.

**Exit gate:** Updates are applied only from an explicit review, post-operation validation determines the outcome, and no operation is reported successful based only on process startup or exit status.

### v0.0.7 - Changelog Generation and Markdown Export

Complete the initial update-to-changelog workflow using targeted Modrinth access. The supported boundary is explicit snapshot generation, visible provider evidence, durable artifacts and revisions, and native Markdown export. Automatic cache expiration, CurseForge API access, Packwiz mutation, releases, and cleanup remain outside this milestone.

- Identify changed mods from a selected snapshot.
- Resolve Modrinth project and version identities with confidence and evidence.
- Request only the changelog data required for the selected snapshot.
- Cache exact provider responses with retrieval and freshness information.
- Support exact cached data when offline.
- Make missing, ambiguous, unresolved, stale, and failed lookups visible.
- Accept an optional changelog introduction.
- Store generated changelog artifacts and generation status durably.
- Support editable revisions while preserving the original generated result.
- Export Markdown and record destination, time, format, and exported content or hash without deleting the stored artifact.

**Exit gate:** A user can generate, review, edit, export, and revisit a snapshot changelog, while confirmed and unconfirmed provider information remain distinguishable.

### v0.0.8 - Releases and Captured State Comparison

- Create named releases from captured, validated local state.
- Support release notes, descriptions, publication status, and optional links to snapshots.
- Capture the complete relevant Packwiz state independently of current files.
- Record optional Git repository, branch, commit, tag, cleanliness, and remote provenance.
- Allow releases without Git and mark unavailable commit reconstruction clearly.
- Compare two releases using captured state rather than the current working tree.
- Show added, removed, changed, and metadata-changed mods, loader changes, release notes, changelog information, and available commits.
- Add optional public GitHub metadata enrichment without making it a dependency.

**Exit gate:** Historical release comparisons remain meaningful after the project changes, moves, disconnects, or loses access to GitHub.

### v0.0.9 - Data Management, Cleanup, and Hardening

- Add archive, restore, detach, and permanent-delete workflows for supported records.
- Show dependency and external-reference impact previews before destructive actions.
- Require typed confirmation for permanent deletion.
- Preserve external project directories, Git history, and user export files.
- Identify orphaned records and unavailable export destinations.
- Add scoped cleanup for cached provider metadata and obsolete export records.
- Show storage usage and removable data.
- Complete keyboard, focus, status communication, and non-color accessibility requirements.
- Exercise Windows path behavior, process cancellation, partial operations, malformed TOML, disconnected projects, offline caches, and recovery warnings.
- Document supported Packwiz compatibility and known limitations.

**Exit gate:** Application data can be managed without database editing, cleanup is explainable and reversible by default, and the critical safety boundaries are covered by automated tests.

## Cross-Version Practices

Every version included the smallest relevant slice of:

- Rust domain contracts and structured error translation.
- Svelte loading, empty, success, failure, and cancellation states.
- Unit tests for parsing and business rules.
- Integration tests for filesystem, SQLite, and Packwiz boundaries where practical.
- Acceptance checks tied to the corresponding section of `docs/CM-MODPACK-UTIL-SPEC.md`.
- Documentation updates for user-visible behavior and compatibility limits.

## Deferred Beyond the Initial Sequence

- CurseForge API integration and CurseForge-native changelog retrieval.
- Private GitHub authentication.
- Cloud synchronization, collaboration, and user accounts.
- Unattended updates and automatic dependency conflict resolution.
- Multiplayer server deployment or runtime management.
- Changelog export formats other than Markdown.
- Automatic cache expiration policies.
- Application-created backups as a replacement or supplement for Git.
