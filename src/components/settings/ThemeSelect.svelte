<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import DesktopIcon from "phosphor-svelte/lib/DesktopIcon";
  import MoonIcon from "phosphor-svelte/lib/MoonIcon";
  import SunIcon from "phosphor-svelte/lib/SunIcon";
  import Button from "../../components/ui/Button.svelte";

  type Theme = "system" | "light" | "dark";
  let { value = $bindable<Theme>("system"), error = $bindable("") } = $props<{ value?: Theme; error?: string }>();
  let pending = $state(false);

  async function change(next: Theme) {
    const previous = value;
    value = next;
    document.documentElement.dataset.theme = next;
    pending = true;
    error = "";
    try {
      await invoke("set_setting", { key: "appearance.theme", valueJson: JSON.stringify(next) });
    } catch (cause) {
      value = previous;
      document.documentElement.dataset.theme = previous;
      error = `Theme could not be saved: ${String(cause)}`;
    } finally {
      pending = false;
    }
  }
</script>

<div class="theme-options" aria-label="Theme selection">
  {#each [
    { id: "system", label: "System", icon: DesktopIcon, hint: "Follow the desktop theme" },
    { id: "light", label: "Light", icon: SunIcon, hint: "Use a light theme" },
    { id: "dark", label: "Dark", icon: MoonIcon, hint: "Use a dark theme" }
  ] as option}
    <Button class="theme-option" variant="outline" size="lg" type="button" disabled={pending} onclick={() => change(option.id as Theme)} aria-pressed={value === option.id}>
      <span class="theme-icon" aria-hidden="true"><option.icon size={22} weight="regular" /></span>
      <span><strong>{option.label}</strong><small>{option.hint}</small></span>
    </Button>
  {/each}
</div>
{#if error}<p class="error" role="alert">{error}</p>{/if}

<style>
  .theme-options { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: var(--space-3); }
  :global(.theme-option) { justify-content: flex-start; min-height: 68px; padding: var(--space-3); color: var(--color-text); text-align: left; }
  :global(.theme-option:hover), :global(.theme-option[aria-pressed="true"]) { border-color: var(--color-accent-strong); }
  :global(.theme-option[aria-pressed="true"]) { background: color-mix(in srgb, var(--color-accent) 12%, var(--color-surface)); }
  :global(.theme-option:disabled) { cursor: wait; opacity: .65; }
  .theme-icon { display: inline-flex; color: var(--color-accent-strong); }
  strong, small { display: block; }
  small { margin-top: 3px; color: var(--color-text-muted); font-size: 11px; }
  .error { color: var(--color-danger); font-size: 12px; }
  @media (max-width: 620px) { .theme-options { grid-template-columns: 1fr; } }
</style>
