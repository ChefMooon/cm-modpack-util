## v0.0.8 Packwiz compatibility profile

The initial discovery profile is deliberately narrow: the tested Packwiz 1.1.0
Windows x86_64 executable, invoked in the registered canonical project root,
with the separated argument vector `update`, `-a`. Packwiz does not expose a
stable version subcommand, so executable identity/build evidence must be
verified before this profile is considered supported. An unknown executable or
version is unsupported, never a best-effort discovery.

Discovery treats the output corpus as supported only when the combined bounded
stdout/stderr stream contains both `Updates found:` and the exact confirmation
prompt `Do you want to update? [Y/n]`. An explicit `All files are up to date!`
marker is a valid no-updates result and does not require cancellation input. If
the process handle has already disconnected after producing that marker, the
result may have an unavailable exit code; it remains normal only when no
termination or cancellation state was recorded and the fingerprint is equal.
The prompt must occur exactly once. After that prompt, the runner may write
exactly lowercase `n` followed by a newline. It must never write `y` or pass
`--yes`. A normal result additionally requires the expected no-change marker,
exit code 0, bounded output, and equal complete before/after fingerprints.
Missing, unexpected, or repeated prompts, early exit, timeout, write failure,
changed/unreadable state, and malformed candidate output are unsafe or
indeterminate and produce no authoritative candidates.

The fingerprint covers `pack.toml`, its referenced index, indexed
`*.pw.toml` metadata, and relevant directory membership/metadata under the
registered root. Symlinks and junctions are canonicalized and must remain
inside that root. Fingerprints are incomplete when any relevant evidence cannot
be read or compared. Candidate names, versions, provider, side, pin, and links
remain explicit unknown evidence unless present in local inventory; version
change classification is major/minor/bugfix only when both semantic versions
are valid, otherwise unknown.

Known limitations: the profile does not support alternate Packwiz output
formats or versions, performs no provider requests, and does not apply or
persist update decisions. The compatibility profile remains the v0.0.8
boundary for v0.0.9; lifecycle and cleanup actions do not broaden Packwiz
support or modify Packwiz project files.

## v0.0.6 mutation boundary

The installed Packwiz binary was audited with a disposable fixture. Its pin
commands use one positional project slug and do not emit a selection prompt:

```text
packwiz pin <slug>
packwiz unpin <slug>
```

CM Modpack Util derives that slug from the exact native `.pw.toml` filename
after resolving one fresh inventory entry inside the registered root. It never
passes a metadata path, display name, shell string, or `--yes`. After the
process exits, Rust re-reads the same entry and reports success only when the
observed Packwiz pin state matches the requested state. Pin and unpin attempts
are recorded as immutable operation history and can be cancelled through the
bounded process runner.

Targeted updates are supported through one native command per selected
candidate:

```text
packwiz update <slug>
```

The slug is derived from the exact local `.pw.toml` filename after the
snapshot candidate is re-resolved against a fresh inventory. The command is
run with separated arguments from the registered project root, without
`--yes`, shell interpolation, display-name matching, or provider-ID targeting.
The tested Packwiz 1.1.0 Windows x86_64 behavior is non-interactive for this
targeted form and uses the normal provider/network flow.

Each selected target gets its own bounded process, before/after fingerprint,
inventory re-read, verification result, stdout/stderr evidence, and immutable
operation-history record. A failed target does not prevent later selected
targets from running. The aggregate is `complete` only when every target is
verified, `partial` when some targets verify, `failed` when none verify, and
`cancelled` when the user cancels the sequence. A process exit code alone is
never treated as a verified update.

The all-target command remains discovery-only and is not used for apply:
`packwiz update -a` is intentionally unsupported for mutation.

## packwiz update
Update an external file (or all external files) in the modpack

packwiz update [name] [flags]
Options
  -a, --all    Update all external files
  -h, --help   help for update
Options inherited from parent commands
      --cache string              The directory where packwiz will cache downloaded mods (default "/opt/buildhome/.cache/packwiz/cache")
      --config string             The config file to use (default "/opt/buildhome/.config/packwiz/.packwiz.toml")
      --meta-folder string        The folder in which new metadata files will be added, defaulting to a folder based on the category (mods, resourcepacks, etc; if the category is unknown the current directory is used)
      --meta-folder-base string   The base folder from which meta-folder will be resolved, defaulting to the current directory (so you can put all mods/etc in a subfolder while still using the default behaviour) (default ".")
      --pack-file string          The modpack metadata file to use (default "pack.toml")
  -y, --yes                       Accept all prompts with the default or "yes" option (non-interactive mode) - may pick unwanted options in search results

## packwiz unpin
Unpin a file so it receives updates

packwiz unpin <slug>
Options
  -h, --help   help for unpin
Options inherited from parent commands
      --cache string              The directory where packwiz will cache downloaded mods (default "/opt/buildhome/.cache/packwiz/cache")
      --config string             The config file to use (default "/opt/buildhome/.config/packwiz/.packwiz.toml")
      --meta-folder string        The folder in which new metadata files will be added, defaulting to a folder based on the category (mods, resourcepacks, etc; if the category is unknown the current directory is used)
      --meta-folder-base string   The base folder from which meta-folder will be resolved, defaulting to the current directory (so you can put all mods/etc in a subfolder while still using the default behaviour) (default ".")
      --pack-file string          The modpack metadata file to use (default "pack.toml")
  -y, --yes                       Accept all prompts with the default or "yes" option (non-interactive mode) - may pick unwanted options in search results


## packwiz pin
Pin a file so it does not get updated automatically

packwiz pin <slug>
Options
  -h, --help   help for pin
Options inherited from parent commands
      --cache string              The directory where packwiz will cache downloaded mods (default "/opt/buildhome/.cache/packwiz/cache")
      --config string             The config file to use (default "/opt/buildhome/.config/packwiz/.packwiz.toml")
      --meta-folder string        The folder in which new metadata files will be added, defaulting to a folder based on the category (mods, resourcepacks, etc; if the category is unknown the current directory is used)
      --meta-folder-base string   The base folder from which meta-folder will be resolved, defaulting to the current directory (so you can put all mods/etc in a subfolder while still using the default behaviour) (default ".")
      --pack-file string          The modpack metadata file to use (default "pack.toml")
  -y, --yes                       Accept all prompts with the default or "yes" option (non-interactive mode) - may pick unwanted options in search results