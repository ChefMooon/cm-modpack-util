<script lang="ts">
  import ToastItem from "./ToastItem.svelte";
  import { toastState } from "./toast.svelte";
  import type { ToastStackDirection } from "./types";

  let { stackDirection = "below" }: { stackDirection?: ToastStackDirection } = $props();
</script>

<section
  class="toast-viewport {stackDirection}"
  aria-label="Notifications"
  aria-live="polite"
  aria-relevant="additions"
>
  {#each toastState.visible as item (item.id)}
    <ToastItem {item} />
  {/each}
</section>

<style>
  .toast-viewport { position: fixed; right: 20px; bottom: 20px; z-index: 1000; display: flex; width: min(380px, calc(100vw - 32px)); gap: 10px; pointer-events: none; }
  .toast-viewport.below { flex-direction: column; }
  .toast-viewport.above { flex-direction: column-reverse; }
  .toast-viewport :global(.toast) { pointer-events: auto; }
  @media (max-width: 600px) { .toast-viewport { right: 16px; bottom: 16px; width: calc(100vw - 32px); } }
</style>
