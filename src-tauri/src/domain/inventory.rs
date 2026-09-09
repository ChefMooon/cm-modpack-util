use super::{
    CommandError, Evidence, GitStatusObservation, GitWorkingTreeState, InventoryCounts,
    InventoryEntry, InventoryProvider, InventorySide, ModpackOverview, TrustedPageLink,
};
use crate::safety::canonical_registered_path;
use std::fs;
use std::path::Path;
use std::process::Command;
use toml::Value;

pub fn read_inventory(root: &Path) -> Result<Vec<InventoryEntry>, CommandError> {
    let root = root.canonicalize().map_err(|error| {
        CommandError::new(
            "project_unavailable",
            "Project directory could not be opened",
        )
        .with_details(error.to_string())
    })?;
    if !root.is_dir() {
        return Err(CommandError::new(
            "project_not_directory",
            "Registered project path is not a directory",
        ));
    }

    let pack = read_toml(&root.join("pack.toml"), "pack.toml")?;
    let index_file = pack
        .get("index")
        .and_then(Value::as_table)
        .and_then(|table| table.get("file"))
        .and_then(Value::as_str)
        .ok_or_else(|| {
            CommandError::new(
                "invalid_index_reference",
                "Packwiz index file is unavailable",
            )
        })?;
    let index_path = canonical_registered_path(&root, &root.join(index_file)).map_err(|_| {
        CommandError::new(
            "invalid_index_reference",
            "Packwiz index reference leaves the modpack directory",
        )
    })?;
    let index = read_toml(&index_path, "referenced index")?;
    let files = index
        .get("files")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            CommandError::new(
                "missing_index_files",
                "Packwiz index must contain a files array",
            )
        })?;

    let mut entries = Vec::new();
    for file_entry in files {
        let Some(table) = file_entry.as_table() else {
            continue;
        };
        if table.get("metafile").and_then(Value::as_bool) != Some(true) {
            continue;
        }
        let Some(file) = table.get("file").and_then(Value::as_str) else {
            continue;
        };
        if !file.ends_with(".pw.toml") {
            continue;
        }
        let metadata_path = canonical_registered_path(&root, &root.join(file)).map_err(|_| {
            CommandError::new(
                "invalid_metadata_reference",
                "Packwiz metadata reference leaves the modpack directory",
            )
        })?;
        let metadata = read_toml(&metadata_path, "mod metadata")?;
        entries.push(parse_entry(&root, &metadata_path, &metadata));
    }
    Ok(entries)
}

pub fn packwiz_slug(metadata_path: &Path) -> Result<String, CommandError> {
    let file_name = metadata_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            CommandError::new(
                "invalid_metadata_identity",
                "Metadata filename is unavailable",
            )
        })?;
    let slug = file_name
        .strip_suffix(".pw.toml")
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            CommandError::new(
                "invalid_metadata_identity",
                "Metadata filename is not a Packwiz .pw.toml file",
            )
        })?;
    if !slug
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.'))
    {
        return Err(CommandError::new(
            "invalid_metadata_identity",
            "Metadata filename cannot be converted to a safe Packwiz slug",
        ));
    }
    Ok(slug.to_string())
}

pub fn metadata_filename(metadata_path: &Path) -> Result<String, CommandError> {
    let metadata = read_toml(metadata_path, "mod metadata")?;
    metadata
        .get("filename")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
        .ok_or_else(|| {
            CommandError::new(
                "missing_metadata_filename",
                "Mod metadata does not contain a usable filename",
            )
        })
}

fn read_toml(path: &Path, label: &str) -> Result<Value, CommandError> {
    let text = fs::read_to_string(path).map_err(|error| {
        CommandError::new("project_read_failed", format!("{label} could not be read"))
            .with_details(error.to_string())
    })?;
    text.parse::<Value>().map_err(|error| {
        CommandError::new("malformed_mod_metadata", format!("{label} is malformed"))
            .with_details(error.to_string())
    })
}

