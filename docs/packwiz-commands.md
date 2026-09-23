# Packwiz commands

CM Modpack Util uses Packwiz for update discovery, selected mod updates, and
pin/unpin operations. The supported command profile is the Packwiz 1.1.0
Windows x64 executable (`packwiz.exe`) found on `PATH`. Other executable
versions are unsupported.

## Update discovery

```text
packwiz update -a
```

Discovery runs in the registered modpack directory and is read-only. When
Packwiz displays the supported update confirmation prompt, the app sends only
`n` followed by a newline; it never sends `y` or `--yes`. The no-updates marker
is also accepted without a prompt. Candidates are returned only when the
expected output and unchanged before/after project state can be verified.
Unexpected output, cancellation, or changed/unreadable project state is
reported explicitly rather than treated as a successful discovery.

## Selected updates and pinning

The app invokes one command per explicitly selected mod:

```text
packwiz update <slug>
packwiz pin <slug>
packwiz unpin <slug>
```

The slug comes from the exact native `.pw.toml` filename and is revalidated
against current inventory before execution. Commands run with separated
arguments in the registered modpack directory, without shell interpolation or
`--yes`. After each operation, the app rereads Packwiz state and reports the
result only after verifying the requested change. Selected updates may access
the mod's provider and require network access; pin/unpin changes local Packwiz
metadata.

`packwiz update -a` is for discovery only and is never used to apply updates.
The app does not invoke other Packwiz command forms.

## Implementation history

Version-specific design decisions and validation evidence remain in the
[discovery plan](plans/v0.1.0/v0.0.4.md) and
[update and pinning plan](plans/v0.1.0/v0.0.6.md). This document describes the
current command contract.
