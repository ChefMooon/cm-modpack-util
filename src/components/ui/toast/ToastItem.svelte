<script lang="ts">
  import CheckCircleIcon from "phosphor-svelte/lib/CheckCircleIcon";
  import InfoIcon from "phosphor-svelte/lib/InfoIcon";
  import WarningIcon from "phosphor-svelte/lib/WarningIcon";
  import XCircleIcon from "phosphor-svelte/lib/XCircleIcon";
  import XIcon from "phosphor-svelte/lib/XIcon";
  import Button from "../Button.svelte";
  import Tooltip from "../Tooltip.svelte";
  import { dismiss, pause, resume } from "./toast.svelte";
  import type { Toast } from "./types";

  let { item }: { item: Toast } = $props();
  let hovered = false;
  let focused = false;

  function syncTimer() {
    if (hovered || focused) pause(item.id);
    else resume(item.id);
  }

  function handleFocusOut(event: FocusEvent) {
    const next = event.relatedTarget;
    if (!(next instanceof Node) || !(event.currentTarget instanceof HTMLElement) || !event.currentTarget.contains(next)) {
      focused = false;
      syncTimer();
    }
  }

  function handleAction() {
    if (!item.action) return;
    try {
      const result = item.action.onclick();
      if (result instanceof Promise) {
        void result.catch(() => undefined).finally(() => {
          if (item.action?.dismiss !== false) dismiss(item.id);
        });
      } else if (item.action.dismiss !== false) {
        dismiss(item.id);
      }
    } catch {
      if (item.action.dismiss !== false) dismiss(item.id);
    }
  }
</script>

<article
  class="toast {item.severity}"
  role={item.severity === "error" ? "alert" : "status"}
  tabindex="-1"
  onmouseenter={() => { hovered = true; syncTimer(); }}
  onmouseleave={() => { hovered = false; syncTimer(); }}
  onfocusin={() => { focused = true; syncTimer(); }}
  onfocusout={handleFocusOut}
>
  <div class="icon" aria-hidden="true">
    {#if item.severity === "success"}<CheckCircleIcon size={21} weight="fill" />
    {:else if item.severity === "warning"}<WarningIcon size={21} weight="fill" />
    {:else if item.severity === "error"}<XCircleIcon size={21} weight="fill" />
    {:else}<InfoIcon size={21} weight="fill" />{/if}
  </div>
  <div class="copy">
    <strong>{item.title}</strong>
    {#if item.description}<p>{item.description}</p>{/if}
    {#if item.action}<Button class="toast-action" variant="quiet" size="sm" type="button" onclick={handleAction}>{item.action.label}</Button>{/if}
  </div>
  <Tooltip text="Dismiss"><Button class="toast-dismiss" variant="ghost" size="icon" type="button" aria-label="Dismiss notification" onclick={() => dismiss(item.id)}>
    <XIcon size={17} weight="bold" aria-hidden="true" />
  </Button></Tooltip>
</article>

<style>
  .toast { display: flex; align-items: flex-start; gap: 11px; padding: 14px; border: 1px solid var(--toast-border); border-radius: var(--radius-md); color: var(--color-text); background: var(--color-surface); box-shadow: 0 10px 30px #0006; }
  .toast.info { --toast-accent: var(--color-info); --toast-border: color-mix(in srgb, var(--color-info) 48%, var(--color-border)); }
  .toast.success { --toast-accent: var(--color-success); --toast-border: color-mix(in srgb, var(--color-success) 48%, var(--color-border)); }
  .toast.warning { --toast-accent: var(--color-warning); --toast-border: color-mix(in srgb, var(--color-warning) 48%, var(--color-border)); }
  .toast.error { --toast-accent: var(--color-danger); --toast-border: color-mix(in srgb, var(--color-danger) 48%, var(--color-border)); }
  .icon { flex: 0 0 auto; color: var(--toast-accent); }
  .copy { min-width: 0; flex: 1; }.copy strong { display: block; font-size: 13px; line-height: 1.35; }.copy p { margin: 4px 0 0; color: var(--color-text-muted); font-size: 12px; line-height: 1.45; }
  :global(.toast-action) { align-self: flex-start; margin-top: 9px; min-height: 30px; padding: 0 4px; color: var(--toast-accent); }:global(.toast-action:hover) { color: var(--toast-accent); }
  :global(.toast-dismiss) { flex: 0 0 auto; margin: -3px -5px 0 0; }
  @media (prefers-reduced-motion: no-preference) { .toast { animation: toast-in .18s ease-out; } }
  @keyframes toast-in { from { opacity: 0; transform: translateY(8px); } to { opacity: 1; transform: translateY(0); } }
</style>
