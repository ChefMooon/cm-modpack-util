<script lang="ts">
  import ArchiveIcon from "phosphor-svelte/lib/ArchiveIcon";
  import ArrowClockwiseIcon from "phosphor-svelte/lib/ArrowClockwiseIcon";
  import CheckCircleIcon from "phosphor-svelte/lib/CheckCircleIcon";
  import DotsThreeIcon from "phosphor-svelte/lib/DotsThreeIcon";
  import LinkBreakIcon from "phosphor-svelte/lib/LinkBreakIcon";
  import LinkIcon from "phosphor-svelte/lib/LinkIcon";
  import PencilSimpleIcon from "phosphor-svelte/lib/PencilSimpleIcon";
  import TagIcon from "phosphor-svelte/lib/TagIcon";
  import WarningCircleIcon from "phosphor-svelte/lib/WarningCircleIcon";
  import Button from "../ui/Button.svelte";
  import Tooltip from "../ui/Tooltip.svelte";
  import type { ModpackRecord } from "../../lib/domain";
  import { getModpackTheme } from "../../lib/modpackTheme";

  type LifecycleAction = "archive" | "restore";
  let { modpacks, showArchived = $bindable(false), collapsed = false, focusedModpackId, busy, inspecting, discoveryBusy, discoveryModpackId, onfocus, oncheck, onreconnect, onrefresh, onedit, oncreaterelease, onlifecycle, observed, freshnessLabel, freshnessTitle, hasErrors }: {
    modpacks: ModpackRecord[];
    showArchived?: boolean;
    collapsed?: boolean;
    focusedModpackId: string | null;
    busy: boolean;
    inspecting: boolean;
    discoveryBusy: boolean;
    discoveryModpackId: string | null;
    onfocus: (modpack: ModpackRecord) => void;
    oncheck: (modpack: ModpackRecord) => void | Promise<void>;
    onreconnect: (modpack: ModpackRecord) => void | Promise<void>;
    onrefresh: (modpack: ModpackRecord) => void | Promise<void>;
    onedit: (modpack: ModpackRecord) => void | Promise<void>;
    oncreaterelease: (modpack: ModpackRecord) => void | Promise<void>;
    onlifecycle: (modpack: ModpackRecord, action: LifecycleAction) => void | Promise<void>;
    observed: (value: import("../../lib/domain").Observation<string>) => string;
    freshnessLabel: (value: string | null) => string;
    freshnessTitle: (value: string | null) => string;
    hasErrors: (results: import("../../lib/domain").ValidationResult[]) => boolean;
  } = $props();

  const visibleModpacks = $derived(showArchived ? modpacks : modpacks.filter((modpack) => modpack.application.lifecycle !== "archived"));

  const loaderLabel = (modpack: ModpackRecord) => {
    const loader = modpack.packwiz.declared_versions.find(([key]) => key !== "minecraft");
    return loader ? `Loader: ${loader[0]} ${loader[1]}` : "Loader unavailable";
  };
</script>

