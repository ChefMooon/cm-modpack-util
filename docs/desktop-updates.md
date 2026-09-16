# Desktop update release procedure

For a reusable implementation guide covering the Tauri frontend, Rust
integration, GitHub Actions workflow, signing keys, and Actions secrets, see
the [Tauri updater setup guide](tauri-updater-guide.md).

CM Modpack Util publishes stable Windows x64 releases as NSIS installers.
Release tags use the `v<semver>` form, such as `v0.0.17`. The application
version in `src-tauri/tauri.conf.json` is authoritative and must match
`package.json`, `src-tauri/Cargo.toml`, the tag, and the visible application
version.

## Signing custody

Tauri updater signatures protect the downloaded updater artifact. They are
separate from Microsoft Authenticode signing; the first Windows release does
not include an Authenticode certificate, so SmartScreen or unknown-publisher
warnings are possible.

The maintainer must keep the Tauri updater private key outside this repository
and back it up in a secure password manager or encrypted offline backup. Never
commit, paste, print, or log the private key or its password. The public key is
embedded in `src-tauri/tauri.conf.json`.

Before the first release, add these repository secrets under **Settings >
Secrets and variables > Actions > New repository secret**:

| Secret | Value |
| --- | --- |
| `TAURI_SIGNING_PRIVATE_KEY` | The complete private key file contents |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | The password chosen when the key was generated; leave empty only if the key has no password |

The workflow also uses GitHub's automatically provided `GITHUB_TOKEN` with
`contents: write`; it is not application signing material.

## Temporary test release

1. Confirm the version is synchronized across the authoritative Tauri config,
   `package.json`, and `src-tauri/Cargo.toml`.
2. Add the release entry to `CHANGELOG.md` before committing and tagging. The
   workflow extracts the matching tagged entry into both the GitHub release
   body and the updater manifest. Editing the GitHub draft afterward does not
   change `latest.json`.
3. Confirm both signing secrets exist and are repository Actions secrets.
4. Create and push a `v<semver>` tag.
5. Let `.github/workflows/release.yml` create the draft release on Windows.
6. Inspect the draft assets for the NSIS `*-setup.exe`, its `.sig`, and
   `latest.json`. Confirm the manifest points to the uploaded NSIS asset and
   contains its signature.
7. Manually publish the release with a clearly marked test-only description.
8. Rehearse installation and updating from the tag-specific endpoint:
   `https://github.com/ChefMooon/cm-modpack-util/releases/download/v0.0.10/latest.json`.
9. Remove the temporary release, or retain it only if the release checklist
   explicitly records why it is safe to keep.

The app uses the stable endpoint
`https://github.com/ChefMooon/cm-modpack-util/releases/latest/download/latest.json`
only after the published temporary release rehearsal passes and a stable
release is published. Until then, keep the configured endpoint tag-specific.
Do not use a draft release to validate the public `latest` feed.

## Stable release

After the temporary rehearsal passes, update the authoritative version files,
run the repository validation commands, push the matching `v<semver>` tag, and
review the generated draft assets before publishing. Publish only after the
NSIS installer, `.sig`, and `latest.json` have been checked. The published
stable release must remain the newest non-prerelease GitHub release so the
`releases/latest` endpoint resolves to the intended version.

At runtime, startup and manual checks fetch release metadata only. The app
shows the current version, available version, release date, and notes before
the user selects **Download update**. On Windows, the signed artifact is
downloaded first; the shared toast then offers **Install and restart** or
**Later** before the installer is launched. Failed, offline, malformed,
unsigned, interrupted, and install-failure states remain visible with a retry
or continue-using-app action, and do not block local modpack workflows.

Do not rotate or replace the updater key casually: installed applications trust
the public key embedded at build time, so key replacement requires a staged
update trusted by the old key.
