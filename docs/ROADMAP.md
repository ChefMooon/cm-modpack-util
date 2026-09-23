# CM Modpack Util: 0.1.0 Readiness Roadmap

> This is a temporary release-readiness roadmap for the planned initial `0.1.0`
> release; additional development versions may be released before then. It
> replaces the historical milestone sequence preserved in
> [`docs/archive/ROADMAP-v0.0.1-v0.0.9.md`](archive/ROADMAP-v0.0.1-v0.0.9.md).
> After `0.1.0` ships, replace this document with a post-release product
> roadmap.

## Implemented product scope

### Modpack management

- Register local Packwiz directories with canonical path validation and a review preview.
- Preserve Packwiz files as the external source of truth.
- Reopen, refresh, edit application-owned metadata, archive, restore, detach, and reconnect modpacks.
- Retain disconnected projects and their application history when external directories are unavailable.

See [modpack registration](modpack-registration.md) and the [product specification](CM-MODPACK-UTIL-SPEC.md).

### Inventory, discovery, and review

- Read current Packwiz inventory and show provider, side, pin, version, validation, and project evidence.
- Use the tested Packwiz compatibility profile for update discovery through `packwiz update -a` cancellation probing.
- Preserve immutable discovery snapshots, fingerprints, candidate evidence, decisions, notes, stale detection, retries, and explicit unsafe or indeterminate outcomes.
- Apply selected targeted updates through verified positional slugs, one bounded process per target, with post-operation verification and partial/cancelled outcomes.
- Apply verified positional-slug pin and unpin operations. The all-target `packwiz update -a` form remains discovery-only.

See [Packwiz compatibility and command safety](packwiz-commands.md).

### Releases and changelogs

- Create independent release workspaces from captured evidence.
- Observe relevant external changes and preserve recovery evidence.
- Finalize and compare releases from persisted captures rather than silently rereading the current working tree.
- Generate, revise, and export Markdown changelogs with bounded Modrinth access, exact cache reuse, durable artifacts, and explicit unresolved or failed provider outcomes.
- Manage application-owned records, cleanup eligible cache/export observations, and transfer application data through versioned recovery bundles.

See [release workspace validation](release-workspace-validation.md), [data management](data-management.md), and the detailed plans under [`docs/plans/`](plans/).

### Desktop updates

The desktop updater is implemented and has been tested through the GitHub release flow:

- Startup and Settings checks retrieve stable release metadata without downloading installer bytes.
- The user explicitly starts download and installation.
- Windows release artifacts are built as signed Tauri updater artifacts in the NSIS workflow.
- The application presents the tested install/restart flow and handles deferred updates without repeatedly interrupting the same session.
- Release artifacts have been tested from the GitHub repository and the application successfully updates from that repository.

See [desktop updates](desktop-updates.md) and the [desktop update implementation plan](plans/desktop-updates-plan.md).

## Remaining `0.1.0` work

1. Confirm the final user-facing scope and known limitations for the release.
2. Prepare the `0.1.0` changelog entry and release notes from the completed feature set.
3. Synchronize the final `0.1.0` version across `package.json`, `package-lock.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, `src-tauri/tauri.conf.json`, updater tests, and the current-release example in the updater documentation.
4. Run the full release validation suite after the version bump.
5. Create the annotated `v0.1.0` tag and let the Windows release workflow create its draft.
6. Inspect the generated NSIS installer, updater signature, and `latest.json`, then publish the stable non-prerelease release.
7. Perform one final clean-install and update rehearsal against the published stable feed.

The version bump and tag should happen only after the release notes and final scope are settled.

## Initial release boundaries

The initial release remains intentionally local-first and Windows-focused:

- Packwiz projects, Git metadata, and user export destinations remain external resources and are never silently deleted or rewritten by application management actions.
- The alpha SQLite database has no migration workflow. Incompatible local databases require the documented stop-app/reset procedure in [data management](data-management.md).
- GitHub enrichment, CurseForge API access, cloud synchronization, collaboration, user accounts, unattended updates, automatic dependency conflict resolution, and non-Markdown changelog export remain outside `0.1.0`.
- Microsoft Authenticode signing is not included in the initial Windows release, so SmartScreen or unknown-publisher warnings may occur even though Tauri updater signatures protect updater artifacts.

## Post-`0.1.0` candidates

These are candidates for a future roadmap, not commitments for the initial release:

- Broader Packwiz compatibility and additional tested command profiles.
- CurseForge changelog retrieval and other provider integrations.
- Automatic cache expiration policies and additional export formats.
- GitHub enrichment and optional authenticated workflows.
- Cloud synchronization, collaboration, accounts, and multi-user permissions.
- Application-created backups, rollback assistance, and broader recovery tooling.
- Additional platform releases beyond Windows.
