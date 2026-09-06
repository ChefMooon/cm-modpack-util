export type ToastSeverity = "info" | "success" | "warning" | "error";

export type ToastAction = {
  label: string;
  onclick: () => void | Promise<void>;
  dismiss?: boolean;
};

export type ToastOptions = {
  title: string;
  description?: string;
  severity?: ToastSeverity;
  duration?: number;
  action?: ToastAction;
};

export type Toast = ToastOptions & {
  id: string;
  severity: ToastSeverity;
  duration: number;
};

export type ToastStackDirection = "below" | "above";
