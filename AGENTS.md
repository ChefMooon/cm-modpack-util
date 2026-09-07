# CM Modpack Util

## Project Overview

CM Modpack Util is a local-first Tauri 2 desktop application with a SvelteKit, TypeScript, and Vite frontend and a Rust backend. It manages registered CM modpack projects and reads Packwiz evidence offline.

The Packwiz example project is located at `../example-modpack`, relative to this repository root. It contains the reference `pack.toml`, `index.toml`, and `mods/*.pw.toml` layout used by fixtures and documentation.

## Commands

Install dependencies before the first run:

```bash
npm install
```

Frontend checks and builds:

```bash
npm run check
npm run build
```

Desktop development and packaging:

```bash
npm run tauri dev
npm run tauri build
```

Rust validation:

```bash
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml
```

Run focused checks before broad validation when changing one layer. Use `git diff --check` before handoff. Do not modify `../example-modpack` during tests or development checks.

## Architecture and Boundaries

- `src/routes/` contains route-level Svelte UI. `src/components/` contains reusable UI and settings components.
- `src/lib/` contains frontend types, settings/project invocation wrappers, and command-error translation.
- `src-tauri/src/lib.rs` owns Tauri setup and command registration.
- `src-tauri/src/db/` owns SQLite initialization, schema application, persistence, and database commands. `src-tauri/src/db/schema.sql` is the schema source of truth; while the application is in alpha, do not add or maintain database migrations.
- Breaking database schema changes are acceptable during alpha. If a change makes an existing local database incompatible, tell the user to stop the app and reset/delete the local database files so the current schema can be created again; do not silently discard or rewrite user data.
- `src-tauri/src/domain/` owns Rust domain contracts, validation, inventory, and evidence interpretation.
- `src-tauri/src/safety/` owns canonical path handling and registered-root containment.
- `src-tauri/src/discovery/` owns the profiled Packwiz discovery process, fingerprints, cancellation, and outcome classification.
- Rust is the authority for filesystem access, SQLite, Git inspection, URL trust, process execution, path authorization, and safety classification. Svelte must not duplicate or bypass these decisions.
- Packwiz project files remain authoritative for external modpack state. SQLite stores application metadata, observed evidence, validation results, lifecycle state, and activity/discovery observations.
- Current workflows are local and read-only with respect to external project files. Do not add network requests, Packwiz mutation, update application, guessed provider URLs, arbitrary process execution, or frontend path construction without an explicit scope decision and corresponding tests.

## Frontend Conventions

- Follow the shared design tokens and accessibility rules in `src/app.css` and [`docs/desktop-ui-standards.md`](docs/desktop-ui-standards.md).
- Prefer shared primitives in `src/components/ui/`, especially `Button.svelte`, `Modal.svelte`, `Tooltip.svelte`, and the toast system.
- Use typed wrappers in `src/lib/` for Tauri commands rather than scattering direct `invoke` calls through routes.
- Preserve loading, empty, unavailable, stale, disconnected, validation-failure, and command-failure states where a workflow can produce them.
- Keep keyboard access, visible focus, semantic status, reduced-motion behavior, stable layouts, and color-independent meaning intact.

## Documentation Sources

Read the relevant source before changing behavior:

- [`README.md`](README.md) for setup, structure, supported alpha behavior, and database reset boundaries.
- [`docs/CM-MODPACK-UTIL-SPEC.md`](docs/CM-MODPACK-UTIL-SPEC.md) for product scope and contracts.
- [`docs/ROADMAP.md`](docs/ROADMAP.md) for planned release scope.
- [`docs/modpack-registration.md`](docs/modpack-registration.md) for registration behavior and path safety.
- [`docs/packwiz-commands.md`](docs/packwiz-commands.md) for the supported Packwiz command profile and discovery safety decisions.
- [`docs/desktop-ui-standards.md`](docs/desktop-ui-standards.md) and [`docs/STYLE-GUIDE.md`](docs/STYLE-GUIDE.md) for UI and styling conventions.
- [`docs/toast.md`](docs/toast.md) for the global toast API.
- [`docs/plans/`](docs/plans/) for phase decisions, handoffs, verification evidence, and deferred scope.

When documentation already covers a rule, link to it rather than duplicating a second source of truth.

## Change and Validation Expectations

Keep changes focused and preserve existing user changes in a dirty worktree. Add or update focused tests for Rust contracts, path boundaries, persistence, process safety, and fixture behavior when those areas change. For frontend-only changes, run `npm run check` and `npm run build`; for Rust changes, run formatting and the Cargo test suite as well. Record unresolved validation failures as blocked rather than claiming completion.