fn parse_entry(root: &Path, metadata_path: &Path, metadata: &Value) -> InventoryEntry {
    let local_id = metadata_path
        .strip_prefix(root)
        .unwrap_or(metadata_path)
        .to_string_lossy()
        .replace('\\', "/");
    let provider = provider_evidence(metadata);
    let page_link = match &provider {
        Evidence::Observed(InventoryProvider::Modrinth) => metadata
            .get("update")
            .and_then(Value::as_table)
            .and_then(|update| update.get("modrinth"))
            .and_then(Value::as_table)
            .and_then(|table| table.get("mod-id"))
            .and_then(Value::as_str)
            .filter(|id| {
                !id.is_empty()
                    && id.chars().all(|character| {
                        character.is_ascii_alphanumeric() || "-_".contains(character)
                    })
            })
            .map(|id| TrustedPageLink {
                url: format!("https://modrinth.com/mod/{id}"),
                provider: InventoryProvider::Modrinth,
            }),
        _ => None,
    };

    InventoryEntry {
        local_id,
        metadata_path: metadata_path.to_string_lossy().into_owned(),
        name: string_evidence(metadata, "name"),
        version: filename_version_evidence(metadata),
        provider,
        side: side_evidence(metadata),
        pin: pin_evidence(metadata),
        source_url: source_url_evidence(metadata),
        page_link,
    }
}

fn string_evidence(metadata: &Value, key: &str) -> Evidence<String> {
    match metadata.get(key) {
        Some(Value::String(value)) if !value.trim().is_empty() => Evidence::Observed(value.clone()),
        Some(_) => Evidence::Malformed {
            message: format!("metadata field '{key}' must be a non-empty string"),
        },
        None => Evidence::Unavailable,
    }
}

fn filename_version_evidence(metadata: &Value) -> Evidence<String> {
    let Some(filename) = metadata.get("filename") else {
        return Evidence::Unavailable;
    };
    let Some(filename) = filename.as_str() else {
        return Evidence::Malformed {
            message: "metadata field 'filename' must be a non-empty string".to_string(),
        };
    };
    let Some(stem) = filename.strip_suffix(".jar") else {
        return Evidence::Unavailable;
    };
    let parts: Vec<&str> = stem.split('-').collect();
    let version_start = parts.iter().rposition(|part| {
        part.len() > 2
            && part
                .chars()
                .next()
                .is_some_and(|character| character.is_ascii_digit())
            && part.contains('.')
    });
    version_start
        .map(|index| Evidence::Observed(parts[index..].join("-")))
        .unwrap_or(Evidence::Unavailable)
}

fn side_evidence(metadata: &Value) -> Evidence<InventorySide> {
    match metadata.get("side") {
        Some(Value::String(value)) => match value.as_str() {
            "client" => Evidence::Observed(InventorySide::Client),
            "server" => Evidence::Observed(InventorySide::Server),
            "both" => Evidence::Observed(InventorySide::Both),
            _ => Evidence::Unknown,
        },
        Some(_) => Evidence::Malformed {
            message: "metadata field 'side' must be a string".to_string(),
        },
        None => Evidence::Unavailable,
    }
}

fn pin_evidence(metadata: &Value) -> Evidence<bool> {
    match metadata.get("pin") {
        Some(Value::Boolean(value)) => Evidence::Observed(*value),
        Some(_) => Evidence::Unknown,
        None => Evidence::Observed(false),
    }
}

fn provider_evidence(metadata: &Value) -> Evidence<InventoryProvider> {
    let Some(update) = metadata.get("update") else {
        return Evidence::Unavailable;
    };
    let Some(table) = update.as_table() else {
        return Evidence::Malformed {
            message: "metadata section 'update' must be a table".to_string(),
        };
    };
    let providers = [
        ("modrinth", InventoryProvider::Modrinth),
        ("curseforge", InventoryProvider::Curseforge),
    ];
    let found = providers
        .iter()
        .filter(|(key, _)| table.contains_key(*key))
        .map(|(_, provider)| provider.clone())
        .collect::<Vec<_>>();
    match found.as_slice() {
        [provider] => Evidence::Observed(provider.clone()),
        [] if table.is_empty() => Evidence::Unavailable,
        [] => Evidence::Unknown,
        _ => Evidence::Malformed {
            message: "metadata declares multiple update providers".to_string(),
        },
    }
}

