# CM Modpack Util Spec

## Initial Brainstorm (KEEP AS IS)

The main goal of this application is to make updating a modpack easier. specifically gathering and formatting mod changlog information.

Basic Info
- This is a tuari/rust/svelte application. it is the basic template i have right now with sqlite and basic ui setup.
- Use packwiz commands to manipulate a local modpack managed with packwiz
  - Use packwiz `.toml` files as local evidence of mod information
- Utilize modrinth API to get changelog information
  - Ensure API usage is minimal. only get changelog information when generating changelog.
- This project is managed by git if that is useful.

Initial Features:
- Modpack Project management
  - Able to manage different modpacks. 
    - Each has a directory
    - Basic icon support(to be expanded in future), able to select project color theme.
    - Add tags for sorting (any other useful properties?)
    - Project overview and snapshots for each version Should be able to create a new version and this will begin the update. (not sure how the versions should work)
- Lightweight Packwiz wrapper
  - Check available mod updates
    - Should detect versions and show major/minor/bugfix with visual differences
    - Detect pinned mods
    - Able to pin/unpin mods

Random Ideas:
- Should be able to leave a note on a mod if not updating to give a reason. for example if the update causes a crash.

## Product Intent and Success Criteria

CM Modpack Util is a local-first desktop application for reviewing modpack updates and producing useful mod changelogs. It provides a focused workflow around an existing Packwiz-managed modpack without replacing Packwiz or becoming a second source of truth for modpack files.

The application is successful when a user can:

- Register and switch between multiple local modpack projects.
- Inspect available updates using the local Packwiz TOML files as evidence.
- Review update severity, pins, notes, and compatibility concerns before making changes.
- Apply an explicitly approved set of updates through Packwiz.
- Preserve a useful history of each update session.
- Generate a readable changelog for an update session with minimal external API usage.
- Understand what happened when an update succeeds, partially succeeds, or fails.

The broad initial release includes the core update and changelog loop, named
releases and captured release-state comparison, Git status and provenance,
public GitHub metadata enrichment, archive and restore workflows, and
application-owned cleanup. These capabilities remain subject to the explicit
compatibility and safety boundaries in this specification. CurseForge API
integration, private GitHub authentication, unattended updates, and export
formats other than Markdown remain outside the initial release.

## Users, Roles, and Permissions

The initial product has one local user role: the owner of the desktop installation.

The owner can:

- Add, edit, remove, and open local modpack projects.
- Read and modify files within a registered project directory through supported Packwiz operations.
- Configure project metadata, tags, themes, pins, notes, and snapshots.
- Request Modrinth lookups and generate changelogs.

There is no collaboration, account system, cloud synchronization, or multi-user permission model in the initial scope.

## Core Capabilities and Workflows

### Project Management

A project represents one local Packwiz-managed modpack directory. Project metadata includes:

- Display name.
- Directory path.
- Optional icon.
- Project color or theme selection.
- User-defined tags.
- Favorite state for quick access and filtering.
- Optional description.
- Lifecycle status, such as active, maintenance, or archived.
- Creation, modification, and last-opened timestamps.
- Last project scan timestamp.

When a directory is chosen for registration, the application should parse its
`pack.toml` and show a confirmation preview before creating the project. The
preview may populate the application project display name from `name`, while
also showing the Packwiz-declared author, pack version, and pack format. The
application may retain those values as observed Packwiz metadata, but they are
not silently written back to `pack.toml`. The application-owned display name
remains editable after registration and may differ from the Packwiz name.

The application also maintains derived project state based on the selected directory and its contents:

- Validation status.
- Packwiz pack format and index validation status.
- Observed pack author, declared pack version, and declared Packwiz metadata.
- Minecraft version.
- Mod loader.
- Mod loader version when declared.
- Mod count.
- Git repository and working-tree status, when available.
- Available update count.
- Last successful update timestamp.
- Download-provider counts and client/server-side counts for the current mod inventory.

The application must validate a directory before treating it as a usable project. Initial validation requires a readable `pack.toml` and the relevant readable mod metadata files needed by the supported Packwiz workflow. Registration should read, when present, the top-level `name`, `author`, `version`, and `pack-format` values; the `[index]` file reference, hash format, and hash; and the `[versions]` entries such as `minecraft`, `neoforge`, or other declared loader/tool versions. These values provide registration defaults and validation evidence, while the directory path and Packwiz files remain the external project's source of truth. A missing optional field should be shown as unavailable; a malformed required field or unreadable referenced index must produce a clear validation result rather than a partially trusted project.

The registration preview must distinguish values read from `pack.toml` from application-owned fields such as theme, tags, favorite state, description, and lifecycle status. Choosing a directory must not run an update, modify Packwiz files, or require a network request.

