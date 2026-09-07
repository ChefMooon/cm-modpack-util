<script lang="ts">
  import WarningCircleIcon from "phosphor-svelte/lib/WarningCircleIcon";
  import Button from "../ui/Button.svelte";
  import type { SnapshotRecord } from "../../lib/domain";

  let {
    snapshots,
    loading,
    error,
    filter = $bindable("all"),
    onretry,
    onopen,
    snapshotStatus,
  }: {
    snapshots: SnapshotRecord[];
    loading: boolean;
    error: string;
    filter?: "all" | "releases" | "snapshots";
    onretry: () => void | Promise<void>;
    onopen: (snapshotId: string) => void | Promise<void>;
    snapshotStatus: (snapshot: SnapshotRecord) => string;
  } = $props();

  const visibleSnapshots = $derived(filter === "releases" ? [] : snapshots);
</script>

<div id="versions-panel" class="versions-panel" role="tabpanel" aria-labelledby="versions-tab">
  <div class="detail-heading"><p class="eyebrow">Versions</p><h2>Snapshot history</h2><p>All currently shows supported snapshot records. Release records are not available yet.</p></div>
  <div class="version-filters" role="radiogroup" aria-label="Version record type">
    <button class:active={filter === "all"} type="button" role="radio" aria-checked={filter === "all"} onclick={() => (filter = "all")}>All</button>
    <button type="button" role="radio" aria-checked="false" disabled title="Release support is not available">Releases <span class="unavailable">Unavailable</span></button>
    <button class:active={filter === "snapshots"} type="button" role="radio" aria-checked={filter === "snapshots"} onclick={() => (filter = "snapshots")}>Snapshots</button>
  </div>
  {#if loading}
    <div class="inspection-state" role="status"><div class="loader" aria-hidden="true"></div><p>Loading snapshot history...</p></div>
  {:else if error}
    <div class="inspection-state" role="alert"><WarningCircleIcon size={20} /><p>{error}</p><Button variant="secondary" size="sm" type="button" onclick={onretry}>Retry snapshots</Button></div>
  {:else if filter === "releases"}
    <div class="inspection-state" role="status"><p class="eyebrow">Releases unavailable</p><p>Release discovery and release records are not implemented.</p></div>
  {:else if !visibleSnapshots.length}
    <div class="inspection-state" role="status"><p class="eyebrow">No snapshots</p><p>No supported snapshot records have been created for this modpack.</p></div>
  {:else}
    <div class="snapshot-list" aria-label="Snapshot history">
      {#each visibleSnapshots as snapshot (snapshot.id)}
        <button class="snapshot-row" type="button" onclick={() => onopen(snapshot.id)}>
          <span><strong>{snapshot.label ?? "Snapshot review"}</strong><small>{snapshot.id}</small></span>
          <span><b>Lifecycle</b>{snapshot.lifecycle}</span>
          <span><b>Status</b>{snapshotStatus(snapshot)}</span>
          <span><b>Created</b>{snapshot.created_at}</span>
          <span><b>Candidates</b>{snapshot.candidates.length}</span>
          <span class="snapshot-action">Open review</span>
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .versions-panel { display:grid; align-content:center; min-height:360px; padding:32px; }
  .detail-heading { padding:28px 24px 10px; }
  .detail-heading p:not(.eyebrow) { margin-bottom:0; color:var(--color-text-muted); line-height:1.6; }
  .eyebrow { margin:0 0 10px; color:var(--color-accent-strong); font:700 10px var(--font-mono); letter-spacing:.12em; text-transform:uppercase; }
  .version-filters { display:flex; gap:4px; padding:12px 24px; border-bottom:1px solid var(--color-border); }
  .version-filters button { min-height:34px; padding:0 12px; border:1px solid var(--color-border); color:var(--color-text-muted); background:transparent; font:11px var(--font-mono); cursor:pointer; }
  .version-filters button.active { color:var(--color-text); background:var(--color-surface-raised); box-shadow:inset 0 -3px 0 var(--color-accent); }
  .version-filters button:disabled { cursor:not-allowed; opacity:.6; }
  .snapshot-list { display:grid; gap:1px; margin:16px 24px 24px; border:1px solid var(--color-border); background:var(--color-border); }
  .snapshot-row { display:grid; grid-template-columns:minmax(0,1.5fr) repeat(4,minmax(0,1fr)) minmax(82px,auto); gap:14px; align-items:center; box-sizing:border-box; width:100%; padding:14px; border:0; border-bottom:1px solid var(--color-border); color:var(--color-text-muted); background:var(--color-surface); text-align:left; cursor:pointer; }
  .snapshot-row:last-child { border-bottom:0; }
  .snapshot-row > span { display:grid; gap:5px; min-width:0; font-size:11px; }
  .snapshot-row strong { overflow-wrap:anywhere; color:var(--color-text); }
  .snapshot-row small,.snapshot-row b { color:var(--color-text-subtle); font:10px var(--font-mono); text-transform:uppercase; }
  .snapshot-action { min-width:0; overflow:hidden; color:var(--color-accent-strong); font:700 11px var(--font-mono); white-space:nowrap; text-overflow:ellipsis; }
  .inspection-state { display:grid; justify-items:center; gap:10px; padding:34px 12px 12px; color:var(--color-text-muted); text-align:center; }
  .inspection-state p { margin:0; }
  .loader { width:22px; height:22px; border:2px solid var(--color-border); border-top-color:var(--color-accent); border-radius:50%; animation:spin .8s linear infinite; }
  .unavailable { color:var(--color-text-subtle); font:10px var(--font-mono); }
  .snapshot-row:focus-visible,.version-filters button:focus-visible { outline:none; box-shadow:var(--focus-ring); }
  @keyframes spin { to { transform:rotate(360deg); } }
  @media (max-width:820px) { .version-filters { padding-inline:18px; flex-wrap:wrap; }.snapshot-list { margin-inline:18px; }.snapshot-row { grid-template-columns:1fr 1fr; gap:10px; }.snapshot-row > span:first-child,.snapshot-action { grid-column:1 / -1; } }
  @media (prefers-reduced-motion:reduce) { .loader { animation:none; } }
</style>
