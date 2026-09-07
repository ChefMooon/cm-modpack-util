# Modpack Registration

CM Modpack Util registers local Packwiz directories without running Packwiz, contacting a provider, or editing the selected files.

## Supported Evidence

The initial compatibility profile expects:

- `pack.toml` at the modpack root.
- A readable `[index]` section in `pack.toml` with a relative `file`, `hash-format`, and `hash`.
- A readable referenced index TOML with `[[files]]` entries.
- Mod metadata entries marked with `metafile = true` and ending in `.pw.toml`.
- Mod metadata with a string `name` and `side` of `client`, `server`, or `both`.
- Optional `[update.modrinth]` or `[update.curseforge]` provider tables.

The following Packwiz values are observed locally and are never written back to `pack.toml`: `name`, `author`, `version`, `pack-format`, index reference and hash fields, and declared `[versions]` values.

Missing optional top-level values are shown as unavailable. Malformed TOML, missing required index evidence, unreadable references, invalid metadata, and paths outside the selected modpack boundary fail validation or produce an explicit validation issue.

## Registration Flow

1. Choose a local directory with **Register modpack**.
2. Review the canonical path, observed Packwiz values, and validation results.
3. Edit the application-owned display name if needed.
4. Confirm registration.

Application-owned metadata is stored in SQLite separately from Packwiz observations. It includes display name, icon, theme, tags, favorite state, description, and lifecycle.

## Lifecycle

- **Active:** included in the normal modpack list.
- **Maintenance:** available for future modpack organization.
- **Archived:** retained but visually separated and restorable.
- **Disconnected:** retained with its application identity and history when the external directory is unavailable.

Reconnect always requires an explicit directory selection and a fresh validation pass. Disconnecting, archiving, or removing an application relationship never deletes the external directory or its Git data.

## Refresh and Offline Behavior

Opening and refreshing a modpack read the current local evidence. Refresh is explicit and read-only. A missing, inaccessible, or malformed external modpack is reported as unavailable rather than silently treated as valid.

Registration, opening, refresh, lifecycle changes, and reconnect do not run Packwiz commands or request Modrinth, CurseForge, GitHub, or other network data. Full mod inventory and update workflows are planned for later releases.
