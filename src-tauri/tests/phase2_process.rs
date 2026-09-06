use cm_modpack_util_lib::discovery::{compatibility, fingerprint, process};
use cm_modpack_util_lib::domain::{CancellationState, CompatibilityStatus, PromptState};
use std::path::Path;
use std::time::Duration;

#[test]
fn profile_rejects_missing_or_wrong_executables() {
    let missing = compatibility::validate_executable(Path::new("C:/missing/packwiz.exe"));
    assert_eq!(missing.status, CompatibilityStatus::Unsupported);
    let wrong = compatibility::validate_executable(Path::new("C:/tools/not-packwiz.exe"));
    assert_eq!(wrong.status, CompatibilityStatus::Unsupported);
}

#[test]
fn fingerprint_collection_is_complete_and_stable_for_valid_fixture() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/packwiz/valid");
    let before = fingerprint::collect(&root).unwrap();
    let after = fingerprint::collect(&root).unwrap();
    let comparison = fingerprint::compare(before, after);
    assert!(comparison.comparable);
    assert!(comparison.unchanged);
    assert!(comparison
        .before
        .entries
        .iter()
        .any(|entry| entry.relative_path == "pack.toml"));
}

#[cfg(windows)]
#[test]
fn bounded_runner_captures_fake_process_without_authorizing_missing_prompt() {
    let plan = process::CommandPlan {
        executable: Path::new(r"C:\Windows\System32\cmd.exe").to_path_buf(),
        arguments: vec!["/C".into(), "echo output".into()],
        working_directory: std::env::temp_dir(),
    };
    let evidence = process::run_probe(
        &plan,
        process::RunLimits {
            max_output_bytes: 32,
            timeout: Duration::from_secs(2),
            cancel: None,
        },
    )
    .unwrap();
    assert!(evidence.stdout.contains("output"));
    assert_eq!(evidence.prompt, PromptState::Missing);
    assert_eq!(evidence.cancellation, CancellationState::NotAttempted);
}

#[cfg(windows)]
#[test]
fn bounded_runner_detects_prompt_on_stderr() {
    let plan = process::CommandPlan {
        executable: Path::new(r"C:\Windows\System32\cmd.exe").to_path_buf(),
        arguments: vec![
            "/C".into(),
            "echo Updates found: 1>&2 & echo Do you want to update? [Y/n]: 1>&2 & echo No response 1>&2"
                .into(),
        ],
        working_directory: std::env::temp_dir(),
    };
    let evidence = process::run_probe(
        &plan,
        process::RunLimits {
            max_output_bytes: 256,
            timeout: Duration::from_secs(2),
            cancel: None,
        },
    )
    .unwrap();
    assert_eq!(evidence.prompt, PromptState::ExpectedSeen);
    assert_ne!(evidence.cancellation, CancellationState::NotAttempted);
    assert!(evidence.stderr.contains("Do you want to update? [Y/n]:"));
}

#[cfg(windows)]
#[test]
fn bounded_runner_closes_stdin_after_cancelling_update_prompt() {
    let command = concat!(
        "echo Updates found: & ",
        "echo example 1.0 -^> 1.1 & ",
        "echo Do you want to update? [Y/n]: & ",
        "set /p response= & ",
        "if \"!response!\"==\"n\" (echo No response) else (exit /b 9)"
    );
    let plan = process::CommandPlan {
        executable: Path::new(r"C:\Windows\System32\cmd.exe").to_path_buf(),
        arguments: vec!["/V:ON".into(), "/C".into(), command.into()],
        working_directory: std::env::temp_dir(),
    };
    let evidence = process::run_probe(
        &plan,
        process::RunLimits {
            max_output_bytes: 256,
            timeout: Duration::from_secs(2),
            cancel: None,
        },
    )
    .unwrap();
    assert_eq!(evidence.prompt, PromptState::ExpectedSeen);
    assert_eq!(evidence.cancellation, CancellationState::Confirmed);
    assert!(evidence.stdout.contains("No response"));
}
