use crate::db::Database;
use crate::domain::{
    canonical_json_bytes, sha256_hex, CommandError, DataTransferApplication,
    DataTransferCompression, DataTransferEnvelope, DataTransferExportResult,
    DataTransferImportRequest, DataTransferImportResult, DataTransferPayload, DataTransferPreview,
    DataTransferRecord, DataTransferRecordPreview, DataTransferRecordStatus, DATA_BUNDLE_FORMAT,
    DATA_BUNDLE_VERSION, DATA_TRANSFER_MAX_COMPRESSED_BYTES, DATA_TRANSFER_MAX_DECOMPRESSED_BYTES,
    DATA_TRANSFER_MAX_NESTING, DATA_TRANSFER_MAX_RECORDS, DATA_TRANSFER_MAX_STRING_BYTES,
};
use flate2::{read::GzDecoder, write::GzEncoder, Compression};
use rusqlite::{params_from_iter, types::ValueRef, Connection};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::io::{Cursor, Read, Write};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{Manager, State};
use tauri_plugin_autostart::ManagerExt as AutostartManagerExt;
use tauri_plugin_window_state::{StateFlags, WindowExt};

const INCLUDED_TABLES: &[&str] = &[
    "modpacks",
    "inventory_observations",
    "modpack_activity",
    "discovery_attempts",
    "discovery_candidates",
    "snapshots",
    "snapshot_candidates",
    "snapshot_decisions",
    "snapshot_notes",
    "snapshot_rechecks",
    "operation_attempts",
    "release_workspace_operations",
    "operation_candidates",
    "operation_acknowledgements",
    "pin_observations",
    "changelog_attempts",
    "changelog_artifacts",
    "changelog_revisions",
    "changelog_exports",
    "releases",
    "release_snapshots",
    "release_changelog_artifacts",
    "release_workspaces",
    "release_workspace_candidates",
    "release_workspace_candidate_sources",
    "release_workspace_activity",
    "release_workspace_observations",
    "lifecycle_tombstones",
    "lifecycle_operations",
    "cleanup_operations",
];

const EXCLUDED: &[&str] = &[
    "changelog_cache",
    "packwiz_project_files",
    "git_metadata",
    "external_changelog_destination_files",
];

#[derive(Debug, Clone)]
struct RawRecord {
    table: String,
    primary_key: String,
    fields: BTreeMap<String, Value>,
}

#[tauri::command]
pub fn export_application_data(
    state: State<'_, Database>,
    destination: String,
    compression: DataTransferCompression,
) -> Result<DataTransferExportResult, CommandError> {
    let connection = state
        .0
        .lock()
        .map_err(|_| CommandError::new("database_unavailable", "Database is unavailable"))?;
    export_application_data_inner(&connection, &destination, compression)
}

fn export_application_data_inner(
    connection: &Connection,
    destination: &str,
    compression: DataTransferCompression,
) -> Result<DataTransferExportResult, CommandError> {
    if destination.trim().is_empty() {
        return Err(CommandError::new(
            "invalid_destination",
            "An export destination is required",
        ));
    }

    let raw_records = load_records(connection)?;
    let mut identities = HashMap::new();
    let mut used_identities = HashMap::<String, u32>::new();
    let mut records = INCLUDED_TABLES
        .iter()
        .map(|table| ((*table).to_string(), Vec::new()))
        .collect::<BTreeMap<_, _>>();

    for raw in raw_records {
        let identity = logical_identity(&raw, &mut used_identities);
        identities.insert(
            (raw.table.clone(), raw.primary_key.clone()),
            identity.clone(),
        );
        records
            .entry(raw.table.clone())
            .or_default()
            .push(DataTransferRecord {
                identity,
                fields: raw.fields,
            });
    }

    for family_records in records.values_mut() {
        family_records.sort_by(|left, right| left.identity.cmp(&right.identity));
    }
    logicalize_references(&mut records, &identities);

    let settings = load_settings(connection)?;
    let payload = DataTransferPayload {
        settings,
        records,
        excluded: EXCLUDED.iter().map(|value| (*value).to_string()).collect(),
    };
    let payload_bytes = canonical_json_bytes(&payload).map_err(|error| {
        CommandError::new(
            "serialization_failed",
            "Application data could not be serialized",
        )
        .with_details(error.to_string())
    })?;
    let payload_sha256 = sha256_hex(&payload_bytes);
    let envelope = DataTransferEnvelope {
        format: DATA_BUNDLE_FORMAT.to_string(),
        version: DATA_BUNDLE_VERSION,
        created_at: timestamp(),
        application: DataTransferApplication {
            name: "CM Modpack Util".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        },
        payload_sha256: payload_sha256.clone(),
        payload,
    };
    let envelope_bytes = canonical_json_bytes(&envelope).map_err(|error| {
        CommandError::new(
            "serialization_failed",
            "Data bundle could not be serialized",
        )
        .with_details(error.to_string())
    })?;
    let bytes = match compression {
        DataTransferCompression::Json => envelope_bytes,
        DataTransferCompression::Gzip => {
            let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
            encoder.write_all(&envelope_bytes).map_err(|error| {
                CommandError::new("compression_failed", "Data bundle could not be compressed")
                    .with_details(error.to_string())
            })?;
            encoder.finish().map_err(|error| {
                CommandError::new("compression_failed", "Data bundle could not be finalized")
                    .with_details(error.to_string())
            })?
        }
    };
    fs::write(destination, bytes).map_err(|error| {
        CommandError::new(
            "destination_write_failed",
            "The application data bundle could not be written",
        )
        .with_details(error.to_string())
    })?;

    Ok(DataTransferExportResult {
        destination: destination.to_string(),
        compression,
        format: DATA_BUNDLE_FORMAT.to_string(),
        version: DATA_BUNDLE_VERSION,
        payload_sha256,
        record_count: count_records(&envelope.payload),
        excluded: envelope.payload.excluded,
    })
}

