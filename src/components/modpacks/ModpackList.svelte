<script lang="ts">
  import ArchiveIcon from "phosphor-svelte/lib/ArchiveIcon";
  import ArrowClockwiseIcon from "phosphor-svelte/lib/ArrowClockwiseIcon";
  import CheckCircleIcon from "phosphor-svelte/lib/CheckCircleIcon";
  import DotsThreeIcon from "phosphor-svelte/lib/DotsThreeIcon";
  import LinkBreakIcon from "phosphor-svelte/lib/LinkBreakIcon";
  import LinkIcon from "phosphor-svelte/lib/LinkIcon";
  import PencilSimpleIcon from "phosphor-svelte/lib/PencilSimpleIcon";
  import WarningCircleIcon from "phosphor-svelte/lib/WarningCircleIcon";
  import Button from "../ui/Button.svelte";
  import type { ModpackRecord } from "../../lib/domain";

  type LifecycleAction = "archive" | "restore" | "disconnect";
  let { projects, showArchived = $bindable(false), collapsed = false, focusedModpackId, busy, inspecting, discoveryBusy, discoveryModpackId, onfocus, oncheck, onreconnect, onrefresh, onedit, onlifecycle, observed, freshnessLabel, freshnessTitle, hasErrors }: {
    projects: ModpackRecord[];
    showArchived?: boolean;
    collapsed?: boolean;
    focusedModpackId: string | null;
    busy: boolean;
    inspecting: boolean;
    discoveryBusy: boolean;
    discoveryModpackId: string | null;
    onfocus: (project: ModpackRecord) => void;
    oncheck: (project: ModpackRecord) => void | Promise<void>;
    onreconnect: (project: ModpackRecord) => void | Promise<void>;
    onrefresh: (project: ModpackRecord) => void | Promise<void>;
    onedit: (project: ModpackRecord) => void | Promise<void>;
    onlifecycle: (project: ModpackRecord, action: LifecycleAction) => void | Promise<void>;
    observed: (value: import("../../lib/domain").Observation<string>) => string;
    freshnessLabel: (value: string | null) => string;
    freshnessTitle: (value: string | null) => string;
    hasErrors: (results: import("../../lib/domain").ValidationResult[]) => boolean;
  } = $props();

  const visibleProjects = $derived(showArchived ? projects : projects.filter((project) => project.application.lifecycle !== "archived"));
</script>

