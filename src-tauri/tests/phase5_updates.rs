use serde_json::Value;
use std::fs;
use std::path::PathBuf;

fn repository_file(relative_path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join(relative_path)
}

#[test]
fn updater_configuration_keeps_windows_release_contract() {
    let config: Value = serde_json::from_str(
        &fs::read_to_string(repository_file("src-tauri/tauri.conf.json")).unwrap(),
    )
    .unwrap();

    assert_eq!(config["version"], "0.0.18");
    assert_eq!(config["productName"], "CM-Modpack-Util");
    assert_eq!(config["bundle"]["targets"][0], "nsis");
    assert_eq!(config["bundle"]["createUpdaterArtifacts"], true);
    assert_eq!(
        config["plugins"]["updater"]["endpoints"][0],
        "https://github.com/ChefMooon/cm-modpack-util/releases/latest/download/latest.json"
    );
    assert!(!config["plugins"]["updater"]["pubkey"]
        .as_str()
        .unwrap()
        .is_empty());
}

#[test]
fn native_update_plugins_and_permissions_are_registered() {
    let lib = fs::read_to_string(repository_file("src-tauri/src/lib.rs")).unwrap();
    assert!(lib.contains(".plugin(tauri_plugin_process::init())"));
    assert!(lib.contains(".plugin(tauri_plugin_updater::Builder::new().build())"));

    let capabilities: Value = serde_json::from_str(
        &fs::read_to_string(repository_file("src-tauri/capabilities/default.json")).unwrap(),
    )
    .unwrap();
    let permissions = capabilities["permissions"].as_array().unwrap();
    assert!(permissions
        .iter()
        .any(|permission| permission == "updater:default"));
    assert!(permissions
        .iter()
        .any(|permission| permission == "process:allow-restart"));
}