fn source_url_evidence(metadata: &Value) -> Evidence<String> {
    match metadata
        .get("download")
        .and_then(Value::as_table)
        .and_then(|table| table.get("url"))
    {
        Some(Value::String(value)) if is_http_url(value) => Evidence::Observed(value.clone()),
        Some(Value::String(_)) => Evidence::Malformed {
            message: "download URL must be an absolute HTTP(S) URL".to_string(),
        },
        Some(_) => Evidence::Malformed {
            message: "download URL must be a string".to_string(),
        },
        None => Evidence::Unavailable,
    }
}

fn is_http_url(value: &str) -> bool {
    (value.starts_with("http://") || value.starts_with("https://"))
        && !value.chars().any(char::is_whitespace)
        && value.len() > 8
}

pub fn aggregate(entries: &[InventoryEntry]) -> InventoryCounts {
    let mut counts = InventoryCounts {
        total: entries.len() as u32,
        ..InventoryCounts::default()
    };
    for entry in entries {
        match &entry.provider {
            Evidence::Observed(InventoryProvider::Modrinth) => counts.provider_modrinth += 1,
            Evidence::Observed(InventoryProvider::Curseforge) => counts.provider_curseforge += 1,
            Evidence::Observed(InventoryProvider::Unsupported) => counts.provider_unsupported += 1,
            Evidence::Observed(InventoryProvider::Unknown)
            | Evidence::Unknown
            | Evidence::Unavailable => counts.provider_unknown += 1,
            Evidence::Malformed { .. } => counts.malformed += 1,
        }
        match &entry.side {
            Evidence::Observed(InventorySide::Client) => counts.side_client += 1,
            Evidence::Observed(InventorySide::Server) => counts.side_server += 1,
            Evidence::Observed(InventorySide::Both) => counts.side_both += 1,
            _ => counts.side_unknown += 1,
        }
        match &entry.pin {
            Evidence::Observed(true) => counts.pinned += 1,
            Evidence::Observed(false) => counts.unpinned += 1,
            _ => counts.pin_unknown += 1,
        }
    }
    counts
}

pub fn read_overview(root: &Path) -> Result<ModpackOverview, CommandError> {
    let entries = read_inventory(root)?;
    let preview = super::validation::preview(&root.to_string_lossy())?;
    let git = observe_git(root);
    let minecraft_version = preview
        .packwiz
        .declared_versions
        .iter()
        .find(|(key, _)| key == "minecraft")
        .map(|(_, value)| Evidence::Observed(value.clone()))
        .unwrap_or(Evidence::Unavailable);
    let loader = preview
        .packwiz
        .declared_versions
        .iter()
        .find(|(key, _)| key != "minecraft")
        .map(|(key, value)| Evidence::Observed(format!("{key} {value}")))
        .unwrap_or(Evidence::Unavailable);
    Ok(ModpackOverview {
        validation: preview.validation,
        minecraft_version,
        loader,
        inventory_counts: aggregate(&entries),
        git,
        known_update_count: None,
        activity: Vec::new(),
    })
}

pub fn observe_git(root: &Path) -> GitStatusObservation {
    let output = Command::new("git")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .args(["-C", &root.to_string_lossy(), "status", "--porcelain=v1"])
        .output();
    let Ok(output) = output else {
        return GitStatusObservation {
            state: GitWorkingTreeState::Unavailable,
            repository_root: None,
            merge_or_rebase_in_progress: false,
        };
    };
    if !output.status.success() {
        return GitStatusObservation {
            state: GitWorkingTreeState::NotRepository,
            repository_root: None,
            merge_or_rebase_in_progress: false,
        };
    }
    let status = String::from_utf8_lossy(&output.stdout);
    let conflicted = status
        .lines()
        .any(|line| line.len() >= 2 && (line.as_bytes()[0] == b'U' || line.as_bytes()[1] == b'U'));
    let merge_or_rebase_in_progress = root.join(".git/MERGE_HEAD").exists()
        || root.join(".git/rebase-merge").exists()
        || root.join(".git/rebase-apply").exists();
    GitStatusObservation {
        state: if conflicted {
            GitWorkingTreeState::Conflicted
        } else if status.trim().is_empty() {
            GitWorkingTreeState::Clean
        } else {
            GitWorkingTreeState::Dirty
        },
        repository_root: Some(root.to_string_lossy().into_owned()),
        merge_or_rebase_in_progress,
    }
}

