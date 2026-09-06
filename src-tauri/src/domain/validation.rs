use super::{
    ApplicationProjectMetadata, CommandError, Observation, PackwizObservations, ProjectLifecycle,
    RegistrationPreview, ValidationResult, ValidationSeverity,
};
use crate::safety::canonical_registered_path;
use std::fs;
use std::path::{Path, PathBuf};

pub fn preview(path: &str) -> Result<RegistrationPreview, CommandError> {
    let selected = PathBuf::from(path);
    if !selected.is_absolute() {
        return Err(CommandError::new(
            "invalid_project_path",
            "Project path must be absolute",
        ));
    }
    let root = selected.canonicalize().map_err(|error| {
        CommandError::new(
            "project_unavailable",
            "Project directory could not be opened",
        )
        .with_details(error.to_string())
    })?;
    if !root.is_dir() {
        return Err(CommandError::new(
            "project_not_directory",
            "Selected project path is not a directory",
        ));
    }

    let pack_path = root.join("pack.toml");
    let pack_text = read_required(&pack_path, "pack.toml")?;
    let pack = pack_text.parse::<toml::Value>().map_err(|error| {
        CommandError::new("malformed_pack_toml", "pack.toml is malformed")
            .with_details(error.to_string())
    })?;
    let mut validation = Vec::new();
    let name = string_field(&pack, "name", &mut validation);
    let author = string_field(&pack, "author", &mut validation);
    let version = string_field(&pack, "version", &mut validation);
    let pack_format = string_field(&pack, "pack-format", &mut validation);
    let index = required_table(&pack, "index", &mut validation);
    let index_file =
        index.and_then(|table| required_string(table, "file", &mut validation, "index_file"));
    let index_hash_format = index.and_then(|table| {
        required_string(table, "hash-format", &mut validation, "index_hash_format")
    });
    let index_hash =
        index.and_then(|table| required_string(table, "hash", &mut validation, "index_hash"));
    let declared_versions = pack
        .get("versions")
        .and_then(toml::Value::as_table)
        .map(|versions| {
            versions
                .iter()
                .filter_map(|(key, value)| {
                    value.as_str().map(|value| (key.clone(), value.to_string()))
                })
                .collect()
        })
        .unwrap_or_default();

    if let Some(index_file) = index_file.as_ref() {
        let index_path = root.join(index_file);
        let index_path = canonical_registered_path(&root, &index_path).map_err(|_| {
            CommandError::new(
                "invalid_index_reference",
                "Packwiz index reference leaves the project directory",
            )
        })?;
        let index_text = read_required(&index_path, "referenced index")?;
        let index_value = index_text.parse::<toml::Value>().map_err(|error| {
            CommandError::new("malformed_index_toml", "Referenced index.toml is malformed")
                .with_details(error.to_string())
        })?;
        validate_metadata_files(&root, &index_value, &mut validation)?;
    }

    let application_defaults = ApplicationProjectMetadata {
        display_name: name.clone().unwrap_or_else(|| {
            root.file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("Untitled project")
                .to_string()
        }),
        icon: None,
        theme: None,
        tags: Vec::new(),
        favorite: false,
        description: None,
        lifecycle: ProjectLifecycle::Active,
    };
    Ok(RegistrationPreview {
        canonical_path: root.to_string_lossy().into_owned(),
        application_defaults,
        packwiz: PackwizObservations {
            name: observed_or_unavailable(name),
            author: observed_or_unavailable(author),
            version: observed_or_unavailable(version),
            pack_format: observed_or_unavailable(pack_format),
            index_file: observed_or_unavailable(index_file),
            index_hash_format: observed_or_unavailable(index_hash_format),
            index_hash: observed_or_unavailable(index_hash),
            declared_versions,
        },
        validation,
    })
}

fn read_required(path: &Path, label: &str) -> Result<String, CommandError> {
    fs::read_to_string(path).map_err(|error| {
        CommandError::new("project_read_failed", format!("{label} could not be read"))
            .with_details(error.to_string())
    })
}

fn string_field(
    value: &toml::Value,
    key: &str,
    validation: &mut Vec<ValidationResult>,
) -> Option<String> {
    match value.get(key) {
        Some(value) => value.as_str().map(str::to_string).or_else(|| {
            validation.push(ValidationResult::error(
                "malformed_pack_field",
                format!("Packwiz field '{key}' must be a string"),
            ));
            None
        }),
        None => {
            validation.push(ValidationResult::info(
                "unavailable_pack_field",
                format!("Packwiz field '{key}' is unavailable"),
            ));
            None
        }
    }
}