fn load_settings(connection: &Connection) -> Result<Vec<DataTransferRecord>, CommandError> {
    let mut statement = connection
        .prepare("SELECT key, value_json FROM settings ORDER BY key")
        .map_err(read_error)?;
    let rows = statement
        .query_map([], |row| {
            let key: String = row.get(0)?;
            let value: String = row.get(1)?;
            let mut fields = BTreeMap::new();
            fields.insert("key".to_string(), Value::String(key.clone()));
            fields.insert(
                "value_json".to_string(),
                serde_json::from_str(&value).unwrap_or(Value::String(value)),
            );
            Ok(DataTransferRecord {
                identity: format!("setting:{key}"),
                fields,
            })
        })
        .map_err(read_error)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(read_error)
}

fn load_records(connection: &Connection) -> Result<Vec<RawRecord>, CommandError> {
    let mut records = Vec::new();
    for table in INCLUDED_TABLES {
        let mut statement = connection
            .prepare(&format!("SELECT * FROM \"{table}\""))
            .map_err(read_error)?;
        let column_names = statement
            .column_names()
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>();
        let rows = statement
            .query_map([], |row| {
                let primary_key = primary_key_value(table, row)?;
                let mut fields = BTreeMap::new();
                for (index, column) in column_names.iter().enumerate() {
                    if is_generated_integer_key(table, column) {
                        continue;
                    }
                    fields.insert(column.clone(), value_from_ref(row.get_ref(index)?, column));
                }
                Ok(RawRecord {
                    table: (*table).to_string(),
                    primary_key,
                    fields,
                })
            })
            .map_err(read_error)?;
        for row in rows {
            records.push(row.map_err(read_error)?);
        }
    }
    Ok(records)
}

fn primary_key_value(table: &str, row: &rusqlite::Row<'_>) -> rusqlite::Result<String> {
    let columns = match table {
        "inventory_observations" => &["modpack_id"][..],
        "release_snapshots" => &["release_id"][..],
        "release_changelog_artifacts" => &["release_id", "artifact_id"][..],
        "lifecycle_tombstones" => &["record_type", "record_id"][..],
        _ => &["id"][..],
    };
    let mut values = Vec::with_capacity(columns.len());
    for column in columns {
        let index = row.as_ref().column_index(column)?;
        values.push(match row.get_ref(index)? {
            ValueRef::Null => "null".to_string(),
            ValueRef::Integer(value) => value.to_string(),
            ValueRef::Real(value) => value.to_string(),
            ValueRef::Text(value) => String::from_utf8_lossy(value).into_owned(),
            ValueRef::Blob(value) => format!("{value:x?}"),
        });
    }
    Ok(values.join(":"))
}

fn is_generated_integer_key(table: &str, column: &str) -> bool {
    matches!(
        (table, column),
        ("modpack_activity", "id")
            | ("discovery_attempts", "id")
            | ("discovery_candidates", "id")
            | ("snapshot_decisions", "id")
            | ("snapshot_notes", "id")
            | ("snapshot_rechecks", "id")
            | ("operation_candidates", "id")
            | ("operation_acknowledgements", "id")
            | ("pin_observations", "id")
            | ("release_workspace_candidates", "id")
            | ("release_workspace_activity", "id")
            | ("release_workspace_observations", "id")
    )
}

fn logical_identity(raw: &RawRecord, used: &mut HashMap<String, u32>) -> String {
    if !is_integer_primary_key_table(&raw.table) {
        return format!("{}:{}", raw.table.trim_end_matches('s'), raw.primary_key);
    }
    let projection = serde_json::to_value(&raw.fields).unwrap_or(Value::Null);
    let hash = sha256_hex(&canonical_json_bytes(&projection).unwrap_or_default());
    let base = format!("{}:{hash}", raw.table.trim_end_matches('s'));
    let ordinal = used.entry(base.clone()).or_insert(0);
    let identity = if *ordinal == 0 {
        base
    } else {
        format!("{base}:{}", *ordinal)
    };
    *ordinal += 1;
    identity
}

fn is_integer_primary_key_table(table: &str) -> bool {
    matches!(
        table,
        "modpack_activity"
            | "discovery_attempts"
            | "discovery_candidates"
            | "snapshot_decisions"
            | "snapshot_notes"
            | "snapshot_rechecks"
            | "operation_candidates"
            | "operation_acknowledgements"
            | "pin_observations"
            | "release_workspace_candidates"
            | "release_workspace_activity"
            | "release_workspace_observations"
    )
}

fn logicalize_references(
    records: &mut BTreeMap<String, Vec<DataTransferRecord>>,
    identities: &HashMap<(String, String), String>,
) {
    for (table, family_records) in records {
        for record in family_records {
            for (column, value) in &mut record.fields {
                let Some(target_table) = reference_target(table, column) else {
                    continue;
                };
                let Some(local_key) = value_to_key(value) else {
                    continue;
                };
                if let Some(identity) = identities.get(&(target_table.to_string(), local_key)) {
                    *value = Value::String(identity.clone());
                }
            }
        }
    }
}

