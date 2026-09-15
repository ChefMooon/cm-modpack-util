# Changelog Generation and Export

Snapshot review includes an explicit **Generate changelog** action. Generation uses selected snapshot candidates, keeps the local download provider separate from Modrinth changelog evidence, and shows unresolved, ambiguous, missing, and failed provider results without presenting them as confirmed.

Generated Markdown is stored as an application-owned artifact and can be sent to a destination selected through the native save dialog. Export observations retain the destination, content, SHA-256 hash, status, and timestamp. The application does not delete external files.

Provider access is limited to explicit generation. Exact cached Modrinth responses can be reused offline when the stored project/version association matches the requested local version. Online generation performs one exact version lookup per changed entry and waits three seconds between lookups.

Generation can be stopped between lookups. Fetched entries are retained in a partial artifact, and the review can keep or discard that result without deleting its history. Proposed and final revisions remain distinguishable, and the original generated artifact is preserved when a revision is edited.

See the detailed product contract in [CM-MODPACK-UTIL-SPEC.md](CM-MODPACK-UTIL-SPEC.md) and the implementation record in [plans/v0.1.0/v0.0.7.md](plans/v0.1.0/v0.0.7.md).