fn required_table<'a>(
    value: &'a toml::Value,
    key: &str,
    validation: &mut Vec<ValidationResult>,
) -> Option<&'a toml::map::Map<String, toml::Value>> {
    match value.get(key).and_then(toml::Value::as_table) {
        Some(table) => Some(table),
        None => {
            validation.push(ValidationResult::error(
                "missing_pack_section",
                format!("Packwiz section '[{key}]' is required"),
            ));
            None
        }
    }
}

fn required_string(
    table: &toml::map::Map<String, toml::Value>,
    key: &str,
    validation: &mut Vec<ValidationResult>,
    code: &str,
) -> Option<String> {
    match table.get(key).and_then(toml::Value::as_str) {
        Some(value) if !value.trim().is_empty() => Some(value.to_string()),
        _ => {
            validation.push(ValidationResult::error(
                code,
                format!("Packwiz index field '{key}' is required and must be a string"),
            ));
            None
        }
    }
}

fn validate_metadata_files(
    root: &Path,
    index: &toml::Value,
    validation: &mut Vec<ValidationResult>,
) -> Result<(), CommandError> {
    let Some(files) = index.get("files").and_then(toml::Value::as_array) else {
        validation.push(ValidationResult::error(
            "missing_index_files",
            "Packwiz index must contain a files array",
        ));
        return Ok(());
    };
    for entry in files {
        let Some(table) = entry.as_table() else {
            validation.push(ValidationResult::error(
                "malformed_index_entry",
                "Packwiz index contains a malformed file entry",
            ));
            continue;
        };
        if table.get("metafile").and_then(toml::Value::as_bool) != Some(true) {
            continue;
        }
        let Some(file) = table.get("file").and_then(toml::Value::as_str) else {
            validation.push(ValidationResult::error(
                "malformed_index_entry",
                "Packwiz metadata entry has no file path",
            ));
            continue;
        };
        if !file.ends_with(".pw.toml") {
            validation.push(ValidationResult::warning(
                "unsupported_metadata_file",
                format!("Packwiz metadata file '{file}' is not a supported .pw.toml file"),
            ));
            continue;
        }
        let metadata_path = canonical_registered_path(root, &root.join(file)).map_err(|_| {
            CommandError::new(
                "invalid_metadata_reference",
                "Packwiz metadata reference leaves the project directory",
            )
        })?;
        let metadata = read_required(&metadata_path, "mod metadata")?;
        let metadata = metadata.parse::<toml::Value>().map_err(|error| {
            CommandError::new(
                "malformed_mod_metadata",
                format!("Mod metadata '{file}' is malformed"),
            )
            .with_details(error.to_string())
        })?;
        if metadata.get("name").and_then(toml::Value::as_str).is_none() {
            validation.push(ValidationResult::error(
                "missing_mod_name",
                format!("Mod metadata '{file}' has no valid name"),
            ));
        }
        match metadata.get("side").and_then(toml::Value::as_str) {
            Some("client" | "server" | "both") => {}
            Some(_) | None => validation.push(ValidationResult::warning(
                "unknown_mod_side",
                format!("Mod metadata '{file}' has an unknown side"),
            )),
        }
        if metadata
            .get("update")
            .and_then(toml::Value::as_table)
            .is_none()
        {
            validation.push(ValidationResult::warning(
                "unknown_mod_provider",
                format!("Mod metadata '{file}' has no supported update provider"),
            ));
        }
    }
    Ok(())
}

fn observed_or_unavailable(value: Option<String>) -> Observation<String> {
    value
        .map(Observation::Observed)
        .unwrap_or(Observation::Unavailable)
}

pub fn has_errors(results: &[ValidationResult]) -> bool {
    results
        .iter()
        .any(|result| result.severity == ValidationSeverity::Error)
}

#[cfg(test)]
mod tests {
    use super::{has_errors, preview};
    use crate::domain::Observation;
    use std::path::PathBuf;

    fn fixture(name: &str) -> String {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/packwiz")
            .join(name)
            .to_string_lossy()
            .into_owned()
    }

    #[test]
    fn valid_fixture_produces_observed_preview() {
        let result = preview(&fixture("valid")).unwrap();
        assert!(!has_errors(&result.validation));
        assert_eq!(
            result.packwiz.name,
            Observation::Observed("fixture-pack".to_string())
        );
        assert_eq!(result.packwiz.declared_versions.len(), 2);
    }

    #[test]
    fn optional_values_are_explicitly_unavailable() {
        let result = preview(&fixture("optional")).unwrap();
        assert_eq!(result.packwiz.author, Observation::Unavailable);
        assert_eq!(result.packwiz.version, Observation::Unavailable);
        assert_eq!(result.packwiz.pack_format, Observation::Unavailable);
    }

    #[test]
    fn missing_index_reference_is_a_structured_failure() {
        let error = preview(&fixture("malformed")).unwrap_err();
        assert_eq!(error.code, "invalid_index_reference");
    }
}