fn reference_target(table: &str, column: &str) -> Option<&'static str> {
    match (table, column) {
        (_, "modpack_id") => Some("modpacks"),
        ("discovery_candidates", "attempt_id") => Some("discovery_attempts"),
        (_, "snapshot_id") => Some("snapshots"),
        ("snapshot_decisions", "candidate_id") => Some("release_workspace_candidate_sources"),
        ("operation_candidates", "candidate_id") => Some("snapshot_candidates"),
        (_, "workspace_id") | (_, "release_workspace_id") => Some("release_workspaces"),
        (_, "operation_id") => Some("operation_attempts"),
        (_, "artifact_id") => Some("changelog_artifacts"),
        (_, "revision_id") | (_, "final_changelog_revision_id") => Some("changelog_revisions"),
        (_, "release_id") | (_, "baseline_release_id") => Some("releases"),
        (_, "attempt_id") => Some("changelog_attempts"),
        (_, "source_candidate_id") => Some("release_workspace_candidate_sources"),
        (_, "source_snapshot_candidate_id") => Some("snapshot_candidates"),
        (_, "candidate_id") => Some("snapshot_candidates"),
        (_, "predecessor_id") => Some("snapshots"),
        _ => None,
    }
}

fn value_to_key(value: &Value) -> Option<String> {
    match value {
        Value::String(value) => Some(value.clone()),
        Value::Number(value) => Some(value.to_string()),
        _ => None,
    }
}

fn value_from_ref(value: ValueRef<'_>, column: &str) -> Value {
    match value {
        ValueRef::Null => Value::Null,
        ValueRef::Integer(value) => Value::Number(value.into()),
        ValueRef::Real(value) => serde_json::Number::from_f64(value)
            .map(Value::Number)
            .unwrap_or(Value::Null),
        ValueRef::Text(value) => {
            let text = String::from_utf8_lossy(value).into_owned();
            if column.ends_with("_json") {
                serde_json::from_str(&text).unwrap_or(Value::String(text))
            } else {
                Value::String(text)
            }
        }
        ValueRef::Blob(value) => Value::Array(
            value
                .iter()
                .map(|byte| Value::Number((*byte).into()))
                .collect(),
        ),
    }
}

fn count_records(payload: &DataTransferPayload) -> u64 {
    payload.settings.len() as u64
        + payload
            .records
            .values()
            .map(|records| records.len() as u64)
            .sum::<u64>()
}

fn timestamp() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| format!("{}.{:09}", duration.as_secs(), duration.subsec_nanos()))
        .unwrap_or_else(|_| "0".to_string())
}

fn read_error(error: rusqlite::Error) -> CommandError {
    CommandError::new("database_read_failed", "Application data could not be read")
        .with_details(error.to_string())
}

#[tauri::command]
pub fn preview_application_data(
    state: State<'_, Database>,
    source: String,
) -> Result<DataTransferPreview, CommandError> {
    let connection = state
        .0
        .lock()
        .map_err(|_| CommandError::new("database_unavailable", "Database is unavailable"))?;
    preview_application_data_inner(&connection, Path::new(&source))
}

#[tauri::command]
pub fn import_application_data(
    app: tauri::AppHandle,
    state: State<'_, Database>,
    request: DataTransferImportRequest,
) -> Result<DataTransferImportResult, CommandError> {
    let connection = state
        .0
        .lock()
        .map_err(|_| CommandError::new("database_unavailable", "Database is unavailable"))?;
    apply_os_effects(&app, &request)?;
    import_application_data_inner(&connection, &request)
}

fn apply_os_effects(
    app: &tauri::AppHandle,
    request: &DataTransferImportRequest,
) -> Result<(), CommandError> {
    let (envelope, _) = read_envelope(Path::new(&request.source))?;
    let selected = request
        .selected
        .iter()
        .collect::<std::collections::HashSet<_>>();
    for setting in envelope.payload.settings {
        if !selected.contains(&setting.identity) {
            continue;
        }
        let key = setting.fields.get("key").and_then(Value::as_str);
        let value = setting
            .fields
            .get("value_json")
            .and_then(Value::as_bool)
            .ok_or_else(|| {
                CommandError::new(
                    "os_effect_failed",
                    "An imported operating-system setting has an invalid value",
                )
            })?;
        match key {
            Some("general.launchAtLogin") => {
                let result = if value {
                    app.autolaunch().enable()
                } else {
                    app.autolaunch().disable()
                };
                result.map_err(|error| {
                    CommandError::new(
                        "os_effect_failed",
                        "The launch-at-login setting could not be applied",
                    )
                    .with_details(error.to_string())
                })?;
            }
            Some("general.restoreWindowState") if value => {
                if let Some(window) = app.get_webview_window("main") {
                    window.restore_state(StateFlags::all()).map_err(|error| {
                        CommandError::new(
                            "os_effect_failed",
                            "The window state setting could not be applied",
                        )
                        .with_details(error.to_string())
                    })?;
                }
            }
            Some("general.startMinimized") if value => {
                if let Some(window) = app.get_webview_window("main") {
                    window.minimize().map_err(|error| {
                        CommandError::new(
                            "os_effect_failed",
                            "The start-minimized setting could not be applied",
                        )
                        .with_details(error.to_string())
                    })?;
                }
            }
            _ => {}
        }
    }
    Ok(())
}

