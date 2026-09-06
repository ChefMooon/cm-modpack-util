use crate::domain::{
    CancellationState, CommandError, CompatibilityEvidence, CompatibilityProfile,
    CompatibilityStatus, PromptState, VersionEvidence,
};

/// The only profile currently permitted by discovery. The installed Windows binary exposes
/// `packwiz:1.1.0` in its embedded build metadata; Packwiz has no stable version subcommand.
pub const PROFILE_ID: &str = "packwiz-1.1.0-update-all-cancel-n";
pub const EXECUTABLE_NAME: &str = "packwiz.exe";
pub const CANCELLATION_RESPONSE: &str = "n\n";
pub const NO_UPDATES_MARKER: &str = "All files are up to date!";
pub const MAX_OUTPUT_BYTES: u64 = 4 * 1024 * 1024;
pub const TIMEOUT_SECONDS: u64 = 120;

pub fn tested_profile() -> CompatibilityProfile {
    CompatibilityProfile {
        id: PROFILE_ID.to_string(),
        executable_name: EXECUTABLE_NAME.to_string(),
        version: VersionEvidence::Observed("1.1.0".to_string()),
        command: vec!["update".to_string(), "-a".to_string()],
        // These are intentionally exact, case-sensitive fragments from the tested corpus.
        prompt_patterns: vec![
            "Updates found:".to_string(),
            "Do you want to update? [Y/n]".to_string(),
        ],
        cancellation_response: CANCELLATION_RESPONSE.to_string(),
        cancellation_markers: vec![
            "No response".to_string(),
            "Update cancelled".to_string(),
            "Cancelled!".to_string(),
        ],
        no_update_markers: vec![NO_UPDATES_MARKER.to_string()],
        expected_exit_codes: vec![0],
        max_output_bytes: MAX_OUTPUT_BYTES,
        timeout_seconds: TIMEOUT_SECONDS,
        supported_platforms: vec!["windows-x86_64".to_string()],
        known_limitations: vec![
            "Packwiz does not expose a stable version command; executable identity and embedded build evidence must be verified.".to_string(),
            "Only the update-all interactive output corpus is supported; other prompt or output shapes are unsupported.".to_string(),
            "The profile is offline-testable but live discovery requires a disposable project and network availability for Packwiz itself.".to_string(),
        ],
    }
}

pub fn unsupported(reason: impl Into<String>) -> CompatibilityEvidence {
    CompatibilityEvidence {
        status: CompatibilityStatus::Unsupported,
        profile: None,
        executable_path: None,
        version_output: None,
        diagnostic: Some(CommandError::new("unsupported_compatibility", reason)),
    }
}

pub fn resolve_executable() -> Result<std::path::PathBuf, CommandError> {
    let candidates = if cfg!(windows) {
        vec!["packwiz.exe".to_string()]
    } else {
        vec!["packwiz".to_string()]
    };
    if let Some(path_value) = std::env::var_os("PATH") {
        for directory in std::env::split_paths(&path_value) {
            for name in &candidates {
                let path = directory.join(name);
                if path.is_file() {
                    return path.canonicalize().map_err(|error| {
                        CommandError::new(
                            "executable_unavailable",
                            "Packwiz executable could not be canonicalized",
                        )
                        .with_details(error.to_string())
                    });
                }
            }
        }
    }
    Err(CommandError::new(
        "packwiz_unavailable",
        "The tested Packwiz executable was not found",
    ))
}

pub fn validate_executable(path: &std::path::Path) -> CompatibilityEvidence {
    if !path.is_absolute() || !path.is_file() {
        return unsupported("Packwiz executable is missing or not an absolute file");
    }
    let name_matches = path
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.eq_ignore_ascii_case(EXECUTABLE_NAME))
        .unwrap_or(false);
    if !name_matches {
        return unsupported("Executable name is outside the tested Packwiz profile");
    }
    supported(path.to_string_lossy())
}

pub fn supported(executable_path: impl Into<String>) -> CompatibilityEvidence {
    CompatibilityEvidence {
        status: CompatibilityStatus::Supported,
        profile: Some(tested_profile()),
        executable_path: Some(executable_path.into()),
        version_output: Some("packwiz:1.1.0".to_string()),
        diagnostic: None,
    }
}

