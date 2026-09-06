import type { Toast, ToastOptions } from "./types";

const DEFAULT_DURATION = 5000;
const MAX_VISIBLE = 4;

export const toastState = $state<{ visible: Toast[]; queued: Toast[] }>({
  visible: [],
  queued: [],
});

const timers = new Map<string, ReturnType<typeof setTimeout>>();
const remaining = new Map<string, number>();
const startedAt = new Map<string, number>();
const pausedIds = new Set<string>();
let nextId = 0;

function createId() {
  nextId += 1;
  return `toast-${Date.now()}-${nextId}`;
}

function clearTimer(id: string) {
  const timer = timers.get(id);
  if (timer !== undefined) clearTimeout(timer);
  timers.delete(id);
  startedAt.delete(id);
}

function schedule(id: string, duration: number) {
  clearTimer(id);
  if (duration <= 0) {
    remaining.delete(id);
    return;
  }

  const delay = remaining.get(id) ?? duration;
  remaining.set(id, delay);
  if (pausedIds.has(id)) return;
  startedAt.set(id, Date.now());
  timers.set(id, setTimeout(() => dismiss(id), delay));
}

function isVisible(id: string) {
  return toastState.visible.some((item) => item.id === id);
}

function promoteQueued() {
  while (toastState.visible.length < MAX_VISIBLE && toastState.queued.length > 0) {
    const next = toastState.queued.shift();
    if (!next) break;
    toastState.visible.push(next);
    schedule(next.id, next.duration);
  }
}

export function toast(options: ToastOptions): string {
  const item: Toast = {
    ...options,
    id: createId(),
    severity: options.severity ?? "info",
    duration: options.duration ?? DEFAULT_DURATION,
  };

  if (toastState.visible.length < MAX_VISIBLE) {
    toastState.visible.push(item);
    schedule(item.id, item.duration);
  } else {
    toastState.queued.push(item);
  }

  return item.id;
}

export function update(id: string, changes: Partial<ToastOptions>) {
  const item = [...toastState.visible, ...toastState.queued].find((candidate) => candidate.id === id);
  if (!item) return;

  const previousDuration = item.duration;
  Object.assign(item, changes);
  item.severity = item.severity ?? "info";
  item.duration = item.duration ?? DEFAULT_DURATION;

  if (isVisible(id) && (changes.duration !== undefined || previousDuration !== item.duration)) {
    remaining.delete(id);
    schedule(id, item.duration);
  }
}

export function dismiss(id: string) {
  clearTimer(id);
  remaining.delete(id);
  pausedIds.delete(id);
  const visibleIndex = toastState.visible.findIndex((item) => item.id === id);
  if (visibleIndex >= 0) toastState.visible.splice(visibleIndex, 1);

  const queuedIndex = toastState.queued.findIndex((item) => item.id === id);
  if (queuedIndex >= 0) toastState.queued.splice(queuedIndex, 1);
  promoteQueued();
}

export function dismissAll() {
  for (const item of [...toastState.visible, ...toastState.queued]) clearTimer(item.id);
  toastState.visible.splice(0);
  toastState.queued.splice(0);
  remaining.clear();
  pausedIds.clear();
}

export function pause(id: string) {
  if (!isVisible(id)) return;
  pausedIds.add(id);
  if (!timers.has(id)) return;
  const elapsed = Date.now() - (startedAt.get(id) ?? Date.now());
  const current = remaining.get(id) ?? 0;
  clearTimer(id);
  remaining.set(id, Math.max(0, current - elapsed));
}

export function resume(id: string) {
  pausedIds.delete(id);
  if (!isVisible(id) || timers.has(id)) return;
  const item = toastState.visible.find((candidate) => candidate.id === id);
  if (item) schedule(id, item.duration);
}

export function useToast() {
  return { toast, update, dismiss, dismissAll };
}
