---
title: "Desktop Update Delivery - Implementation Plan"
status: IN_PROGRESS
current_phase: 3
created: 2026-09-11
last_updated: 2026-09-11
---

# Specification & Overview

## 1. Scope & Objective

- **Source:** The update pre-plan assessment, `README.md`,
  `docs/CM-MODPACK-UTIL-SPEC.md`, `docs/ROADMAP.md`, the current Tauri
  configuration, and the official Tauri updater and GitHub pipeline
  documentation.
- **Goal:** Allow the Windows desktop application to discover stable releases
  from the project's GitHub Releases feed, verify the Tauri signatures attached
  to updater artifacts, and let users explicitly
  download, install, and restart into an update without any automatic binary
  download or unattended restart.
- **In scope:**
  - Official Tauri v2 updater plugin integration.
  - Windows stable-release artifacts and a GitHub Actions release workflow.
  - Tauri updater signing configuration and CI secret handling.
  - A static `latest.json` manifest hosted as a GitHub Release asset.
  - Startup and manual update checks.
  - Explicit download/install confirmation.
  - A separate post-install `Restart now` / `Restart later` step.
  - About/settings status, toast discovery, release-notes review, progress,
    retry, offline, and failure states.
  - Version consistency across package, Cargo, Tauri, displayed UI, and tags.
  - Focused frontend, Rust/integration, accessibility, and release-pipeline
    validation.
- **Out of scope:**
  - macOS or Linux artifacts.
  - Prerelease or user-selectable release channels.
  - Automatic binary downloads, silent installation, or automatic restart.
  - Rollback, downgrade UI, delta updates, staged rollout, analytics, or an
    update history screen.
  - GitHub API calls from the frontend or arbitrary installer downloads.
  - Updating Packwiz projects or any external modpack files.

## 2. User Outcome and Flow

The normal flow is:

1. The app checks the updater metadata manifest on startup without downloading
   installer bytes.
2. If a newer stable version exists, the app shows a non-blocking notification.
3. The user opens the update details and reviews the version and release notes.
4. The user explicitly selects **Download and install**.
5. The app reports download/install progress and errors.
6. Once installation is complete, the app enters a stable **Ready to restart**
   state.
7. The user chooses **Restart now** or **Restart later**.
8. **Restart now** invokes the native relaunch path. **Restart later** leaves
   the installed update ready for the next launch and does not repeatedly force
   the user to act during the current session.

The app must never interpret a failed or unavailable check as an available
update, and a failed check must not block normal modpack workflows.

## 3. Closed Decision Register

| ID | Decision | Consequence |
| --- | --- | --- |
| d1-platform | Support Windows only in the first implementation. | The initial CI matrix has one Windows runner and the updater manifest needs a Windows x64 target only. |
| d2-channel | Publish stable releases only. | Do not expose prerelease selection or channel settings. |
| d3-check | Check on startup and through a manual About/settings action. | Startup checks are silent unless an update is found; no periodic background polling is required initially. |
| d4-download | Checking metadata is automatic; downloading/installing is explicit. | `check()` may run on startup, while `downloadAndInstall()` requires user confirmation. |
| d5-restart | Restart is a separate explicit step after installation. | The UI must expose `Ready to restart`, with independent `Restart now` and `Restart later` actions. |
| d6-endpoint | Use the public GitHub Releases `latest.json` endpoint. | Configure the updater endpoint as `https://github.com/ChefMooon/cm-modpack-util/releases/latest/download/latest.json`; do not query GitHub's API from Svelte. |
| d7-updater-signing | Trust updater artifacts through Tauri's embedded public key and CI-held private key. | Tauri updater signing remains enabled even though the Windows installer has no Microsoft Authenticode signature. The private key must never be committed, placed in frontend assets, or exposed to the app runtime. |
| d8-code-signing | Do not require Microsoft Authenticode signing in the first release. | Windows may show SmartScreen or publisher warnings for the installer; release notes and installation guidance must explain that this is separate from Tauri update verification. |
| d9-installer | Prefer the Windows NSIS updater artifact unless validation proves MSI is required. | The workflow and manifest must select one unambiguous updater asset and test it on a clean Windows installation. |
| d10-notification | Persist the last-notified available version through existing settings storage. | The same release should not produce an intrusive startup notification on every launch after the user dismisses it. |
| d11-key-generation | Generate all application signing key material manually, with an interactive walkthrough during Phase 2. | CI may consume a manually created private key through a GitHub Actions secret, but CI must never generate, rotate, or print key material. No Authenticode key is created for the initial release. |
| d12-notification-ui | Use the existing global toast component for update discovery, download/install progress, failures, and restart choices. | Do not create a new banner, notification center, or update-specific floating panel. Extend the shared toast action API only as needed to expose primary and secondary actions. |
| d13-release-validation | Validate the updater through a published temporary test release with a tag-specific endpoint before using the stable `releases/latest` endpoint. | Draft releases cannot validate the public `latest` feed. The temporary release must be clearly marked test-only and removed or retained according to the release checklist. |
| d14-version-source | Make `src-tauri/tauri.conf.json` the authoritative application version source. | Package.json, Cargo.toml, release tags, and visible UI must be synchronized or CI must fail; the UI must not retain hard-coded stale versions. |
| d15-relaunch | Use the official Tauri process plugin's `relaunch()` API for `Restart now`. | Add the process plugin dependency and capability permission explicitly; installation completion alone must not relaunch the running app. |