### Mod Inventory Metadata

The project inventory must make each mod's download source and execution side easy to scan. The application reads this information from the local Packwiz metadata file:

- A `[update.modrinth]` table identifies Modrinth as the download provider.
- A `[update.curseforge]` table identifies CurseForge as the download provider.
- The `side` field identifies whether the mod is `client`, `server`, or `both`.

The inventory should show provider and side as text, badges, icons, or equivalent accessible labels that do not rely on color alone. It should support seeing provider and side alongside the mod name and version in both the normal inventory and update review. Missing, malformed, or unsupported values must be shown as unknown rather than inferred silently. Provider counts and side counts may be used in the project overview and inventory filtering, but they must be derived from the current Packwiz files.

Each mod may expose an **Open mod page** action. The action opens the system browser only when a trustworthy page URL is available, such as an explicit source URL or a safely derived Modrinth project page from a known Modrinth project identity. A CurseForge project ID alone is not sufficient to guess a stable page slug; the action must be unavailable or request resolution when no trustworthy URL is known. Opening a page must never trigger a provider API request merely to render the inventory.

### Project Overview

The overview should show the active project, favorite state, lifecycle status, current validation state, Minecraft version, mod loader, mod count, download-provider counts, client/server-side counts, Git status when available, available updates, pinned mods, deferred decisions, recent snapshots, and the latest operation result. Opening a project must not modify its files or silently run an update.

### Update Discovery and Review

Update discovery uses the Packwiz command `packwiz update -a` to obtain the fastest available list of update candidates. Because this command can proceed into an update operation, the application must run it as a controlled interactive process:

1. Run `packwiz update -a` in the registered project directory.
2. Capture the proposed update list and command output.
3. Detect the update confirmation prompt.
4. Send `n` to cancel before any update is applied.
5. Confirm that the command completed as a cancellation and that the project state did not change.
6. Parse and return the captured candidates for user review.

The application must never send `y` during update discovery. Discovery is only valid when the confirmation prompt was handled, cancellation completed, and post-cancellation validation indicates that no files were modified. If the prompt cannot be detected, cancellation is ambiguous, or the project state changed, the result must be reported as unsafe or indeterminate rather than as a normal update list.

Packwiz output is authoritative for which mods are currently presented as update candidates. The application may enrich those candidates with local metadata, Packwiz pin state, notes, and provider identity information. A pinned mod may not appear in Packwiz's automatic update candidates because Packwiz is instructed not to update it automatically; the application should still show the mod and its pinned state when displaying the project inventory.

Candidate information may include:

- Local file and mod identity.
- Current and available versions.
- Download provider, provider project identity, and source URL when available.
- Client/server side: `client`, `server`, `both`, or unknown.
- An available action to open the mod page in the system browser.
- Version change classification as major, minor, bugfix, or unknown.
- Pin state.
- Existing notes or compatibility warnings.
- Whether a Modrinth identity is known, inferred, unresolved, or user-confirmed.

Version classification is a visual aid rather than a universal semantic-version guarantee. Mod versions that cannot be reliably classified must be shown as unknown instead of being forced into a major, minor, or bugfix category.

Discovery does not apply changes. The user reviews candidates and explicitly confirms which updates should be applied. The application should use a separate explicitly confirmed Packwiz operation for applying selected updates rather than reusing the discovery process as an implicit approval mechanism.

Discovery is supported only for a tested Packwiz compatibility profile. The
profile records the executable version, expected prompt and output patterns,
supported command forms, and known limitations. If the installed Packwiz
version is outside the profile, or if prompt detection, cancellation, or
post-cancellation verification is inconclusive, the application must report
discovery as unsupported, unsafe, or indeterminate rather than presenting the
candidate list as authoritative.

### Applying Updates

The application uses Packwiz commands to apply the selected updates. The operation must:

- Run against the registered project directory.
- Capture command results, output, errors, and exit status.
- Report progress for work that takes noticeable time.
- Support cancellation where Packwiz and the operating system make that practical.
- Re-read the relevant TOML files after the operation.
- Record a successful, partially successful, or failed result in the update snapshot.

The application may apply a selected subset only when the supported Packwiz
compatibility profile proves that each operation targets the exact intended
metadata file or candidate. If exact targeting cannot be proven, the
application must not emulate Packwiz by editing download metadata itself; it
must either require the complete Packwiz update set or refuse the operation and
explain why selective application is unavailable.

The application must not report success merely because a Packwiz process started or returned without an immediately visible error. Post-operation file validation is part of determining the result.

When Git or another recovery path is unavailable, the application must show a prominent warning before applying updates and require explicit user acknowledgement that automatic rollback is unavailable. The snapshot records whether Git or another recovery path was detected and whether the warning was acknowledged.

