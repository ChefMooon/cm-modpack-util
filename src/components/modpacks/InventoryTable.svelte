<script lang="ts">
  import ArrowSquareOutIcon from "phosphor-svelte/lib/ArrowSquareOutIcon";
  import Button from "../ui/Button.svelte";
  import type { Evidence, InventoryEntry } from "../../lib/domain";

  let {
    entries,
    evidenceLabel,
    pinningEntry = null,
    onopenPage,
    onpin,
  }: {
    entries: InventoryEntry[];
    evidenceLabel: <T>(value: Evidence<T>) => string;
    pinningEntry?: string | null;
    onopenPage?: (entry: InventoryEntry) => void | Promise<void>;
    onpin?: (entry: InventoryEntry, pin: boolean) => void;
  } = $props();
</script>

<div class="inventory-table" role="table" aria-label="Current mod inventory">
  <div class="inventory-row inventory-header" role="row"><span>Local entry</span><span>Version</span><span>Provider</span><span>Side</span><span>Pin</span><span>Actions</span></div>
  {#each entries as entry (entry.local_id)}
    <div class="inventory-row" role="row">
      <div><strong>{evidenceLabel(entry.name)}</strong><span class="path">{entry.local_id}</span><span class="metadata-path">{entry.metadata_path}</span></div>
      <span>{evidenceLabel(entry.version)}</span>
      <span>{evidenceLabel(entry.provider)}</span>
      <span>{evidenceLabel(entry.side)}</span>
      <span>{evidenceLabel(entry.pin)}</span>
      <div class="inventory-actions">
        {#if entry.page_link && onopenPage}<Button variant="quiet" size="sm" type="button" onclick={() => onopenPage?.(entry)}><ArrowSquareOutIcon size={14} /> Open</Button>{/if}
        {#if onpin}
          {#if evidenceLabel(entry.pin) === "true"}<Button variant="quiet" size="sm" type="button" disabled={pinningEntry !== null} loading={pinningEntry === entry.local_id} onclick={() => onpin?.(entry, false)}>Unpin</Button>
          {:else if evidenceLabel(entry.pin) === "false"}<Button variant="quiet" size="sm" type="button" disabled={pinningEntry !== null} loading={pinningEntry === entry.local_id} onclick={() => onpin?.(entry, true)}>Pin</Button>
          {:else}<span class="unavailable">Unavailable</span>{/if}
        {/if}
      </div>
    </div>
  {/each}
</div>

<style>
  .inventory-table { border:1px solid var(--color-border); }
  .inventory-row { display:grid; grid-template-columns:minmax(180px,1.7fr) repeat(4,minmax(70px,.7fr)) 120px; gap:12px; align-items:center; padding:12px 13px; border-bottom:1px solid var(--color-border); }
  .inventory-row:last-child { border-bottom:0; }
  .inventory-header { color:var(--color-text-muted); font:700 10px var(--font-mono); text-transform:uppercase; }
  .inventory-row > div:first-child { display:grid; gap:4px; min-width:0; }
  .inventory-row > span { overflow-wrap:anywhere; font-size:12px; }
  .path,.metadata-path,.unavailable { color:var(--color-text-muted); font:11px var(--font-mono); }
  .path,.metadata-path { overflow:hidden; text-overflow:ellipsis; white-space:nowrap; }
  .inventory-actions { display:flex; flex-wrap:wrap; gap:4px; justify-content:flex-end; }
  @media (max-width:860px) { .inventory-header { display:none; }.inventory-row { grid-template-columns:repeat(2,minmax(0,1fr)); align-items:start; }.inventory-row > div:first-child { grid-column:1 / -1; }.inventory-actions { justify-content:flex-start; } }
</style>