## 4. Architecture and Technical Constraints

- Rust/Tauri remains authoritative for updater configuration, network access,
  signature verification, installation, relaunch, and structured failure
  handling. Svelte consumes a typed update-state boundary.
- Add the official `tauri-plugin-updater` Rust and JavaScript packages and the
  official Tauri process plugin for relaunch.
- Configure `createUpdaterArtifacts: true`, the generated public key, and the
  HTTPS endpoint in `src-tauri/tauri.conf.json`.
- Initialize the updater plugin in `src-tauri/src/lib.rs` and add the required
  Tauri capability permissions.
- Use `tauri-apps/tauri-action@v1` in a tag-triggered GitHub Actions workflow.
  The workflow must build the frontend through the existing
  `beforeBuildCommand`, create Tauri updater signatures from GitHub Actions
  secrets, create a draft release for initial validation, and upload
  `latest.json` and signatures. It must not claim or imply that the installer
  has Microsoft Authenticode signing.
- Use `npm ci` in CI and keep the lockfile synchronized with dependency changes.
- Preserve the alpha no-migration policy. Update state such as last-notified
  version may use the existing settings table and must not require a schema
  migration.
- Do not allow static frontend preview mode to claim that an update check
  succeeded when the Tauri runtime is unavailable.
- Existing UI accessibility, reduced-motion, focus, semantic-status, and toast
  conventions remain required.
- The existing global toast mounted by `src/routes/+layout.svelte` is the
  default update notification and action surface. Reuse the existing `Modal`
  only where a toast is not appropriate, such as lengthy release notes or an
  explicit download/install confirmation.

- `latest.json` is release metadata containing the URL and signature for the
  platform updater artifact; it is not described as a separately signed
  document. Tauri verifies the downloaded updater artifact against the
  embedded public key.
- `src-tauri/tauri.conf.json` is the release version authority. A consistency
  check must compare it with `package.json`, `src-tauri/Cargo.toml`, the release
  tag, and the version displayed by the UI.

## 5. Key Management Boundary

- The Tauri updater key pair will be generated manually by the maintainer using
  the Tauri CLI during Phase 2. The implementation session must walk through
  the command, local file location, public-key extraction/verification,
  private-key backup, and password handling interactively; it must not invent
  or silently execute key-generation steps.
- The private key must remain outside the repository and be entered into the
  GitHub Actions secret store manually. The secret value must never be pasted
  into source files, committed, logged, or sent to external services.
- The public key may be copied into `src-tauri/tauri.conf.json` after it has
  been manually verified against the generated key pair.
- No Microsoft Authenticode certificate/key is created for this release scope.
- GitHub's `GITHUB_TOKEN` is an automatically provided, short-lived workflow
  credential used for release publication; it is not application signing
  material and does not need to be generated manually. The workflow must use
  the minimum required `contents: write` permission.
