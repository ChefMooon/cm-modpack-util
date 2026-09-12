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

- Register and switch between multiple local modpacks.
- Inspect available updates using local Packwiz TOML files and a safe Packwiz discovery probe.
- Preserve an immutable snapshot of each discovery result and its stable before-state evidence.
- Assemble an update into an explicit release workspace with review, apply, recovery, verification, and finalization phases.
- Preserve candidate decisions, operation attempts, recovery evidence, and release history when an update succeeds, partially succeeds, is cancelled, or fails.
- Generate proposed and final changelog revisions with minimal targeted provider API usage.
- Compare finalized releases from captured evidence without rereading the current working tree.
- Understand what happened when a workspace or operation is stale, externally changed, unavailable, partial, or failed.

The current alpha includes modpack registration and inventory inspection,
durable update snapshots, recoverable release workspaces, captured-state
comparison, optional Git provenance, release-scoped Packwiz observation,
changelog generation, and Markdown export. GitHub enrichment, archive and
restore workflows, permanent cleanup, CurseForge API integration, private
GitHub authentication, unattended updates, and export formats other than
Markdown remain outside the current release boundary. These capabilities
remain subject to the explicit compatibility and safety boundaries in this
specification.

## Users, Roles, and Permissions

The initial product has one local user role: the owner of the desktop installation.

The owner can:

- Add, edit, remove, and open local modpacks.
- Read and modify files within a registered modpack directory through supported Packwiz operations.
- Configure modpack metadata, tags, themes, pins, notes, snapshots, and release workspaces.
- Request Modrinth lookups and generate changelogs.

There is no collaboration, account system, cloud synchronization, or multi-user permission model in the initial scope.

## Core Capabilities and Workflows

### Modpack Management

A modpack represents one registered local Packwiz directory and its application-owned metadata and history. Modpack metadata includes:

- Display name.
- Directory path.
- Optional icon.
- Modpack color or theme selection.
- User-defined tags.
- Favorite state for quick access and filtering.
- Optional description.
- Lifecycle status, such as active, maintenance, or archived.
- Creation, modification, and last-opened timestamps.
- Last refresh timestamp.

When a directory is chosen for registration, the application should parse its
`pack.toml` and show a confirmation preview before creating the modpack. The
preview may populate the application display name from `name`, while
also showing the Packwiz-declared author, pack version, and pack format. The
application may retain those values as observed Packwiz metadata, but they are
not silently written back to `pack.toml`. The application-owned modpack name
remains editable after registration and may differ from the Packwiz name.

The application also maintains derived modpack state based on the selected directory and its contents:

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

The application must validate a directory before treating it as a usable modpack. Initial validation requires a readable `pack.toml` and the relevant readable mod metadata files needed by the supported Packwiz workflow. Registration should read, when present, the top-level `name`, `author`, `version`, and `pack-format` values; the `[index]` file reference, hash format, and hash; and the `[versions]` entries such as `minecraft`, `neoforge`, or other declared loader/tool versions. These values provide registration defaults and validation evidence, while the directory path and Packwiz files remain the external modpack's source of truth. A missing optional field should be shown as unavailable; a malformed required field or unreadable referenced index must produce a clear validation result rather than a partially trusted modpack.

The registration preview must distinguish values read from `pack.toml` from application-owned fields such as theme, tags, favorite state, description, and lifecycle status. Choosing a directory must not run an update, modify Packwiz files, or require a network request. Duplicate canonical paths are rejected; a moved or inaccessible modpack is retained as disconnected until the user explicitly reconnects it.

### Modpack Inventory Metadata

The modpack inventory must make each mod's download source and execution side easy to scan. The application reads this information from the local Packwiz metadata file:

- A `[update.modrinth]` table identifies Modrinth as the download provider.
- A `[update.curseforge]` table identifies CurseForge as the download provider.
- The `side` field identifies whether the mod is `client`, `server`, or `both`.
- The optional `pin` field identifies the Packwiz pin state: `pin = true` is pinned, while an absent `pin` field is unpinned (`false`).

The inventory should show provider, side, and pin state as text, badges, icons, or equivalent accessible labels that do not rely on color alone. It should support seeing provider, side, and pin state alongside the mod name and version in both the normal inventory and update review. A valid boolean `pin` value is current local evidence, and an absent `pin` field is current evidence of the default unpinned state (`false`). A present but malformed or unsupported `pin` value must remain explicitly unknown or unavailable rather than being coerced. Provider counts, side counts, and pinned counts may be used in the modpack overview and inventory filtering, but they must be derived from the current Packwiz files.

Each mod may expose an **Open mod page** action. The action opens the system browser only when a trustworthy page URL is available, such as an explicit source URL or a safely derived Modrinth project page from a known Modrinth project identity. A CurseForge project ID alone is not sufficient to guess a stable page slug; the action must be unavailable or request resolution when no trustworthy URL is known. Opening a page must never trigger a provider API request merely to render the inventory.

### Modpack Overview

The overview should show the active modpack, favorite state, lifecycle status, current validation state, Minecraft version, mod loader, mod count, download-provider counts, client/server-side counts, Git status when available, available updates, pinned mods, recent snapshots, release workspaces, finalized releases, and the latest operation result. Opening or refreshing a modpack must not modify its files or silently run an update. Opening a finalized or published release is historical review and must not start file observation.

### Update Discovery and Review

Update discovery uses the Packwiz command `packwiz update -a` to obtain the fastest available list of update candidates. Because this command can proceed into an update operation, the application must run it as a controlled interactive process:

