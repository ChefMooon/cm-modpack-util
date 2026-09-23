# Data Management

CM Modpack Util stores application-owned state in SQLite inside Tauri's application data directory. Release builds use `cm-modpack-util.sqlite` directly in that directory; development builds use `development/cm-modpack-util.sqlite`. The database contains settings, registered modpacks, observed evidence, snapshots, operations, release workspaces, releases, changelog artifacts, provider cache, export observations, and management state. Rust owns database access and exposes typed Tauri commands to the frontend.

## Application data transfer

Management can export application-owned state as a versioned `cm-modpack-util-data` bundle. The first supported format is canonical UTF-8 JSON, optionally gzip-compressed, with SHA-256 integrity metadata covering the uncompressed payload. Bundles include settings, registered modpack metadata and observations, snapshots, decisions, notes, releases, release workspaces, operation/history records, changelog artifacts and revisions, and durable changelog export records.

Import validates the format, version, integrity hash, resource limits, and compatibility before showing a deterministic preview. It merges only selected non-conflicting records in one transaction. Identical records, same-ID conflicts, same-path conflicts, dependent-record skips, unavailable paths, and no-op imports remain explicit. Unavailable modpack paths stay disconnected, generated integer keys are remapped locally, and imported lifecycle/history records are evidence only. Supported operating-system settings effects must succeed before the database commit or the complete import is rejected.

The bundle is application-data recovery and transfer, not a Packwiz project backup. Packwiz project directories and files remain authoritative and are never copied, rewritten, or reconnected. Provider-response cache, Git metadata/history, and external changelog destination files are excluded; stored changelog export records remain included, but their destination files are never read or modified. Import can be cancelled through preview and before commit; an active transaction runs to completion.

## Cleanup and lifecycle management

Management actions operate only on application-owned SQLite records and retained provider-cache or export observations. Archive, restore, and detach actions are previewed before mutation. Permanent deletion requires an impact preview and the exact target-specific phrase `DELETE <record-id>`.

Cleanup is scoped to eligible, unreferenced provider-cache rows and obsolete export records. Protected references, orphaned records, unavailable export destinations, and partial outcomes remain explicit rather than being treated as successful deletion.

Packwiz project directories, Packwiz files, Git metadata/history, and user export destination files remain external resources. Lifecycle and cleanup actions never delete, move, or rewrite them.

## Database behavior

Debug-assertion builds, including `npm run tauri dev`, use `development/cm-modpack-util.sqlite` beneath Tauri's application data directory. Release builds, including `npm run tauri build`, continue to use `cm-modpack-util.sqlite` directly in the application data directory. The development database starts independently; the application does not copy, import, or migrate production data into it.

The alpha database is initialized from the current schema only when it is empty. Existing databases must contain the current schema compatibility sentinel. There is no database migration, replacement, or in-app database rebuild workflow in alpha. Application-data bundles are not raw database backups.

The schema compatibility sentinel is version 1. Existing databases with another schema version are intentionally incompatible and must be reset manually; the app does not migrate or rewrite them.

To reset development data, stop the development app and delete `cm-modpack-util.sqlite`, `cm-modpack-util.sqlite-wal`, and `cm-modpack-util.sqlite-shm` from the `development` subdirectory of Tauri's application data directory. The app recreates the development database on the next launch. Do not delete the files directly in the application data directory when resetting development data; those are the production database and its sidecars. Existing alpha installations using `settings.sqlite` must be reset manually; the app does not silently migrate or rename that file. Packwiz modpack files remain outside the database lifecycle and are not deleted by a database reset.

Settings values are stored as JSON by key, allowing new preferences to be added without changing the table structure. If a future structural schema change is required, document it explicitly and reset or manually upgrade the local database rather than silently deleting user data. The development reset action is available on the Settings page.