- Losing the updater private key prevents already-installed applications from
  trusting future artifacts. Key replacement is not an independent revocation
  mechanism because the old public key is embedded in installed applications;
  any rotation requires a carefully staged update trusted by the old key.
- If another credential becomes necessary during implementation, Phase 2 must
  stop and document its purpose, scope, storage location, rotation/revocation
  procedure, and manual creation steps before it is introduced.

# Requirements Traceability

| Requirement | Planned treatment | Delivery phase |
| --- | --- | --- |
| Build Windows release artifacts | Add tag-triggered `tauri-action` workflow with Windows runner, `npm ci`, Rust setup, signing, and release upload. | Phase 2 |
| Publish updater manifest | Enable updater artifacts and upload `latest.json` plus signatures to GitHub Releases. | Phase 2 |
| Verify trusted updates | Embed public key in Tauri configuration and keep private key in GitHub Actions secrets. | Phase 2 |
| Check without downloading | Implement startup/manual metadata checks and explicit update states. | Phase 3 |
| User-controlled install | Require a confirmation action before binary download/install. | Phase 3 |
| Separate restart decision | Persist/display `Ready to restart`; expose independent `Restart now` and `Restart later`. | Phase 3 |
| Explain the update | Show version, release date, release notes, and a GitHub release fallback link. | Phase 4 |
| Avoid repeated interruption | Store last-notified version and define dismissal behavior. | Phases 3-4 |
| Reuse common notification UI | Use the existing toast for update notification, actions, progress, errors, and restart choices; extend its action API rather than adding a new notification surface. | Phases 3-4 |
| Preserve normal workflows on failure | Treat offline, malformed, unsigned, and failed checks as explicit non-blocking states. | Phases 3-5 |
| Validate the shipped path | Run application checks and a temporary published-test-release install/update rehearsal. | Phase 5 |

# Execution Plan & Handoffs

## Phase 1: Baseline Audit and Release Contract

- **Status:** COMPLETED
- **Objective:** Verify current version/build behavior and make the release,
  updater, and restart contracts executable before changing dependencies.

### Tasks

- [x] Audit the current Tauri configuration, capabilities, plugin initialization,
  window lifecycle, settings storage, About UI, header version display, toast
  API, modal primitive, and frontend invocation patterns.
- [x] Replace the stale hard-coded visible version strategy with one authoritative
  runtime/build version source suitable for both the header and About screen.
- [x] Define the typed update state contract:
  `idle`, `checking`, `up_to_date`, `available`, `download_confirm`,
  `downloading`, `installing`, `ready_to_restart`, `restarting`, `later`,
  `offline`, and `failed`.
- [x] Define user-facing error categories and next actions for manifest failure,
  invalid metadata, signature rejection, download failure, install failure,
  relaunch failure, and unavailable Tauri runtime.
- [x] Confirm the exact Windows installer/updater artifact produced by the current
  Tauri version and select NSIS or document why MSI is required.
- [x] Define whether `Restart later` remains visible in the About screen and
  header for the current session, and how it behaves on the next launch.
- [x] Record the public endpoint, stable-only policy, version-tag convention, and
  signing-key custody procedure in this plan before implementation begins.

### Verification & Acceptance Criteria

- [x] Every update state has an owner, user-facing meaning, legal transitions,
  and a failure/retry action.
- [x] The restart contract is explicit: installation does not relaunch the app;
  only `Restart now` does.
- [x] No current version surface can remain at `v0.0.7` while release metadata
  reports another version.

### Plan Compliance Checklist

- [x] **Required Files:** `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`,
  `src-tauri/src/lib.rs`, `src-tauri/capabilities/default.json`,
  `package.json`, `src/lib/`, `src/components/header/Header.svelte`,
  `src/routes/settings/+page.svelte`, `src/routes/+layout.svelte`,
  `src/components/ui/toast/`, and this plan are audited or updated as required.
- [x] **Boundaries:** Phase 1 does not add updater dependencies, release
  workflows, signing keys, network calls, installer downloads, or restart
  behavior.
- [x] **Legacy Code Removed:** Any stale hard-coded version source replaced by
  the authoritative runtime/build source; no duplicate version authority remains.
