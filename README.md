# CM Modpack Util

A local-first foundation for CM modpack projects, built with **Tauri 2**, **SvelteKit**, **TypeScript**, **Vite**, and **SQLite**.

Version 0.0.1 provides the Projects, Activity, and Settings shell, persisted desktop preferences, and tested safety boundaries for future project registration. Project folders, Packwiz files, network providers, Git history, and update operations are intentionally unavailable until later releases. The app does not fabricate project evidence or modify external project directories.

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

- `src/routes/+page.svelte` — Projects empty state and v0.0.1 foundation status
- `src/routes/activity/+page.svelte` — unavailable operation history state
- `src/routes/settings/+page.svelte` — settings screen with persisted theme preference
- `src/lib/settings.ts` — product-owned typed settings invocation helper
- `src/lib/domain.ts` — shared project, validation, and operation contracts
- `src/lib/errors.ts` — structured Tauri command error translation
- `src/app.css` — shared theme tokens and global accessibility styles
- `src/components/` — shared UI, settings components, and reusable application components
- `src/components/ui/Button.svelte` — common button primitive with shared variants and sizes
- `src/components/ui/Modal.svelte` — accessible modal dialog component
- `src/components/ui/Tooltip.svelte` — reusable tooltip wrapper
- `src/components/ui/toast/` — global toast state, viewport, and toast item components
- `src-tauri/src/lib.rs` — Rust commands and Tauri setup
- `src-tauri/src/db/` — SQLite database setup and settings repository
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

## v0.0.1 boundaries

- **Projects:** No project is registered in v0.0.1. Registration and validation begin in v0.0.2.
- **Activity:** No operation history is shown because project registration, Packwiz execution, filesystem scans, and network requests are unavailable.
- **Source of truth:** Packwiz files will remain authoritative for external modpack state once integration exists. SQLite currently owns only application settings.
- **Safety:** Rust owns canonicalization and registered-project containment. The boundary accepts existing absolute paths only after they are under an explicitly registered root; relative, nonexistent, escaping, and unregistered paths are rejected. Symlink and junction policy is deferred.
- **Database:** Startup applies the idempotent settings schema with `CREATE TABLE IF NOT EXISTS`; no migration framework or workflow tables are present.
- **Accessibility:** The shell preserves visible focus, keyboard navigation, accessible names, tooltips for unfamiliar controls, and color-independent status meaning.

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

## Settings database

The desktop app stores settings in `settings.sqlite` inside Tauri's application data directory. The Rust layer owns database access and exposes typed Tauri commands to the frontend. Settings values are stored as JSON by key, allowing new preferences to be added without changing the table structure.

This template intentionally does not use migrations. `src-tauri/src/db/schema.sql` is applied with `CREATE TABLE IF NOT EXISTS` during startup. If a future structural schema change is required, document it explicitly and reset or manually upgrade the local database rather than silently deleting user data. The development reset action is available on the Settings page.

## Recommended VS Code extensions

- [Svelte for VS Code](https://marketplace.visualstudio.com/items?itemName=svelte.svelte-vscode)
- [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode)
- [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)
