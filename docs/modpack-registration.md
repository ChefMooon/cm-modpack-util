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

### Application-owned tag editing

Registration review and the focused modpack **Settings** tab use the same
structured Tags editor. Existing tags are shown as individually removable
values, and the text input stays available for the next tag. Press **Enter** or
type a comma to add a tag; spaces are valid inside a tag. A pasted or typed
comma-separated value commits each complete segment and leaves an unfinished
trailing segment in the input for continued editing.

Before tags are sent to the application metadata command, values are trimmed,
blank values are discarded, and duplicates are compared case-insensitively.
The first-entered spelling is preserved. Duplicate entries are ignored and
reported with a polite status message; they are not persisted. Removing a tag
changes only that value. Settings saves remain grouped: a failed save keeps the
draft, while a successful save reloads the normalized tag array. Formatting-only
differences in legacy casing or duplicate values do not make Settings appear
dirty.

The per-modpack theme is an application-owned identity color. New modpacks start with the neutral Cyan palette entry. Users can choose from eight named palette colors or provide a validated six-digit hex color in the registration review and Settings tab. The selected color is shown as a named swatch in the modpack list and may accent local modpack identity surfaces; it does not change the global application theme or any Packwiz file. Older missing or unrecognized theme values remain readable and display with the neutral fallback until replaced.

## Lifecycle

- **Active:** included in the normal modpack list.
- **Maintenance:** available for future modpack organization.
- **Archived:** retained but visually separated and restorable.
- **Disconnected:** retained with its application identity and history when the external directory is unavailable.

Reconnect always requires an explicit directory selection and a fresh validation pass. Archiving or removing an application relationship never deletes the external directory or its Git data. Disconnected is a recovery state for an unavailable directory rather than a manual organization action.

## Refresh and Offline Behavior

Opening and refreshing a modpack read the current local evidence. Refresh is explicit and read-only. A missing, inaccessible, or malformed external modpack is reported as unavailable rather than silently treated as valid.

Registration, opening, refresh, lifecycle changes, and reconnect do not run Packwiz commands or request Modrinth, CurseForge, GitHub, or other network data. Inventory inspection, update discovery, review, and targeted update workflows are available after registration through their separate application actions.