<section id="modpack-list" class:collapsed class="modpack-list" aria-label="Registered modpacks">
  <div class="list-heading"><p class="eyebrow">Registered modpacks</p><div class="list-controls"><span class="count">{visibleModpacks.length} {showArchived ? "visible" : "active"} {visibleModpacks.length === 1 ? "modpack" : "modpacks"}</span><Button variant="ghost" size="sm" type="button" onclick={() => (showArchived = !showArchived)}>{showArchived ? "Hide archived" : "Show archived"}</Button></div></div>
  {#if visibleModpacks.length === 0}<p class="no-results">{showArchived ? "No registered modpacks are available." : "Archived modpacks are hidden from the active list."}</p>{/if}
  {#each visibleModpacks as modpack (modpack.id)}
    {@const errorCount = modpack.validation.filter((item) => item.severity === "error").length}
    {@const theme = getModpackTheme(modpack.application.theme)}
    <article class:disconnected={modpack.application.lifecycle === "disconnected"} class:archived={modpack.application.lifecycle === "archived"} class:selected={focusedModpackId === modpack.id} class="modpack-row">
      <button class="modpack-select" type="button" disabled={busy || inspecting} aria-label={`Focus ${modpack.application.display_name}`} aria-current={focusedModpackId === modpack.id ? "true" : undefined} onclick={() => onfocus(modpack)}>
        <div class="modpack-mark" aria-hidden="true">{#if modpack.application.lifecycle === "disconnected"}<LinkBreakIcon size={22} />{:else if hasErrors(modpack.validation)}<WarningCircleIcon size={22} />{:else}<CheckCircleIcon size={22} />{/if}</div>
        <div class="modpack-main"><div class="modpack-title"><h3>{modpack.application.display_name}</h3><span class="modpack-theme" title={`Theme: ${theme.label}`}><span class="theme-swatch" style={`--theme-color: ${theme.color}`} aria-hidden="true"></span><span>{theme.label}</span></span></div></div>
        <div class="modpack-info"><p class="path" title={modpack.canonical_path}>{modpack.canonical_path}</p><div class="evidence" aria-label="Modpack evidence summary"><span>Packwiz {observed(modpack.packwiz.version)}</span><span>{loaderLabel(modpack)}</span><span title={freshnessTitle(modpack.last_refreshed_at)}>{freshnessLabel(modpack.last_refreshed_at)}</span></div>{#if errorCount}<div class="issue-summary"><WarningCircleIcon size={14} aria-hidden="true" /><span>{errorCount} validation {errorCount === 1 ? "issue" : "issues"}</span></div>{/if}</div>
      </button>
      <details class="modpack-menu"><summary aria-label={`More actions for ${modpack.application.display_name}`}><Tooltip text={`More actions for ${modpack.application.display_name}`} position="bottom"><DotsThreeIcon size={18} weight="bold" /></Tooltip></summary><div class="modpack-menu-items"><Button variant="primary" size="sm" type="button" disabled={discoveryBusy || modpack.application.lifecycle !== "active"} loading={discoveryBusy && discoveryModpackId === modpack.id} onclick={() => oncheck(modpack)}><ArrowClockwiseIcon size={15} /> Check for updates</Button><Button variant="secondary" size="sm" type="button" disabled={busy || modpack.application.lifecycle !== "active"} onclick={() => oncreaterelease(modpack)}><TagIcon size={15} /> Create release</Button>{#if modpack.application.lifecycle === "disconnected"}<Button variant="secondary" size="sm" type="button" disabled={busy} onclick={() => onreconnect(modpack)}><LinkIcon size={15} /> Reconnect</Button>{:else}<Button variant="ghost" size="sm" type="button" disabled={busy} onclick={() => onrefresh(modpack)}><ArrowClockwiseIcon size={15} /> Refresh evidence</Button>{/if}<Button variant="ghost" size="sm" type="button" disabled={busy} onclick={() => onedit(modpack)}><PencilSimpleIcon size={15} /> Edit details</Button>{#if modpack.application.lifecycle === "archived"}<Button variant="ghost" size="sm" type="button" disabled={busy} onclick={() => onlifecycle(modpack, "restore")}>Restore</Button>{:else if modpack.application.lifecycle !== "disconnected"}<Button variant="ghost" size="sm" type="button" disabled={busy} onclick={() => onlifecycle(modpack, "archive")}><ArchiveIcon size={15} /> Archive</Button>{/if}</div></details>
    </article>
  {/each}
</section>

<style>
  .modpack-list { background:var(--color-surface); }.modpack-list.collapsed { visibility:hidden; opacity:0; pointer-events:none; }.list-heading { display:flex; align-items:center; justify-content:space-between; gap:10px 16px; padding:16px; border-bottom:1px solid var(--color-border); }.eyebrow { margin:0; color:var(--color-accent-strong); font:700 10px var(--font-mono); letter-spacing:.12em; text-transform:uppercase; }.list-controls { display:flex; align-items:center; gap:12px; }.count,.path,.evidence { color:var(--color-text-muted); font:11px var(--font-mono); }.count { white-space:nowrap; }.modpack-row { position:relative; min-height:0; padding:18px 20px; border-bottom:1px solid var(--color-border); }.modpack-row:last-child { border-bottom:0; }.modpack-row.disconnected,.modpack-row.archived { background:color-mix(in srgb,var(--color-surface-raised) 55%,var(--color-surface)); }.modpack-row.selected { box-shadow:inset 4px 0 0 var(--color-accent); background:color-mix(in srgb,var(--color-accent) 10%,var(--color-surface)); }.modpack-mark { color:var(--color-success); padding-top:2px; }.disconnected .modpack-mark { color:var(--color-warning); }.modpack-select { display:grid; grid-template-columns:auto minmax(0,1fr); align-items:start; gap:12px; min-width:0; width:100%; padding:0; border:0; color:inherit; background:transparent; text-align:left; cursor:pointer; }.modpack-select:disabled { cursor:wait; }.modpack-main,.modpack-info { min-width:0; }.modpack-title { min-width:0; }.modpack-title h3 { margin:0; overflow:hidden; text-overflow:ellipsis; white-space:nowrap; }.modpack-info { grid-column:1 / -1; margin-top:2px; }.path { margin:7px 0; overflow:hidden; text-overflow:ellipsis; white-space:nowrap; }.evidence { display:flex; gap:14px; flex-wrap:wrap; font-size:10px; }.evidence span { white-space:nowrap; }.issue-summary { display:flex; align-items:center; gap:6px; margin-top:12px; color:var(--color-danger); font:11px var(--font-mono); }.modpack-menu { position:absolute; z-index:7; top:12px; right:12px; }.modpack-menu summary { display:inline-flex; min-height:34px; align-items:center; gap:6px; padding:0 10px; color:var(--color-text-muted); font-size:12px; font-weight:700; list-style:none; cursor:pointer; }.modpack-menu summary::-webkit-details-marker { display:none; }.modpack-menu summary:hover { color:var(--color-text); background:var(--color-surface-raised); }.modpack-menu-items { position:absolute; z-index:8; right:0; top:calc(100% + 6px); display:grid; min-width:150px; padding:5px; border:1px solid var(--color-border); background:var(--color-surface); box-shadow:0 8px 20px color-mix(in srgb,var(--color-ink) 18%,transparent); }.modpack-menu-items :global(.button) { justify-content:flex-start; width:100%; }.no-results { padding:24px; color:var(--color-text-muted); font-size:13px; }
  .modpack-title { display:flex; align-items:center; gap:10px; min-width:0; padding-right:48px; }
  .modpack-title h3 { min-width:0; }
  .modpack-theme { display:inline-flex; align-items:center; gap:5px; flex:0 0 auto; color:var(--color-text-muted); font:10px var(--font-mono); white-space:nowrap; }
  .theme-swatch { display:inline-block; width:12px; height:12px; border:1px solid color-mix(in srgb,var(--theme-color) 60%,var(--color-text)); border-radius:50%; background:var(--theme-color); }
  .modpack-menu summary { display: inline-flex; align-items: center; justify-content: center; width: 32px; min-height: 32px; padding: 0; list-style: none; }
  .modpack-menu summary::-webkit-details-marker { display: none; }
  .modpack-menu :global(.tooltip-content) { right: 0; left: auto; transform: translate(0, -4px); }
  .modpack-menu :global(.tooltip.bottom .tooltip-content::after) { right: 5px; left: auto; }
  .modpack-menu :global(.tooltip.bottom:hover .tooltip-content),
  .modpack-menu :global(.tooltip.bottom:focus-within .tooltip-content) { transform: translate(0, 0); }

  @media (max-width: 520px) {
    .modpack-row { padding: 16px 14px; }
    .modpack-title { padding-right: 44px; }
    .modpack-menu { top: 8px; right: 8px; }
    .modpack-menu summary { min-height: 30px; padding: 0 8px; font-size: 11px; }
    .evidence { gap: 8px 12px; }
  }
</style>
