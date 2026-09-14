use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

pub const DATA_BUNDLE_FORMAT: &str = "cm-modpack-util-data";
pub const DATA_BUNDLE_VERSION: u32 = 1;
pub const DATA_TRANSFER_MAX_COMPRESSED_BYTES: usize = 64 * 1024 * 1024;
pub const DATA_TRANSFER_MAX_DECOMPRESSED_BYTES: usize = 256 * 1024 * 1024;
pub const DATA_TRANSFER_MAX_RECORDS: usize = 100_000;
pub const DATA_TRANSFER_MAX_NESTING: usize = 32;
pub const DATA_TRANSFER_MAX_STRING_BYTES: usize = 8 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DataTransferRecord {
    pub identity: String,
    pub fields: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DataTransferPayload {
    pub settings: Vec<DataTransferRecord>,
    pub records: BTreeMap<String, Vec<DataTransferRecord>>,
    pub excluded: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DataTransferApplication {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DataTransferEnvelope {
    pub format: String,
    pub version: u32,
    pub created_at: String,
    pub application: DataTransferApplication,
    pub payload_sha256: String,
    pub payload: DataTransferPayload,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DataTransferCompression {
    Json,
    Gzip,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DataTransferExportResult {
    pub destination: String,
    pub compression: DataTransferCompression,
    pub format: String,
    pub version: u32,
    pub payload_sha256: String,
    pub record_count: u64,
    pub excluded: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DataTransferRecordStatus {
    Addition,
    Identical,
    Conflict,
    Unavailable,
    DependentSkip,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DataTransferRecordPreview {
    pub family: String,
    pub identity: String,
    pub status: DataTransferRecordStatus,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DataTransferPreview {
    pub fingerprint: String,
    pub format: String,
    pub version: u32,
    pub record_count: u64,
    pub records: Vec<DataTransferRecordPreview>,
    pub excluded: Vec<String>,
    pub safe_record_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DataTransferImportRequest {
    pub source: String,
    pub preview_fingerprint: String,
    pub selected: Vec<String>,
    #[serde(default)]
    pub cancelled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DataTransferImportResult {
    pub imported: u64,
    pub skipped: u64,
    pub fingerprint: String,
}

pub fn canonicalize(value: &Value) -> Value {
    match value {
        Value::Object(object) => {
            let mut sorted = Map::new();
            for (key, value) in object {
                sorted.insert(key.clone(), canonicalize(value));
            }
            Value::Object(sorted)
        }
        Value::Array(values) => Value::Array(values.iter().map(canonicalize).collect()),
        _ => value.clone(),
    }
}

pub fn canonical_json_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>, serde_json::Error> {
    let value = serde_json::to_value(value)?;
    serde_json::to_vec(&canonicalize(&value))
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::{canonical_json_bytes, sha256_hex};
    use serde_json::json;

    #[test]
    fn canonical_json_sorts_objects_but_preserves_arrays() {
        let left = canonical_json_bytes(&json!({ "b": 1, "a": [2, 1] })).unwrap();
        let right = canonical_json_bytes(&json!({ "a": [2, 1], "b": 1 })).unwrap();
        assert_eq!(left, right);
        assert_eq!(left, br#"{"a":[2,1],"b":1}"#);
    }

    #[test]
    fn sha256_is_lowercase_hex() {
        assert_eq!(
            sha256_hex(b"cm-modpack-util"),
            "f3d080f4ab0d32ca6e0a85814314a011e6947ab749baa7ed2f427b044047db2e"
        );
    }
}
