<script lang="ts">
  import XIcon from "phosphor-svelte/lib/XIcon";
  import Button from "./Button.svelte";
  import Tooltip from "./Tooltip.svelte";
  import type { Snippet } from "svelte";

  let { open = $bindable(false), title, size = "compact", children, onclose }: { open?: boolean; title: string; size?: "compact" | "wide"; children: Snippet; onclose?: () => void } = $props();
  let panel = $state<HTMLDivElement | undefined>(undefined);
  let returnFocus: HTMLElement | null = null;

  function close() {
    open = false;
    onclose?.();
  }

  $effect(() => {
    const previousOverflow = document.body.style.overflow;
    const wasOpen = open;
    if (open) {
      returnFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
      document.body.style.overflow = "hidden";
      queueMicrotask(() => panel?.querySelector<HTMLElement>("button, [href], input")?.focus());
    }
    return () => {
      document.body.style.overflow = previousOverflow;
      if (wasOpen && returnFocus) {
        returnFocus.focus();
        returnFocus = null;
      }
    };
  });

</script>

{#if open}
  <div class="modal-backdrop" role="presentation" onclick={(event) => event.target === event.currentTarget && close()}>
    <div class:wide={size === "wide"} class="modal-panel" bind:this={panel} role="dialog" aria-modal="true" aria-labelledby="dialog-title" tabindex="-1">
      <header><h2 id="dialog-title">{title}</h2><Tooltip text="Close"><Button class="close" variant="ghost" size="icon" type="button" aria-label="Close dialog" onclick={close}><XIcon size={18} weight="bold" aria-hidden="true" /></Button></Tooltip></header>
      <div class="content">{@render children()}</div>
    </div>
  </div>
{/if}

<style>
  .modal-backdrop { position:fixed; z-index:1000; inset:0; display:grid; place-items:center; padding:20px; background:#0008; }
  .modal-panel { width:min(440px, calc(100vw - 40px)); max-height:calc(100vh - 40px); overflow:hidden; padding:22px; border:1px solid var(--color-border); border-radius:var(--radius-md); color:var(--color-text); background:var(--color-surface); box-shadow:0 20px 60px #0008; }
  .modal-panel.wide { width:min(1080px, calc(100vw - 40px)); display:flex; min-height:0; flex-direction:column; }
  header { display:flex; align-items:center; justify-content:space-between; gap:16px; } h2 { margin:0; font-size:18px; }.content { margin-top:18px; color:var(--color-text-muted); font-size:14px; line-height:1.5; }.wide .content { min-height:0; max-height:calc(100vh - 128px); overflow-y:auto; padding-right:6px; }:global(.close) { margin:-8px -8px 0 0; }
  @media (max-width:700px) { .modal-panel.wide { width:min(1080px, calc(100vw - 24px)); } .modal-panel { padding:18px; } .wide .content { max-height:calc(100vh - 112px); } }
</style>