fn read_envelope(path: &Path) -> Result<(DataTransferEnvelope, String), CommandError> {
    let compressed = fs::read(path).map_err(|error| {
        CommandError::new(
            "source_read_failed",
            "The application data bundle could not be read",
        )
        .with_details(error.to_string())
    })?;
    if compressed.len() > DATA_TRANSFER_MAX_COMPRESSED_BYTES {
        return Err(CommandError::new(
            "resource_limit",
            "The data bundle is too large",
        ));
    }
    let bytes = if compressed.starts_with(&[0x1f, 0x8b]) {
        let decoder = GzDecoder::new(Cursor::new(&compressed));
        let mut output = Vec::new();
        decoder
            .take((DATA_TRANSFER_MAX_DECOMPRESSED_BYTES + 1) as u64)
            .read_to_end(&mut output)
            .map_err(|error| {
                CommandError::new(
                    "invalid_compression",
                    "The data bundle gzip stream is invalid",
                )
                .with_details(error.to_string())
            })?;
        output
    } else {
        compressed.clone()
    };
    if bytes.len() > DATA_TRANSFER_MAX_DECOMPRESSED_BYTES {
        return Err(CommandError::new(
            "resource_limit",
            "The decompressed data bundle is too large",
        ));
    }
    let value: Value = serde_json::from_slice(&bytes).map_err(|error| {
        CommandError::new("invalid_json", "The data bundle is not valid JSON")
            .with_details(error.to_string())
    })?;
    validate_limits(&value, 0)?;
    let envelope: DataTransferEnvelope = serde_json::from_value(value).map_err(|error| {
        CommandError::new("invalid_bundle", "The data bundle has an invalid envelope")
            .with_details(error.to_string())
    })?;
    if count_records(&envelope.payload) as usize > DATA_TRANSFER_MAX_RECORDS {
        return Err(CommandError::new(
            "resource_limit",
            "The data bundle contains too many records",
        ));
    }
    if envelope.format != DATA_BUNDLE_FORMAT {
        return Err(CommandError::new(
            "unsupported_format",
            "The data bundle format is not supported",
        ));
    }
    if envelope.version != DATA_BUNDLE_VERSION {
        return Err(CommandError::new(
            "unsupported_version",
            "The data bundle version is not supported",
        ));
    }
    let payload_bytes = canonical_json_bytes(&envelope.payload).map_err(|error| {
        CommandError::new("invalid_bundle", "The data bundle payload is invalid")
            .with_details(error.to_string())
    })?;
    if sha256_hex(&payload_bytes) != envelope.payload_sha256 {
        return Err(CommandError::new(
            "integrity_failure",
            "The data bundle integrity hash does not match",
        ));
    }
    let fingerprint = sha256_hex(&canonical_json_bytes(&envelope).map_err(|error| {
        CommandError::new(
            "invalid_bundle",
            "The data bundle could not be canonicalized",
        )
        .with_details(error.to_string())
    })?);
    Ok((envelope, fingerprint))
}

fn validate_limits(value: &Value, depth: usize) -> Result<usize, CommandError> {
    if depth > DATA_TRANSFER_MAX_NESTING {
        return Err(CommandError::new(
            "resource_limit",
            "The data bundle nesting depth exceeds the limit",
        ));
    }
    match value {
        Value::String(text) if text.len() > DATA_TRANSFER_MAX_STRING_BYTES => {
            Err(CommandError::new(
                "resource_limit",
                "The data bundle contains an oversized string",
            ))
        }
        Value::Array(values) => values.iter().try_fold(0, |count, value| {
            Ok(count + validate_limits(value, depth + 1)?)
        }),
        Value::Object(values) => values.iter().try_fold(0, |count, (key, value)| {
            if key.len() > DATA_TRANSFER_MAX_STRING_BYTES {
                return Err(CommandError::new(
                    "resource_limit",
                    "The data bundle contains an oversized key",
                ));
            }
            Ok(count + validate_limits(value, depth + 1)?)
        }),
        _ => Ok(0),
    }
}