1. Run `packwiz update -a` in the registered modpack directory.
2. Capture the proposed update list and command output.
3. Detect the update confirmation prompt.
4. Send `n` to cancel before any update is applied.
5. Confirm that the command completed as a cancellation and that the modpack state did not change.
6. Parse and return the captured candidates for user review.

The application must never send `y` during update discovery. Discovery is only valid when the confirmation prompt was handled, cancellation completed, and post-cancellation validation indicates that no files were modified. If the prompt cannot be detected, cancellation is ambiguous, or the modpack state changed, the result must be reported as unsafe or indeterminate rather than as a normal update list.

Packwiz output is authoritative for which mods are currently presented as update candidates. The application may enrich those candidates with local metadata, Packwiz pin state, notes, and provider identity information. A pinned mod may not appear in Packwiz's automatic update candidates because Packwiz is instructed not to update it automatically; the application should still show the mod and its pinned state when displaying the modpack inventory.

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

Discovery does not apply changes. A normal discovery result creates an immutable snapshot containing the candidates and a stable before-state capture. The snapshot is historical evidence, not an editable update workspace. The user starts a separate release workspace from the snapshot, where candidate selections and Packwiz operations are explicitly confirmed. Snapshot evidence remains visible and is not rewritten by workspace decisions.

Discovery is supported only for a tested Packwiz compatibility profile. The
profile records the executable version, expected prompt and output patterns,
supported command forms, and known limitations. If the installed Packwiz
version is outside the profile, or if prompt detection, cancellation, or
post-cancellation verification is inconclusive, the application must report
discovery as unsupported, unsafe, or indeterminate rather than presenting the
candidate list as authoritative.

### Release Workspace and Applying Updates

The release workspace is the editable assembly area for a proposed modpack release. It may begin from a reviewable snapshot, an explicitly selected current-modpack baseline, a finalized release baseline, or a detached snapshot baseline. Starting a workspace captures and persists its baseline evidence; it does not modify Packwiz files. A modpack may have multiple independent editable workspaces, and opening or starting one must not replace another.

The workspace exposes a small user-facing phase model:

- **Review:** inspect the immutable baseline and choose selected, skipped, deferred, blocked, pinned, or uncertain candidates.
- **Apply:** confirm the candidate summary and apply selected changes through the Rust Packwiz boundary.
- **Resolve:** address partial results, failed or changed-but-unverified candidates, stale evidence, recovery requirements, or external edits.
- **Verify:** inspect a fresh stable post-operation capture and the observed before/after result.
- **Finalize:** select a final changelog revision and explicitly freeze the validated release evidence.
- **Publish:** optionally publish the finalized record or withdraw it; publication is separate from finalization.

The application uses Packwiz commands to apply selected updates from the release workspace. The operation must:

- Run against the registered modpack directory.
- Capture command results, output, errors, and exit status.
- Report progress for work that takes noticeable time.
- Support cancellation where Packwiz and the operating system make that practical.
- Re-read the relevant TOML files after the operation.
- Record an aggregate operation and immutable candidate-level outcomes under the release workspace.

The application may apply a selected subset only when the supported Packwiz
compatibility profile proves that each operation targets the exact intended
metadata file or candidate. If exact targeting cannot be proven, the
application must not emulate Packwiz by editing download metadata itself; it
must either require the complete Packwiz update set or refuse the operation and
explain why selective application is unavailable.

The application must not report success merely because a Packwiz process started or returned without an immediately visible error. Post-operation file validation and a stable capture are part of determining the result. Candidate outcomes distinguish at least `applied`, `failed_before_change`, `changed_but_unverified`, `skipped`, `blocked`, `retryable`, `pinned`, `deferred`, and `uncertain`. Retries create linked attempts and preserve the prior result.

When Git or another recovery path is unavailable, the application must show a prominent warning before applying updates and require explicit user acknowledgement that automatic rollback is unavailable. The workspace operation records the recovery observation and acknowledgement; Git remains optional and is not an automatic rollback guarantee.

Updates are blocked when the application cannot establish exclusive operation
access, cannot read and validate the modpack, detects an active Git conflict,
merge, or rebase state, or cannot capture a usable before-state. No-Git and
dirty-Git modpacks may proceed after acknowledgement. A clean Git tree is
reported as stronger recovery evidence, not as a guarantee that rollback is
automatic or complete.

The live modpack fingerprint must normally match the persisted workspace
baseline before an apply begins. When the fingerprint differs, the application
may reconcile a selected candidate that is already present at its proposed
target version, provided the current inventory differs only for selected
candidates. This records the result as verified without silently refreshing the
baseline. Unrelated or ambiguous changes remain blocked and require an explicit
rebase or a new snapshot.

While an editable workspace is open, Rust may observe only the Packwiz-relevant
fingerprint boundary: `pack.toml`, the referenced index, indexed `*.pw.toml`
metadata, and relevant covered content roots. Events are debounced and followed
by a stable reread. An external change preserves decisions and snapshot
provenance but records changed scope, evidence freshness, and a blocking reason
when appropriate. An external edit overlapping an app-owned operation requires
recovery rather than being merged silently. Observation stops when the
workspace is closed, abandoned, ready to finalize, finalized, withdrawn, or
published, and never starts for a historical published release.

### Pins, Skips, and Notes

Persistent pinning is owned by Packwiz. The application reads the current pin state from each local mod metadata file, treating `pin = true` as pinned and an absent `pin` field as the authoritative unpinned (`false`) observation. A present malformed or unsupported value is not coerced. When the user pins a mod, the application invokes `packwiz pin` for the selected file. When the user unpins a mod, the application invokes `packwiz unpin` for the selected file. The application then re-reads and validates the mod metadata so Packwiz's resulting state is confirmed and displayed; it must not edit the `pin` field directly.

