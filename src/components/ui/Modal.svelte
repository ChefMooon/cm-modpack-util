<script lang="ts">
  import XIcon from "phosphor-svelte/lib/XIcon";
  import Button from "./Button.svelte";
  import Tooltip from "./Tooltip.svelte";
  import type { Snippet } from "svelte";
  import { onDestroy } from "svelte";

  let { open = $bindable(false), title, size = "compact", children, onclose }: { open?: boolean; title: string; size?: "compact" | "wide"; children: Snippet; onclose?: () => void } = $props();
  let dialog: HTMLDialogElement;
  let returnFocus: HTMLElement | null = null;

  function close() {
    open = false;
    onclose?.();
  }

  $effect(() => {
    if (!dialog) return;
    const previousOverflow = document.body.style.overflow;
    const wasOpen = open;
    if (open) {
      if (!dialog.open) {
        returnFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
        dialog.showModal();
      }
      document.body.style.overflow = "hidden";
      queueMicrotask(() => dialog.querySelector<HTMLElement>("button, [href], input")?.focus());
    } else if (dialog.open) {
      dialog.close();
    }
    return () => {
      document.body.style.overflow = previousOverflow;
      if (wasOpen && returnFocus) {
        returnFocus.focus();
        returnFocus = null;
      }
    };
  });

  function handleCancel(event: Event) {
    event.preventDefault();
    close();
  }

  onDestroy(() => {
    if (dialog?.open) dialog.close();
  });
</script>

<dialog class:wide={size === "wide"} bind:this={dialog} aria-labelledby="dialog-title" oncancel={handleCancel} onclick={(event) => event.target === dialog && close()}>
  <form method="dialog" onsubmit={close}>
    <header><h2 id="dialog-title">{title}</h2><Tooltip text="Close"><Button class="close" variant="ghost" size="icon" type="submit" aria-label="Close dialog"><XIcon size={18} weight="bold" aria-hidden="true" /></Button></Tooltip></header>
    <div class="content">{@render children()}</div>
  </form>
</dialog>

<style>
  dialog { width: min(440px, calc(100vw - 40px)); max-height: calc(100vh - 40px); overflow:hidden; padding: 0; border: 1px solid var(--color-border); border-radius: var(--radius-md); color: var(--color-text); background: var(--color-surface); box-shadow: 0 20px 60px #0008; }
  dialog.wide { width: min(1080px, calc(100vw - 40px)); }
  dialog::backdrop { background: #0008; }
  form { max-height: inherit; padding: 22px; } header { display: flex; align-items: center; justify-content: space-between; gap: 16px; } h2 { margin: 0; font-size: 18px; }.content { margin-top: 18px; color: var(--color-text-muted); font-size: 14px; line-height: 1.5; }.wide form { display:flex; min-height:0; flex-direction:column; overflow:hidden; }.wide .content { min-height:0; max-height:calc(100vh - 128px); overflow-y:auto; padding-right:6px; }:global(.close) { margin: -8px -8px 0 0; }
  @media (max-width: 700px) { dialog.wide { width: min(1080px, calc(100vw - 24px)); } form { padding: 18px; } .wide .content { max-height: calc(100vh - 112px); } }
</style>
