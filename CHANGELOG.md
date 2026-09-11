# Changelog

This file records the project's internal development milestones. None of the
versions listed below were published releases. Published release notes should
be added here only after a release is made. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) conventions.


## v0.0.9

### Added

- Application-owned archive, restore, detach, delete-preview, and cleanup
  workflows with protected-reference and partial-outcome reporting.
- Storage reporting that distinguishes application-owned bytes from external and
  unavailable observations.
- Release workspaces with immutable captured baselines, release metadata,
  resumable review, provisional status, and captured-state comparison.
- Changelog generation from snapshot candidates, with provider-cache reuse,
  bounded online lookups, partial-result retention, cancellation, and Markdown
  export observations.
- Settings, management, release, activity, and accessibility hardening across
  the desktop shell.

### Changed

- The README now documents current compatibility, safety, and reset boundaries
  instead of repeating milestone-specific implementation history.
- The Packwiz compatibility profile remains intentionally narrow and is
  documented separately in [docs/packwiz-commands.md](docs/packwiz-commands.md).

### Fixed

- Destructive actions are limited to application-owned records and require
  impact previews plus target-specific confirmation where applicable.
- Incompatible alpha databases fail explicitly and retain the documented
  stop-app/reset procedure; the application does not silently migrate or
  rewrite them.

## v0.0.8

### Added

- Named release history, captured release evidence, release workspaces, and
  comparison based on persisted captures.
- Optional Git provenance and explicit no-Git, dirty, conflicted, and
  unavailable-history states.
- Provisional release handling and validated promotion rules.

## v0.0.7

### Added

- Snapshot changelog generation and Markdown export.
- Offline provider-cache reuse and explicit unresolved, ambiguous, missing, and
  failed provider outcomes.

## v0.0.6

### Added

- Verified positional-slug Packwiz pin and unpin operations.
- Sequential selected updates, cancellation boundaries, recovery
  acknowledgement, and immutable operation evidence.

## v0.0.5

### Added

- Durable discovery snapshots with immutable candidate and fingerprint evidence.
- Review decisions, notes, stale detection, retry linkage, and explicit
  handling for cancelled, unsafe, unsupported, failed, and indeterminate
  outcomes.

## v0.0.2

### Added

- Local Packwiz registration, validation, lifecycle controls, offline reopen,
  and canonical registered-root safety.
- Application-owned SQLite metadata and explicit preservation of external
  Packwiz files.
