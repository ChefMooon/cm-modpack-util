use crate::domain::{FingerprintComparison, FingerprintEntry, ModpackFingerprint};
use crate::safety::canonical_registered_path;
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;
use toml::Value;

/// Files and directory evidence that can be affected by `packwiz update -a`.
pub const REQUIRED_FILES: &[&str] = &["pack.toml"];
pub const INDEX_AND_METADATA_GLOB: &str = "referenced index.toml and indexed *.pw.toml metadata";
pub const RELEVANT_DIRECTORY_ROOTS: &[&str] = &[
    "mods",
    "resourcepacks",
    "shaderpacks",
    "datapacks",
    "config",
];

pub fn collect(root: &Path) -> Result<ModpackFingerprint, String> {
    let root = root.canonicalize().map_err(|error| error.to_string())?;
    if !root.is_dir() {
        return Err("registered root is not a directory".to_string());
    }
    let pack_path = canonical_registered_path(&root, &root.join("pack.toml"))
        .map_err(|error| format!("pack.toml is outside or unavailable: {error:?}"))?;
    let pack_text = fs::read_to_string(&pack_path).map_err(|error| error.to_string())?;
    let pack: Value = pack_text
        .parse()
        .map_err(|error: toml::de::Error| error.to_string())?;
    let index_name = pack
        .get("index")
        .and_then(Value::as_table)
        .and_then(|table| table.get("file"))
        .and_then(Value::as_str)
        .ok_or_else(|| "index reference is unavailable".to_string())?;
    let index_path = canonical_registered_path(&root, &root.join(index_name))
        .map_err(|error| format!("index is outside or unavailable: {error:?}"))?;
    let index_text = fs::read_to_string(&index_path).map_err(|error| error.to_string())?;
    let index: Value = index_text
        .parse()
        .map_err(|error: toml::de::Error| error.to_string())?;
    let mut paths = vec![pack_path, index_path];
    if let Some(files) = index.get("files").and_then(Value::as_array) {
        for file in files.iter().filter_map(Value::as_table) {
            if file.get("metafile").and_then(Value::as_bool) == Some(true) {
                if let Some(name) = file.get("file").and_then(Value::as_str) {
                    paths.push(canonical_registered_path(&root, &root.join(name)).map_err(
                        |error| format!("metadata is outside or unavailable: {error:?}"),
                    )?);
                }
            }
        }
    }
    for directory in RELEVANT_DIRECTORY_ROOTS {
        let path = root.join(directory);
        if path.exists() {
            collect_directory(&root, &path, &mut paths)?;
        }
    }
    paths.sort();
    paths.dedup();
    let mut entries = paths
        .iter()
        .map(|path| fingerprint_entry(&root, path))
        .collect::<Result<Vec<_>, _>>()?;
    entries.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    Ok(ModpackFingerprint {
        root: root.to_string_lossy().into_owned(),
        entries,
        complete: true,
        diagnostic: None,
    })
}

fn collect_directory(root: &Path, path: &Path, paths: &mut Vec<PathBuf>) -> Result<(), String> {
    let canonical = canonical_registered_path(root, path)
        .map_err(|error| format!("directory is outside or unavailable: {error:?}"))?;
    paths.push(canonical.clone());
    for entry in fs::read_dir(canonical).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path).map_err(|error| error.to_string())?;
        if metadata.is_dir() {
            collect_directory(root, &path, paths)?;
        } else {
            paths.push(
                canonical_registered_path(root, &path)
                    .map_err(|error| format!("entry is outside or unavailable: {error:?}"))?,
            );
        }
    }
    Ok(())
}

fn fingerprint_entry(root: &Path, path: &Path) -> Result<FingerprintEntry, String> {
    let metadata = fs::metadata(path).map_err(|error| error.to_string())?;
    let bytes = if metadata.is_file() {
        Some(fs::read(path).map_err(|error| error.to_string())?)
    } else {
        None
    };
    let content_hash = bytes.as_ref().map(|bytes| {
        let mut hasher = DefaultHasher::new();
        bytes.hash(&mut hasher);
        format!("{:016x}", hasher.finish())
    });
    let modified_ns = metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_nanos());
    Ok(FingerprintEntry {
        relative_path: path
            .strip_prefix(root)
            .map_err(|error| error.to_string())?
            .to_string_lossy()
            .replace('\\', "/"),
        kind: if metadata.is_dir() {
            "directory"
        } else {
            "file"
        }
        .to_string(),
        size: if metadata.is_file() {
            Some(metadata.len())
        } else {
            None
        },
        modified_ns,
        content_hash,
    })
}

pub fn empty_fingerprint(root: impl Into<String>) -> ModpackFingerprint {
    ModpackFingerprint {
        root: root.into(),
        entries: Vec::new(),
        complete: false,
        diagnostic: Some("fingerprint collection has not run".to_string()),
    }
}

pub fn compare(before: ModpackFingerprint, after: ModpackFingerprint) -> FingerprintComparison {
    let comparable = before.complete && after.complete && before.root == after.root;
    let differences = if comparable {
        diff_entries(&before.entries, &after.entries)
    } else {
        vec!["before and after fingerprints are incomplete or have different roots".to_string()]
    };
    FingerprintComparison {
        before,
        after,
        unchanged: comparable && differences.is_empty(),
        comparable,
        differences,
    }
}

fn diff_entries(before: &[FingerprintEntry], after: &[FingerprintEntry]) -> Vec<String> {
    let equivalent = before.iter().zip(after).all(|(before, after)| {
        before.relative_path == after.relative_path
            && before.kind == after.kind
            && before.size == after.size
            && before.content_hash == after.content_hash
    }) && before.len() == after.len();
    if equivalent {
        Vec::new()
    } else {
        vec!["relevant project evidence differs".to_string()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn complete(entries: Vec<FingerprintEntry>) -> ModpackFingerprint {
        ModpackFingerprint {
            root: "root".into(),
            entries,
            complete: true,
            diagnostic: None,
        }
    }
    fn file(hash: &str) -> FingerprintEntry {
        FingerprintEntry {
            relative_path: "pack.toml".into(),
            kind: "file".into(),
            size: Some(10),
            modified_ns: Some(1),
            content_hash: Some(hash.into()),
        }
    }
    #[test]
    fn equal_complete_fingerprints_are_unchanged() {
        let result = compare(complete(vec![file("a")]), complete(vec![file("a")]));
        assert!(result.comparable);
        assert!(result.unchanged);
    }
    #[test]
    fn content_timestamp_membership_and_root_changes_are_not_normal() {
        assert!(!compare(complete(vec![file("a")]), complete(vec![file("b")])).unchanged);
        assert!(!compare(complete(vec![file("a")]), complete(vec![])).unchanged);
        let mut other = complete(vec![file("a")]);
        other.root = "other".into();
        assert!(!compare(complete(vec![file("a")]), other).comparable);
    }
    #[test]
    fn incomplete_or_unreadable_fingerprints_never_claim_unchanged() {
        assert!(!compare(empty_fingerprint("root"), empty_fingerprint("root")).unchanged);
    }

    #[test]
    fn timestamp_changes_do_not_make_equivalent_state_stale() {
        let mut before = file("a");
        before.relative_path = "mods".into();
        before.kind = "directory".into();
        before.size = None;
        before.content_hash = None;
        before.modified_ns = Some(1);
        let mut after = before.clone();
        after.modified_ns = Some(2);
        let mut file_after = file("a");
        file_after.modified_ns = Some(2);

        let result = compare(
            complete(vec![before, file("a")]),
            complete(vec![after, file_after]),
        );
        assert!(result.unchanged);
    }
}
