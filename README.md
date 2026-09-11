# CM Modpack Util

A local-first foundation for CM modpacks, built with **Tauri 2**, **SvelteKit**, **TypeScript**, **Vite**, and **SQLite**.

The v0.0.9 alpha provides local Packwiz modpack registration, inventory inspection, durable update-check snapshots, recoverable release workspaces, captured-state comparison, review decisions and notes, verified positional-slug pin/unpin operations, sequential selected updates, mutation history, modpack overview facts, changelog generation, Markdown export, data management, cleanup, and the Modpacks, Releases, Activity, Management, and Settings shell. It reads Packwiz evidence offline for snapshot baselines and release comparison, while explicitly selected targeted updates and changelog generation may use their documented provider/network behavior. New releases begin from reviewable snapshots; direct capture-to-release creation is rejected.

## v0.0.9 data management boundaries

Management actions operate only on application-owned SQLite records and retained
provider-cache or export observations. Archive, restore, and detach actions are
previewed before mutation; permanent deletion requires an impact preview and the
exact target-specific phrase `DELETE <record-id>`. Cleanup defaults to scoped,
unreferenced provider-cache rows and obsolete export records, and reports
protected references or partial outcomes instead of claiming that all data was
removed.

Registered Packwiz directories, Packwiz files, Git metadata/history, and user
export destination files are external resources and are never deleted, moved, or
rewritten by lifecycle or cleanup actions. A broken application parent is shown
as an orphaned record; an unavailable export destination is a separate persisted
observation and does not authorize external-file access. Storage reporting counts
application-owned bytes and lists external or unavailable observations separately.

## v0.0.8 release boundaries