- [x] **Acceptance Checks:** Every state, transition, error category, restart
  rule, artifact choice, endpoint, release policy, and key-custody rule is
  explicitly recorded and verified.

### Phase 1 Handoff & Verification Report

- **Compliance Check:** PASSED
- **Verification Result:** PASSED
- **Execution Proof / Logs:**
  - `npx tauri info` -> Tauri 2.11.5, Windows x64 environment confirmed.
  - `npm run check` -> `svelte-check found 0 errors and 0 warnings`.
  - `npm run build` -> Vite/SvelteKit production build completed successfully.
  - `git diff --check` -> no whitespace errors.
  - Source search -> no stale `v0.0.7` visible-version references remain under `src/`.
- **Artifacts Created/Modified:**
  - `src/lib/appVersion.svelte.ts` - Tauri runtime version source with static-preview unavailable handling.
  - `src/lib/updates.ts` - typed update states, state guidance, error categories, legal transitions, and recovery actions.
  - `src/components/layout/AppShell.svelte`, `src/components/header/Header.svelte`, `src/routes/settings/+page.svelte` - shared header/About version display.
  - `src-tauri/tauri.conf.json` - Windows NSIS-only bundle target.
  - `docs/plans/desktop-updates-plan.md` - execution metadata, compliance sections, Phase 1 decisions, and handoff.
- **Decisions & Deviations:** The current Tauri CLI reports version 2.11.5. Windows packaging is explicitly narrowed from `all` to NSIS, producing the standard `CM.Modpack.Util_<semver>_x64-setup.exe` installer/updater artifact; MSI is not required for this scope. The stable updater endpoint is `https://github.com/ChefMooon/cm-modpack-util/releases/latest/download/latest.json`; temporary validation uses a tag-specific release endpoint. Stable tags use `v<semver>`, and prereleases are excluded. The updater private key remains maintainer-held outside the repository, is entered manually into GitHub Actions secrets, and is never logged or committed. `Restart later` is a session-only deferred state: it remains available through the common toast/About update status during the current session, and a fresh launch starts without `ready_to_restart`.
- **Next Phase Context:** Phase 2 may add updater dependencies and the Windows release workflow. It must use the selected NSIS artifact, manually generated Tauri updater keys, draft-release validation, and the stable endpoint only after temporary-release rehearsal.

## Phase 2: Windows Release Pipeline with Tauri Updater Signatures

- **Status:** COMPLETED
- **Objective:** Produce a Windows installer without Authenticode signing and
  release metadata containing a Tauri signature for the updater artifact.

### Tasks

- [x] Generate a Tauri updater signing key outside the repository and store the
  private key in the repository's GitHub Actions secrets. **This is a guided
  manual maintainer step, not an automated task:** walk through the command,
  choose a secure local key path, optionally set a password, verify the public
  key, back up the private key securely, and only then configure the CI secret.
- [x] Confirm that no other cryptographic key is required for the selected
  Windows-only, non-Authenticode release. Do not create an Authenticode key or
  certificate.
- [x] Confirm the manually generated public/private key pair matches before
  committing the public key to configuration. Record the verification result
  without recording private key material.
- [x] Explicitly do not add a Microsoft Authenticode certificate or signing
  step. Record that Windows may display SmartScreen or unknown-publisher
  warnings for the installer.
- [x] Add the official updater dependencies and configure the public key,
  endpoint, and updater artifact generation.
- [x] Add `.github/workflows/release.yml` triggered by version tags, initially
  producing a draft release that the maintainer manually publishes as a
  clearly labeled temporary test release.
- [x] Configure `tauri-apps/tauri-action@v1` with `GITHUB_TOKEN`,
  `uploadUpdaterJson: true`, `uploadUpdaterSignatures: true`, and the selected
  Windows updater artifact.
- [x] Ensure CI uses the repository's Node/Rust setup, `npm ci`, existing Tauri
  build commands, and the correct permissions for release contents.
- [x] Validate that the release contains the installer, Tauri signature, and
  `latest.json`, and that the manifest's Windows URL and signature correspond
  to the uploaded asset.