Updates are blocked when the application cannot establish exclusive operation
access, cannot read and validate the project, detects an active Git conflict,
merge, or rebase state, or cannot capture a usable before-state. No-Git and
dirty-Git projects may proceed after acknowledgement. A clean Git tree is
reported as stronger recovery evidence, not as a guarantee that rollback is
automatic or complete.

### Pins, Skips, and Notes

Persistent pinning is owned by Packwiz. When the user pins a mod, the application invokes `packwiz pin` for the selected file. When the user unpins a mod, the application invokes `packwiz unpin` for the selected file. The application then re-reads and validates the mod metadata so Packwiz's resulting state is confirmed and displayed.

The Packwiz pin and unpin commands may prompt when a file must be selected. The
Rust wrapper must capture the prompt, present or select one exact local
metadata path, and refuse to continue when the prompt cannot be disambiguated.
The `--yes` option must not be used for this flow because Packwiz documents
that accepting all prompts may select unwanted search results. The application
then re-reads and validates the metadata and records the resulting Packwiz
state. SQLite may retain the observation and operation history, but it is not a
second authority for the current pin state.

Snapshot-specific skips, blocks, and notes remain application-owned decisions. A pinned mod may be absent from Packwiz's automatic update candidate list, but it remains visible in the project inventory and can be unpinned before a future update check.

The product distinguishes between:

- **Pinned:** an ongoing Packwiz-owned policy that prevents the file from receiving automatic updates.
- **Skipped:** a decision made for one update snapshot.
- **Blocked:** an update that cannot currently be applied because validation or compatibility rules reject it.

Notes may explain a project-level policy or a decision in a particular snapshot. Typical examples include a crash, a dependency concern, or a decision to wait for another mod.

### Update Snapshots

A snapshot represents one update session, not necessarily a public modpack release version. Creating a new snapshot begins the review and update workflow.

Snapshots are useful even when no release is created. They preserve the decisions and results of an update attempt, including updates that were skipped, blocked, cancelled, or only partially completed. The normal update flow begins with a snapshot, and a completed snapshot may then be used as the basis for creating a release.

A snapshot records:

- Project identity.
- Creation and completion times.
- Optional user-facing label.
- State before the update.
- Selected, applied, skipped, pinned, and blocked mod decisions.
- Notes associated with decisions.
- Packwiz operation results.
- State observed after the operation.
- Generated changelog content and its retrieval status, when applicable.

An optional release version can be associated with a snapshot later, but release numbering is not required for the initial workflow. A snapshot does not automatically become a release: the user creates a release when the resulting local repository state is ready to be named and recorded.

### Release Tracking and Version Comparison

The application tracks public modpack releases separately from update snapshots. A release represents a named version of a project and records the local modpack state observed at the time the release is created.

A release may reference:

- User-facing release version, such as `1.4.0`.
- Optional release name or description.
- Optional release notes written by the user.
- Git repository path and remote information.
- Optional exact Git commit SHA.
- Optional Git tag or GitHub release.
- Repository branch at the time the release was recorded.
- Whether the repository was clean when the release was recorded.
- Release creation date and publication date, when known.
- The associated update snapshot, when the release was produced by this application.
- Captured Packwiz project state, including the mod list, versions, download-provider declarations, mod-side declarations, source URLs when available, Minecraft version, mod loader, and other relevant release evidence.

The captured release state is the authoritative record for a release created by the application. A release can exist without a Git repository or commit. When a commit is available, it provides optional provenance and an additional way to reconstruct or verify the release state. The application should capture the local state directly rather than requiring the user to commit changes before creating a release.

Release comparison should allow the user to select two releases and see:

- Mods added and removed.
- Mods whose versions changed.
- Mods whose provider or source metadata changed.
- Changes in Minecraft version or mod loader when present.
- The associated update notes and generated changelog information.
- Release notes written for the selected release.
- The commits between the two release commits, when both releases belong to the same repository history.

For releases that include a commit, the application may read historical files directly from Git objects to verify or enrich the captured state. It must not silently checkout an old commit into the user's active working tree. A temporary isolated worktree or equivalent read-only reconstruction may be used only when a Packwiz operation requires a filesystem directory rather than individual historical files.

GitHub integration can enrich release records with remote repository links, tags, GitHub release metadata, commit links, and release notes. It is optional for the local-first workflow: a local Git commit remains sufficient to track and compare a release. Public repositories may be readable without authentication; private repository access will require explicit user authorization or a configured credential and must not be assumed.

### Changelog Generation

Changelog generation is an explicit action for a selected snapshot. It is not part of every update check.

The user may provide an optional introduction message during changelog generation. When provided, the message appears at the beginning of the generated changelog before the mod-by-mod changes. The introduction is useful for release context, compatibility warnings, migration guidance, or other information that does not belong to an individual mod entry.