- **Workspace:** A release workspace starts explicitly from a reviewable snapshot, retains release-owned decisions, and remains resumable until the user abandons or finalizes it. Snapshot evidence and workspace history are separate.
- **Comparison:** The Releases screen compares two persisted captures. It does not reread the active Packwiz directory, check out historical Git state, or replace unavailable evidence with current files.
- **Metadata:** Name, version, description, notes, publication status, and an optional snapshot association are application-owned fields. Capture time, fingerprints, provenance, and captured Packwiz evidence remain immutable.
- **Git:** Git provenance is optional. No repository, commit, tag, remote, or clean working tree is required. No-Git, dirty, conflicted, missing-commit, and unavailable-history states remain explicit evidence.
- **Provisional releases:** Partial but eligible evidence may remain visibly provisional. Promotion to published status requires a fresh validated capture; editing status alone cannot promote stale evidence.
- **Network boundary:** GitHub enrichment is deferred. Local release creation, loading, and comparison make no GitHub request and do not require an account or public remote.
- **Database reset:** The alpha schema has a compatibility sentinel and no migrations. If startup reports a missing or incompatible schema, stop the app and delete the database plus its WAL and SHM sidecars using the [alpha database behavior](#alpha-database-behavior) instructions.

## Changelog workflow

The snapshot review includes an explicit **Generate changelog** action. It uses
the selected snapshot's immutable candidates, keeps the local download provider
separate from Modrinth changelog evidence, and shows unresolved, ambiguous,
missing, and failed provider results without presenting them as confirmed.
Generated Markdown is stored as an application-owned artifact and can be sent
to a destination selected through the native save dialog. Export observations
retain the destination, content, SHA-256 hash, status, and timestamp; the
application does not delete external files.

Provider access is limited to explicit generation. Exact cached Modrinth
responses can be reused offline only when the stored project/version
association matches the requested local version. Online generation performs one
exact version lookup per changed entry and waits three seconds between lookups
to reduce API pressure. Progress events report preparation, each lookup,
artifact persistence, and completion in the review view. Generation can be
stopped between lookups; fetched entries are retained in a partial artifact and
the review can keep or discard that result without deleting its history. Existing alpha databases use
the current schema; stop the app and reset `cm-modpack-util.sqlite` if an
incompatible local schema prevents startup.
## v0.0.6 mutation boundaries

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

## Project structure

- `src/routes/+page.svelte` — Modpack registration, validation, metadata, and lifecycle workflow
- `src/routes/activity/+page.svelte` — durable snapshot and operation history
- `src/routes/releases/+page.svelte` — named release history, capture metadata, and comparison
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

## v0.0.2 boundaries

- **Modpacks:** Local Packwiz directories can be previewed, validated, registered, reopened offline, refreshed, archived, restored, disconnected, reconnected, and edited without external file mutation.
- **Activity:** Operation history is not yet implemented because Packwiz execution, filesystem scans beyond registration evidence, and network requests remain unavailable.
- **Source of truth:** Packwiz files remain authoritative for external modpack state. SQLite owns application metadata, observed evidence, validation results, and lifecycle state.
- **Safety:** Rust owns canonicalization and registered-modpack containment. Equivalent path spellings are rejected as duplicates, and relative, nonexistent, escaping, and unregistered paths are rejected. Symlinks and junctions follow canonical operating-system path resolution and remain inside the registered root.
- **Database:** Startup bootstraps only an empty database and checks the schema compatibility sentinel thereafter; no migration framework is present.
- **Accessibility:** The shell preserves visible focus, keyboard navigation, accessible names, tooltips for unfamiliar controls, and color-independent status meaning.

## Alpha database behavior

- The local SQLite database is initialized with the current schema only when it is empty. Existing databases must contain the current schema compatibility sentinel.
- There is no migration, backup, or in-app database rebuild workflow in alpha.
- The v0.0.9 schema compatibility sentinel is version 4. Existing version-3 alpha databases are intentionally incompatible and must be reset manually using the stop-app/delete-database procedure below; the app does not migrate or rewrite them.
- When a clean reset is needed during development, stop the app and delete `cm-modpack-util.sqlite`, `cm-modpack-util.sqlite-wal`, and `cm-modpack-util.sqlite-shm` from Tauri's application data directory. The app recreates the application database on the next launch. Existing alpha installations using `settings.sqlite` must be reset manually; the app does not silently migrate or rename that file.
- Packwiz modpack files remain outside the database lifecycle and are not deleted by a database reset.

## v0.0.5 snapshot review boundaries

- **Review:** A safe, normal discovery result creates a durable snapshot with immutable candidate and fingerprint evidence. Selected, skipped, blocked, and deferred values are application-owned decisions; the Packwiz `pin` field remains evidence only.
- **Abnormal outcomes:** Cancelled, unsafe, unsupported, failed, and indeterminate attempts remain visible but are not reviewable as authoritative candidate results.
- **Freshness:** Native Rust rechecks the registered modpack's relevant fingerprint. Changed, missing, unreadable, incomplete, or incomparable evidence marks the snapshot stale and blocks further review writes.
- **History:** Decisions and notes are append-only records. Closing, cancelling, and retrying never rewrites the original snapshot. A retry receives a new snapshot ID linked to its predecessor.
- **Pin operations:** Pin and unpin resolve one fresh native inventory entry, derive its `.pw.toml` slug, run `packwiz pin <slug>` or `packwiz unpin <slug>` without `--yes`, re-read the metadata, and report success only after verification. The Activity route preserves the immutable attempt and its evidence.
- **Apply boundary:** Apply uses only exact local metadata slugs with one `packwiz update <slug>` process per selected candidate. The audited `update -a` path remains discovery-only and is not used for mutation. A process exit code alone is not considered success; the application re-reads inventory and records per-target verification and failures.
- **Reset fallback:** Stop the app and delete the incompatible application database and its WAL and SHM sidecars. This does not delete Packwiz files or silently rewrite application data.

## Release workspace boundaries

- A modpack may have any number of editable release workspaces. Each workspace owns its lifecycle, candidate decisions, activity, and immutable baseline independently; opening or starting one workspace never resumes or replaces another.
- Unlinking a discovery snapshot removes only its provenance link. The workspace retains the snapshot-derived immutable baseline and records the unlink in workspace activity.
- Rebasing is explicit and in-place. It captures the current registered project files as a new immutable workspace baseline, clears snapshot/release provenance, returns the workspace to draft review, and never occurs implicitly during observation or apply.

## Common UI components

Reusable UI components live in `src/components/ui/` and should be preferred over creating one-off controls in routes.

### Button

`Button.svelte` provides consistent button styling, focus states, disabled behavior, and accessibility support. It defaults to `type="button"` to avoid accidental form submission.

Supported variants include `primary`, `secondary`, `outline`, `danger`, `ghost`, `quiet`, `info`, `success`, and `warning`. Supported sizes are `sm`, `md`, `lg`, and `icon`.

```svelte
<script lang="ts">
  import Button from "../components/ui/Button.svelte";
  import XIcon from "phosphor-svelte/lib/XIcon";
</script>

<Button variant="primary" type="submit">Save changes</Button>
<Button variant="danger">Delete</Button>
<Button variant="ghost" size="icon" aria-label="Close">
  <XIcon size={16} aria-hidden="true" />
</Button>
```

Use an accessible `aria-label` and/or `title` for icon-only buttons. Native button attributes such as `aria-pressed`, `aria-expanded`, and `onclick` are forwarded to the underlying button.

### Toast notifications

The global toast system is documented in [`docs/toast.md`](docs/toast.md). Use `useToast()` to create, update, or dismiss notifications. The viewport is mounted by the root layout, so toasts remain visible during route changes.

```svelte
<script lang="ts">
  import Button from "../components/ui/Button.svelte";
  import { useToast } from "../components/ui/toast/toast.svelte";

  const { toast } = useToast();
</script>

<Button onclick={() => toast({ title: "Saved", severity: "success" })}>
  Save
</Button>
```

Toasts support `info`, `success`, `warning`, and `error` severities, optional descriptions and actions, automatic dismissal, persistent notifications with `duration: 0`, queueing, and pause-on-hover/focus behavior.

### Modal and Tooltip

- `Modal.svelte` provides a modal dialog with focus handling, Escape-to-close behavior, and a built-in accessible close button.
- `Tooltip.svelte` provides a lightweight wrapper for adding tooltip text to inline content.

## Adding a Rust command

Define a function with `#[tauri::command]`, register it in `invoke_handler`, then call it from Svelte with `invoke`:

```ts
import { invoke } from "@tauri-apps/api/core";

const result = await invoke("my_command", { value: "hello" });
```

See the [Tauri documentation](https://v2.tauri.app/) for APIs, capabilities, and release configuration.

## Application database

The desktop app stores its application state in `cm-modpack-util.sqlite` inside Tauri's application data directory. The database contains settings, registered modpacks, observed evidence, snapshots, operations, and changelog artifacts. The Rust layer owns database access and exposes typed Tauri commands to the frontend. Settings values are stored as JSON by key, allowing new preferences to be added without changing the table structure.

This alpha intentionally does not use migrations. `src-tauri/src/db/schema.sql` is applied with `CREATE TABLE IF NOT EXISTS` during startup. If a future structural schema change is required, document it explicitly and reset or manually upgrade the local database rather than silently deleting user data. The development reset action is available on the Settings page.

### Terminology

Within CM Modpack Util, a **modpack** is a registered local Packwiz directory and its application-owned metadata, observations, snapshots, and history. Provider APIs may still use **project** in fields such as Modrinth `project_id`, CurseForge or Packwiz `project-id`, provider URLs, and external fixture values; those names are external contracts and must not be renamed as part of the application vocabulary.

## Recommended VS Code extensions

- [Svelte for VS Code](https://marketplace.visualstudio.com/items?itemName=svelte.svelte-vscode)
- [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode)
- [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)
