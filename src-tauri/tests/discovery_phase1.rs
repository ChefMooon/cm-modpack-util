use cm_modpack_util_lib::discovery::{compatibility, fingerprint};
use cm_modpack_util_lib::domain::{CancellationState, PromptState};
use std::fs;
use std::path::Path;

fn fixture(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/packwiz/discovery")
        .join(name);
    fs::read_to_string(path).unwrap()
}

#[test]
fn supported_fixture_proves_prompt_then_n_cancellation() {
    let output = fixture("supported-cancel.txt");
    let prompt = compatibility::classify_prompt(&output);
    assert_eq!(prompt, PromptState::ExpectedSeen);
    assert_eq!(
        compatibility::classify_cancellation(&output, Some(0), &prompt),
        CancellationState::Confirmed
    );
}

#[test]
fn unsafe_prompt_fixtures_never_authorize_cancellation() {
    for name in [
        "missing-prompt.txt",
        "ambiguous-prompt.txt",
        "early-exit.txt",
    ] {
        let output = fixture(name);
        let prompt = compatibility::classify_prompt(&output);
        assert_ne!(prompt, PromptState::ExpectedSeen, "{name}");
        assert_eq!(
            compatibility::classify_cancellation(&output, Some(0), &prompt),
            CancellationState::NotAttempted
        );
    }
}

#[test]
fn profile_and_fingerprint_contracts_forbid_mutating_update_arguments() {
    let profile = compatibility::tested_profile();
    assert_eq!(profile.command, ["update", "-a"]);
    assert_eq!(profile.cancellation_response, "n\n");
    assert!(!profile
        .command
        .iter()
        .any(|value| value == "--yes" || value == "y"));
    assert!(!profile.cancellation_response.contains('y'));
    assert!(fingerprint::REQUIRED_FILES.contains(&"pack.toml"));
    assert!(fingerprint::INDEX_AND_METADATA_GLOB.contains("indexed"));
}

#[test]
fn malformed_and_timeout_fixtures_are_not_candidate_proof() {
    for name in ["malformed-output.txt", "timeout.txt"] {
        let output = fixture(name);
        let prompt = compatibility::classify_prompt(&output);
        assert_eq!(
            prompt,
            PromptState::ExpectedSeen,
            "fixture still needs cancellation evidence: {name}"
        );
        assert_ne!(
            compatibility::classify_cancellation(&output, None, &prompt),
            CancellationState::Confirmed
        );
    }
}