When generating a changelog, the application:

1. Identifies the mods changed in the selected snapshot.
2. Resolves each mod to a Modrinth project and relevant version where possible.
3. Requests changelog information only for the required mods and versions.
4. Places the optional introduction message at the beginning, when provided.
5. Formats the results into a reviewable changelog.
6. Stores the generated result, introduction message, generation settings, and retrieval status with the snapshot so it can be revisited without repeating the same request.

Generated changelogs are durable application records and can be referenced from the snapshot or an associated release at a later date. The user may export a generated changelog to a selected file format and location. An export records the format, destination, export time, and exported content or content hash, while the application retains the generated changelog independently of the exported file.

The user may create an editable revision of a generated changelog before export. The original generated result must remain preserved, and the revision must identify the user edits and the source artifact from which it was created. The initial export format is Markdown; other formats are deferred.

Exporting a changelog must not remove or replace the stored generated result. Re-exporting the same changelog may create a new export record, allowing the application to show the latest export while preserving the history of previous exports.

A missing or ambiguous Modrinth match must be visible to the user and must not be presented as confirmed changelog information.

### Data Management and Cleanup

All application-owned data must have a clear management path. The user should be able to view, edit, archive, restore, and delete supported records without directly editing the SQLite database.

Data management must support:

- Editing project metadata, tags, pins, notes, release details, and changelog introductions.
- Archiving projects, snapshots, releases, and changelog artifacts without immediately destroying their history.
- Restoring archived records when their referenced project data is still available.
- Permanently deleting selected application records after an explicit confirmation.
- Removing stale or unresolved provider metadata and refreshing it when needed.
- Removing obsolete changelog exports from the application’s export history without deleting the underlying generated artifact.
- Cleaning cached external metadata according to user-selected scope or retention rules.
- Identifying orphaned records, such as exports whose artifact was deleted or snapshots whose project was removed.
- Showing the records and external files affected before a destructive cleanup is confirmed.

Cleanup operations must preserve referential integrity. Deleting a project from the application must not delete its external modpack directory, Git repository, Git history, or user-created export files. Cleanup archives records by default. Permanent deletion of a release or snapshot must show dependent changelog artifacts, exports, notes, and decisions in an impact preview and allow only the fixed supported choices for archiving or detaching records whose historical meaning remains intact. The application must not silently cascade destructive deletion.

Permanent deletion of projects, releases, or changelog artifacts requires an impact preview and typed confirmation. Archived records are hidden from normal search and comparison results by default, with an explicit option to include them.

Application data management should include a way to inspect storage usage and identify records or cached data that can be removed. Database cleanup must not remove the only stored copy of a generated changelog or captured release state without an explicit destructive action.

## Provider and Mod Identity Rules

Packwiz files are the first source of evidence regardless of where a mod was originally downloaded. A mod may originate from Modrinth, CurseForge, or another provider.

The initial release uses Modrinth as the changelog provider even for mods whose download source is CurseForge. High-confidence Modrinth matches may be accepted automatically. Ambiguous matches require user confirmation, and unresolved matches remain visible without being silently assigned. Possible evidence includes:

- A Modrinth project identifier or URL already present in the local metadata.
- A recognizable Modrinth URL or slug.
- A project name, author, and compatible game or loader metadata that can be matched through Modrinth discovery.
- User confirmation when automatic matching is ambiguous.

The download provider must be read from the Packwiz update table independently of the changelog provider. The application must preserve whether the local file declares `update.modrinth`, `update.curseforge`, or an unknown/unsupported provider, and must not relabel a CurseForge download as a Modrinth download merely because Modrinth supplies changelog data.

The application must retain the distinction between the download provider and the changelog provider. A CurseForge download does not imply that CurseForge API access is required for the initial release.

Identity is dual. The local Packwiz entry identity owns current-file notes and
update decisions, while the external provider project and version identity owns
changelog matching. A historical observation retains the exact local metadata
path, displayed fields, source metadata, resolved provider identity, and match
confidence so a provider rename or local file move does not rewrite history.

CurseForge API integration is a future enhancement. It may require an API key, additional configuration, provider-specific rate limiting, and separate handling of CurseForge changelog data. The initial architecture should leave room for multiple metadata providers without requiring that enhancement now.

## Domain Concepts and Business Rules

The core concepts are:

