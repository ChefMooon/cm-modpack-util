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