The Packwiz pin and unpin commands may prompt when a file must be selected. The
Rust wrapper must capture the prompt, present or select one exact local
metadata path, and refuse to continue when the prompt cannot be disambiguated.
The `--yes` option must not be used for this flow because Packwiz documents
that accepting all prompts may select unwanted search results. The application
then re-reads and validates the metadata and records the resulting Packwiz
state. SQLite may retain the observation and operation history, but it is not a
second authority for the current pin state.

Snapshot-specific historical notes remain application-owned records, but
editable release decisions belong to the release workspace. A pinned mod may be
absent from Packwiz's automatic update candidate list, but it remains visible
in the modpack inventory and can be unpinned before a future update check.

The product distinguishes between:

- **Pinned:** an ongoing Packwiz-owned policy that prevents the file from receiving automatic updates.
- **Skipped:** a decision made for one release workspace.
- **Blocked:** an update that cannot currently be applied because validation or compatibility rules reject it.
- **Deferred:** a candidate intentionally held for later resolution.
- **Uncertain:** a candidate whose identity or evidence is not sufficient for a safe decision.

Notes may explain a modpack policy, a historical snapshot, a workspace decision,
or a release. Typical examples include a crash, a dependency concern, an
external edit, or a decision to wait for another mod.

### Update Snapshots

A snapshot represents one safe update discovery result and its evidence, not a
public modpack release and not an editable release workspace. Creating a new
snapshot records the discovery attempt and begins the review workflow.

Snapshots remain useful when no release is created. They preserve the Packwiz
output, candidate evidence, safety diagnostics, stable before-state capture,
rechecks, notes, and discovery outcome. A snapshot can be draft, reviewable,
closed, cancelled, or stale. Only a reviewable snapshot can directly seed a
release workspace; unsafe, unsupported, indeterminate, failed, cancelled, or
stale evidence must remain visible without being presented as an authoritative
candidate list.

A snapshot records:

- Modpack identity.
- Creation, update, and optional close times.
- Optional user-facing label.
- The discovery result, process evidence, compatibility evidence, and relevant fingerprint comparison.
- A full stable Packwiz baseline capture when the snapshot is reviewable.
- Immutable candidate records and rechecks.
- Historical decisions and notes, when present.
- Retry linkage to a predecessor snapshot, when applicable.

A snapshot does not own update application, recovery, final changelog
selection, finalization, or publication. Starting a release workspace preserves
the snapshot and layers release-owned candidate decisions over its immutable
candidate evidence. Unlinking a snapshot from an editable workspace removes
only the provenance link; the detached baseline remains visible and immutable.
Rebasing from current files is explicit, clears the prior snapshot/release
provenance, and returns the workspace to draft review.

### Release Tracking and Version Comparison

The application tracks release workspaces and finalized modpack releases
separately from update snapshots. A release workspace is an editable,
recoverable assembly of a proposed release. A finalized release is a named
version backed by an immutable capture of the validated local modpack state.
The workspace and release records must not be presented as interchangeable.

A release workspace may contain:

- User-facing release name, optional version, description, and notes.
- A baseline origin: current modpack, source snapshot, finalized release, or detached snapshot.
- An optional source snapshot link and optional baseline release link.
- Release-owned candidate decisions, notes, operation attempts, recovery evidence, and activity.
- Baseline and final capture evidence, evidence freshness, blocking reason, and the current user-facing phase.
- Proposed changelog artifacts and editable revisions.
- A final changelog revision selected for finalization.
- Lifecycle, evidence, and publication status as separate fields.

Finalization requires a fresh stable validated final capture that differs from
the workspace baseline, has no unresolved candidate or recovery outcomes, and
has a selected final changelog revision associated with that capture. A no-op,
stale, unstable, unavailable, malformed, unverified, or proposal-only result
cannot be finalized. Finalized evidence and the selected final changelog
revision are immutable. Publication is an explicit separate transition; a
finalized unpublished release remains historical and reviewable, while a
published release is read-only. Withdrawal is distinct from publication and
does not rewrite the captured evidence.

A finalized release can exist without a Git repository or commit. When Git
evidence is available, the release retains the repository root, branch, exact
commit, tag, remote, working-tree state, and conflict/rebase observation as
optional provenance. Unavailable provenance is explicit and non-fatal when the
capture itself is valid. The captured state, not the current working tree, is
the authority for release history and comparison.

Release comparison should allow the user to select two releases and see:

- Mods added and removed.
- Mods whose versions changed.
- Mods whose provider or source metadata changed.
- Changes in Minecraft version or mod loader when present.
- The associated update notes and generated changelog information.
- Release notes written for the selected release.
- Commit evidence when both releases provide comparable evidence from the same repository.

Comparison is computed from persisted captures and must not read current Packwiz
files or silently reconstruct historical files. Historical Git reads, worktrees,
and inferred commit ranges are not required by the current release boundary.

GitHub enrichment is deferred. No GitHub request, imported remote release
metadata, public repository dependency, or private authentication is required
for local release creation, finalization, publication state, or comparison.

### Changelog Generation

Changelog generation is an explicit action and is not part of every update
check. Existing snapshot artifacts remain durable history, while release
workspaces add a distinct proposed/final workflow.

The user may provide an optional introduction message during changelog generation. When provided, the message appears at the beginning of the generated changelog before the mod-by-mod changes. The introduction is useful for release context, compatibility warnings, migration guidance, or other information that does not belong to an individual mod entry.

When generating a snapshot or workspace changelog, the application:

