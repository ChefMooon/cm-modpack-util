# CM Modpack Util

A local-first foundation for CM modpacks, built with **Tauri 2**, **SvelteKit**, **TypeScript**, **Vite**, and **SQLite**.

The current alpha provides local Packwiz modpack registration, inventory inspection,
durable update-check snapshots, recoverable release workspaces, captured-state
comparison, review decisions and notes, verified positional-slug pin/unpin
operations, sequential selected updates, mutation history, modpack overview
facts, changelog generation, Markdown export, data management, cleanup, and the
Modpacks, Activity, Management, and Settings shell, including release workspaces.
See
[CHANGELOG.md](CHANGELOG.md) for the release history.
Windows release and updater-key custody are documented in
[docs/desktop-updates.md](docs/desktop-updates.md).
The desktop updater checks stable GitHub Release metadata at startup and from
Settings without downloading installer bytes. Installation requires explicit
confirmation. Windows downloads first, then presents **Install and restart**
or **Later** before launching the installer.

## Features

- [Local modpack workspace](#modpack-workflows) for registering Packwiz
  directories, validating them, reviewing inventory, and managing metadata.
- [Snapshot review and selected updates](#snapshot-review-and-updates) with
  immutable evidence, freshness checks, verified Packwiz operations, and
  cancellation or partial-outcome reporting.
- [Release workspaces and comparison](#release-workspaces) built from
  persisted captures rather than the current working tree.
- [Changelog generation and Markdown export](#changelog-generation-and-export)
  with offline cache reuse, bounded provider access, and retained export
  observations.
- [Data management and cleanup](#data-management-and-cleanup) for
  application-owned records, provider cache, and export observations.
- [Settings and accessibility](#basic-settings) for startup behavior, theme,
  reduced motion, and preference reset.

## Modpack workflows

Register a local Packwiz directory, validate its path and metadata, reopen it
offline, refresh its observations, and edit application-owned metadata. Packwiz
files remain the source of truth for external modpack state; the application
does not rewrite them during registration or lifecycle actions.

## Snapshot review and updates

Run the supported Packwiz discovery profile to create an immutable snapshot of
the relevant local evidence. Review decisions and notes remain durable, and
freshness checks prevent writes when the registered project changes. Selected
pin, unpin, and update operations use verified local metadata slugs and retain
their process and verification evidence.

## Release workspaces

Create independent release workspaces from reviewable snapshots, retain
release-owned decisions, and compare persisted captures. Release evidence
includes captured Packwiz state and optional Git provenance; it does not require
Git, a clean working tree, a public remote, or GitHub access.

## Changelog generation and export

The snapshot review includes an explicit **Generate changelog** action. It uses
selected snapshot candidates, keeps the local download provider separate from
Modrinth changelog evidence, and shows unresolved, ambiguous, missing, and
failed provider results without presenting them as confirmed.

Generated Markdown is stored as an application-owned artifact and can be sent
to a destination selected through the native save dialog. Export observations
retain the destination, content, SHA-256 hash, status, and timestamp; the
application does not delete external files.

Provider access is limited to explicit generation. Exact cached Modrinth
responses can be reused offline when the stored project/version association
matches the requested local version. Online generation performs one exact
version lookup per changed entry and waits three seconds between lookups.
Generation can be stopped between lookups; fetched entries are retained in a
partial artifact and the review can keep or discard that result without
deleting its history.

## Data management and cleanup

Management actions operate only on application-owned SQLite records and retained
provider-cache or export observations. Archive, restore, and detach actions are
previewed before mutation. Permanent deletion requires an impact preview and
the exact target-specific phrase `DELETE <record-id>`.

Cleanup is scoped to eligible, unreferenced provider-cache rows and obsolete
export records. Protected references, orphaned records, unavailable export
destinations, and partial outcomes remain explicit rather than being treated as
successful deletion.

## Current compatibility and data boundaries

- Packwiz support follows the tested [compatibility profile](docs/packwiz-commands.md).
  Unsupported executable identities and output formats remain explicit unsupported
  or indeterminate outcomes; the application does not guess or broaden support.
- Packwiz project directories, Packwiz files, Git metadata/history, and user export
  destination files remain external resources. Lifecycle and cleanup actions never
  delete, move, or rewrite them.
- Release workspaces compare persisted captures and do not silently reread the
  active Packwiz directory or replace unavailable evidence with current files.
- GitHub enrichment is deferred. Local release creation, loading, and comparison
  do not require an account or public remote.
- Snapshots retain immutable evidence and explicit review decisions. Freshness
  checks block writes when the registered Packwiz project changes, and retries
  receive new linked snapshot IDs.
- Pin, unpin, and selected update operations use verified local metadata slugs,
  preserve immutable operation evidence, and report cancellation or partial
  outcomes explicitly.
- The alpha database has no migration workflow. If startup reports a missing or
  incompatible schema, stop the app and reset the database as described in
  [Alpha database behavior](#alpha-database-behavior).

## Prerequisites

- [Node.js](https://nodejs.org/) 20 or newer
- [Rust](https://www.rust-lang.org/tools/install) and Cargo
- Platform dependencies for [Tauri](https://v2.tauri.app/start/prerequisites/)

## Getting started

```bash
npm install
npm run tauri dev
```

The Vite-only frontend can also be run with `npm run dev`.

## Available scripts

| Command | Description |
| --- | --- |
| `npm run tauri dev` | Start the desktop app in development mode |
| `npm run tauri build` | Create a production desktop bundle |
| `npm run dev` | Start the SvelteKit development server |
| `npm run build` | Build the static frontend |
| `npm run check` | Run Svelte and TypeScript checks |
| `npm test` | Run focused frontend update-contract checks |

## Project structure

- `src/routes/+page.svelte` — Modpack registration, validation, metadata, and lifecycle workflow
- `src/routes/activity/+page.svelte` — durable snapshot and operation history
- `src/routes/management/+page.svelte` — application record lifecycle, cleanup, and storage reporting
- `src/routes/settings/+page.svelte` — settings screen with persisted theme preference
- `src/lib/settings.ts` — product-owned typed settings invocation helper
- `src/lib/domain.ts` — shared modpack, validation, and operation contracts
- `src/lib/errors.ts` — structured Tauri command error translation
- `src/app.css` — shared theme tokens and global accessibility styles
- `src/components/` — shared UI, settings components, and reusable application components
- `src/components/ui/Button.svelte` — common button primitive with shared variants and sizes
- `src/components/ui/Modal.svelte` — accessible modal dialog component
- `src/components/ui/Tooltip.svelte` — reusable tooltip wrapper
- `src/components/ui/toast/` — global toast state, viewport, and toast item components
- `src-tauri/src/lib.rs` — Rust commands and Tauri setup
- `src-tauri/src/db/` — SQLite application database setup and settings repository
- `src-tauri/src/domain/` — Rust-owned domain and structured error contracts
- `src-tauri/src/safety/` — canonical registered-root path boundary and tests
- `src-tauri/src/db/schema.sql` — canonical schema (no migration system)
- `src-tauri/tauri.conf.json` — window and bundle configuration
- `static/` — static assets

## Basic settings

The `/settings` route provides the application's desktop preferences. Settings are persisted in the local SQLite database through typed Tauri commands and are applied immediately where supported.

### General

- **Hide to tray when closing** — Keeps the app running in the system tray when its window is closed.
- **Restore last window size and position** — Restores the previous window dimensions and screen position.
- **Launch at login** — Starts the app automatically when the user signs in. This uses the Tauri autostart plugin.
- **Start minimized** — Starts the app in the background with its window minimized.

### Appearance

- **Theme** — Follows the system theme by default, with optional light and dark modes.

### Accessibility

- **Reduce motion** — Minimizes animations and transitions. The operating system's reduced-motion preference is respected by default.

All of these settings can be reset from the Advanced section of the Settings page. Resetting restores the defaults and disables launch-at-login.

## Alpha database behavior

- The local SQLite database is initialized with the current schema only when it is empty. Existing databases must contain the current schema compatibility sentinel.
- There is no migration, backup, or in-app database rebuild workflow in alpha.
- The v0.0.9 schema compatibility sentinel is version 4. Existing version-3 alpha databases are intentionally incompatible and must be reset manually using the stop-app/delete-database procedure below; the app does not migrate or rewrite them.
- When a clean reset is needed during development, stop the app and delete `cm-modpack-util.sqlite`, `cm-modpack-util.sqlite-wal`, and `cm-modpack-util.sqlite-shm` from Tauri's application data directory. The app recreates the application database on the next launch. Existing alpha installations using `settings.sqlite` must be reset manually; the app does not silently migrate or rename that file.
- Packwiz modpack files remain outside the database lifecycle and are not deleted by a database reset.

## Application database

The desktop app stores its application state in `cm-modpack-util.sqlite` inside
Tauri's application data directory. The database contains settings, registered
modpacks, observed evidence, snapshots, operations, release workspaces,
releases, changelog artifacts, provider cache, export observations, and
management state. The Rust layer owns database access and exposes typed Tauri
commands to the frontend. Settings values are stored as JSON by key, allowing
new preferences to be added without changing the table structure.

This alpha intentionally does not use migrations. An empty database is created
from `src-tauri/src/db/schema.sql`; an existing database must contain the
current schema compatibility sentinel. If a future structural schema change is
required, document it explicitly and reset or manually upgrade the local
database rather than silently deleting user data. The development reset action
is available on the Settings page.

### Terminology

Within CM Modpack Util, a **modpack** is a registered local Packwiz directory and its application-owned metadata, observations, snapshots, and history. Provider APIs may still use **project** in fields such as Modrinth `project_id`, CurseForge or Packwiz `project-id`, provider URLs, and external fixture values; those names are external contracts and must not be renamed as part of the application vocabulary.

## Recommended VS Code extensions

- [Svelte for VS Code](https://marketplace.visualstudio.com/items?itemName=svelte.svelte-vscode)
- [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode)
- [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)