- **Project:** application metadata associated with one local Packwiz directory, including its favorite state, description, lifecycle status, tags, and timestamps.
- **Mod record:** an observation of a mod entry read from project files.
- **Local entry identity:** the stable application identity for a Packwiz metadata entry, including its observed path and local evidence.
- **Download provider:** the provider declared by the local Packwiz update table, such as Modrinth or CurseForge.
- **Provider identity:** the known or unresolved identity of a mod on an external provider.
- **Provider version identity:** the provider-specific version associated with a changelog request, distinct from the local version string.
- **Mod side:** the Packwiz `side` value describing whether a mod runs on the client, server, both, or has an unknown value.
- **Mod page link:** a trustworthy external project-page URL associated with a local mod entry; it may be unavailable even when a provider or project ID is known.
- **Pin:** a durable Packwiz-owned update policy managed by the application through `packwiz pin` and `packwiz unpin`.
- **Snapshot:** one review-and-update session for a project.
- **Release:** a named modpack version backed by a captured Packwiz state, with optional Git repository provenance.
- **Release status:** the publication state of a release or snapshot, separate from whether its update operation succeeded.
- **Snapshot decision:** the selected, skipped, applied, pinned, or blocked outcome for a mod in a snapshot.
- **Note:** user context attached to a project, mod policy, snapshot decision, or release.
- **Release notes:** user-authored context describing a named release, separate from generated changelog content and individual mod decision notes.
- **Changelog introduction:** optional user-authored text placed at the beginning of one generated changelog.
- **Changelog artifact:** durable generated changelog content associated with a snapshot or release.
- **Changelog export:** a file produced from a changelog artifact, with its format, destination, timestamp, and export history recorded.
- **Archive state:** whether an application record is active, archived, or pending permanent deletion.
- **External metadata cache:** stored provider responses used to avoid unnecessary repeated requests.

Important rules:

- Packwiz files remain authoritative for the current modpack configuration.
- The current download provider and mod side are read from Packwiz metadata and are displayed as observations, not application-owned overrides.
- Provider statistics and side statistics are derived from the current inventory and may become unknown when the corresponding Packwiz values are missing or malformed.
- Application metadata must not silently overwrite Packwiz state.
- Historical snapshots must remain understandable after the current project files change.
- Packwiz owns persistent pin state, and the application reflects the state found in the mod metadata.
- SQLite may retain pin observations and operation history, but it is not authoritative for current pin state.
- Pinning and unpinning must target one exact Packwiz metadata file.
- The application must not use Packwiz's `--yes` option when doing so could select an unintended file.
- Unknown version changes remain unknown rather than being misclassified.
- External changelog data is not confirmed until its provider identity and version association are known.
- A failed or partial operation is recorded as such and is never represented as a successful snapshot.
- A release created without Git remains valid because its captured modpack state is stored with the release.
- A release with a Git commit may be reproducible or verifiable from that commit, subject to the commit remaining available.
- Comparing releases uses their captured release states and never whichever files happen to be in the current working tree.
- A provider match must retain its confidence and evidence; an unresolved or ambiguous match is never treated as confirmed changelog data.
- Opening a mod page uses only a trustworthy known URL; the application must not construct a guessed CurseForge URL from a project ID alone.

## Data and Lifecycle Expectations

The application database stores projects, tags, pin observations, notes, snapshots, releases, release-to-snapshot associations, decisions, generated changelog artifacts, changelog exports, cached provider metadata, and application settings. The selected project directory stores the actual Packwiz configuration and mod metadata, while Git stores versioned repository history when available. Packwiz remains authoritative for current persistent pin state, current mod configuration, download-provider declarations, mod-side declarations, and source URLs present in the files. Historical observations and captured release states retain these values so comparisons do not depend on the current working tree.

Projects can be registered, validated, opened, edited, marked as favorite, assigned a lifecycle status, archived, restored, removed from the application, and reconnected to a directory if it moves. Registration uses a canonical directory path and rejects duplicate registrations of the same directory. A missing or inaccessible path is marked disconnected; reconnecting requires an explicit user-selected directory that passes validation and preserves the application project identity. Removing a project from the application must not delete the external modpack directory unless a future explicit destructive workflow is designed and confirmed. Removing or archiving a project must clearly explain what happens to its snapshots, releases, notes, changelog artifacts, exports, pins, and cached metadata.

Snapshots progress through operational states that distinguish at least review, applying, completed, partially completed, failed, and cancelled outcomes. A retry after a partial, failed, or cancelled operation creates a new linked snapshot or operation; the original outcome remains immutable. Applying a snapshot is invalidated when the relevant project state changes externally after discovery, and a fresh discovery/review is required. A completed or otherwise validated snapshot may be associated with a release after the desired local repository state has been captured. Creating a release does not require a Git commit. Releases may have a publication status such as draft, provisional, in review, ready, released, or superseded. A provisional release may capture a validated state from a partial or failed operation, but its status must remain visible and it must not be represented as a normal publishable release. Operational status answers whether the update operation worked; release status answers where the resulting version is in the publishing workflow. A release may contain user-authored release notes in addition to generated changelog content and snapshot or mod decision notes. A snapshot should preserve enough pre-operation and post-operation evidence to explain the result.