1. Identifies the required changed or selected mods from the relevant snapshot or workspace evidence.
2. Resolves each mod to a Modrinth project and relevant version where possible.
3. Requests changelog information only for the required mods and versions.
4. Places the optional introduction message at the beginning, when provided.
5. Formats the results into a reviewable changelog.
6. Stores the generated result, introduction message, request context, retrieval status, and source evidence with the snapshot or workspace artifact so it can be revisited without repeating the same request.

Proposed workspace changelog content is derived from selected candidates and is
editable, but it must be labeled as proposed and must never be presented as
proof that changes were applied. The final changelog is selected only after a
validated observed result and is frozen with the final release capture. The
final artifact must identify the capture and revision it represents.

Generated changelogs are durable application records and can be referenced from
the snapshot, workspace, or finalized release. The user may export a generated
changelog to a selected file format and location. An export records the format,
destination, export time, exported content or content hash, and status, while
the application retains the generated changelog independently of the exported
file.

The user may create an editable revision of a generated changelog before export. The original generated result must remain preserved, and the revision must identify the user edits and the source artifact from which it was created. The initial export format is Markdown; other formats are deferred.

Exporting a changelog must not remove or replace the stored generated result. Re-exporting the same changelog may create a new export record, allowing the application to show the latest export while preserving the history of previous exports.

A missing or ambiguous Modrinth match must be visible to the user and must not be presented as confirmed changelog information.

### Data Management and Cleanup

The current alpha provides visible management for supported modpack metadata,
snapshot history, release workspaces, releases, changelog revisions, exports,
and connection state. It does not yet provide the full destructive cleanup
workflow; that remains a later capability and must not be implied by the core
update or release flow.

The supported current-alpha management paths include:

- Editing modpack metadata, tags, pins, notes, workspace metadata, release fields, and changelog revisions.
- Archiving, restoring, and reconnecting modpacks without deleting their external directories; unavailable directories remain recoverable as disconnected records.
- Resuming or explicitly abandoning release workspaces while preserving snapshot and workspace history.
- Exporting and revisiting changelog artifacts without replacing the stored generated result.
- Retaining unavailable export destinations and provider failures as visible history.

Future cleanup operations must preserve referential integrity. Removing a
modpack from the application must not delete its external directory, Git
repository, Git history, or user-created export files. Permanent deletion of a
release, workspace, snapshot, or changelog artifact must require an impact
preview and typed confirmation, must identify dependent records, and must never
silently cascade destructive deletion.

Archived records should be hidden from normal search and comparison results by
default when archive/restore is expanded.

Any later storage cleanup must show affected records and external references and
must not remove the only stored copy of a generated changelog or captured
release state without an explicit destructive action.

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

- **Modpack:** application metadata associated with one registered local Packwiz directory, including its favorite state, description, lifecycle status, tags, observations, and timestamps.
- **Mod record:** an observation of a mod entry read from modpack files.
- **Local entry identity:** the stable application identity for a Packwiz metadata entry, including its observed path and local evidence.
- **Download provider:** the provider declared by the local Packwiz update table, such as Modrinth or CurseForge.
- **Provider identity:** the known or unresolved identity of a mod on an external provider.
- **Provider version identity:** the provider-specific version associated with a changelog request, distinct from the local version string.
- **Mod side:** the Packwiz `side` value describing whether a mod runs on the client, server, both, or has an unknown value.
- **Mod page link:** a trustworthy external project-page URL associated with a local mod entry; it may be unavailable even when a provider or project ID is known.
- **Pin:** a durable Packwiz-owned update policy managed by the application through `packwiz pin` and `packwiz unpin`.
- **Snapshot:** immutable discovery history containing candidate evidence, safety diagnostics, and a stable before-state capture.
- **Release workspace:** an editable, recoverable assembly of one proposed release, including candidate decisions, operations, recovery, changelog revisions, and workspace activity.
- **Release baseline:** the immutable capture from which a workspace starts; it may originate from the current modpack, a snapshot, a finalized release, or a detached snapshot.
- **Release capture:** a versioned, stable observation of Packwiz manifest, runtime, inventory, fingerprints, and optional Git provenance.
- **Release:** a named modpack version backed by an immutable final capture and optional Git provenance.
- **Workspace lifecycle:** the operational state of a workspace: `draft`, `applying`, `recovery_required`, `provisional`, `ready_to_finalize`, `finalized`, `published`, `withdrawn`, or `abandoned`.
- **Evidence status:** the workspace baseline/final evidence state, distinct from lifecycle and publication status.
- **Publication status:** whether a release is `draft`, `provisional`, `published`, or `withdrawn`, distinct from update outcome and workspace lifecycle.
- **Snapshot decision:** an immutable historical decision record; release-workspace decisions are separate and must not rewrite snapshot history.
- **Candidate outcome:** the observed result for a workspace candidate, including `applied`, `failed_before_change`, `changed_but_unverified`, `skipped`, `blocked`, `retryable`, `pinned`, `deferred`, or `uncertain`.
- **Note:** user context attached to a modpack, mod policy, snapshot, workspace decision, or release.
- **Release notes:** user-authored context describing a named release, separate from generated changelog content and individual mod decision notes.
- **Changelog introduction:** optional user-authored text placed at the beginning of one generated changelog.
- **Changelog artifact:** durable generated changelog content associated with a snapshot, workspace, or release, labeled as proposed or final.
- **Changelog revision:** an editable derivative of an artifact; the selected final revision is frozen during finalization.
- **Changelog export:** a file produced from a changelog artifact, with its format, destination, timestamp, and export history recorded.
- **Archive state:** whether an application record is active, archived, or pending permanent deletion.
- **External metadata cache:** stored provider responses used to avoid unnecessary repeated requests.