- [x] Document temporary test-release publication, cleanup/retention, and the
  later stable tag/release procedure.

### Verification & Acceptance Criteria

- [x] A tag-triggered workflow completes on Windows and creates the expected
  draft release assets.
- [x] A published temporary test release can be queried through its tag-specific
  updater endpoint. The generated metadata manifest is valid SemVer and
  contains the expected Windows platform entry, URL, artifact signature, and
  release version.
- [x] No private key, signing password, Authenticode certificate, or token appears
  in repository files,
  build logs, release notes, or frontend bundles.
- [x] The maintainer can identify where the private key is backed up, how the
  GitHub Actions secret is updated, and why loss or replacement requires a
  staged update trusted by the old key.
- [x] A clean Windows installation can be installed from the published
  temporary test release.
- [x] The temporary published test release is used for the updater rehearsal;
  the public `releases/latest` endpoint is enabled only after that rehearsal
  passes.
- [x] Release documentation distinguishes the absence of Windows Authenticode
  signing from the presence of Tauri updater artifact signatures.

### Plan Compliance Checklist

- [x] **Required Files:** `package.json`, `package-lock.json`,
  `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, `src-tauri/tauri.conf.json`,
  `.github/workflows/release.yml`, `docs/desktop-updates.md`, `README.md`, and
  this plan were verified against the Phase 2 diff.
- [x] **Boundaries:** Windows stable releases only; no Authenticode key or
  signing step; no private key in the repository; the configured endpoint is
  the tag-specific `v0.0.10` test endpoint until rehearsal passes.
- [x] **Legacy Code Removed:** The prior all-target bundle configuration was
  replaced by the NSIS-only target; no superseded release workflow existed.
- [x] **Acceptance Checks:** The tag-triggered workflow, published temporary
  release, key backup/match confirmation, manifest inspection, public
  tag-specific endpoint access, and clean Windows installation are complete.

### Phase 2 Handoff & Verification Report

- **Compliance Check:** PASSED
- **Verification Result:** PASSED
- **Execution Proof / Logs:**
  - `npm install @tauri-apps/plugin-updater@^2` -> installed
    `@tauri-apps/plugin-updater` 2.11.0 and synchronized `package-lock.json`.
  - `cargo add tauri-plugin-updater@2 --manifest-path src-tauri/Cargo.toml` ->
    installed `tauri-plugin-updater` 2.11.0 and synchronized `Cargo.lock`.
  - `npm run check` -> `svelte-check found 0 errors and 0 warnings`.
  - `cargo check --manifest-path src-tauri/Cargo.toml` -> passed.
  - `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` -> passed.
  - `npx tauri build --no-bundle` -> passed and produced the release executable.
  - `git diff --check` -> no whitespace errors.
  - Repository search -> no private key, password, Authenticode certificate, or
    token material is present.
  - GitHub Actions run `34634705427` -> completed successfully for tag
    `v0.0.10` in approximately 17 minutes.
  - Draft release `CM Modpack Util v0.0.10` -> contains the NSIS installer,
    `.sig`, and `latest.json`.
  - `latest.json` inspection -> version `0.0.10`, valid Windows x64 platform
    entries, matching installer asset URL, and non-empty Tauri signature.
  - Temporary release `v0.0.10` -> published as a prerelease and installer
    verified successfully by the maintainer.
  - Public `latest.json` endpoint -> returned `200` with version `0.0.10`,
    Windows platform entries, and non-empty signatures.
  - Public installer and signature assets -> returned `200`.
  - Maintainer confirmation -> signing key was backed up and the public/private
    pair matches.
- **Artifacts Created/Modified:**
  - `package.json`, `package-lock.json` - official updater JavaScript dependency.
  - `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock` - official updater Rust dependency.
  - `src-tauri/tauri.conf.json` - verified public key, tag-specific `v0.0.10`
    test endpoint, updater artifact generation, and NSIS target.
  - `.github/workflows/release.yml` - Windows tag workflow, version consistency
    check, draft release, updater JSON/signature upload, and CI secret inputs.
  - `docs/desktop-updates.md`, `README.md` - key custody and release procedure.
- **Decisions & Deviations:** The supplied value was treated as public key
  material only. The initial endpoint is the selected tag-specific
  `v0.0.10` test endpoint; the synchronized release version is now `0.0.10`.
  The stable `releases/latest` endpoint remains a Phase 5 promotion step. No
  Authenticode key or signing step was added.
- **Next Phase Context:** Phase 2 is complete. Phase 3 may begin with native
  updater integration and explicit download/install/restart behavior.

## Phase 3: Updater Integration and Restart State

- **Status:** BLOCKED
- **Objective:** Add the native update check/install boundary with explicit
  download and restart decisions.

### Tasks

- [x] Initialize the updater plugin in `src-tauri/src/lib.rs` and configure
  capability permissions required by the installed Tauri plugin version.
- [x] Initialize the official Tauri process plugin and add its explicit
  relaunch capability permission.
- [x] Add a focused typed frontend wrapper/module for update checks, download
  progress, install completion, and relaunch.
- [x] Run a startup check after the app shell is usable, without blocking
  registration, inventory, or settings workflows.
- [x] Add the manual check command to the About/settings surface.
- [x] Persist the last-notified available version using the existing settings
  command boundary.
- [x] Require explicit confirmation before calling the download/install path.
- [x] Add a request-level test double or controlled endpoint that distinguishes
  manifest requests from installer-artifact requests, proving startup checks do
  not download installer bytes.
- [x] Use a persistent common toast for download/install progress and update it
  in place rather than creating an update-specific progress panel.
- [x] Transition to `ready_to_restart` only after installation reports complete.
- [x] Keep `Restart now` and `Restart later` as separate labeled actions on the
  common toast. Extend the shared toast action type/rendering to support a
  primary and secondary action if the current single-action API is insufficient;
  do not create a second notification component.
- [x] Add the official Tauri process plugin and capability permission, then make
  `Restart now` call its `relaunch()` API; `Restart later` must not close or
  relaunch the app.
- [x] Prevent duplicate checks, downloads, installs, and relaunch requests.
- [x] Ensure update failures are visible and retryable without falsely reporting
  success.

### Verification & Acceptance Criteria

- [x] Startup checks metadata but do not download installer bytes.
- [x] No binary download occurs before the user selects **Download and install**.
- [x] After install, the application remains open in `ready_to_restart` until
  the user chooses a restart action.
- [x] `Restart later` preserves the current session and does not show an
  intrusive repeat prompt during that session.
- [ ] `Restart now` relaunches into the installed version in a Windows rehearsal.
- [x] A fresh launch uses the installed runtime version as the source of truth
  and clears any session-only `ready_to_restart` state.
- [x] Offline and malformed/invalid responses leave the normal application usable.

### Plan Compliance Checklist

- [x] **Required Files:** `src-tauri/src/lib.rs`, `src-tauri/capabilities/default.json`,
  `src-tauri/Cargo.toml`, `package.json`, `src/lib/updates.svelte.ts`,
  `src/lib/updateTestDouble.ts`, `src/components/ui/toast/types.ts`,
  `src/components/ui/toast/ToastItem.svelte`, `src/routes/+layout.svelte`,
  `src/routes/settings/+page.svelte`, and lockfiles were verified against the
  Phase 3 task list.
- [x] **Boundaries:** No frontend GitHub API calls, startup binary download,
  silent install, automatic restart, or second notification surface.
- [x] **Legacy Code Removed:** Any superseded update/relaunch path is removed
  rather than left alongside the native implementation.
- [ ] **Acceptance Checks:** Code-level checks passed; Windows restart rehearsal
  remains pending for Phase 5 validation.

### Phase 3 Handoff & Verification Report

- **Compliance Check:** PASSED
- **Verification Result:** BLOCKED
- **Execution Proof / Logs:** Pending
  - `npm run check` -> `svelte-check found 0 errors and 0 warnings`.
  - `cargo check --manifest-path src-tauri/Cargo.toml` -> passed.
  - `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` -> passed.
  - `git diff --check` -> passed.
- **Artifacts Created/Modified:**
  - `src-tauri/src/lib.rs`, `src-tauri/capabilities/default.json`,
    `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock` - native updater/process
    registration, permissions, and dependencies.
  - `package.json`, `package-lock.json` - official process plugin dependency.
  - `src/lib/updates.svelte.ts` - typed updater coordinator, startup/manual
    checks, persisted notification suppression, explicit install confirmation,
    progress, install completion, and relaunch handling.
  - `src/lib/updateTestDouble.ts` - request-level manifest/artifact distinction.
  - `src/components/ui/toast/types.ts`,
    `src/components/ui/toast/ToastItem.svelte` - persistent primary and
    secondary toast actions.
  - `src/routes/+layout.svelte`, `src/routes/settings/+page.svelte` - startup
    check and About/settings manual update status.
- **Decisions & Deviations:** No deviations from the Phase 3 architecture.
  The Windows restart rehearsal is not claimed from static validation and
  remains a Phase 5 release-validation prerequisite.
- **Next Phase Context:** Phase 4 may refine the About/update UX and release
  notes modal. Phase 5 must run the clean Windows install/update rehearsal,
  including `Restart now`, before this phase can be considered fully accepted.

## Phase 4: UI/UX and Accessibility

- **Status:** NOT STARTED
- **Objective:** Make update availability discoverable without disrupting the
  local-first modpack workflow.

### Tasks

- [ ] Add a compact update status to the existing About/settings area showing
  current version, last check result/time, available version, and manual check
  action. Do not create a separate update page.
- [ ] Add a non-blocking startup notification through the existing global toast
  with **View update** and **Later** actions.
- [ ] Extend the common toast action API to support two explicit labeled
  actions, with defined primary/secondary order, keyboard focus order, and
  persistent behavior for install/restart states. Do not rely on an unlabeled
  dismiss button to mean **Restart later**.
- [ ] Reuse the existing `Modal` for release notes, version/date, installer
  information, the **View release on GitHub** fallback, and explicit
  **Download and install** confirmation. Do not create an update-specific modal
  primitive.
- [ ] Add progress, cancellation/error, retry, installed, and ready-to-restart
  states by updating one persistent common toast in place.
- [ ] Make **Restart now** primary and **Restart later** secondary on the
  `ready_to_restart` toast only; do not present restart controls earlier.
- [ ] Preserve keyboard navigation, visible focus, semantic status announcements,
  reduced-motion behavior, stable layouts, and color-independent meaning.
- [ ] Test long release notes, narrow window sizes, light/dark/system themes,
  keyboard-only use, and screen-reader-readable state changes.

### Verification & Acceptance Criteria

- [ ] An update can be discovered and acted on without opening Settings if the
  user follows the startup notification.
- [ ] Update discovery, install progress, errors, and restart choices are
  rendered through the common toast system; no duplicate notification surface
  exists.
- [ ] The user can always identify the current version, available version, and
  next action.
- [ ] Every loading, empty, unavailable, stale, failed, installed, and deferred
  state has understandable copy and an available recovery action.

### Plan Compliance Checklist

- [ ] **Required Files:** Pending Phase 4 execution; About/settings surface,
  shared toast components, modal usage, and focused accessibility tests must be
  verified against the Phase 4 task list.
- [ ] **Boundaries:** No update-specific notification surface or modal
  primitive; preserve existing accessibility and reduced-motion conventions.
- [ ] **Legacy Code Removed:** Any stale hard-coded version or duplicate update
  notification rendering is removed.
- [ ] **Acceptance Checks:** Pending Phase 4 execution.

### Phase 4 Handoff & Verification Report

- **Compliance Check:** PENDING
- **Verification Result:** PENDING
- **Execution Proof / Logs:** Pending
- **Artifacts Created/Modified:** Pending
- **Decisions & Deviations:** Pending
- **Next Phase Context:** Pending

## Phase 5: Validation, Documentation, and Release Rehearsal

- **Status:** NOT STARTED
- **Objective:** Verify the complete shipped path and document operational
  ownership before enabling normal stable releases.

### Tasks

- [ ] Add focused frontend checks for update state transitions and duplicate
  action prevention.
- [ ] Add Rust/plugin configuration or integration coverage for updater command
  registration and structured error translation where the repository test
  harness supports it.
- [ ] Run:
  `npm run check`, `npm run build`,
  `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`,
  `cargo test --manifest-path src-tauri/Cargo.toml`, and `git diff --check`.
- [ ] Perform a temporary published-test-release rehearsal on a clean Windows
  installation: install an older version, query the tag-specific endpoint,
  confirm no artifact download occurs before consent, install, choose
  `Restart later`, exit/reopen if needed, then choose `Restart now` and verify
  the new version.
- [ ] Test signature failure, missing asset, malformed manifest, offline check,
  interrupted download, failed install, and relaunch failure.
- [ ] Update `README.md` and the appropriate release documentation with:
  stable-release procedure, tag/version rules, Tauri updater-key custody,
  required GitHub Actions secrets, the absence of Authenticode signing,
  SmartScreen expectations, temporary test-release verification, and user-facing update
  behavior.
- [ ] Switch the configured endpoint to
  `releases/latest/download/latest.json` only after the temporary published
  test release passes all updater criteria.
- [ ] Promote the first validated stable release only after all acceptance
  criteria pass.

### Verification & Acceptance Criteria

- [ ] All repository checks pass, or any pre-existing failure is recorded as
  blocked with evidence.
- [ ] The clean-install update rehearsal proves the separate restart decision.
- [ ] Release documentation is sufficient for a future maintainer to publish
  an update without exposing signing material.
- [ ] The final release feed is reachable through the configured HTTPS endpoint
  and the installed app recognizes the release as trusted.

### Plan Compliance Checklist

- [ ] **Required Files:** Pending Phase 5 execution; focused tests, release
  documentation, and final validation artifacts must be verified against the
  Phase 5 task list.
- [ ] **Boundaries:** No stable endpoint promotion before temporary release
  rehearsal passes; no secrets or private key material in repository artifacts.
- [ ] **Legacy Code Removed:** Temporary validation-only configuration is removed
  or explicitly retained according to the release checklist.
- [ ] **Acceptance Checks:** Pending Phase 5 execution.

### Phase 5 Handoff & Verification Report

- **Compliance Check:** PENDING
- **Verification Result:** PENDING
- **Execution Proof / Logs:** Pending
- **Artifacts Created/Modified:** Pending
- **Decisions & Deviations:** Pending
- **Next Phase Context:** Pending

# Handoff Notes

- Do not commit the generated private signing key or its password.
- Do not generate signing keys in GitHub Actions, scripts, package lifecycle
  hooks, or application code.
- When Phase 2 begins, guide the maintainer through manual Tauri updater-key
  generation and verification before editing the release workflow or embedding
  the public key.
- Do not implement update checks through frontend `fetch` calls to GitHub.
- Do not download an installer as part of a startup check.
- Do not silently restart after installation.
- Do not add macOS/Linux matrix entries until platform scope is explicitly
  reopened.
- The first implementation should use a draft release for asset inspection,
  then a manually published temporary test release so the actual Windows
  artifact, Tauri updater signature, metadata manifest, install behavior, and
  relaunch behavior are validated before users receive a normal stable update.
- Windows Authenticode signing is intentionally deferred. It can be added later
  without changing the Tauri updater trust key, provided the artifact and
  manifest contract remains compatible.

# References

- [Tauri updater plugin](https://v2.tauri.app/plugin/updater/)
- [Tauri GitHub release pipeline](https://v2.tauri.app/distribute/pipelines/github/)
- [tauri-action](https://github.com/tauri-apps/tauri-action)
- [Project README](../../README.md)
- [Project specification](../CM-MODPACK-UTIL-SPEC.md)
- [Project roadmap](../ROADMAP.md)

# Overall Plan Completion Status

- **Final State:** IN_PROGRESS
- **Total Phases Completed:** 2 / 5
- **Summary of Outcome:** Phase 1 established the authoritative runtime version
  display, typed update and error contracts, Windows NSIS artifact selection,
  restart/defer behavior, and release/signing custody constraints. Phase 2
  repository integration is complete, but the plan is blocked on manual
  private-key secret setup and the published temporary-release rehearsal.
