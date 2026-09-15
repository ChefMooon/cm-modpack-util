# Settings

The `/settings` route provides the application's desktop preferences. Settings are persisted in the local SQLite database through typed Tauri commands and are applied immediately where supported.

## General

- **Hide to tray when closing** keeps the app running in the system tray when its window is closed.
- **Restore last window size and position** restores the previous window dimensions and screen position.
- **Launch at login** starts the app automatically when the user signs in through the Tauri autostart plugin.
- **Start minimized** starts the app in the background with its window minimized.

## Appearance

- **Theme** follows the system theme by default, with optional light and dark modes.

## Accessibility

- **Reduce motion** minimizes animations and transitions. The operating system's reduced-motion preference is respected by default.

All settings can be reset from the Advanced section of the Settings page. Resetting restores the defaults and disables launch-at-login.

## Terminology

Within CM Modpack Util, a **modpack** is a registered local Packwiz directory and its application-owned metadata, observations, snapshots, and history. Provider APIs may still use **project** in fields such as Modrinth `project_id`, CurseForge or Packwiz `project-id`, provider URLs, and external fixture values; those names are external contracts and are not renamed as part of the application vocabulary.
