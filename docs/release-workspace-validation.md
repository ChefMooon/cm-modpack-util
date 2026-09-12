# Release Workspace Validation

The release workspace watcher is Rust-owned and uses `notify` to observe only the Packwiz fingerprint boundary. Browser-only checks cannot prove native filesystem timing, so validate the following in a Tauri desktop run after resetting an incompatible alpha database as described in `README.md`:

1. Check for updates, open a reviewable snapshot, and choose `Start Release Review`.
2. Confirm the workspace appears in Versions and resumes after closing and reopening the app.
3. Start a second release workspace for the same modpack from another snapshot or current project state. Confirm both editable workspaces remain independently visible and resumable.
4. Edit `pack.toml`, the referenced index, an indexed `*.pw.toml`, or a file under a fingerprint-covered content root from a terminal. Confirm the workspace records the changed scope as external evidence, remains usable without an acknowledgement or rebase action, and preserves candidate decisions and snapshot provenance.
5. Use `Unlink snapshot` on an editable workspace. Confirm the source link disappears while the detached snapshot-derived baseline remains visible and immutable.
6. Use `Rebase from current files` explicitly. Confirm the workspace records a new current-project baseline, clears prior snapshot/release provenance, returns to draft review, and does not mutate Packwiz files.
7. Edit an irrelevant file in the registered directory. Confirm no workspace observation is emitted.
8. Start an app-owned apply, edit a relevant file during the operation, and confirm the activity records overlap and the workspace remains blocked for explicit resolution.
9. Abandon a workspace and confirm Packwiz files are unchanged, history remains visible, and a later snapshot can start a new workspace.
10. Finalize a validated changed result, confirm observation stops at `ready_to_finalize`/finalized, publish it, close and reopen the app, and confirm the historical record remains read-only and does not restart a watcher.
11. Reopen the published record, use the visible `Withdraw release` action, confirm the warning, and verify it becomes withdrawn, preserves the captured evidence and activity history, remains read-only, and does not restart observation. Confirm withdrawal is rejected for any non-published lifecycle.
12. Open a draft with no candidates and a draft with candidates but none selected. Confirm the workspace explains why Apply is unavailable and offers discovery, rebase, or abandonment without implying that a no-op can be finalized.
13. Generate a proposed workspace changelog after an external edit. Confirm the proposal includes a clearly labeled externally observed section and does not describe the external edit as applied by CM Modpack Util.
14. Open a workspace with more than ten activity records. Confirm the history is collapsed by default, the newest ten records show readable timestamps, and `Show more history` appends older records without duplicates.
15. Collapse the activity history while observing files. Confirm the observer status and `Observe files`/`Stop observing` controls remain visible in the evidence metadata area and watcher behavior is unchanged.

Record the desktop build, fixture/project path, lifecycle transitions, observed event payloads, and any native timing limitation with the release validation evidence.