Important rules:

- Packwiz files remain authoritative for the current modpack configuration.
- The current download provider and mod side are read from Packwiz metadata and are displayed as observations, not application-owned overrides.
- Provider statistics and side statistics are derived from the current inventory and may become unknown when the corresponding Packwiz values are missing or malformed.
- Application metadata must not silently overwrite Packwiz state.
- Historical snapshots, workspaces, and finalized releases must remain understandable after the current modpack files change.
- Packwiz owns persistent pin state, and the application reflects `pin = true` or an absent `pin` field found in the local mod metadata as current evidence of pinned or unpinned state. Present malformed or unsupported pin values remain unknown or unavailable.
- SQLite may retain pin observations and operation history, but it is not authoritative for current pin state.
- Pinning and unpinning must target one exact Packwiz metadata file.
- The application must not use Packwiz's `--yes` option when doing so could select an unintended file.
- Unknown version changes remain unknown rather than being misclassified.
- External changelog data is not confirmed until its provider identity and version association are known.
- A failed, partial, cancelled, unsafe, or indeterminate operation remains visible and is never represented as a successful final release.
- A release without Git remains valid because its captured modpack state is stored with the release.
- A release with Git provenance retains the observation without making a commit, clean tree, remote, or historical reconstruction mandatory.
- Comparing releases uses their persisted captured states and never whichever files happen to be in the current working tree.
- Finalization requires a changed, stable, validated final capture, no unresolved candidate or recovery outcome, and a selected final changelog revision.
- Finalized and published evidence is immutable; publication and withdrawal are guarded status transitions rather than recaptures.
- External observation is evidence only. It never silently authorizes an update, changes a decision, or rewrites snapshot provenance.
- A provider match must retain its confidence and evidence; an unresolved or ambiguous match is never treated as confirmed changelog data.
- Opening a mod page uses only a trustworthy known URL; the application must not construct a guessed CurseForge URL from a project ID alone.

## Data and Lifecycle Expectations

The application database stores modpack metadata, application settings,
current inventory observations, activity, discovery attempts and candidates,
snapshots and stable baselines, snapshot notes and rechecks, operation attempts,
workspace operations and candidate outcomes, recovery acknowledgements, pin
observations, changelog cache and attempts, changelog artifacts/revisions/
exports, release metadata and immutable captures, release-workspace metadata,
workspace candidate sources and decisions, workspace activity and external
observations. The selected modpack directory stores the actual Packwiz
configuration and mod metadata, while Git stores repository history when
available. Packwiz remains authoritative for current persistent pin state,
current mod configuration, provider declarations, side declarations, and
source URLs present in the files.

Modpacks can be registered, validated, opened, refreshed, edited, favorited,
archived, restored, disconnected, and reconnected. Registration uses a
canonical directory path and rejects duplicate registrations of the same
directory. A missing or inaccessible path is retained as disconnected;
reconnection requires an explicitly selected directory that passes validation
and preserves the modpack's application identity. Removing a modpack from the
application must not delete its external directory, Git repository, Git
history, or user-created exports.

Snapshots progress through `draft`, `reviewable`, `closed`, `cancelled`, and
`stale` lifecycles. A normal reviewable snapshot retains a stable baseline
capture and immutable candidate evidence. Abnormal discovery outcomes remain
visible but cannot be treated as authoritative candidates. A retry creates a
new linked snapshot and does not rewrite the predecessor.

Release workspaces retain their source snapshot link, baseline origin, baseline
capture, release-owned candidate decisions, operation attempts, activity,
evidence status, publication status, and finalization fields. A workspace may
be resumed after navigation or application restart. Explicit abandonment
preserves history and does not revert Packwiz files. Unlinking a snapshot
removes only the source link; explicit rebasing captures the current files as a
new baseline, clears prior snapshot/release provenance, and returns to draft
review. Multiple independent workspaces may remain visible for a modpack.

Workspace lifecycle, evidence status, and publication status are separate. The
user-facing phases are review, apply, resolve, verify, finalize, and publish.
The backend may retain more detailed diagnostics, but the frontend must expose
the current phase, blocking reason, evidence freshness, and primary next
action. Applying normally requires a live fingerprint match to the immutable
baseline; a selected update already matching its target may be reconciled as
verified when no unrelated inventory changes are present. Partial, failed,
cancelled, stale, external-change, and recovery states remain visible and
retryable where safe.

Workspace activity is stored with precise UTC epoch timestamps and is presented
newest first in the release review. The review loads ten activity records at a
time through a Rust-owned cursor query; older records are requested explicitly
with `Show more history`. The activity history is collapsed by default, while
observer status and controls remain visible in the evidence metadata strip and
continue independently of history visibility.

A workspace can finalize only from a changed, stable, validated final capture
with no unresolved candidate or recovery outcome and a selected final changelog
revision. Finalization freezes the final capture and revision. Publication and
withdrawal are separate guarded transitions; published records are read-only
historical evidence and do not observe later modpack changes.

Release records retain captured Packwiz state even if the modpack is later
moved, renamed, disconnected, or changed. Git provenance retains repository,
branch, commit, tag, remote, clean/dirty/conflicted state, and availability
observations when possible, but unavailable provenance is non-fatal to a valid
local capture. Release comparison uses persisted captures and never current
files.

Generated changelog artifacts remain referenceable from their snapshot,
workspace, or release after export. Proposed revisions remain editable; the
selected final revision is frozen during finalization. Export records retain the
destination, format, content or hash, status, and timestamp. Moving or deleting
an exported file does not remove the stored artifact or export history.

