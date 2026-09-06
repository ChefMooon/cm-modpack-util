<script lang="ts">
  import ArrowRightIcon from "phosphor-svelte/lib/ArrowRightIcon";
  import CaretDownIcon from "phosphor-svelte/lib/CaretDownIcon";
  import CheckIcon from "phosphor-svelte/lib/CheckIcon";
  import GearIcon from "phosphor-svelte/lib/GearIcon";
  import MagnifyingGlassIcon from "phosphor-svelte/lib/MagnifyingGlassIcon";
  import PackageIcon from "phosphor-svelte/lib/PackageIcon";
  import PlusIcon from "phosphor-svelte/lib/PlusIcon";
  import WarningIcon from "phosphor-svelte/lib/WarningIcon";
  import XIcon from "phosphor-svelte/lib/XIcon";
  import Button from "../components/ui/Button.svelte";
  import Tooltip from "../components/ui/Tooltip.svelte";

  type ModStatus = "ready" | "pinned" | "warning";
  type ModUpdate = { name: string; current: string; next: string; provider: string; side: string; status: ModStatus };
  type View = "Updates" | "Inventory" | "History";

  const updates: ModUpdate[] = [
    { name: "Create", current: "0.5.1-f", next: "0.5.1-g", provider: "Modrinth", side: "Both", status: "ready" },
    { name: "Create: Steam 'n' Rails", current: "1.6.4", next: "1.6.5", provider: "Modrinth", side: "Both", status: "ready" },
    { name: "JEI", current: "19.21.0.247", next: "19.21.0.248", provider: "CurseForge", side: "Client", status: "pinned" },
    { name: "Sophisticated Backpacks", current: "3.20.17.1", next: "3.20.18.2", provider: "Modrinth", side: "Both", status: "warning" },
    { name: "Xaero's Minimap", current: "24.6.1", next: "24.7.0", provider: "Modrinth", side: "Client", status: "ready" }
  ];
  const inventory = ["Create", "Create: Steam 'n' Rails", "JEI", "Sophisticated Backpacks", "Xaero's Minimap"];
  const history = [
    { time: "14:32:08", action: "Local scan completed", detail: "87 Packwiz metadata files read" },
    { time: "13:08:44", action: "Update review opened", detail: "5 possible updates found" },
    { time: "Yesterday", action: "Project registered", detail: "Clockwork added from local path" }
  ];

  let selected = $state(new Set(updates.filter((update) => update.status === "ready").map((update) => update.name)));
  let query = $state("");
  let activeView = $state<View>("Updates");
  let notice = $state("Scan complete. 5 possible updates found in local TOML files.");
  const visibleUpdates = $derived(updates.filter((update) => update.name.toLowerCase().includes(query.trim().toLowerCase())));

  function toggleUpdate(name: string) {
    const next = new Set(selected);
    if (next.has(name)) next.delete(name); else next.add(name);
    selected = next;
  }
  function selectReady() { selected = new Set(visibleUpdates.filter((update) => update.status === "ready").map((update) => update.name)); }
  function clearFilter() { query = ""; }
  function showUnavailable(action: string) { notice = `${action} is unavailable until a project scan is connected.`; }
</script>

<svelte:head>
  <title>CM Modpack Util</title>
  <meta name="description" content="Review and maintain Packwiz modpack metadata." />
</svelte:head>