<section id="project-list" class:collapsed class="project-list" aria-label="Registered projects">
  <div class="list-heading"><p class="eyebrow">Registered projects</p><div class="list-controls"><span class="count">{visibleProjects.length} active {visibleProjects.length === 1 ? "project" : "projects"}</span><Button variant="ghost" size="sm" type="button" onclick={() => (showArchived = !showArchived)}>{showArchived ? "Hide archived" : "Show archived"}</Button></div></div>
  {#if visibleProjects.length === 0}<p class="no-results">Archived projects are hidden from the active list.</p>{/if}
  {#each visibleProjects as project (project.id)}
    {@const errorCount = project.validation.filter((item) => item.severity === "error").length}
    <article class:disconnected={project.application.lifecycle === "disconnected"} class:archived={project.application.lifecycle === "archived"} class:selected={focusedModpackId === project.id} class="project-row">
      <button class="project-select" type="button" disabled={busy || inspecting} aria-label={`Focus ${project.application.display_name}`} aria-current={focusedModpackId === project.id ? "true" : undefined} onclick={() => onfocus(project)}>
        <div class="project-mark" aria-hidden="true">{#if project.application.lifecycle === "disconnected"}<LinkBreakIcon size={22} />{:else if hasErrors(project.validation)}<WarningCircleIcon size={22} />{:else}<CheckCircleIcon size={22} />{/if}</div>
        <div class="project-main"><div class="project-title"><h3>{project.application.display_name}</h3></div></div>
        <div class="project-info"><p class="path" title={project.canonical_path}>{project.canonical_path}</p><div class="evidence" aria-label="Project evidence summary"><span>Packwiz {observed(project.packwiz.version)}</span><span>{project.packwiz.declared_versions.length ? "Loader observed" : "Loader unavailable"}</span><span title={freshnessTitle(project.last_refreshed_at)}>{freshnessLabel(project.last_refreshed_at)}</span></div>{#if errorCount}<div class="issue-summary"><WarningCircleIcon size={14} aria-hidden="true" /><span>{errorCount} validation {errorCount === 1 ? "issue" : "issues"}</span></div>{/if}</div>
      </button>
      <details class="project-menu"><summary aria-label={`More actions for ${project.application.display_name}`}><DotsThreeIcon size={18} weight="bold" /> More</summary><div class="project-menu-items"><Button variant="primary" size="sm" type="button" disabled={discoveryBusy || project.application.lifecycle !== "active"} loading={discoveryBusy && discoveryModpackId === project.id} onclick={() => oncheck(project)}><ArrowClockwiseIcon size={15} /> Check for updates</Button>{#if project.application.lifecycle === "disconnected"}<Button variant="secondary" size="sm" type="button" disabled={busy} onclick={() => onreconnect(project)}><LinkIcon size={15} /> Reconnect</Button>{:else}<Button variant="ghost" size="sm" type="button" disabled={busy} onclick={() => onrefresh(project)}><ArrowClockwiseIcon size={15} /> Refresh evidence</Button>{/if}<Button variant="ghost" size="sm" type="button" disabled={busy} onclick={() => onedit(project)}><PencilSimpleIcon size={15} /> Edit details</Button>{#if project.application.lifecycle === "archived"}<Button variant="ghost" size="sm" type="button" disabled={busy} onclick={() => onlifecycle(project, "restore")}>Restore</Button>{:else if project.application.lifecycle !== "disconnected"}<Button variant="ghost" size="sm" type="button" disabled={busy} onclick={() => onlifecycle(project, "archive")}><ArchiveIcon size={15} /> Archive</Button>{/if}{#if project.application.lifecycle !== "archived" && project.application.lifecycle !== "disconnected"}<Button variant="ghost" size="sm" type="button" disabled={busy} onclick={() => onlifecycle(project, "disconnect")}>Disconnect</Button>{/if}</div></details>
    </article>
  {/each}
</section>

<style>
  .project-list { background:var(--color-surface); }.project-list.collapsed { visibility:hidden; opacity:0; pointer-events:none; }.list-heading { display:flex; align-items:center; justify-content:space-between; gap:10px 16px; padding:16px; border-bottom:1px solid var(--color-border); }.eyebrow { margin:0; color:var(--color-accent-strong); font:700 10px var(--font-mono); letter-spacing:.12em; text-transform:uppercase; }.list-controls { display:flex; align-items:center; gap:12px; }.count,.path,.evidence { color:var(--color-text-muted); font:11px var(--font-mono); }.count { white-space:nowrap; }.project-row { position:relative; min-height:0; padding:18px 20px; border-bottom:1px solid var(--color-border); }.project-row:last-child { border-bottom:0; }.project-row.disconnected,.project-row.archived { background:color-mix(in srgb,var(--color-surface-raised) 55%,var(--color-surface)); }.project-row.selected { box-shadow:inset 4px 0 0 var(--color-accent); background:color-mix(in srgb,var(--color-accent) 10%,var(--color-surface)); }.project-mark { color:var(--color-success); padding-top:2px; }.disconnected .project-mark { color:var(--color-warning); }.project-select { display:grid; grid-template-columns:auto minmax(0,1fr); align-items:start; gap:12px; min-width:0; width:100%; padding:0; border:0; color:inherit; background:transparent; text-align:left; cursor:pointer; }.project-select:disabled { cursor:wait; }.project-main,.project-info { min-width:0; }.project-title { min-width:0; }.project-title h3 { margin:0; overflow:hidden; text-overflow:ellipsis; white-space:nowrap; }.project-info { grid-column:1 / -1; margin-top:2px; }.path { margin:7px 0; overflow:hidden; text-overflow:ellipsis; white-space:nowrap; }.evidence { display:flex; gap:14px; flex-wrap:wrap; font-size:10px; }.evidence span { white-space:nowrap; }.issue-summary { display:flex; align-items:center; gap:6px; margin-top:12px; color:var(--color-danger); font:11px var(--font-mono); }.project-menu { position:absolute; z-index:7; top:12px; right:12px; }.project-menu summary { display:inline-flex; min-height:34px; align-items:center; gap:6px; padding:0 10px; color:var(--color-text-muted); font-size:12px; font-weight:700; list-style:none; cursor:pointer; }.project-menu summary::-webkit-details-marker { display:none; }.project-menu summary:hover { color:var(--color-text); background:var(--color-surface-raised); }.project-menu-items { position:absolute; z-index:8; right:0; top:calc(100% + 6px); display:grid; min-width:150px; padding:5px; border:1px solid var(--color-border); background:var(--color-surface); box-shadow:0 8px 20px color-mix(in srgb,var(--color-ink) 18%,transparent); }.project-menu-items :global(.button) { justify-content:flex-start; width:100%; }.no-results { padding:24px; color:var(--color-text-muted); font-size:13px; }
  @media (max-width: 520px) {
    .project-row { padding: 16px 14px; }
    .project-title { padding-right: 84px; }
    .project-menu { top: 8px; right: 8px; }
    .project-menu summary { min-height: 30px; padding: 0 8px; font-size: 11px; }
    .evidence { gap: 8px 12px; }
  }
</style>