Cached provider responses retain retrieval context, request/response metadata,
association, and timestamps. Exact cached Modrinth data may support offline
generation only when its project and version association is known. Cache
expiration and scoped cleanup remain later data-management work.

The alpha schema has a compatibility sentinel and no migration system. When an
existing database is missing or incompatible with the current schema, the app
must fail clearly; the user must stop the app and manually delete the database
and its WAL/SHM sidecars before relaunch. The app must not silently migrate,
rewrite, or discard application data. Packwiz files remain outside this reset
boundary.

## Non-Functional Requirements

- **Local-first:** Core modpack browsing, stored snapshot/workspace/release history, pins, notes, and previously cached changelogs work without network access.
- **Data safety:** Operations are limited to the registered modpack directory and provide clear results for failures, partial completion, stale evidence, and external changes.
- **Minimal API use:** Modrinth is queried for changelog generation and related identity resolution only when required, not on every modpack open or routine local inspection.
- **Explainability:** The user can see why a mod is listed, skipped, pinned, blocked, or missing changelog information.
- **Responsiveness:** Long-running Packwiz and network work does not freeze the interface and exposes progress or a clear busy state.
- **Recoverability:** Git status or another backup/recovery mechanism may be detected and reported, but Git is not required for the initial product unless a later safety decision makes it mandatory.
- **Data manageability:** Supported application-owned records can be edited, archived, restored, or disconnected through visible workflows; full cleanup and permanent deletion remain deferred and must eventually include clear dependency and impact information.
- **Data safety:** Application cleanup never silently deletes external modpack directories, Git history, or user-created export files.
- **Accessibility:** Desktop workflows support keyboard navigation, readable status communication, visible focus, and color-independent meaning for update severity.

## Integrations and External Dependencies

### Packwiz

Packwiz is an external command-line dependency used to inspect and modify a
registered local modpack. Rust owns process execution, path validation, output
capture, cancellation behavior, and post-operation verification. The initial
wrapper targets one tested Packwiz compatibility profile. That profile defines
the executable version, supported command forms, prompt and output patterns,
selective-targeting capability, and known limitations. Unsupported or
compatibility-unknown versions must be reported clearly and must not silently
receive best-effort parsing.

Update discovery uses the tested cancel-after-prompt probe because the documented
command surface does not provide a non-mutating update-list command. The wrapper
must never send `y` during discovery. Discovery is normal only when the expected
prompt is detected, `n` is accepted, the process exits in the expected
cancellation state, and the relevant modpack fingerprint is unchanged. Any
uncertainty is unsafe or indeterminate. Pin and unpin use interactive exact
selection and post-operation verification; `--yes` is prohibited when it could
select an unintended file. New update application is owned by a release
workspace and must use its baseline and candidate decisions.

### Modrinth

Modrinth is the initial external metadata and changelog provider. Requests are explicit, narrow, and cached. Network errors, rate limits, missing projects, deleted versions, and ambiguous identity matches are user-visible states. Exact cached responses may be used offline when their provider project and version association is known. Cached content must show its retrieval time and freshness state and must remain distinguishable from newly retrieved confirmed data.

### CurseForge

CurseForge-origin mods are supported as local inputs. Initial changelog lookup may use Modrinth if a corresponding project can be identified. Direct CurseForge API integration, API-key handling, and CurseForge-native changelog retrieval are future enhancements.

### Git

The application is managed by Git, and managed modpack directories may also use
Git. Git provides optional working-tree status and release provenance. The
initial product does not depend on every modpack being a Git repository;
modpacks without Git can still use current-state management, snapshots, release
workspaces, finalized captures, and captured-state comparison. Historical Git
reconstruction and inferred commit ranges are not required.

### GitHub

GitHub enrichment is deferred. The current release boundary makes no GitHub
request, does not import public release metadata, and does not require a public
remote or authentication. A later integration must not replace local Git or
captured Packwiz evidence as the authority for repository content and release
history.

## Architecture Guidelines

The application is a Tauri desktop application with a Svelte frontend, Rust application layer, and SQLite persistence.

### Frontend Boundary

Svelte owns view state, filtering, selection, forms, review controls, progress presentation, and user-facing error states. It should request domain operations from Rust rather than constructing shell commands or directly manipulating filesystem paths.

### Rust Boundary

Rust owns filesystem access, Packwiz process execution, path safety, Modrinth
network access, SQLite access, operation coordination, progress, cancellation,
release-workspace observation, and error translation. Tauri commands and events
should expose domain-oriented contracts such as modpack validation, update
discovery, workspace operations, release finalization, and changelog generation.

### Persistence Boundary

SQLite owns application state and historical records. Packwiz files own the
current external modpack configuration. Git commits own versioned repository
content when available. Database records should retain stable identifiers,
snapshot/workspace/release relationships, capture fingerprints, exact local
metadata paths, candidate outcomes, changelog provenance, and snapshots of
relevant observed values so history is not dependent on the current contents of
a mutable TOML file or on the current working-tree checkout.

### Security and Privacy

The application should use least-privilege Tauri capabilities. A modpack must be explicitly registered and canonicalized before operations are allowed. Filesystem operations must remain within the registered modpack boundary, reject ambiguous or escaping paths, and define behavior for links and junctions. Packwiz resolution and execution must be observable and must not allow modpack metadata to inject unintended command arguments. The Rust watcher must observe only the release-workspace fingerprint boundary and must not become a second filesystem authority in Svelte. Network access should be limited to expected provider endpoints and should avoid sending unnecessary local modpack data. API keys, when CurseForge support is added, must not be stored in plain modpack metadata or exposed to the Svelte layer unnecessarily.

### Long-Running Work and Errors