fn preview_application_data_inner(
    connection: &Connection,
    source: &Path,
) -> Result<DataTransferPreview, CommandError> {
    let (envelope, source_fingerprint) = read_envelope(source)?;
    let current = current_records(connection)?;
    let fingerprint = preview_fingerprint(&source_fingerprint, &current)?;
    let mut incoming = Vec::new();
    for record in envelope.payload.settings {
        incoming.push(("settings".to_string(), record));
    }
    for (family, records) in envelope.payload.records {
        for record in records {
            incoming.push((family.clone(), record));
        }
    }
    let existing_paths = current
        .values()
        .filter(|record| record.fields.contains_key("canonical_path"))
        .filter_map(|record| {
            record
                .fields
                .get("canonical_path")
                .and_then(Value::as_str)
                .map(|path| (canonical_path_key(path), record.identity.clone()))
        })
        .collect::<HashMap<_, _>>();
    let mut previews = incoming
        .iter()
        .map(|(family, record)| classify(family, record, current.get(&record_key(family, record))))
        .collect::<Vec<_>>();
    for ((family, record), preview) in incoming.iter().zip(previews.iter_mut()) {
        if family == "modpacks" {
            if let Some(path) = record.fields.get("canonical_path").and_then(Value::as_str) {
                if let Some(existing_identity) = existing_paths.get(&canonical_path_key(path)) {
                    if existing_identity != &record.identity {
                        preview.status = DataTransferRecordStatus::Conflict;
                        preview.reason = Some("canonical_path_conflict".to_string());
                    }
                }
            }
        }
    }
    let conflicting_modpacks = previews
        .iter()
        .filter(|item| {
            item.family == "modpacks" && item.status == DataTransferRecordStatus::Conflict
        })
        .map(|item| item.identity.clone())
        .collect::<std::collections::HashSet<_>>();
    for ((family, record), preview) in incoming.iter().zip(previews.iter_mut()) {
        if family != "modpacks"
            && record
                .fields
                .values()
                .filter_map(Value::as_str)
                .any(|value| conflicting_modpacks.contains(value))
        {
            preview.status = DataTransferRecordStatus::DependentSkip;
            preview.reason = Some("conflicting_modpack".to_string());
        }
    }
    let safe_record_count = previews
        .iter()
        .filter(|record| {
            matches!(
                record.status,
                DataTransferRecordStatus::Addition | DataTransferRecordStatus::Unavailable
            )
        })
        .count() as u64;
    Ok(DataTransferPreview {
        fingerprint,
        format: envelope.format,
        version: envelope.version,
        record_count: previews.len() as u64,
        records: previews,
        excluded: envelope.payload.excluded,
        safe_record_count,
    })
}

fn classify(
    family: &str,
    record: &DataTransferRecord,
    existing: Option<&DataTransferRecord>,
) -> DataTransferRecordPreview {
    let status = if let Some(existing) = existing {
        if existing.fields == record.fields {
            DataTransferRecordStatus::Identical
        } else {
            DataTransferRecordStatus::Conflict
        }
    } else if family == "modpacks"
        && record
            .fields
            .get("canonical_path")
            .and_then(Value::as_str)
            .is_some_and(|path| !Path::new(path).exists())
    {
        DataTransferRecordStatus::Unavailable
    } else {
        DataTransferRecordStatus::Addition
    };
    DataTransferRecordPreview {
        family: family.to_string(),
        identity: record.identity.clone(),
        status,
        reason: None,
    }
}

fn record_key(family: &str, record: &DataTransferRecord) -> String {
    format!("{family}:{}", record.identity)
}

fn current_records(
    connection: &Connection,
) -> Result<HashMap<String, DataTransferRecord>, CommandError> {
    let mut result = HashMap::new();
    let mut used_identities = HashMap::new();
    for raw in load_records(connection)? {
        let identity = if is_integer_primary_key_table(&raw.table) {
            logical_identity(&raw, &mut used_identities)
        } else {
            format!("{}:{}", raw.table.trim_end_matches('s'), raw.primary_key)
        };
        result.insert(
            record_key(
                &raw.table,
                &DataTransferRecord {
                    identity: identity.clone(),
                    fields: raw.fields.clone(),
                },
            ),
            DataTransferRecord {
                identity,
                fields: raw.fields,
            },
        );
    }

    for record in load_settings(connection)? {
        result.insert(record_key("settings", &record), record);
    }
    Ok(result)
}

fn preview_fingerprint(
    source_fingerprint: &str,
    current: &HashMap<String, DataTransferRecord>,
) -> Result<String, CommandError> {
    let current_bytes = canonical_json_bytes(current).map_err(|error| {
        CommandError::new(
            "serialization_failed",
            "The database fingerprint could not be computed",
        )
        .with_details(error.to_string())
    })?;
    Ok(sha256_hex(
        format!("{source_fingerprint}:{}", sha256_hex(&current_bytes)).as_bytes(),
    ))
}

