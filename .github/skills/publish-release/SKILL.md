---
name: publish-release
description: 'Prepare and publish a CM Modpack Util Windows release. Use when bumping the application version, updating release notes, creating a v<semver> tag, pushing the release workflow, validating Tauri updater assets, or rehearsing the stable latest-release update flow.'
---

# CM Modpack Util Release Workflow

## When to Use

Use this skill for every Windows release, updater rehearsal, or version bump.
The project publishes stable Windows NSIS artifacts through
`.github/workflows/release.yml`.

## Release Procedure

1. Add or update the matching `CHANGELOG.md` release entry before changing the
   version. The release workflow runs
   `scripts/extract-release-notes.ps1` for the pushed tag and copies the
   matching changelog section into the GitHub release body and Tauri
   `latest.json`; editing the GitHub draft after the workflow runs does not
   update updater notes.
2. Synchronize the same SemVer value in:
   - `package.json`
   - `package-lock.json` root metadata
   - `src-tauri/Cargo.toml`
   - `src-tauri/Cargo.lock` application package entry
   - `src-tauri/tauri.conf.json`
   - `src-tauri/tests/phase5_updates.rs`
   - the current-release example in `docs/desktop-updates.md`
3. Keep `src-tauri/tauri.conf.json` pointed at
   `https://github.com/ChefMooon/cm-modpack-util/releases/latest/download/latest.json`
   for normal stable-feed testing. Do not revert to a tag-specific endpoint
   unless performing a deliberately isolated temporary-release rehearsal.
4. Run `npm run check`, `npm test`, `npm run build`,
   `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`,
   `cargo test --manifest-path src-tauri/Cargo.toml`, and `git diff --check`.
5. Commit the complete release change set with a Conventional Commit message,
   create an annotated `v<semver>` tag, and push both `main` and the tag.
6. Wait for the Windows workflow to finish. Inspect the draft NSIS installer,
   `.sig`, and `latest.json` before publishing.
7. Publish the release as non-prerelease when it is intended to drive
   `releases/latest`. The latest feed ignores prereleases.

## Release Notes Check

Before pushing the tag, verify the changelog extractor locally with the exact
tag. Normal mode prints the release notes; `-ValidateOnly` checks the entry
without returning its text:

```powershell
./scripts/extract-release-notes.ps1 -Tag vX.Y.Z
./scripts/extract-release-notes.ps1 -Tag vX.Y.Z -ValidateOnly
```

The command must find the matching `## [X.Y.Z]` or `## vX.Y.Z` entry in
`CHANGELOG.md`, and the entry must contain at least one bullet-point note. The
validation command outputs `True` only when these checks pass and exits with a
failure code otherwise. The workflow fails before packaging if the tag format
or matching changelog entry is invalid.

## Update Rehearsal

- An older build can discover a newer release only if that older build already
  uses the stable `releases/latest` endpoint.
- On Windows, updater installation exits the app. The app therefore downloads
  first, then offers **Install and restart** or **Later** before invoking the
  installer. Do not use a post-install restart toast as the Windows contract.
- Verify that **Later** does not re-notify the same version during navigation or
  a repeated startup check, and that Settings still offers installation for a
  downloaded update.
- Never commit updater private keys, passwords, Authenticode material, or
  release credentials.
