<script lang="ts">
  import DotsThreeIcon from "phosphor-svelte/lib/DotsThreeIcon";
  import type { Snippet } from "svelte";
  import Tooltip from "./Tooltip.svelte";

  let {
    label,
    inline = false,
    openAbove = false,
    children,
  }: {
    label: string;
    inline?: boolean;
    openAbove?: boolean;
    children?: Snippet;
  } = $props();
</script>

<details class:inline={inline} class:open-above={openAbove} class="quick-action-menu">
  <summary aria-label={label}>
    <Tooltip text={label} position="bottom">
      <DotsThreeIcon size={18} weight="bold" />
    </Tooltip>
  </summary>
  <div class="quick-action-menu-items">
    {@render children?.()}
  </div>
</details>

<style>
  .quick-action-menu { position:absolute; z-index:7; top:12px; right:12px; }
  .quick-action-menu.inline { position:relative; top:auto; right:auto; justify-self:end; z-index:2; }
  .quick-action-menu[open] { z-index:10; }
  .quick-action-menu summary { display:inline-flex; width:32px; min-height:32px; align-items:center; justify-content:center; gap:6px; padding:0; color:var(--color-text-muted); font-size:12px; font-weight:700; list-style:none; cursor:pointer; }
  .quick-action-menu summary::-webkit-details-marker { display:none; }
  .quick-action-menu summary:hover { color:var(--color-text); background:var(--color-surface-raised); }
  .quick-action-menu.inline summary { width:30px; height:30px; padding:0; }
  .quick-action-menu-items { position:absolute; z-index:8; right:0; top:calc(100% + 6px); display:grid; min-width:150px; padding:5px; border:1px solid var(--color-border); background:var(--color-surface); box-shadow:0 8px 20px color-mix(in srgb,var(--color-ink) 18%,transparent); }
  .quick-action-menu.open-above .quick-action-menu-items { top:auto; bottom:calc(100% + 6px); }
  .quick-action-menu-items :global(.button) { justify-content:flex-start; width:100%; }
  .quick-action-menu :global(.tooltip-content) { right:0; left:auto; transform:translate(0, -4px); }
  .quick-action-menu :global(.tooltip.bottom .tooltip-content::after) { right:5px; left:auto; }
  .quick-action-menu :global(.tooltip.bottom:hover .tooltip-content),
  .quick-action-menu :global(.tooltip.bottom:focus-within .tooltip-content) { transform:translate(0, 0); }
  @media (max-width:520px) {
    .quick-action-menu { top:8px; right:8px; }
    .quick-action-menu:not(.inline) summary { min-height:30px; padding:0 8px; font-size:11px; }
  }
</style>