pub fn classify_prompt(output: &str) -> PromptState {
    let profile = tested_profile();
    let confirmation = profile.prompt_patterns[1].as_str();
    let confirmation_count = output.matches(confirmation).count();
    let has_all_fragments = profile
        .prompt_patterns
        .iter()
        .all(|pattern| output.contains(pattern.as_str()));
    if confirmation_count > 1 {
        PromptState::Ambiguous
    } else if has_all_fragments {
        PromptState::ExpectedSeen
    } else if profile
        .no_update_markers
        .iter()
        .any(|marker| output.contains(marker))
    {
        PromptState::NoUpdates
    } else if output.contains("[Y/n]") || output.contains("[y/N]") {
        PromptState::Unexpected
    } else {
        PromptState::Missing
    }
}

pub fn classify_cancellation(
    output: &str,
    exit_code: Option<i32>,
    prompt: &PromptState,
) -> CancellationState {
    if !matches!(prompt, PromptState::ExpectedSeen) {
        return CancellationState::NotAttempted;
    }
    if exit_code == Some(0)
        && tested_profile()
            .cancellation_markers
            .iter()
            .any(|marker| output.contains(marker))
    {
        CancellationState::Confirmed
    } else if exit_code.is_none() {
        CancellationState::Terminated
    } else {
        CancellationState::PrematureExit
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SUPPORTED: &str = "Updates found:\nexample 1.0 -> 1.1\nDo you want to update? [Y/n]:\nNo response; leaving files unchanged";

    #[test]
    fn profile_has_only_the_safe_command_and_response() {
        let profile = tested_profile();
        assert_eq!(profile.command, ["update", "-a"]);
        assert_eq!(profile.cancellation_response, "n\n");
        assert!(!profile
            .command
            .iter()
            .any(|arg| arg == "--yes" || arg == "y"));
        assert!(!profile.cancellation_response.starts_with('y'));
    }

    #[test]
    fn supported_fixture_requires_all_prompt_fragments() {
        assert_eq!(classify_prompt(SUPPORTED), PromptState::ExpectedSeen);
        assert_eq!(
            classify_prompt("Loading modpack...\nAll files are up to date!"),
            PromptState::NoUpdates
        );
        assert_eq!(
            classify_prompt("stderr: Updates found:\nDo you want to update? [Y/n]:"),
            PromptState::ExpectedSeen
        );
        assert_eq!(
            classify_prompt("stderr: Updates found:\nDo you want to update? [Y/n]:"),
            PromptState::ExpectedSeen
        );
        assert_eq!(classify_prompt("Updates found: only"), PromptState::Missing);
        assert_eq!(
            classify_prompt("Update all files? [y/N]"),
            PromptState::Unexpected
        );
    }

    #[test]
    fn cancellation_requires_expected_prompt_and_marker() {
        let prompt = classify_prompt(SUPPORTED);
        assert_eq!(
            classify_cancellation(SUPPORTED, Some(0), &prompt),
            CancellationState::Confirmed
        );
        assert_eq!(
            classify_cancellation(
                "Updates found:\nUbe's Delight: old.jar -> new.jar\nDo you want to update? [Y/n]: Cancelled!",
                Some(0),
                &prompt,
            ),
            CancellationState::Confirmed
        );
        assert_eq!(
            classify_cancellation(SUPPORTED, Some(1), &prompt),
            CancellationState::PrematureExit
        );
        assert_eq!(
            classify_cancellation(SUPPORTED, None, &prompt),
            CancellationState::Terminated
        );
        assert_eq!(
            classify_cancellation(SUPPORTED, Some(0), &PromptState::Missing),
            CancellationState::NotAttempted
        );
    }

    #[test]
    fn unsupported_evidence_is_distinct() {
        let evidence = unsupported("version not in profile");
        assert_eq!(evidence.status, CompatibilityStatus::Unsupported);
        assert!(evidence.profile.is_none());
    }
}