Packwiz operations, release-workspace observation, and external requests must be
represented as observable operations with progress, completion, cancellation,
freshness, and structured failure states. Errors should preserve enough context
for the user to diagnose the modpack, workspace, command, provider, or network
problem without exposing raw implementation details as the only explanation.

## Technology Decisions and Rationale

| Decision | Rationale |
|----------|-----------|
| Local-first desktop application | Modpack files are local, and the primary workflow should remain useful without a service or account. |
| Tauri, Rust, and Svelte | Native filesystem and process access belong in Rust while Svelte provides the review-oriented desktop UI. |
| SQLite for application data | Modpack metadata, snapshots, release workspaces, decisions, notes, captures, and caches need durable local relationships and history. |
| Packwiz remains authoritative for modpack files | The application augments Packwiz instead of duplicating its configuration model. |
| Snapshots preserve discovery evidence | Immutable candidates and a stable baseline let a review remain explainable without turning the snapshot into a mutable release workspace. |
| Favorites are first-class modpack metadata | A favorite is a quick-access preference, not an open-ended category, so it should not depend on a removable or renamed tag. |
| Modpack, workspace, evidence, and publication status are separate | A modpack can be active while a workspace is recovering, or a release can be finalized while remaining unpublished. |
| Release captures are immutable | A finalized release remains valid without Git by storing the modpack evidence observed in its immutable final capture. |
| Git commits are optional release provenance | A commit can populate or verify historical data, but committing changes is not required to create a release. |
| Packwiz owns persistent pin state | The application manages pin and unpin actions through Packwiz and reflects the resulting metadata instead of maintaining a conflicting pin state. |
| GitHub enrichment is deferred | Local Git and captured evidence are sufficient for release history and comparison without a service dependency. |
| Initial Packwiz compatibility targets the installed version | A narrow tested compatibility target reduces command and output differences while allowing unsupported versions to be reported clearly. |
| Markdown is the initial changelog export format | Markdown is portable, editable, Git-friendly, and suitable for GitHub release content or modpack documentation. |
| Proposed and final changelogs are distinct | Users can correct or contextualize proposed text without presenting it as applied evidence; the selected final revision is frozen with the final capture. |
| Cached provider data is retained until user cleanup | Historical changelogs remain useful without surprise expiration, while scoped cleanup controls address storage concerns. |
| Explicit workspace review before applying updates | Pins, compatibility notes, skipped updates, recovery, and stale evidence require deliberate user decisions. |
| Modrinth as the initial changelog provider | It supports the first changelog workflow while avoiding a required CurseForge API key. |
| Provider identity separate from download provider | A CurseForge-origin mod may still have usable Modrinth changelog data. |
| Alpha schema sentinel without migrations | Breaking schema changes fail clearly and require a manual database/WAL/SHM reset rather than silently rewriting data. |

## Assumptions, Open Questions, and Non-Goals

### Confirmed Assumptions

- The application is local-first and desktop-based.
- Users manage one or more local Packwiz modpacks.
- Modpacks have first-class favorite, description, lifecycle status, and timestamp metadata; these are not represented as tags.
- Modpack facts derived from Packwiz files or Git are refreshed from the external directory rather than treated as manually maintained metadata.
- The initial Packwiz wrapper targets the currently installed and tested Packwiz version.
- Changelog exports initially use Markdown only; other formats are deferred.
- Snapshot and workspace changelogs may have editable revisions while preserving the original generated artifact; the selected final revision is frozen during finalization.
- Cached provider data remains until the user performs scoped cleanup.
- Destructive deletion is deferred; when implemented it requires an impact preview and typed confirmation.
- A reviewable snapshot retains a full stable Packwiz baseline capture and immutable candidate evidence.
- A release workspace may start from a snapshot, the current modpack, a finalized release, or a detached snapshot baseline; explicit unlink and rebase actions are distinct.
- Multiple independent editable release workspaces may remain visible for one modpack.
- Release workspace lifecycle, evidence status, and publication status remain separate.
- Finalization requires a changed, stable, validated final capture, no unresolved candidate or recovery outcomes, and a selected final changelog revision.
- Finalized and published captures remain immutable; publication and withdrawal are explicit separate transitions.
- Releases may be created or finalized without Git; an exact commit and other Git provenance are optional observations.
- GitHub enrichment is deferred and must not be required for local release tracking or comparison.
- Dirty, conflicted, no-Git, missing-commit, and unavailable-history states remain explicit evidence.
- Updates may be applied without Git or another backup path only after explicit acknowledgement that automatic rollback is unavailable.
- Updates are blocked for inaccessible or invalid modpacks, concurrent application operations, active Git conflict/merge/rebase states, stale baselines, or unavailable evidence.
- External changes preserve workspace decisions and snapshot provenance but can block apply, verification, or finalization until explicitly resolved.
- A retry after a partial, failed, cancelled, or recovery-required operation creates a linked attempt or snapshot rather than mutating the original outcome.
- Partial eligible evidence may remain visibly provisional, but provisional evidence cannot be finalized without a fresh validated capture.
- Modpack registration rejects duplicate canonical paths, and moved modpacks require explicit reconnect.
- Packwiz pin and unpin selection is interactive and exact; the application refuses ambiguous selections.
- Provider cache responses may be used offline only when the provider project and version association is exact, with retrieval and freshness information visible.
- Filesystem operations and workspace observation are confined to an explicitly registered canonical modpack boundary and the relevant Packwiz fingerprint scope.
- Packwiz is used to inspect and apply updates.
- Modrinth is queried only when changelog generation or required identity resolution needs it.
- CurseForge-origin mods must be considered, but direct CurseForge API access is deferred.
- Git may be useful for status and recovery but is not a required dependency initially.
- The alpha schema has a compatibility sentinel and no migration system; incompatible databases require a manual database/WAL/SHM reset.