Release records should retain the captured modpack state even if the repository is later moved, renamed, or disconnected. When a commit is recorded, the release should also retain the commit SHA and relevant remote or tag information. If that commit cannot be found locally or through the configured remote, the release remains fully comparable using its captured state but is marked unavailable for commit-based reconstruction or verification.

Generated changelog artifacts remain referenceable from their snapshot or release after export. Export records preserve the relationship between the artifact and each file produced from it. If an exported file is moved, renamed, deleted, or becomes inaccessible, the stored artifact and export history remain available in the application, while the affected export is marked unavailable at its recorded destination.

Snapshots, releases, and changelog artifacts should be independently archivable where possible. A cleanup operation may remove cached provider responses or unavailable export references without removing the captured release state or generated changelog content. Permanent deletion must be explicit and must describe whether dependent records will be detached, archived, or deleted.

Cached Modrinth data should include retrieval context and timestamps. Cache expiration and refresh rules can be refined when the first provider integration is implemented, but changelog generation must remain usable when the network is unavailable if the required data was previously cached.

## Non-Functional Requirements

- **Local-first:** Core project browsing, stored history, pins, notes, and previously cached changelogs work without network access.
- **Data safety:** Operations are limited to the registered project directory and provide clear results for failures and partial completion.
- **Minimal API use:** Modrinth is queried for changelog generation and related identity resolution only when required, not on every project open or routine local inspection.
- **Explainability:** The user can see why a mod is listed, skipped, pinned, blocked, or missing changelog information.
- **Responsiveness:** Long-running Packwiz and network work does not freeze the interface and exposes progress or a clear busy state.
- **Recoverability:** Git status or another backup/recovery mechanism may be detected and reported, but Git is not required for the initial product unless a later safety decision makes it mandatory.
- **Data manageability:** Application-owned records can be edited, archived, restored, cleaned up, and permanently deleted through visible workflows with clear dependency and impact information.
- **Data safety:** Cleanup never silently deletes external project directories, Git history, or user-created export files.
- **Accessibility:** Desktop workflows support keyboard navigation, readable status communication, visible focus, and color-independent meaning for update severity.

## Integrations and External Dependencies

### Packwiz

Packwiz is an external command-line dependency used to inspect and modify a local project. Rust owns process execution, path validation, output capture, cancellation behavior, and post-operation verification. The initial wrapper targets one tested Packwiz compatibility profile. That profile defines the executable version, supported command forms, prompt and output patterns, selective-targeting capability, and known limitations. Unsupported or compatibility-unknown versions must be reported clearly and must not silently receive best-effort parsing.

Update discovery uses the tested cancel-after-prompt probe because the documented
command surface does not provide a non-mutating update-list command. The wrapper
must never send `y` during discovery. Discovery is normal only when the expected
prompt is detected, `n` is accepted, the process exits in the expected
cancellation state, and the relevant project fingerprint is unchanged. Any
uncertainty is unsafe or indeterminate. Pin and unpin use interactive exact
selection and post-operation verification; `--yes` is prohibited when it could
select an unintended file.

### Modrinth

Modrinth is the initial external metadata and changelog provider. Requests are explicit, narrow, and cached. Network errors, rate limits, missing projects, deleted versions, and ambiguous identity matches are user-visible states. Exact cached responses may be used offline when their provider project and version association is known. Cached content must show its retrieval time and freshness state and must remain distinguishable from newly retrieved confirmed data.

### CurseForge

CurseForge-origin mods are supported as local inputs. Initial changelog lookup may use Modrinth if a corresponding project can be identified. Direct CurseForge API integration, API-key handling, and CurseForge-native changelog retrieval are future enhancements.

### Git

The application is managed by Git, and managed modpack directories may also use Git. Git provides repository state, commit history, optional release provenance, and historical Packwiz evidence. The initial product does not depend on every project being a Git repository; projects without Git can still use current-state management, update snapshots, and captured releases, but cannot provide commit-based reconstruction of older states.

### GitHub

GitHub is an optional remote integration for repositories that are hosted there. The initial integration supports public repository metadata and may import a release tag, title, body, URL, and creation or publication dates. Imported data remains distinguishable from local edits. Private repository authentication is deferred. GitHub must not replace local Git as the authority for repository content, and the application must continue to work when GitHub is unavailable.

## Architecture Guidelines

The application is a Tauri desktop application with a Svelte frontend, Rust application layer, and SQLite persistence.

### Frontend Boundary

Svelte owns view state, filtering, selection, forms, review controls, progress presentation, and user-facing error states. It should request domain operations from Rust rather than constructing shell commands or directly manipulating filesystem paths.

