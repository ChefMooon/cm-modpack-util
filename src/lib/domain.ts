export type OperationStatus =
  | "pending"
  | "running"
  | "succeeded"
  | "failed"
  | "cancelled"
  | "unavailable";

export type ValidationSeverity = "info" | "warning" | "error";

export type ProjectReference = {
  id: string;
  name: string;
  rootPath: string;
};

export type ValidationResult = {
  valid: boolean;
  severity: ValidationSeverity;
  code: string;
  message: string;
};

export type CommandError = {
  code: string;
  message: string;
  details?: string;
};

export type Observation<T> = { observed: T } | "unavailable";

export type ProjectLifecycle = "active" | "maintenance" | "archived" | "disconnected";

export type ApplicationProjectMetadata = {
  display_name: string;
  icon: string | null;
  theme: string | null;
  tags: string[];
  favorite: boolean;
  description: string | null;
  lifecycle: ProjectLifecycle;
};

export type PackwizObservations = {
  name: Observation<string>;
  author: Observation<string>;
  version: Observation<string>;
  pack_format: Observation<string>;
  index_file: Observation<string>;
  index_hash_format: Observation<string>;
  index_hash: Observation<string>;
  declared_versions: [string, string][];
};

export type RegistrationPreview = {
  canonical_path: string;
  application_defaults: ApplicationProjectMetadata;
  packwiz: PackwizObservations;
  validation: ValidationResult[];
};

export type ProjectRecord = {
  id: string;
  canonical_path: string;
  application: ApplicationProjectMetadata;
  packwiz: PackwizObservations;
  validation: ValidationResult[];
  created_at: string;
  updated_at: string;
  last_opened_at: string | null;
  last_refreshed_at: string | null;
};