fn import_application_data_inner(
    connection: &Connection,
    request: &DataTransferImportRequest,
) -> Result<DataTransferImportResult, CommandError> {
    let (envelope, source_fingerprint) = read_envelope(Path::new(&request.source))?;
    let current_before = current_records(connection)?;
    let fingerprint = preview_fingerprint(&source_fingerprint, &current_before)?;
    if fingerprint != request.preview_fingerprint {
        return Err(CommandError::new(
            "stale_preview",
            "The preview is stale; review the bundle again before importing",
        ));
    }
    let preview = preview_application_data_inner(connection, Path::new(&request.source))?;
    if preview.fingerprint != request.preview_fingerprint {
        return Err(CommandError::new(
            "stale_preview",
            "Application data changed after the preview was created",
        ));
    }
    if request.cancelled {
        return Err(CommandError::new(
            "cancelled",
            "Application data import was cancelled before commit",
        ));
    }
    let selected = request
        .selected
        .iter()
        .collect::<std::collections::HashSet<_>>();
    let current = current_records(connection)?;
    let mut local_keys = local_key_map(connection)?;
    let transaction = connection.unchecked_transaction().map_err(write_error)?;
    let mut imported = 0;
    let mut skipped = 0;
    let records = envelope.payload.records;
    for record in envelope.payload.settings {
        if selected.contains(&record.identity)
            && !current.contains_key(&record_key("settings", &record))
        {
            let value = record
                .fields
                .get("value_json")
                .cloned()
                .unwrap_or(Value::Null);
            transaction
                .execute(
                    "INSERT OR REPLACE INTO settings (key, value_json, updated_at) VALUES (?1, ?2, ?3)",
                    [&record.fields["key"].as_str().unwrap_or_default().to_string(), &serde_json::to_string(&value).unwrap_or_else(|_| "null".to_string()), &timestamp()],
                )
                .map_err(write_error)?;
            imported += 1;
        } else {
            skipped += 1;
        }
    }
    let mut pending = INCLUDED_TABLES
        .iter()
        .flat_map(|family| {
            records
                .get(*family)
                .into_iter()
                .flatten()
                .filter(|record| selected.contains(&record.identity))
                .filter(|record| !current.contains_key(&record_key(family, record)))
                .map(|record| ((*family).to_string(), record.clone()))
        })
        .collect::<Vec<_>>();
    skipped += records
        .iter()
        .flat_map(|(family, records)| {
            records.iter().filter(|record| {
                !selected.contains(&record.identity)
                    || current.contains_key(&record_key(family, record))
            })
        })
        .count() as u64;

    while !pending.is_empty() {
        let mut deferred = Vec::new();
        let mut inserted_this_pass = 0;
        for (family, record) in pending {
            if !dependencies_ready(&family, &record, &local_keys, &records) {
                deferred.push((family, record));
                continue;
            }
            insert_record(&transaction, &family, &record, &local_keys)?;
            let local_key = if is_integer_primary_key_table(&family) {
                transaction.last_insert_rowid().to_string()
            } else {
                primary_key_from_fields(&family, &record.fields).ok_or_else(|| {
                    CommandError::new(
                        "invalid_bundle",
                        "A record has no importable primary-key fields",
                    )
                })?
            };
            local_keys.insert(record.identity.clone(), local_key);
            imported += 1;
            inserted_this_pass += 1;
        }
        if inserted_this_pass == 0 {
            let (family, record) = deferred
                .into_iter()
                .next()
                .expect("deferred records should not be empty");
            insert_record(&transaction, &family, &record, &local_keys)?;
            unreachable!("unresolved selected record should fail its foreign-key constraint");
        }
        pending = deferred;
    }
    transaction.commit().map_err(write_error)?;
    Ok(DataTransferImportResult {
        imported,
        skipped,
        fingerprint,
    })
}

fn local_key_map(connection: &Connection) -> Result<HashMap<String, String>, CommandError> {
    let mut result = HashMap::new();
    let mut used_identities = HashMap::new();
    for raw in load_records(connection)? {
        let identity = if is_integer_primary_key_table(&raw.table) {
            logical_identity(&raw, &mut used_identities)
        } else {
            format!("{}:{}", raw.table.trim_end_matches('s'), raw.primary_key)
        };
        result.insert(identity, raw.primary_key);
    }
    Ok(result)
}

fn dependencies_ready(
    family: &str,
    record: &DataTransferRecord,
    local_keys: &HashMap<String, String>,
    records: &BTreeMap<String, Vec<DataTransferRecord>>,
) -> bool {
    record.fields.iter().all(|(column, value)| {
        let Some(target_table) = reference_target(family, column) else {
            return true;
        };
        let Some(reference) = value_to_key(value) else {
            return true;
        };
        if local_keys.contains_key(&reference) {
            return true;
        }
        !records
            .get(target_table)
            .into_iter()
            .flatten()
            .any(|candidate| candidate.identity == reference)
    })
}

fn insert_record(
    transaction: &rusqlite::Transaction<'_>,
    family: &str,
    record: &DataTransferRecord,
    local_keys: &HashMap<String, String>,
) -> Result<(), CommandError> {
    let mut columns = Vec::new();
    let mut values = Vec::new();
    for (column, value) in &record.fields {
        if !valid_identifier(column) || is_generated_integer_key(family, column) {
            continue;
        }
        columns.push(format!("\"{column}\""));
        let value = if let Some(identity) = value.as_str() {
            local_keys.get(identity).map_or_else(
                || json_to_sql(&Value::String(identity.to_string())),
                |local| rusqlite::types::Value::Text(local.clone()),
            )
        } else {
            json_to_sql(value)
        };
        values.push(value);
    }
    if columns.is_empty() {
        return Err(CommandError::new(
            "invalid_bundle",
            "A record has no importable fields",
        ));
    }
    let sql = format!(
        "INSERT INTO \"{family}\" ({}) VALUES ({})",
        columns.join(", "),
        (1..=columns.len())
            .map(|index| format!("?{index}"))
            .collect::<Vec<_>>()
            .join(", ")
    );
    transaction
        .execute(&sql, params_from_iter(values.iter()))
        .map_err(|error| {
            let details = format!("{family}:{} values={values:?} ({error})", record.identity);
            write_error(error).with_details(details)
        })?;
    Ok(())
}

fn json_to_sql(value: &Value) -> rusqlite::types::Value {
    match value {
        Value::Null => rusqlite::types::Value::Null,
        Value::Bool(value) => rusqlite::types::Value::Integer(*value as i64),
        Value::Number(value) if value.is_i64() => {
            rusqlite::types::Value::Integer(value.as_i64().unwrap())
        }
        Value::Number(value) => rusqlite::types::Value::Real(value.as_f64().unwrap_or_default()),
        Value::String(value) => rusqlite::types::Value::Text(value.clone()),
        Value::Array(_) | Value::Object(_) => rusqlite::types::Value::Text(
            serde_json::to_string(value).unwrap_or_else(|_| "null".to_string()),
        ),
    }
}

