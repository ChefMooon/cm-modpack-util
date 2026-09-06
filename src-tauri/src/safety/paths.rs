use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathScopeError {
    RelativePath,
    RootUnavailable,
    CandidateUnavailable,
    OutsideRegisteredRoot,
}

pub fn canonical_registered_path(
    registered_root: &Path,
    candidate: &Path,
) -> Result<PathBuf, PathScopeError> {
    if !candidate.is_absolute() {
        return Err(PathScopeError::RelativePath);
    }

    let root = registered_root
        .canonicalize()
        .map_err(|_| PathScopeError::RootUnavailable)?;
    let canonical_candidate = candidate
        .canonicalize()
        .map_err(|_| PathScopeError::CandidateUnavailable)?;

    if !same_path_scope(&root, &canonical_candidate) {
        return Err(PathScopeError::OutsideRegisteredRoot);
    }

    Ok(canonical_candidate)
}

fn same_path_scope(root: &Path, candidate: &Path) -> bool {
    #[cfg(windows)]
    {
        let root_components: Vec<String> = root
            .components()
            .map(|component| component.as_os_str().to_string_lossy().to_ascii_lowercase())
            .collect();
        let candidate_components: Vec<String> = candidate
            .components()
            .map(|component| component.as_os_str().to_string_lossy().to_ascii_lowercase())
            .collect();
        candidate_components.len() >= root_components.len()
            && candidate_components
                .iter()
                .zip(root_components.iter())
                .all(|(candidate, root)| candidate == root)
    }

    #[cfg(not(windows))]
    {
        candidate.starts_with(root)
    }
}

#[cfg(test)]
mod tests {
    use super::{canonical_registered_path, PathScopeError};
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    struct Fixture {
        root: PathBuf,
        inside: PathBuf,
        outside: PathBuf,
    }

    impl Fixture {
        fn new() -> Self {
            let suffix = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let base = std::env::temp_dir().join(format!("cm-modpack-util-paths-{suffix}"));
            let root = base.join("registered");
            let inside = root.join("mods");
            let outside = base.join("other");
            fs::create_dir_all(&inside).unwrap();
            fs::create_dir_all(&outside).unwrap();
            Self {
                root,
                inside,
                outside,
            }
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let base = self.root.parent().unwrap();
            let _ = fs::remove_dir_all(base);
        }
    }

    #[test]
    fn accepts_existing_absolute_path_inside_registered_root() {
        let fixture = Fixture::new();
        let result = canonical_registered_path(&fixture.root, &fixture.inside).unwrap();
        assert!(result.ends_with(Path::new("registered").join("mods")));
    }

    #[test]
    fn rejects_relative_paths() {
        let fixture = Fixture::new();
        assert_eq!(
            canonical_registered_path(&fixture.root, Path::new("mods")),
            Err(PathScopeError::RelativePath)
        );
    }

    #[test]
    fn rejects_nonexistent_paths() {
        let fixture = Fixture::new();
        assert_eq!(
            canonical_registered_path(&fixture.root, &fixture.root.join("missing")),
            Err(PathScopeError::CandidateUnavailable)
        );
    }

    #[test]
    fn rejects_paths_outside_registered_root() {
        let fixture = Fixture::new();
        assert_eq!(
            canonical_registered_path(&fixture.root, &fixture.outside),
            Err(PathScopeError::OutsideRegisteredRoot)
        );
    }

    #[test]
    fn rejects_unregistered_root_lookup() {
        let fixture = Fixture::new();
        assert_eq!(
            canonical_registered_path(&fixture.root.join("missing"), &fixture.inside),
            Err(PathScopeError::RootUnavailable)
        );
    }
}