### Rust Boundary

Rust owns filesystem access, Packwiz process execution, path safety, Modrinth network access, SQLite access, operation coordination, progress, cancellation, and error translation. Tauri commands and events should expose domain-oriented contracts such as project validation, update discovery, update application, and changelog generation.

### Persistence Boundary

SQLite owns application state and historical records. Packwiz files own the current external modpack configuration. Git commits own versioned repository content when available. Database records should retain stable identifiers, release commit references, before/after project fingerprints, exact local metadata paths, and snapshots of relevant observed values so history is not dependent on the current contents of a mutable TOML file or on the current working-tree checkout.

### Security and Privacy

The application should use least-privilege Tauri capabilities. A project must be explicitly registered and canonicalized before operations are allowed. Filesystem operations must remain within the registered project boundary, reject ambiguous or escaping paths, and define behavior for links and junctions. Packwiz resolution and execution must be observable and must not allow project metadata to inject unintended command arguments. Network access should be limited to expected provider endpoints and should avoid sending unnecessary local project data. API keys, when CurseForge support is added, must not be stored in plain project metadata or exposed to the Svelte layer unnecessarily.

### Long-Running Work and Errors

Packwiz operations and external requests must be represented as observable operations with progress, completion, cancellation, and structured failure states. Errors should preserve enough context for the user to diagnose the project, command, provider, or network problem without exposing raw implementation details as the only explanation.

## Technology Decisions and Rationale

| Decision | Rationale |
|----------|-----------|
| Local-first desktop application | Modpack files are local, and the primary workflow should remain useful without a service or account. |
| Tauri, Rust, and Svelte | Native filesystem and process access belong in Rust while Svelte provides the review-oriented desktop UI. |
| SQLite for application data | Project metadata, snapshots, decisions, notes, and caches need durable local relationships and history. |
| Packwiz remains authoritative for modpack files | The application augments Packwiz instead of duplicating its configuration model. |
| Snapshots represent update sessions | This gives the workflow a stable history without prematurely imposing a release-numbering system. |
| Favorites are first-class project metadata | A favorite is a quick-access preference, not an open-ended category, so it should not depend on a removable or renamed tag. |
| Project lifecycle and snapshot release status are separate | A project can be active while a snapshot is in review, or archived while its last snapshot remains released. |
| Releases capture local state at creation time | A release remains valid without Git by storing the modpack evidence observed at the time it was created. |
| Git commits are optional release provenance | A commit can populate or verify historical data, but committing changes is not required to create a release. |
| Packwiz owns persistent pin state | The application manages pin and unpin actions through Packwiz and reflects the resulting metadata instead of maintaining a conflicting pin state. |
| GitHub is optional enrichment | Local Git is sufficient for release reconstruction; GitHub adds remote links and release metadata without becoming a service dependency. |
| Initial Packwiz compatibility targets the installed version | A narrow tested compatibility target reduces command and output differences while allowing unsupported versions to be reported clearly. |
| Markdown is the initial changelog export format | Markdown is portable, editable, Git-friendly, and suitable for GitHub release content or modpack documentation. |
| Generated changelogs retain editable revisions | Users can correct or contextualize generated text without losing the original provider-derived result. |
| Cached provider data is retained until user cleanup | Historical changelogs remain useful without surprise expiration, while scoped cleanup controls address storage concerns. |
| Explicit review before applying updates | Pins, compatibility notes, and skipped updates require a deliberate user decision. |
| Modrinth as the initial changelog provider | It supports the first changelog workflow while avoiding a required CurseForge API key. |
| Provider identity separate from download provider | A CurseForge-origin mod may still have usable Modrinth changelog data. |

## Assumptions, Open Questions, and Non-Goals

### Confirmed Assumptions