#[cfg(test)]
mod tests {
    use super::{aggregate, filename_version_evidence, observe_git, packwiz_slug, read_inventory};
    use crate::domain::{Evidence, GitWorkingTreeState, InventoryProvider, InventorySide};
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use std::time::{SystemTime, UNIX_EPOCH};
    use toml::Value;

    fn fixture(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/packwiz")
            .join(name)
    }

    #[test]
    fn mixed_fixture_derives_independent_counts_and_safe_modrinth_link() {
        let entries = read_inventory(&fixture("mixed")).unwrap();
        let counts = aggregate(&entries);
        assert_eq!(counts.total, 3);
        assert_eq!(counts.provider_modrinth, 1);
        assert_eq!(counts.provider_curseforge, 1);
        assert_eq!(counts.provider_unknown, 1);
        assert_eq!(counts.side_client, 1);
        assert_eq!(counts.side_server, 1);
        assert_eq!(counts.side_both, 1);
        assert_eq!(counts.pinned, 1);
        assert_eq!(counts.unpinned, 2);
        assert!(entries[0].page_link.is_some());
        assert!(matches!(entries[0].version, Evidence::Unavailable));
        assert!(matches!(entries[1].version, Evidence::Unavailable));
        assert!(matches!(entries[2].version, Evidence::Unavailable));
    }

    #[test]
    fn edge_fixture_keeps_malformed_and_unknown_evidence_explicit() {
        let entries = read_inventory(&fixture("edge-cases")).unwrap();
        assert!(matches!(entries[0].provider, Evidence::Unavailable));
        assert!(matches!(entries[0].pin, Evidence::Observed(false)));
        assert!(matches!(entries[1].name, Evidence::Malformed { .. }));
        assert!(matches!(entries[1].side, Evidence::Unknown));
        assert!(matches!(entries[1].pin, Evidence::Unknown));
        assert!(matches!(entries[1].source_url, Evidence::Malformed { .. }));
        assert!(matches!(
            entries[0].side,
            Evidence::Observed(InventorySide::Client)
        ));
        assert!(!matches!(
            entries[0].provider,
            Evidence::Observed(InventoryProvider::Curseforge)
        ));
    }

    #[test]
    fn filename_version_extracts_human_readable_version() {
        let metadata: Value = "filename = \"colourfulclocks-neoforge-1.21.1-0.1.4-beta.jar\""
            .parse()
            .unwrap();
        assert_eq!(
            filename_version_evidence(&metadata),
            Evidence::Observed("0.1.4-beta".to_string())
        );

        assert!(matches!(
            filename_version_evidence(&"filename = 42".parse().unwrap()),
            Evidence::Malformed { .. }
        ));
    }

    #[test]
    fn packwiz_slug_uses_only_the_exact_metadata_filename() {
        assert_eq!(
            packwiz_slug(Path::new("C:/pack/mods/ubes-delight.pw.toml")).unwrap(),
            "ubes-delight"
        );
        assert!(packwiz_slug(Path::new("C:/pack/mods/ubes-delight.toml")).is_err());
        assert!(packwiz_slug(Path::new("C:/pack/mods/ubes delight.pw.toml")).is_err());
    }

    #[test]
    fn git_observation_distinguishes_non_repository_and_dirty_repository() {
        let root = std::env::temp_dir().join(format!(
            "cm-modpack-util-git-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        assert_eq!(observe_git(&root).state, GitWorkingTreeState::NotRepository);
        Command::new("git")
            .args(["-C", &root.to_string_lossy(), "init", "-q"])
            .status()
            .unwrap();
        fs::write(root.join("tracked.txt"), "changed").unwrap();
        assert_eq!(observe_git(&root).state, GitWorkingTreeState::Dirty);
        fs::remove_dir_all(root).unwrap();
    }
}
