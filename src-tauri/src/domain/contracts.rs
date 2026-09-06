use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Observation<T> {
    Observed(T),
    Unavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProjectLifecycle {
    Active,
    Maintenance,
    Archived,
    Disconnected,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApplicationProjectMetadata {
    pub display_name: String,
    pub icon: Option<String>,
    pub theme: Option<String>,
    pub tags: Vec<String>,
    pub favorite: bool,
    pub description: Option<String>,
    pub lifecycle: ProjectLifecycle,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PackwizObservations {
    pub name: Observation<String>,
    pub author: Observation<String>,
    pub version: Observation<String>,
    pub pack_format: Observation<String>,
    pub index_file: Observation<String>,
    pub index_hash_format: Observation<String>,
    pub index_hash: Observation<String>,
    pub declared_versions: Vec<(String, String)>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RegistrationPreview {
    pub canonical_path: String,
    pub application_defaults: ApplicationProjectMetadata,
    pub packwiz: PackwizObservations,
    pub validation: Vec<ValidationResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectRecord {
    pub id: String,
    pub canonical_path: String,
    pub application: ApplicationProjectMetadata,
    pub packwiz: PackwizObservations,
    pub validation: Vec<ValidationResult>,
    pub created_at: String,
    pub updated_at: String,
    pub last_opened_at: Option<String>,
    pub last_refreshed_at: Option<String>,
}

impl ValidationResult {
    pub fn info(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            valid: true,
            severity: ValidationSeverity::Info,
            code: code.into(),
            message: message.into(),
        }
    }

    pub fn warning(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            valid: true,
            severity: ValidationSeverity::Warning,
            code: code.into(),
            message: message.into(),
        }
    }

    pub fn error(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            valid: false,
            severity: ValidationSeverity::Error,
            code: code.into(),
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ProjectReference {
    pub id: String,
    pub name: String,
    pub root_path: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OperationStatus {
    Pending,
    Running,
    Succeeded,
    Failed,
    Cancelled,
    Unavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ValidationSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ValidationResult {
    pub valid: bool,
    pub severity: ValidationSeverity,
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CommandError {
    pub code: String,
    pub message: String,
    pub details: Option<String>,
}

impl CommandError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            details: None,
        }
    }

    pub fn with_details(mut self, details: impl Into<String>) -> Self {
        self.details = Some(details.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ApplicationProjectMetadata, CommandError, Observation, OperationStatus, ProjectLifecycle,
        ValidationSeverity,
    };

    #[test]
    fn contracts_serialize_stable_wire_values() {
        assert_eq!(
            serde_json::to_string(&OperationStatus::Unavailable).unwrap(),
            "\"unavailable\""
        );
        assert_eq!(
            serde_json::to_string(&ValidationSeverity::Warning).unwrap(),
            "\"warning\""
        );
        assert_eq!(
            serde_json::to_string(&CommandError::new(
                "not_registered",
                "Project is not registered"
            ))
            .unwrap(),
            r#"{"code":"not_registered","message":"Project is not registered","details":null}"#
        );
    }

    #[test]
    fn application_and_external_values_are_distinguishable() {
        let application = ApplicationProjectMetadata {
            display_name: "My pack".to_string(),
            icon: None,
            theme: Some("ember".to_string()),
            tags: vec!["survival".to_string()],
            favorite: true,
            description: None,
            lifecycle: ProjectLifecycle::Active,
        };
        assert_eq!(application.lifecycle, ProjectLifecycle::Active);
        assert_eq!(Observation::<String>::Unavailable, Observation::Unavailable);
        assert_eq!(
            serde_json::to_string(&Observation::Unavailable::<String>).unwrap(),
            "\"unavailable\""
        );
    }
}