fn primary_key_from_fields(family: &str, fields: &BTreeMap<String, Value>) -> Option<String> {
    let columns = match family {
        "inventory_observations" => vec!["modpack_id"],
        "release_snapshots" => vec!["release_id"],
        "release_changelog_artifacts" => vec!["release_id", "artifact_id"],
        "lifecycle_tombstones" => vec!["record_type", "record_id"],
        _ => vec!["id"],
    };
    let values = columns
        .iter()
        .map(|column| {
            fields.get(*column).map(|value| {
                value
                    .as_str()
                    .map_or_else(|| value.to_string(), ToString::to_string)
            })
        })
        .collect::<Option<Vec<_>>>()?;
    Some(values.join(":"))
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte == b'_' || byte.is_ascii_alphanumeric())
}

fn canonical_path_key(path: &str) -> String {
    fs::canonicalize(path)
        .map(|path| path.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_else(|_| path.replace('/', "\\").to_ascii_lowercase())
}

fn write_error(error: rusqlite::Error) -> CommandError {
    CommandError::new(
        "database_write_failed",
        "Application data could not be imported",
    )
    .with_details(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::{
        export_application_data_inner, import_application_data_inner,
        preview_application_data_inner, read_envelope, DataTransferCompression,
    };
    use crate::db::initialize;
    use crate::domain::DataTransferImportRequest;
    use flate2::read::GzDecoder;
    use rusqlite::params;
    use serde_json::Value;
    use std::fs;
    use std::io::Read;
    use std::path::Path;

    #[test]
    fn exports_settings_and_exclusions_without_provider_cache() {
        let database = initialize(Path::new(":memory:")).expect("database should initialize");
        let connection = database
            .0
            .lock()
            .expect("database lock should be available");
        connection
            .execute(
                "INSERT INTO settings (key, value_json, updated_at) VALUES (?1, ?2, ?3)",
                params!["appearance.theme", "\"dark\"", "1"],
            )
            .expect("setting should be inserted");
        connection
            .execute(
                "INSERT INTO changelog_cache (id, cache_key_json, raw_response, association_json, retrieved_at, request_context, response_context) VALUES ('cache', '{}', 'provider response', '{}', '1', '{}', '{}')",
                [],
            )
            .expect("cache should be inserted");

        let path = std::env::temp_dir().join("cm-modpack-util-phase2-export.json");
        let result = export_application_data_inner(
            &connection,
            path.to_str().expect("temporary path should be valid"),
            DataTransferCompression::Json,
        )
        .expect("export should succeed");
        let envelope: Value =
            serde_json::from_slice(&fs::read(&path).expect("export should exist"))
                .expect("export should be JSON");

        assert_eq!(result.record_count, 1);
        assert_eq!(envelope["format"], "cm-modpack-util-data");
        assert_eq!(envelope["version"], 1);
        assert_eq!(
            envelope["payload"]["records"]["changelog_cache"],
            Value::Null
        );
        assert_eq!(
            envelope["payload"]["records"]
                .as_object()
                .expect("record families should be an object")
                .len(),
            30
        );
        assert!(envelope["payload"]["excluded"]
            .as_array()
            .expect("excluded data should be listed")
            .iter()
            .any(|value| value == "changelog_cache"));
        assert_eq!(
            envelope["payload"]["settings"][0]["identity"],
            "setting:appearance.theme"
        );
        let repeat_path = std::env::temp_dir().join("cm-modpack-util-phase2-export-repeat.json");
        let repeat = export_application_data_inner(
            &connection,
            repeat_path
                .to_str()
                .expect("temporary path should be valid"),
            DataTransferCompression::Json,
        )
        .expect("repeat export should succeed");
        assert_eq!(result.payload_sha256, repeat.payload_sha256);
        fs::remove_file(path).expect("temporary export should be removed");
        fs::remove_file(repeat_path).expect("repeat export should be removed");
    }

    #[test]
    fn exports_gzip_with_gzip_magic_bytes() {
        let database = initialize(Path::new(":memory:")).expect("database should initialize");
        let connection = database
            .0
            .lock()
            .expect("database lock should be available");
        let path = std::env::temp_dir().join("cm-modpack-util-phase2-export.json.gz");
        export_application_data_inner(
            &connection,
            path.to_str().expect("temporary path should be valid"),
            DataTransferCompression::Gzip,
        )
        .expect("gzip export should succeed");
        let bytes = fs::read(&path).expect("gzip export should exist");
        assert_eq!(&bytes[..2], &[0x1f, 0x8b]);
        let mut decoder = GzDecoder::new(bytes.as_slice());
        let mut decompressed = Vec::new();
        decoder
            .read_to_end(&mut decompressed)
            .expect("gzip should decompress");
        let envelope: Value =
            serde_json::from_slice(&decompressed).expect("decompressed bundle should be JSON");
        assert_eq!(envelope["format"], "cm-modpack-util-data");
        fs::remove_file(path).expect("temporary export should be removed");
    }

    #[test]
    fn previews_and_imports_selected_records_without_replacing_existing_state() {
        let source_db =
            initialize(Path::new(":memory:")).expect("source database should initialize");
        let source_connection = source_db.0.lock().expect("source lock should be available");
        source_connection
            .execute(
                "INSERT INTO settings (key, value_json, updated_at) VALUES (?1, ?2, ?3)",
                params!["appearance.theme", "\"dark\"", "1"],
            )
            .expect("setting should be inserted");
        let path = std::env::temp_dir().join("cm-modpack-util-phase3-roundtrip.json");
        export_application_data_inner(
            &source_connection,
            path.to_str().expect("temporary path should be valid"),
            DataTransferCompression::Json,
        )
        .expect("export should succeed");
        drop(source_connection);

        let target_db =
            initialize(Path::new(":memory:")).expect("target database should initialize");
        let target_connection = target_db.0.lock().expect("target lock should be available");
        let preview = preview_application_data_inner(&target_connection, &path)
            .expect("preview should succeed");
        assert_eq!(preview.record_count, 1);
        assert_eq!(preview.safe_record_count, 1);
        let result = import_application_data_inner(
            &target_connection,
            &DataTransferImportRequest {
                source: path.to_string_lossy().into_owned(),
                preview_fingerprint: preview.fingerprint,
                selected: vec!["setting:appearance.theme".to_string()],
                cancelled: false,
            },
        )
        .expect("import should succeed");
        assert_eq!(result.imported, 1);
        assert_eq!(
            target_connection
                .query_row(
                    "SELECT value_json FROM settings WHERE key = 'appearance.theme'",
                    [],
                    |row| row.get::<_, String>(0)
                )
                .expect("imported setting should exist"),
            "\"dark\""
        );
        fs::remove_file(path).expect("temporary import should be removed");
    }

    #[test]
    fn imports_selected_records_in_foreign_key_order() {
        let source_db =
            initialize(Path::new(":memory:")).expect("source database should initialize");
        let source_connection = source_db.0.lock().expect("source lock should be available");
        source_connection
            .execute(
                "INSERT INTO modpacks (id, canonical_path, application_json, packwiz_json, validation_json, created_at, updated_at) VALUES ('modpack', 'C:\\\\missing-modpack', '{}', '{}', '{}', '1', '1')",
                [],
            )
            .expect("modpack should be inserted");
        source_connection
            .execute(
                "INSERT INTO snapshots (id, modpack_id, lifecycle, outcome, result_json, created_at, updated_at) VALUES ('snapshot', 'modpack', 'open', 'complete', '{}', '1', '1')",
                [],
            )
            .expect("snapshot should be inserted");
        source_connection
            .execute(
                "INSERT INTO snapshot_candidates (id, snapshot_id, candidate_json, observed_at) VALUES ('snapshot-candidate', 'snapshot', '{}', '1')",
                [],
            )
            .expect("snapshot candidate should be inserted");
        source_connection
            .execute(
                "INSERT INTO release_workspaces (id, modpack_id, baseline_origin, name, lifecycle, evidence_status, publication_status, created_at, updated_at) VALUES ('workspace', 'modpack', 'current_project', 'Workspace', 'draft', 'available', 'unpublished', '1', '1')",
                [],
            )
            .expect("release workspace should be inserted");
        source_connection
            .execute(
                "INSERT INTO release_workspace_candidate_sources (id, workspace_id, source_snapshot_candidate_id, candidate_json, observed_at) VALUES ('source', 'workspace', 'snapshot-candidate', '{}', '1')",
                [],
            )
            .expect("candidate source should be inserted");
        source_connection
            .execute(
                "INSERT INTO snapshot_decisions (snapshot_id, candidate_id, decision, recorded_at) VALUES ('snapshot', 'source', 'accepted', '1')",
                [],
            )
            .expect("snapshot decision should be inserted");
        let path = std::env::temp_dir().join("cm-modpack-util-import-order.json");
        export_application_data_inner(
            &source_connection,
            path.to_str().expect("temporary path should be valid"),
            DataTransferCompression::Json,
        )
        .expect("export should succeed");
        drop(source_connection);

        let target_db =
            initialize(Path::new(":memory:")).expect("target database should initialize");
        let target_connection = target_db.0.lock().expect("target lock should be available");
        let preview = preview_application_data_inner(&target_connection, &path)
            .expect("preview should succeed");
        let selected = preview
            .records
            .iter()
            .map(|record| record.identity.clone())
            .collect();
        let result = import_application_data_inner(
            &target_connection,
            &DataTransferImportRequest {
                source: path.to_string_lossy().into_owned(),
                preview_fingerprint: preview.fingerprint,
                selected,
                cancelled: false,
            },
        )
        .expect("import should satisfy foreign keys");
        assert_eq!(result.imported, 6);
        assert_eq!(
            target_connection
                .query_row("SELECT COUNT(*) FROM snapshot_decisions", [], |row| {
                    row.get::<_, i64>(0)
                })
                .expect("snapshot decision should exist"),
            1
        );
        fs::remove_file(path).expect("temporary import should be removed");
    }

    #[test]
    fn rejects_tampered_bundle_before_database_access() {
        let path = std::env::temp_dir().join("cm-modpack-util-phase3-invalid.json");
        fs::write(&path, br#"{"format":"cm-modpack-util-data","version":1}"#)
            .expect("invalid bundle should be written");
        let error = read_envelope(&path).expect_err("invalid bundle should fail");
        assert_eq!(error.code, "invalid_bundle");
        fs::remove_file(path).expect("temporary invalid bundle should be removed");
    }
}
