use super::{Evidence, GitProvenance, GitWorkingTreeState};
use std::path::Path;
use std::process::Command;

pub fn observe(root: &Path) -> GitProvenance {
    let status = super::inventory::observe_git(root);
    if matches!(
        status.state,
        GitWorkingTreeState::NotRepository | GitWorkingTreeState::Unavailable
    ) {
        return GitProvenance {
            repository_root: status.repository_root,
            branch: Evidence::Unavailable,
            commit: Evidence::Unavailable,
            tag: Evidence::Unavailable,
            remote: Evidence::Unavailable,
            working_tree: status.state,
            merge_or_rebase_in_progress: status.merge_or_rebase_in_progress,
            history_available: false,
            diagnostic: None,
        };
    }

    let repository_root = git_value(root, &["rev-parse", "--show-toplevel"]);
    let branch = git_value(root, &["symbolic-ref", "--short", "HEAD"]);
    let commit = git_value(root, &["rev-parse", "HEAD"]);
    let tag = git_value(root, &["describe", "--tags", "--exact-match", "HEAD"]);
    let remote = git_value(root, &["remote", "get-url", "origin"]);
    let history_available = matches!(commit, Evidence::Observed(_));
    GitProvenance {
        repository_root: match repository_root {
            Evidence::Observed(value) => Some(value),
            _ => status.repository_root,
        },
        branch,
        commit,
        tag,
        remote,
        working_tree: status.state,
        merge_or_rebase_in_progress: status.merge_or_rebase_in_progress,
        history_available,
        diagnostic: None,
    }
}

fn git_value(root: &Path, arguments: &[&str]) -> Evidence<String> {
    let output = Command::new("git")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .arg("-C")
        .arg(root)
        .args(arguments)
        .output();
    let Ok(output) = output else {
        return Evidence::Unavailable;
    };
    if !output.status.success() {
        return Evidence::Unavailable;
    }
    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if value.is_empty() {
        Evidence::Unavailable
    } else {
        Evidence::Observed(value)
    }
}

#[cfg(test)]
mod tests {
    use super::observe;
    use crate::domain::{Evidence, GitWorkingTreeState};
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn fixture_without_git_is_non_fatal_and_explicit() {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("cm-modpack-util-no-git-{suffix}"));
        fs::create_dir_all(&root).unwrap();
        let provenance = observe(&root);
        fs::remove_dir_all(&root).unwrap();
        assert_eq!(provenance.working_tree, GitWorkingTreeState::NotRepository);
        assert!(matches!(provenance.commit, Evidence::Unavailable));
        assert!(!provenance.history_available);
    }
}
