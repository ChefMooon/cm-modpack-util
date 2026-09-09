<script lang="ts">
  import CheckIcon from "phosphor-svelte/lib/CheckIcon";
  import PaintBrushIcon from "phosphor-svelte/lib/PaintBrushIcon";
  import {
    MODPACK_DEFAULT_THEME,
    MODPACK_PALETTE,
    getModpackTheme,
    isValidCustomModpackTheme,
    normalizeModpackTheme,
  } from "../../lib/modpackTheme";

  let {
    value = $bindable<string | null>(MODPACK_DEFAULT_THEME),
    label = "Theme / color",
    description = "Choose the identity color shown for this modpack.",
    error = $bindable(""),
  } = $props<{
    value?: string | null;
    label?: string;
    description?: string;
    error?: string;
  }>();

  let customValue = $state("");
  let menuOpen = $state(false);
  const currentTheme = $derived(getModpackTheme(value));
  const currentValue = $derived(normalizeModpackTheme(value));

  function choose(next: string) {
    value = next;
    customValue = "";
    error = "";
    menuOpen = false;
  }

  function openCustom() {
    customValue = currentTheme.custom ? currentTheme.value : "#";
    error = "";
    menuOpen = true;
  }

  function applyCustom() {
    const next = customValue.trim().toLowerCase();
    if (!isValidCustomModpackTheme(next)) {
      error = "Use a six-digit hex color such as #2e6f54.";
      return;
    }
    choose(next);
  }
</script>

<div class="theme-select">
  <div class="theme-label-row">
    <span class="theme-label">{label}</span>
    <span class="theme-current" aria-live="polite">
      <span class="swatch" style={`--swatch: ${currentTheme.color}; --swatch-foreground: ${currentTheme.foreground}`} aria-hidden="true"></span>
      {currentTheme.label}
    </span>
  </div>
  <details bind:open={menuOpen} class="theme-menu">
    <summary class="theme-trigger" aria-label={`${label}: ${currentTheme.label}`}>
      <span class="swatch large" style={`--swatch: ${currentTheme.color}; --swatch-foreground: ${currentTheme.foreground}`} aria-hidden="true"></span>
      <span>{currentTheme.label}</span>
      <span class="trigger-caret" aria-hidden="true">▾</span>
    </summary>
    <div class="theme-options" aria-label="Modpack color choices">
      {#each MODPACK_PALETTE as option}
        <button class:selected={currentValue === option.key} class="theme-option" type="button" aria-pressed={currentValue === option.key} onclick={() => choose(option.key)}>
          <span class="swatch" style={`--swatch: ${option.color}; --swatch-foreground: ${option.foreground}`} aria-hidden="true"></span>
          <span>{option.label}</span>
          {#if currentValue === option.key}<CheckIcon size={15} aria-hidden="true" />{/if}
        </button>
      {/each}
      <button class:selected={currentTheme.custom} class="theme-option" type="button" aria-pressed={currentTheme.custom} onclick={openCustom}>
        <PaintBrushIcon size={16} aria-hidden="true" />
        <span>Custom color</span>
        {#if currentTheme.custom}<CheckIcon size={15} aria-hidden="true" />{/if}
      </button>
      {#if menuOpen && (currentTheme.custom || customValue)}
        <div class="custom-editor">
          <label for="custom-modpack-color">Hex color</label>
          <div class="custom-input-row">
            <input id="custom-modpack-color" type="color" value={isValidCustomModpackTheme(customValue) ? customValue : "#007f88"} oninput={(event) => (customValue = event.currentTarget.value)} />
            <input type="text" inputmode="text" maxlength="7" placeholder="#2e6f54" value={customValue} oninput={(event) => (customValue = event.currentTarget.value)} aria-describedby={error ? "custom-modpack-color-error" : undefined} />
            <button type="button" class="apply-custom" onclick={applyCustom}>Apply</button>
          </div>
          {#if error}<p id="custom-modpack-color-error" class="error" role="alert">{error}</p>{/if}
        </div>
      {/if}
    </div>
  </details>
  <p class="theme-description">{description}</p>
</div>

<style>
  .theme-select { display: grid; gap: 7px; min-width: 0; align-content: start; }
  .theme-label-row { display: flex; align-items: center; justify-content: space-between; gap: 10px; height: 18px; line-height: 1; }
  .theme-label { color: var(--color-text); font-size: 12px; }
  .theme-current { display: inline-flex; align-items: center; gap: 6px; height: 18px; color: var(--color-text-muted); font: 11px var(--font-mono); line-height: 1; }
  .theme-description { margin: 0; color: var(--color-text-muted); font-size: 11px; line-height: 1.4; }
  .theme-menu { position: relative; min-width: 0; }
  .theme-trigger { display: flex; align-items: center; gap: 9px; width: 100%; height: 40px; min-height: 40px; padding: 0 10px; border: 1px solid var(--color-border); border-radius: var(--radius-sm); color: var(--color-text); background: var(--color-bg); cursor: pointer; list-style: none; }
  .theme-trigger::-webkit-details-marker { display: none; }
  .trigger-caret { margin-left: auto; color: var(--color-text-muted); }
  .theme-options { position: absolute; z-index: 20; top: calc(100% + 4px); left: 0; right: 0; display: grid; gap: 3px; min-width: 220px; padding: 6px; border: 1px solid var(--color-border); border-radius: var(--radius-sm); background: var(--color-surface); box-shadow: 0 8px 20px rgb(0 0 0 / 18%); }
  .theme-option { display: flex; align-items: center; gap: 9px; min-height: 34px; padding: 5px 7px; border: 1px solid transparent; color: var(--color-text); background: transparent; text-align: left; cursor: pointer; }
  .theme-option:hover, .theme-option.selected { border-color: var(--color-accent-strong); background: color-mix(in srgb, var(--color-accent) 10%, var(--color-surface)); }
  .theme-option :global(svg) { margin-left: auto; color: var(--color-accent-strong); }
  .swatch { display: inline-block; flex: 0 0 auto; width: 14px; height: 14px; border: 1px solid color-mix(in srgb, var(--swatch) 65%, var(--color-text)); border-radius: 50%; background: var(--swatch); }
  .swatch.large { width: 20px; height: 20px; }
  .custom-editor { display: grid; gap: 6px; padding: 8px; border-top: 1px solid var(--color-border); color: var(--color-text-muted); font-size: 11px; }
  .custom-input-row { display: grid; grid-template-columns: 34px minmax(0, 1fr) auto; gap: 6px; }
  .custom-input-row input[type="color"] { width: 34px; height: 32px; padding: 2px; border: 1px solid var(--color-border); background: var(--color-bg); }
  .custom-input-row input[type="text"] { min-width: 0; padding: 0 7px; border: 1px solid var(--color-border); color: var(--color-text); background: var(--color-bg); font: 11px var(--font-mono); }
  .apply-custom { padding: 0 8px; border: 1px solid var(--color-border); color: var(--color-text); background: var(--color-surface-raised); cursor: pointer; }
  .error { margin: 0; color: var(--color-danger); line-height: 1.4; }
  :global(.theme-trigger:focus-visible), .theme-option:focus-visible, .apply-custom:focus-visible, .custom-input-row input:focus-visible { outline: none; box-shadow: var(--focus-ring); }
</style>
