import type { CommandError } from "./domain";

export function translateCommandError(cause: unknown): CommandError {
  if (typeof cause === "object" && cause !== null) {
    const candidate = cause as Partial<CommandError>;
    if (typeof candidate.code === "string" && typeof candidate.message === "string") {
      return {
        code: candidate.code,
        message: candidate.message,
        ...(typeof candidate.details === "string" ? { details: candidate.details } : {})
      };
    }
  }

  return {
    code: "unknown_error",
    message: cause instanceof Error ? cause.message : String(cause)
  };
}

export function commandErrorMessage(cause: unknown): string {
  const error = translateCommandError(cause);
  return error.details ? `${error.message} (${error.details})` : error.message;
}
