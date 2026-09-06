<script lang="ts">
  import type { Snippet } from "svelte";
  import type { HTMLButtonAttributes } from "svelte/elements";

  type ButtonVariant = "primary" | "secondary" | "outline" | "danger" | "ghost" | "quiet" | "info" | "success" | "warning";
  type ButtonSize = "sm" | "md" | "lg" | "icon";

  type Props = Omit<HTMLButtonAttributes, "class"> & {
    variant?: ButtonVariant;
    size?: ButtonSize;
    loading?: boolean;
    fullWidth?: boolean;
    children?: Snippet;
    class?: string;
  };

  let {
    variant = "primary",
    size = "md",
    loading = false,
    fullWidth = false,
    children,
    class: className = "",
    type = "button",
    disabled = false,
    ...rest
  }: Props = $props();

  const classes = $derived(["button", variant, size, fullWidth && "full-width", className].filter(Boolean).join(" "));
</script>

<button {...rest} {type} class={classes} disabled={disabled || loading} aria-busy={loading ? "true" : undefined}>
  {@render children?.()}
</button>

<style>
  .button { display: inline-flex; align-items: center; justify-content: center; gap: var(--space-2); min-height: 42px; padding: 0 16px; border: 1px solid transparent; border-radius: var(--radius-sm); color: var(--color-on-accent); background: var(--color-accent); font: inherit; font-size: 13px; font-weight: 700; line-height: 1.2; cursor: pointer; transition: filter .15s, background-color .15s, border-color .15s, color .15s; }
  .button:hover:not(:disabled) { filter: brightness(1.08); }
  .button:active:not(:disabled) { filter: brightness(.96); }
  .button:focus-visible { outline: none; box-shadow: var(--focus-ring); }
  .button:disabled { cursor: not-allowed; opacity: .5; }
  .button.sm { min-height: 34px; padding: 0 12px; font-size: 12px; }.button.lg { min-height: 48px; padding: 0 20px; }.button.icon { width: 36px; min-height: 36px; padding: 0; }
  .button.full-width { width: 100%; }
  .button.primary { color: var(--color-on-accent); background: var(--color-accent); }
  .button.secondary { border-color: var(--color-border); color: var(--color-text); background: var(--color-surface-raised); }
  .button.outline { border-color: var(--color-border); color: var(--color-text); background: transparent; }
  .button.danger { border-color: var(--color-danger); color: var(--color-danger); background: transparent; }
  .button.ghost { color: var(--color-text-muted); background: transparent; }.button.ghost:hover:not(:disabled) { color: var(--color-text); background: var(--color-surface-raised); }
  .button.quiet { min-height: 30px; padding: 0 4px; color: var(--color-accent-strong); background: transparent; }.button.quiet:hover:not(:disabled) { filter: none; text-decoration: underline; }
  .button.info { color: var(--color-info); background: color-mix(in srgb, var(--color-info) 15%, var(--color-surface)); }.button.success { color: var(--color-success); background: color-mix(in srgb, var(--color-success) 15%, var(--color-surface)); }.button.warning { color: var(--color-warning); background: color-mix(in srgb, var(--color-warning) 15%, var(--color-surface)); }
  @media (prefers-reduced-motion: reduce) { .button { transition: none; } }
</style>
