import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";

export type DataTransferCompression = "json" | "gzip";

export type DataTransferExportResult = {
  destination: string;
  compression: DataTransferCompression;
  format: string;
  version: number;
  payload_sha256: string;
  record_count: number;
  excluded: string[];
};

export type DataTransferRecordStatus =
  | "addition"
  | "identical"
  | "conflict"
  | "unavailable"
  | "dependent_skip";

export type DataTransferRecordPreview = {
  family: string;
  identity: string;
  status: DataTransferRecordStatus;
  reason: string | null;
};

export type DataTransferPreview = {
  fingerprint: string;
  format: string;
  version: number;
  record_count: number;
  records: DataTransferRecordPreview[];
  excluded: string[];
  safe_record_count: number;
};

export type DataTransferImportRequest = {
  source: string;
  preview_fingerprint: string;
  selected: string[];
  cancelled?: boolean;
};

export type DataTransferImportResult = {
  imported: number;
  skipped: number;
  fingerprint: string;
};

export async function chooseApplicationDataDestination(compression: DataTransferCompression): Promise<string | null> {
  return save({
    title: "Export application data",
    defaultPath: compression === "gzip" ? "cm-modpack-util-data.json.gz" : "cm-modpack-util-data.json",
    filters: [{ name: compression === "gzip" ? "Gzip JSON" : "JSON", extensions: compression === "gzip" ? ["json", "gz"] : ["json"] }],
  });
}

export async function chooseApplicationDataSource(): Promise<string | null> {
  const selected = await open({
    title: "Import application data",
    multiple: false,
    directory: false,
    filters: [{ name: "Application data", extensions: ["json", "gz"] }],
  });
  return typeof selected === "string" ? selected : null;
}

export function exportApplicationData(
  destination: string,
  compression: DataTransferCompression,
): Promise<DataTransferExportResult> {
  return invoke("export_application_data", { destination, compression });
}

export function previewApplicationData(source: string): Promise<DataTransferPreview> {
  return invoke("preview_application_data", { source });
}

export function importApplicationData(
  request: DataTransferImportRequest,
): Promise<DataTransferImportResult> {
  return invoke("import_application_data", { request });
}