<main class="app-frame">
  <aside class="project-rail" aria-label="Project navigation">
    <a class="brand-mark" href="/" aria-label="CM Modpack Util home">CM</a>
    <div class="rail-label">PROJECTS</div>
    <nav class="project-list">
      <button class="project active" type="button" aria-current="page"><span class="project-icon yellow">CM</span><span>Clockwork<br /><small>0.8.4 / DEV</small></span></button>
      <button class="project" type="button"><span class="project-icon cyan">VA</span><span>Vanilla Plus<br /><small>1.21.1 / STABLE</small></span></button>
      <button class="project" type="button"><span class="project-icon green">SK</span><span>Skyblock Lab<br /><small>0.3.0 / TESTING</small></span></button>
    </nav>
    <button class="rail-action register" type="button" onclick={() => showUnavailable("Register project")}><PlusIcon size={16} weight="bold" /><span>Register project</span></button>
    <div class="rail-bottom"><a class="rail-action" href="/settings"><GearIcon size={17} /><span>Settings</span></a><span class="build">BUILD 0.1.0</span></div>
  </aside>

  <section class="workspace">
    <header class="topbar"><div class="breadcrumb"><span>PROJECT</span><strong>Clockwork</strong><CaretDownIcon size={13} weight="bold" /></div><div class="topbar-actions"><span class="connection"><i></i> LOCAL SOURCE</span><Tooltip text="Open settings"><a class="icon-button" href="/settings" aria-label="Open settings"><GearIcon size={18} /></a></Tooltip></div></header>
    <div class="content">
      <header class="page-heading"><div><p class="eyebrow">UPDATE REVIEW / SESSION 014</p><h1>Review available updates</h1><p class="lede">Compare local Packwiz metadata before changing the project files.</p></div><div class="heading-actions"><Button variant="secondary" size="sm" type="button" onclick={() => showUnavailable("Scanning")}>Scan again</Button><Button variant="primary" size="sm" type="button" disabled={!selected.size} onclick={() => showUnavailable("Applying updates")}>Apply {selected.size} updates <ArrowRightIcon size={15} weight="bold" /></Button></div></header>
      <div class="notice" role="status"><CheckIcon size={17} weight="bold" /><span><strong>{notice.split(".")[0]}.</strong>{notice.includes(".") ? notice.slice(notice.indexOf(".") + 1) : ""}</span><span class="notice-time">14:32:08</span></div>
      <section class="summary-grid" aria-label="Project summary">
        <div class="summary-panel"><span class="panel-label">PROJECT STATUS</span><strong class="status-value"><i class="dot green-dot"></i> READY</strong><span class="panel-meta">Last scan 2 min ago</span></div>
        <div class="summary-panel"><span class="panel-label">MOD INVENTORY</span><strong class="big-value">87</strong><span class="panel-meta">84 active / 3 pinned</span></div>
        <div class="summary-panel"><span class="panel-label">PACKWIZ VERSION</span><strong class="mono-value">1.2.0</strong><span class="panel-meta">Minecraft 1.20.1 / NeoForge</span></div>
        <div class="summary-panel attention"><span class="panel-label">REVIEW QUEUE</span><strong class="big-value">{selected.size}<span class="slash"> / </span>{updates.length}</strong><span class="panel-meta">Updates selected</span></div>
      </section>
      <div class="view-tabs" role="tablist" aria-label="Project views">
        {#each ["Updates", "Inventory", "History"] as view}
          {@const typedView = view as View}
          <button id={`tab-${typedView.toLowerCase()}`} class:active={activeView === typedView} type="button" role="tab" aria-selected={activeView === typedView} aria-controls={`panel-${typedView.toLowerCase()}`} tabindex={activeView === typedView ? 0 : -1} onclick={() => (activeView = typedView)}>{typedView}<span>{typedView === "Updates" ? "05" : typedView === "Inventory" ? "87" : "13"}</span></button>
        {/each}
      </div>
      {#if activeView === "Updates"}
        <div id="panel-updates" class="update-panel" role="tabpanel" aria-labelledby="tab-updates">
          <div class="panel-toolbar"><div><p class="eyebrow">LOCAL EVIDENCE</p><h2>Available updates</h2></div><div class="toolbar-actions"><label class="search"><MagnifyingGlassIcon size={16} aria-hidden="true" /><input bind:value={query} placeholder="Filter mods" aria-label="Filter mods" onkeydown={(event) => event.key === "Escape" && clearFilter()} />{#if query}<Tooltip text="Clear filter"><button class="clear-button" type="button" aria-label="Clear mod filter" onclick={clearFilter}><XIcon size={14} weight="bold" /></button></Tooltip>{/if}</label><button class="text-button" type="button" onclick={selectReady}>Select ready</button></div></div>
          <div class="table-wrap"><table><thead><tr><th class="check-col"><span class="sr-only">Select</span></th><th>MOD / PROJECT</th><th>CURRENT</th><th>AVAILABLE</th><th>PROVIDER</th><th>SIDE</th><th>STATE</th></tr></thead><tbody>
            {#each visibleUpdates as update}
              <tr class:selected-row={selected.has(update.name)}><td class="check-col"><input type="checkbox" checked={selected.has(update.name)} disabled={update.status !== "ready"} onchange={() => toggleUpdate(update.name)} aria-label={`Select ${update.name}`} /></td><td><strong class="mod-name"><PackageIcon size={16} weight="duotone" />{update.name}</strong><span class="source-file">Read from local TOML</span></td><td class="mono">{update.current}</td><td class="mono next-version">{update.next}</td><td><span class="tag">{update.provider}</span></td><td><span class="side">{update.side}</span></td><td>
                {#if update.status === "ready"}<span class="state ready-state"><i></i> Ready</span>{:else if update.status === "pinned"}<span class="state pinned-state">Pinned</span>{:else}<span class="state warning-state"><WarningIcon size={13} weight="fill" /> Review</span>{/if}
              </td></tr>
            {:else}<tr><td colspan="7" class="empty-row">No mods matched <span class="mono">"{query}"</span>.</td></tr>{/each}
          </tbody></table></div>
          <footer class="table-footer"><span>Showing {visibleUpdates.length} of {updates.length} updates</span><span><span class="legend-dot cyan-dot"></span> Local TOML evidence <span class="footer-divider">|</span> <span class="legend-dot yellow-dot"></span> Requires review</span></footer>
        </div>
      {:else if activeView === "Inventory"}
        <div id="panel-inventory" class="list-panel" role="tabpanel" aria-labelledby="tab-inventory"><p class="eyebrow">LOCAL INVENTORY</p><h2>Installed mod metadata</h2>{#each inventory as item, index}<div class="list-row"><span class="mono">{String(index + 1).padStart(2, "0")}</span><strong>{item}</strong><span class="source-file">mods/{item.toLowerCase().replaceAll(" ", "-")}.pw.toml</span><span class="state ready-state">ACTIVE</span></div>{/each}</div>
      {:else}
        <div id="panel-history" class="list-panel" role="tabpanel" aria-labelledby="tab-history"><p class="eyebrow">APPLICATION HISTORY</p><h2>Recent project activity</h2>{#each history as item}<div class="list-row history-row"><span class="mono">{item.time}</span><strong>{item.action}</strong><span>{item.detail}</span></div>{/each}</div>
      {/if}
      <section class="activity-panel" aria-labelledby="activity-title"><div class="activity-heading"><div><p class="eyebrow">OPERATION LOG</p><h2 id="activity-title">Latest activity</h2></div><span class="live-label"><i></i> LOCAL</span></div><div class="log"><p><span>14:32:08</span><b class="log-ok">OK</b> Scanned 87 local mod metadata files</p><p><span>14:32:08</span><b class="log-ok">OK</b> Read Packwiz index <strong class="mono">index.toml</strong></p><p><span>14:32:09</span><b class="log-note">NOTE</b> 1 pinned mod excluded from selection</p></div></section>
    </div>
    <footer class="status-bar"><span><i class="dot green-dot"></i> Project validated</span><span class="status-path">C:\Modpacks\Clockwork</span><span>GIT <strong>clean</strong></span><span>Last saved 14:32:09</span></footer>
  </section>
</main>

<style>
  .app-frame { display: grid; grid-template-columns: 218px minmax(0, 1fr); min-height: 100vh; background: var(--color-paper); color: var(--color-ink); }
  .project-rail { display: flex; flex-direction: column; min-height: 100vh; padding: 24px 14px 18px; border-right: 1px solid var(--color-line); background: var(--color-console); color: var(--color-console-text); }.brand-mark { display: grid; width: 36px; height: 36px; margin: 0 10px 42px; place-items: center; border: 2px solid var(--color-yellow); color: var(--color-yellow); font: 800 12px var(--font-mono); letter-spacing: .08em; text-decoration: none; }.rail-label, .panel-label, .eyebrow { color: var(--color-cyan); font: 700 10px var(--font-mono); letter-spacing: .12em; }.rail-label { margin: 0 10px 12px; color: var(--color-text-subtle); }.project-list { display: grid; gap: 4px; }.project, .rail-action { display: flex; align-items: center; border: 0; color: inherit; background: transparent; text-align: left; }.project { gap: 10px; width: 100%; padding: 10px; border-left: 2px solid transparent; font-size: 12px; line-height: 1.35; }.project:hover, .project.active { border-left-color: var(--color-yellow); background: color-mix(in srgb, var(--color-panel-muted) 55%, transparent); }.project small { color: var(--color-text-subtle); font: 9px var(--font-mono); }.project-icon { display: grid; width: 28px; height: 28px; place-items: center; flex: 0 0 28px; color: var(--color-console); font: 800 9px var(--font-mono); }.yellow { background: var(--color-yellow); }.cyan { background: var(--color-cyan); }.green { background: var(--color-green); }.register { gap: 8px; margin: 20px 10px; padding: 8px 0; color: var(--color-text-subtle); font-size: 11px; }.register:hover, .rail-action:hover { color: var(--color-yellow); }.rail-bottom { display: grid; gap: 18px; margin-top: auto; }.rail-action { gap: 9px; padding: 8px 10px; color: var(--color-text-subtle); font-size: 11px; text-decoration: none; }.build { padding: 0 10px; color: var(--color-text-subtle); font: 9px var(--font-mono); }
  .workspace { min-width: 0; }.topbar { display: flex; align-items: center; justify-content: space-between; min-height: 64px; padding: 0 38px; border-bottom: 1px solid var(--color-line); background: var(--color-panel-muted); }.breadcrumb, .topbar-actions, .connection, .heading-actions, .notice, .status-value, .mod-name, .state, .live-label { display: flex; align-items: center; }.breadcrumb { gap: 10px; font: 11px var(--font-mono); }.breadcrumb span { color: var(--color-text-subtle); font-size: 10px; letter-spacing: .1em; }.topbar-actions { gap: 18px; }.connection { gap: 7px; color: var(--color-text-muted); font: 9px var(--font-mono); letter-spacing: .08em; }.connection i, .live-label i { width: 6px; height: 6px; background: var(--color-green); }.icon-button { display: grid; width: 32px; height: 32px; place-items: center; border: 1px solid var(--color-line); color: var(--color-ink); background: var(--color-panel); }.icon-button:hover { border-color: var(--color-cyan); color: var(--color-cyan); }
  .content { max-width: 1440px; margin: 0 auto; padding: 42px 38px 54px; }.page-heading { display: flex; align-items: flex-end; justify-content: space-between; gap: 24px; margin-bottom: 26px; }.eyebrow { margin: 0 0 10px; }.page-heading h1 { margin: 0; font-size: 24px; line-height: 1.1; }.lede { margin: 12px 0 0; color: var(--color-text-muted); font-size: 14px; }.heading-actions { gap: 8px; flex-shrink: 0; }.notice { gap: 10px; margin-bottom: 18px; padding: 10px 13px; border: 1px solid var(--color-green); color: var(--color-green); background: color-mix(in srgb, var(--color-green) 12%, var(--color-panel)); font-size: 12px; }.notice-time { margin-left: auto; color: var(--color-text-muted); font: 10px var(--font-mono); }
  .summary-grid { display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 10px; margin-bottom: 30px; }.summary-panel { min-height: 106px; padding: 16px; border: 1px solid var(--color-line); background: var(--color-panel); }.summary-panel.attention { border-color: var(--color-yellow); background: color-mix(in srgb, var(--color-yellow) 15%, var(--color-panel)); }.summary-panel .panel-label { display: block; margin-bottom: 11px; color: var(--color-text-muted); }.status-value { gap: 7px; font: 18px var(--font-mono); }.big-value, .mono-value { display: block; font: 22px var(--font-mono); }.mono-value { color: var(--color-violet); }.slash { color: var(--color-yellow); font-size: 15px; }.panel-meta { display: block; margin-top: 7px; color: var(--color-text-muted); font: 9px var(--font-mono); }.dot { display: inline-block; width: 7px; height: 7px; }.green-dot { background: var(--color-green); }.cyan-dot { background: var(--color-cyan); }.yellow-dot { background: var(--color-yellow); }
  .view-tabs { display: flex; gap: 2px; border-bottom: 1px solid var(--color-line); }.view-tabs button { display: flex; align-items: center; gap: 8px; padding: 11px 14px; border: 0; border-bottom: 2px solid transparent; color: var(--color-text-muted); background: transparent; font: 11px var(--font-mono); }.view-tabs button span { color: var(--color-text-subtle); font-size: 9px; }.view-tabs button:hover, .view-tabs button.active { border-bottom-color: var(--color-cyan); color: var(--color-ink); }.view-tabs button.active span { color: var(--color-cyan); }
  .update-panel, .list-panel, .activity-panel { margin-top: 18px; border: 1px solid var(--color-line); background: var(--color-panel); box-shadow: 3px 3px 0 var(--color-ink); }.panel-toolbar, .activity-heading { display: flex; align-items: center; justify-content: space-between; gap: 16px; padding: 20px 20px 16px; }.panel-toolbar h2, .activity-heading h2, .list-panel h2 { margin: 0; font-size: 19px; }.panel-toolbar .eyebrow, .activity-heading .eyebrow, .list-panel .eyebrow { margin-bottom: 7px; }.toolbar-actions { display: flex; align-items: center; gap: 12px; }.search { display: flex; align-items: center; gap: 7px; width: 190px; padding: 0 9px; border: 1px solid var(--color-line); color: var(--color-text-muted); background: var(--color-paper); }.search input { width: 100%; min-width: 0; padding: 8px 0; border: 0; outline: 0; color: var(--color-ink); background: transparent; font-size: 11px; }.search:focus-within { border-color: var(--color-cyan); }.clear-button { display: grid; width: 24px; height: 24px; place-items: center; border: 0; color: var(--color-text-muted); background: transparent; }.text-button { padding: 5px 0; border: 0; color: var(--color-cyan); background: transparent; font: 10px var(--font-mono); }.text-button:hover { color: var(--color-ink); }.table-wrap { overflow-x: auto; border-top: 1px solid var(--color-panel-muted); border-bottom: 1px solid var(--color-panel-muted); }table { width: 100%; min-width: 780px; border-collapse: collapse; text-align: left; }th { padding: 9px 12px; color: var(--color-text-muted); background: var(--color-panel-muted); font: 700 9px var(--font-mono); letter-spacing: .06em; }td { padding: 12px; border-top: 1px solid var(--color-panel-muted); color: var(--color-text-muted); font-size: 11px; vertical-align: middle; }tr.selected-row { background: color-mix(in srgb, var(--color-green) 9%, var(--color-panel)); }.check-col { width: 40px; padding-right: 0; text-align: center; }input[type="checkbox"] { width: 15px; height: 15px; accent-color: var(--color-cyan); }.mod-name { gap: 7px; color: var(--color-ink); font-size: 12px; }.source-file { display: block; margin-top: 4px; color: var(--color-text-subtle); font: 9px var(--font-mono); }.mono { color: var(--color-text-muted); font: 10px var(--font-mono); }.next-version { color: var(--color-green); }.tag, .side { display: inline-block; padding: 4px 6px; border: 1px solid color-mix(in srgb, var(--color-green) 60%, var(--color-line)); color: var(--color-green); background: color-mix(in srgb, var(--color-green) 10%, var(--color-panel)); font: 9px var(--font-mono); }.side { border-color: color-mix(in srgb, var(--color-violet) 60%, var(--color-line)); color: var(--color-violet); background: color-mix(in srgb, var(--color-violet) 10%, var(--color-panel)); }.state { gap: 5px; font: 10px var(--font-mono); }.ready-state { color: var(--color-green); }.ready-state i { width: 5px; height: 5px; background: var(--color-green); }.pinned-state { color: var(--color-yellow); }.warning-state { color: var(--color-coral); }.empty-row { padding: 26px; color: var(--color-text-muted); text-align: center; }.table-footer { display: flex; justify-content: space-between; gap: 14px; padding: 10px 12px; color: var(--color-text-muted); font: 9px var(--font-mono); }.legend-dot { display: inline-block; width: 6px; height: 6px; margin: 0 4px 0 9px; }.footer-divider { margin: 0 5px; color: var(--color-text-subtle); }
  .list-panel { padding: 20px; }.list-row { display: grid; grid-template-columns: 34px minmax(160px, .8fr) minmax(180px, 1.5fr) auto; align-items: center; gap: 12px; padding: 12px 0; border-top: 1px solid var(--color-panel-muted); font-size: 12px; }.list-panel .eyebrow { margin-bottom: 7px; }.list-panel h2 { margin-bottom: 18px; }.history-row { grid-template-columns: 90px minmax(180px, .8fr) 1fr; }.activity-panel { display: grid; grid-template-columns: minmax(180px, .6fr) minmax(0, 1.4fr); margin-top: 26px; box-shadow: none; background: var(--color-console); color: var(--color-console-text); }.activity-heading { align-items: flex-start; }.activity-heading h2 { color: var(--color-console-text); }.activity-heading .eyebrow { color: var(--color-yellow); }.live-label { gap: 7px; color: var(--color-green); font: 9px var(--font-mono); }.log { padding: 18px 20px; border-left: 1px solid var(--color-line); font: 10px/1.8 var(--font-mono); }.log p { margin: 0; color: var(--color-text-subtle); }.log p + p { margin-top: 4px; }.log p > span { margin-right: 12px; color: var(--color-text-subtle); }.log b { margin-right: 9px; font-size: 9px; }.log-ok { color: var(--color-green); }.log-note { color: var(--color-yellow); }.log strong { color: var(--color-console-text); font-weight: 400; }.status-bar { display: flex; align-items: center; gap: 22px; min-height: 31px; padding: 0 38px; border-top: 1px solid var(--color-line); color: var(--color-text-muted); background: var(--color-panel-muted); font: 9px var(--font-mono); }.status-bar span { display: inline-flex; align-items: center; gap: 6px; }.status-path { margin-right: auto; }.status-bar strong { color: var(--color-green); }.sr-only { position: absolute; width: 1px; height: 1px; overflow: hidden; clip: rect(0, 0, 0, 0); white-space: nowrap; }
  @media (max-width: 1050px) { .summary-grid { grid-template-columns: repeat(2, minmax(0, 1fr)); }.page-heading { align-items: flex-start; flex-direction: column; }.activity-panel { grid-template-columns: 1fr; }.log { border-top: 1px solid var(--color-line); border-left: 0; } }
  @media (max-width: 720px) { .app-frame { display: block; }.project-rail { min-height: auto; padding: 14px; }.brand-mark { margin: 0 0 18px; }.rail-label, .rail-bottom, .register { display: none; }.project-list { display: flex; overflow-x: auto; gap: 4px; }.project { min-width: 164px; }.topbar, .status-bar { padding-right: 18px; padding-left: 18px; }.topbar { min-height: 52px; }.connection { display: none; }.content { padding: 28px 18px 38px; }.summary-grid { grid-template-columns: 1fr 1fr; gap: 7px; }.summary-panel { min-height: 96px; padding: 12px; }.panel-toolbar { align-items: flex-start; flex-direction: column; }.toolbar-actions { width: 100%; }.search { flex: 1; }.table-footer { align-items: flex-start; flex-direction: column; gap: 4px; }.list-row, .history-row { grid-template-columns: 28px 1fr; }.list-row .source-file, .list-row .state, .history-row span:last-child { grid-column: 2; }.status-bar { align-items: flex-start; flex-wrap: wrap; gap: 5px 14px; padding-top: 8px; padding-bottom: 8px; }.status-path { width: 100%; order: 4; } }
</style>
