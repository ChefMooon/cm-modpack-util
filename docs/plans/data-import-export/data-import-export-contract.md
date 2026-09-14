# Application Data Bundle Contract

This document is the Phase 1 contract for the application-owned data bundle.
It is independent of the SQLite schema version and does not represent a raw
SQLite export.

## Bundle envelope

The first supported format is `cm-modpack-util-data` version `1`. The file is
either canonical UTF-8 JSON or gzip-compressed canonical UTF-8 JSON. Gzip is
identified by the standard `1f 8b` magic bytes; otherwise the input is treated
as JSON. No other compression or container format is accepted.

The envelope has this shape:

```json
{
  "format": "cm-modpack-util-data",
  "version": 1,
  "created_at": "RFC3339 timestamp",
  "application": { "name": "CM Modpack Util", "version": "app version" },
  "payload_sha256": "64 lowercase hexadecimal characters",
  "payload": {
    "settings": [],
    "records": {
      "modpacks": [],
      "inventory_observations": []
    },
    "excluded": ["changelog_cache", "packwiz_project_files", "git_metadata"]
  }
}
```

`payload_sha256` covers the canonical UTF-8 bytes of the `payload` value, not
the envelope and not compressed bytes. The envelope is parsed, the payload is
canonicalized, and the hash is verified after decompression before any import
classification or write. Export emits the same canonical form every time for
the same database state, except for the explicitly informational
`created_at` and application version metadata.

Canonical JSON uses repository-owned recursive object-key sorting by Unicode
code point, preserves array order, emits UTF-8 without a BOM, and uses JSON
numbers only for bounded integer values represented by the Rust contract.
Timestamps, identifiers, hashes, paths, enum values, and opaque JSON fields
are strings or JSON values exactly as stored by their typed contracts. NaN,
infinity, duplicate object keys, invalid UTF-8, and non-canonical numeric
forms are invalid. The canonicalizer must not sort arrays.

## Resource and compatibility limits

These limits are part of version 1 and produce typed `resource_limit`
validation errors:

| Resource | Limit |
| --- | ---: |
| Compressed input | 64 MiB |
| Decompressed payload | 256 MiB |
| Total records | 100,000 |
| Maximum object/array nesting | 32 |
| Maximum individual string | 1 MiB |
| Maximum envelope format version | 1 |

Malformed JSON or an invalid envelope is `invalid_bundle`; a well-formed but
unsupported format/version is `unsupported_version`; a hash mismatch is
`integrity_failure`; a limit breach is `resource_limit`. The command boundary
also uses explicit `cancelled`, `stale_preview`, and `os_effect_failed` errors.

## Entity inventory and dependency matrix

The bundle contains application-owned durable state only. `schema_metadata` is
derived local metadata and is recreated by database initialization. Every
other schema table is classified below.

| Table / entity family | Classification | Logical identity | Comparison projection | Import order / key rule |
| --- | --- | --- | --- | --- |
| `settings` | included | `setting:<key>` | key, value JSON | first; upsert only when non-conflicting |
| `modpacks` | included | `modpack:<id>` | all application fields, including canonical path | root; path is canonicalized for comparison, never rewritten |
| `inventory_observations` | included | `inventory-observation:<modpack-id>` | all columns | after modpacks |
| `modpack_activity` | included | `modpack-activity:<modpack-id>:<logical-row>` | modpack, event, timestamp, message | after modpacks; generated ID remapped |
| `discovery_attempts` | included | `discovery-attempt:<modpack-id>:<logical-row>` | modpack and all stored evidence | after modpacks; generated ID remapped |
| `discovery_candidates` | included | `discovery-candidate:<attempt-logical-id>:<logical-row>` | attempt and candidate JSON | after discovery attempts; generated ID remapped |
| `snapshots` | included | `snapshot:<id>` | all columns except local row representation | after modpacks; predecessor self-reference resolved in a second pass |
| `snapshot_candidates` | included | `snapshot-candidate:<id>` | all columns | after snapshots |
| `snapshot_decisions` | included | `snapshot-decision:<snapshot-logical-id>:<logical-row>` | snapshot, candidate, decision, note, timestamp | after workspace candidate sources; generated ID remapped |
| `snapshot_notes` | included | `snapshot-note:<modpack-id>:<logical-row>` | all columns | after modpacks, snapshots, snapshot candidates |
| `snapshot_rechecks` | included | `snapshot-recheck:<snapshot-logical-id>:<logical-row>` | all columns | after snapshots; generated ID remapped |
| `operation_attempts` | included | `operation-attempt:<id>` | all columns except local references | after modpacks, workspaces, snapshots; predecessor resolved in a second pass |
| `release_workspace_operations` | included | `workspace-operation:<id>` | all columns | after workspaces and snapshots |
| `operation_candidates` | included | `operation-candidate:<operation-logical-id>:<logical-row>` | all columns | after operations and snapshot candidates |
| `operation_acknowledgements` | included | `operation-ack:<operation-logical-id>` | all columns | after operations |
| `pin_observations` | included | `pin-observation:<operation-logical-id>:<logical-row>` | all columns | after operations; generated ID remapped |
| `changelog_cache` | excluded | none | none | Disposable provider-response cache is never exported or imported |
| `changelog_attempts` | included | `changelog-attempt:<id>` | all columns | after modpacks and snapshots |
| `changelog_artifacts` | included | `changelog-artifact:<id>` | all columns except local references | after attempts, snapshots, workspaces |
| `changelog_revisions` | included | `changelog-revision:<id>` | all columns except local references | after artifacts; prior revision resolved in a second pass |
| `changelog_exports` | included | `changelog-export:<id>` | all stored record fields and destination path | after artifacts and revisions; destination file is not copied or touched |
| `releases` | included | `release:<id>` | all columns except local references | after modpacks |
| `release_snapshots` | included | `release-snapshot:<release-id>` | release and snapshot logical IDs | after releases and snapshots |
| `release_changelog_artifacts` | included | `release-changelog:<release-id>:<artifact-id>` | both logical IDs | after releases and artifacts |
| `release_workspaces` | included | `release-workspace:<id>` | all columns except local references | after modpacks, snapshots, releases; nullable links resolved |
| `release_workspace_candidates` | included | `workspace-candidate:<workspace-logical-id>:<logical-row>` | all columns | after workspace candidate sources; generated ID remapped |
| `release_workspace_candidate_sources` | included | `workspace-candidate-source:<id>` | all columns except local references | after workspaces and snapshot candidates |
| `release_workspace_activity` | included | `workspace-activity:<workspace-logical-id>:<logical-row>` | all columns | after workspaces; generated ID remapped |
| `release_workspace_observations` | included | `workspace-observation:<workspace-logical-id>:<logical-row>` | all columns | after workspaces; generated ID remapped |
| `lifecycle_tombstones` | included | `tombstone:<record-type>:<record-id>` | all columns | after referenced application records when present; immutable evidence |
| `lifecycle_operations` | included | `lifecycle-operation:<id>` | all columns | after tombstones and target records; immutable evidence |
| `cleanup_operations` | included | `cleanup-operation:<id>` | all columns | independent history; immutable evidence |
| `schema_metadata` | derived | none | none | Recreated locally; schema version is not portable bundle data |

