<script lang="ts">
  import type { Snippet } from "svelte";

  type TooltipPosition = "top" | "bottom";
  type TooltipAlignment = "center" | "end";

  let {
    text,
    position = "top",
    alignment = "center",
    children,
  }: { text: string; position?: TooltipPosition; alignment?: TooltipAlignment; children: Snippet } = $props();
</script>

<span class="tooltip" class:bottom={position === "bottom"} class:end={alignment === "end"}>
  {@render children()}
  <span class="tooltip-content" role="tooltip">{text}</span>
</span>

<style>
  .tooltip { position: relative; display: inline-flex; }
  .tooltip-content { position: absolute; bottom: calc(100% + 8px); left: 50%; z-index: 1100; width: max-content; max-width: 240px; padding: 6px 9px; border: 1px solid var(--color-border); border-radius: var(--radius-sm); color: var(--color-text); background: var(--color-surface-raised); box-shadow: 0 6px 18px color-mix(in srgb, var(--color-bg) 55%, transparent); font: 400 11px var(--font-ui); line-height: 1.3; opacity: 0; pointer-events: none; transform: translate(-50%, 4px); transition: opacity .15s, transform .15s; }
  .tooltip-content::after { position: absolute; top: 100%; left: 50%; width: 7px; height: 7px; border-right: 1px solid var(--color-border); border-bottom: 1px solid var(--color-border); background: var(--color-surface-raised); content: ""; transform: translate(-50%, -4px) rotate(45deg); }
  .tooltip.end .tooltip-content { right: 0; left: auto; transform: translateY(4px); }
  .tooltip.end .tooltip-content::after { right: 10px; left: auto; transform: translateY(-4px) rotate(45deg); }
  .tooltip.bottom .tooltip-content { top: calc(100% + 8px); bottom: auto; transform: translate(-50%, -4px); }
  .tooltip.bottom .tooltip-content::after { top: auto; bottom: 100%; border-top: 1px solid var(--color-border); border-left: 1px solid var(--color-border); border-right: 0; border-bottom: 0; transform: translate(-50%, 4px) rotate(45deg); }
  .tooltip.bottom.end .tooltip-content { transform: translateY(-4px); }
  .tooltip.bottom.end .tooltip-content::after { transform: translateY(4px) rotate(45deg); }
  .tooltip:hover .tooltip-content, .tooltip:focus-within .tooltip-content { opacity: 1; pointer-events: auto; transform: translate(-50%, 0); }
  .tooltip.end:hover .tooltip-content, .tooltip.end:focus-within .tooltip-content { transform: translateY(0); }
  @media (prefers-reduced-motion: reduce) { .tooltip-content { transition: none; } }
</style>