- The application is local-first and desktop-based.
- Users manage one or more local Packwiz modpacks.
- Projects have first-class favorite, description, lifecycle status, and timestamp metadata; these are not represented as tags.
- Project facts derived from Packwiz files or Git are refreshed from the external project rather than treated as manually maintained metadata.
- The initial Packwiz wrapper targets the currently installed and tested Packwiz version.
- Changelog exports initially use Markdown only; other formats are deferred.
- Generated changelogs may have editable revisions while preserving the original generated artifact.
- Cached provider data remains until the user performs scoped cleanup.
- Archived records are hidden from normal search and comparison by default.
- Destructive deletion requires an impact preview and typed confirmation.
- Releases capture the local modpack state at creation time; an exact Git commit may be recorded as optional provenance.
- Git commits can populate or verify past release data, but a release does not require a commit.
- GitHub metadata is optional and must not be required for local release tracking or comparison.
- Initial GitHub integration supports public repositories and imports tag, title, body, URL, and dates; private authentication is deferred.
- Releases may be created from dirty working trees with a warning and captured local state.
- Updates may be applied without Git or another backup path only after explicit acknowledgement that automatic rollback is unavailable.
- Updates are blocked for inaccessible or invalid projects, concurrent application operations, active Git conflict/merge/rebase states, or an unavailable before-state.
- External changes after discovery invalidate the review and require fresh discovery before applying updates.
- A retry after a partial, failed, or cancelled operation creates a new linked snapshot or operation rather than mutating the original outcome.
- A release may capture a validated partial or failed state only as a visibly provisional release record.
- Project registration rejects duplicate canonical paths and moved projects require explicit reconnect.
- Packwiz pin and unpin selection is interactive and exact; the application refuses ambiguous selections.
- Provider cache responses may be used offline only when the provider project and version association is exact, with retrieval and freshness information visible.
- Filesystem operations are confined to an explicitly registered canonical project boundary.
- Packwiz is used to inspect and apply updates.
- Modrinth is queried only when changelog generation or required identity resolution needs it.
- CurseForge-origin mods must be considered, but direct CurseForge API access is deferred.
- Git may be useful for status and recovery but is not a required dependency initially.

### Open Questions

The critical product decisions for the initial specification are resolved. The
following items are intentionally deferred and do not change the initial
workflow contract:

- Broader Packwiz version and layout compatibility beyond the initially tested version.
- CurseForge API integration and private GitHub repository authentication.
- Changelog export formats beyond Markdown.
- Automatic expiration policies for cached provider data.
- Additional GitHub release synchronization behavior beyond importing public release metadata.
- Detailed behavior for filesystem symlinks and Windows junctions, pending the first path-safety implementation and test fixtures.
- Whether application-created backups should supplement Git in a later release.

### Non-Goals for the Initial Product

- Replacing Packwiz.
- Hosting or distributing mod files.
- Requiring CurseForge API access or an API key.
- Cloud synchronization, collaboration, or user accounts.
- Fully automatic unattended updates.
- Universal dependency conflict resolution.
- Multiplayer server deployment or runtime management.
- Requiring GitHub hosting for projects or releases.

### Workflow Acceptance Criteria

The following conditions make the critical behavior observable and testable:

- **Project registration:** the application accepts a readable validated Packwiz directory, previews the `pack.toml` name, author, declared pack version, pack format, index reference, and declared game/loader versions, stores its canonical path, rejects duplicate canonical paths, and marks missing or inaccessible paths as disconnected without deleting history. Registration must not modify the selected directory or require network access; malformed manifest data or an unreadable referenced index must be visible in the validation result.
- **Discovery safety:** a supported Packwiz profile detects the expected update prompt, sends only cancellation, observes the expected cancelled result, and verifies an unchanged relevant project fingerprint before presenting candidates as normal results. Any missing evidence is unsafe or indeterminate.
- **Pin safety:** pin and unpin target one exact metadata path, never use ambiguous `--yes` selection, and verify the resulting Packwiz metadata before reporting success.
- **Apply safety:** the application captures a before-state, blocks the explicit unsafe states, invalidates stale reviews after external changes, records output and exit status, and verifies the after-state before classifying the operation.
- **Partial outcomes:** a partial, failed, or cancelled operation remains visible and immutable; retry creates a linked new operation or snapshot.
- **Identity:** each historical observation retains local Packwiz evidence and provider identity evidence separately, including unresolved and ambiguous states.
- **Inventory metadata:** a readable mod entry exposes its declared download provider and `side` value, preserves unknown values explicitly, and offers an external-page action only when a trustworthy URL is available.
- **Changelog fallback:** generation uses only required provider requests, can use an exact cached response offline, and visibly labels stale, missing, ambiguous, or unresolved data.
- **Release capture:** a release retains a captured validated Packwiz state independent of current files or Git availability. Partial or failed source operations produce only visibly provisional release records.
- **Cleanup:** destructive actions show affected application records and external references, use archive-first behavior, require typed confirmation for permanent deletion, and never delete external project directories or user export files.

## Coherence Review

The specification is coherent around a local-first review workflow:

- Packwiz files own current modpack state.
- SQLite owns application metadata and historical decisions.
- Snapshots connect review decisions, Packwiz operations, and changelog generation.
- Modrinth is queried only for targeted changelog and identity needs.
- CurseForge-origin mods are not excluded, but unresolved provider identity remains visible rather than being guessed silently.

The main remaining implementation risks are Packwiz command/version compatibility,
reliable provider identity matching, filesystem path behavior on Windows, and
recovery after partial file changes. The specification now makes those risks
visible through compatibility profiles, explicit unsafe states, dual identity
records, before/after fingerprints, and workflow acceptance criteria.