### Open Questions

The critical product decisions for the initial specification are resolved. The
following items are intentionally deferred and do not change the initial
workflow contract:

- Broader Packwiz version and layout compatibility beyond the initially tested version.
- CurseForge API integration and private GitHub repository authentication.
- Changelog export formats beyond Markdown.
- Automatic expiration policies for cached provider data.
- Detailed behavior for filesystem symlinks and Windows junctions, pending the first path-safety implementation and test fixtures.
- Whether application-created backups should supplement Git in a later release.
- Native Tauri/manual validation of watcher timing, restart/resume, abandonment, overlapping external edits, and publication teardown.

### Non-Goals for the Initial Product

- Replacing Packwiz.
- Hosting or distributing mod files.
- Requiring CurseForge API access or an API key.
- Cloud synchronization, collaboration, or user accounts.
- Fully automatic unattended updates.
- Universal dependency conflict resolution.
- Multiplayer server deployment or runtime management.
- Requiring GitHub hosting for modpacks or releases.
- Automatic release publication or unattended release-workspace execution.

### Workflow Acceptance Criteria

The following conditions make the critical behavior observable and testable:

- **Modpack registration:** the application accepts a readable validated Packwiz directory, previews the `pack.toml` name, author, declared pack version, pack format, index reference, and declared game/loader versions, stores its canonical path, rejects duplicate canonical paths, and marks missing or inaccessible paths as disconnected without deleting history. Registration must not modify the selected directory or require network access; malformed manifest data or an unreadable referenced index must be visible in the validation result.
- **Discovery safety:** a supported Packwiz profile detects the expected update prompt, sends only cancellation, observes the expected cancelled result, and verifies an unchanged relevant modpack fingerprint before presenting candidates as normal results. Any missing evidence is unsafe or indeterminate.
- **Pin safety:** pin and unpin target one exact metadata path, never use ambiguous `--yes` selection, and verify the resulting Packwiz metadata before reporting success.
- **Snapshot baseline:** a reviewable snapshot retains a full stable Packwiz capture and immutable candidate evidence; unsafe, unsupported, indeterminate, failed, cancelled, or stale discovery cannot seed an authoritative workspace.
- **Workspace safety:** a workspace owns candidate decisions and operations, verifies the live fingerprint against its immutable baseline before apply, preserves decision and provenance history, and exposes phase, blocker, freshness, and next action.
- **Apply safety:** the application captures before/after evidence, blocks stale or unsafe states, records output and exit status, and verifies the result before classifying each candidate.
- **Partial outcomes:** a partial, failed, cancelled, or recovery-required operation remains visible and immutable; retry creates a linked new attempt or snapshot.
- **Workspace recovery:** candidate outcomes distinguish applied, failed-before-change, changed-but-unverified, skipped, blocked, retryable, pinned, deferred, and uncertain states without representing unresolved work as successful final evidence.
- **Observation safety:** Rust observes only the Packwiz fingerprint boundary while an editable workspace is active, debounces and stably rereads changes, preserves decisions/provenance, and records overlap with app-owned operations as recovery rather than silently merging it.
- **Identity:** each historical observation retains local Packwiz evidence and provider identity evidence separately, including unresolved and ambiguous states.
- **Inventory metadata:** a readable mod entry exposes its declared download provider and `side` value, preserves unknown values explicitly, and offers an external-page action only when a trustworthy URL is available.
- **Changelog stages:** generation uses only required provider requests, can use an exact cached response offline, visibly labels stale, missing, ambiguous, or unresolved data, distinguishes proposed from final content, and freezes the selected final revision only with the final capture.
- **Release finalization:** a release workspace can finalize only with changed, stable, validated final evidence, no unresolved candidates or recovery outcomes, and a selected final changelog revision. No-op, stale, unstable, unverified, or proposal-only evidence is rejected.
- **Release comparison:** finalized release comparisons use persisted captures and optional Git provenance only; current working-tree files and historical checkout are never silent fallbacks.
- **Cleanup boundary:** current alpha workflows never delete external modpack directories, Git history, or user exports. Future destructive cleanup must show impact, require typed confirmation, and preserve referential integrity.

## Coherence Review

The specification is coherent around a local-first review workflow:

- Packwiz files own current modpack state.
- SQLite owns modpack metadata, immutable snapshot evidence, release-workspace activity and decisions, captured releases, changelog artifacts, and operation history.
- Snapshots preserve safe discovery evidence and stable baselines; release workspaces own mutable decisions, Packwiz operations, recovery, and changelog assembly.
- Finalized releases own immutable captured state; publication is separate and historical records never fall back to current files.
- Modrinth is queried only for targeted changelog and identity needs.
- CurseForge-origin mods are not excluded, but unresolved provider identity remains visible rather than being guessed silently.
- Rust owns filesystem, Packwiz, capture, persistence, comparison, and workspace-observation authority; Svelte presents typed state and user actions.
- Git provenance is optional, GitHub enrichment is deferred, and destructive cleanup remains outside the current alpha workflow.

The main remaining validation risks are Packwiz command/version compatibility,
reliable provider identity matching, filesystem path behavior on Windows, and
native watcher timing and teardown. The specification makes those risks
visible through compatibility profiles, explicit unsafe states, dual identity
records, stable capture fingerprints, workspace phases, candidate-level
outcomes, and workflow acceptance criteria. Native Tauri/manual lifecycle
evidence remains a validation task rather than an assumed result.