For generated integer primary keys, `logical-row` is a deterministic
`family + parent logical identity + canonical non-key projection` identity.
When equal projections occur, rows are ordered by `(timestamp, original
integer id)` and receive a deterministic ordinal. The integer is never
exported as authoritative identity. Import allocates a new local integer and
rewrites every foreign-key reference through the bundle logical-ID map.

Self-references and the small cross-family dependency cycle are handled with
two-pass insertion: insert rows with nullable self/cross references cleared,
record the pending logical references, then update them after all referenced
families exist. A missing required reference makes that record unsafe and
produces a dependent-record skip rather than a partial row.

## Comparison, conflicts, and unavailable paths

Records compare by logical identity and canonical comparison projection:

- `addition`: bundle identity is absent locally.
- `identical`: identity and projection match.
- `same_id_conflict`: identity exists with a different projection; local data
  remains untouched.
- `same_path_conflict`: a different modpack identity has the same
  platform-aware canonical path; neither modpack is merged.
- `unavailable_path`: a modpack path does not currently exist. It remains
  registered with disconnected lifecycle state and is never relocated or
  reconnected automatically.
- `dependent_skip`: a record requires a conflicting, missing, or skipped
  parent. Unrelated safe records remain eligible.
- `excluded`: data explicitly outside the bundle scope is reported as omitted,
  never treated as an import failure.
- `no_op`: the bundle contains no eligible additions after identical records,
  conflicts, and dependent skips are classified.

Path comparison uses the existing Rust safety/canonicalization helpers and
platform-aware case rules. Packwiz files, project directories, Git data,
provider cache, and external changelog destination files are never read or
modified.

## Preview/result contracts

The Rust domain contract will be represented as typed `DataTransferPreview`,
`DataTransferRecordClassification`, `DataTransferConflict`,
`DataTransferDependentSkip`, `DataTransferExcludedData`, and
`DataTransferResult` values in the data-transfer domain module. The frontend
wrapper receives these values without reproducing merge logic.

The preview includes:

- bundle format/version and verified payload hash;
- deterministic database/data-transfer fingerprint;
- counts and record lists for additions, identical records, conflicts,
  unavailable paths, excluded data, dependent skips, and no-op state;
- selected safe record identities and whether supported native settings effects
  are required;
- cancellation and stale-preview eligibility.

The result includes committed counts, skipped/conflicting identities,
unavailable paths, excluded data, applied native effects, and explicit
failure classification. A preview is read-only. Cancellation is accepted
during selection, validation, and preview and until the commit transaction
starts; an active transaction runs to completion. Supported native settings
effects are applied before commit, and any failure rejects the complete import
with `os_effect_failed`.

Lifecycle tombstones, lifecycle operations, cleanup operations, and other
durable history are imported as immutable evidence. Their presence never
authorizes a new lifecycle or cleanup action.

## Planned implementation locations

- `src-tauri/src/domain/data_transfer.rs`: envelope, canonicalization,
  identity, preview/result, error, and limit contracts.
- `src-tauri/src/db/data_transfer.rs`: ordered export/import reads and the
  single transactional merge.
- `src-tauri/src/lib.rs`: central Tauri command registration.
- `src/lib/data-transfer.ts`: typed invocation boundary and dialog-facing
  helpers.
- Focused Rust fixtures/tests will live beside the owning database/domain
  modules and will cover each classification and dependency rule.
