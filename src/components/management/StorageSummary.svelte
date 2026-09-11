<script lang="ts">
  import type { StorageReport } from "../../lib/domain";

  let { report }: { report: StorageReport } = $props();

  function formatBytes(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    const units = ["KB", "MB", "GB"];
    let value = bytes;
    let unit = -1;
    do { value /= 1024; unit += 1; } while (value >= 1024 && unit < units.length - 1);
    return `${value.toFixed(value >= 10 ? 0 : 1)} ${units[unit]}`;
  }
</script>

<section class="storage-card" aria-labelledby="storage-title">
  <div class="section-heading"><div><p class="eyebrow">Storage</p><h2 id="storage-title">Application-owned data</h2></div><strong class="total">{formatBytes(report.application_owned_bytes)}</strong></div>
  <div class="totals"><span><strong>{formatBytes(report.removable_bytes)}</strong> removable</span><span><strong>{formatBytes(report.protected_bytes)}</strong> retained/protected</span></div>
  <div class="category-list">
    {#each report.categories as category}
      <div class="category">
        <div><strong>{category.name}</strong><span>{formatBytes(category.bytes)}</span></div>
        <p class:external={category.availability !== "available"}>{category.removable ? "Removable application data" : category.protected ? "Retained/protected" : category.availability}</p>
      </div>
    {/each}
  </div>
  <p class="boundary">External Packwiz directories, Git history, and user export destination files are outside these totals and are never removable application data.</p>
</section>

<style>
  .storage-card { height:100%; padding:20px; border:1px solid var(--color-border); background:var(--color-surface); }.section-heading { display:flex; justify-content:space-between; gap:14px; align-items:flex-start; }.eyebrow { margin:0 0 8px; color:var(--color-accent-strong); font:700 10px var(--font-mono); letter-spacing:.12em; text-transform:uppercase; }.storage-card h2 { margin:0; font-size:20px; }.total { color:var(--color-accent-strong); font:700 20px var(--font-mono); }.totals { display:flex; flex-wrap:wrap; gap:14px; margin:18px 0; color:var(--color-text-muted); font-size:12px; }.totals strong { color:var(--color-text); font-family:var(--font-mono); }.category-list { border-top:1px solid var(--color-border); }.category { display:flex; justify-content:space-between; gap:14px; padding:11px 0; border-bottom:1px solid var(--color-border); }.category div { display:grid; gap:4px; }.category span,.category p { margin:0; color:var(--color-text-muted); font:11px var(--font-mono); }.category p { color:var(--color-success); text-align:right; }.category p.external { color:var(--color-warning); }.boundary { margin-bottom:0; color:var(--color-text-muted); font-size:12px; line-height:1.5; }@media (max-width:520px) { .section-heading,.category { flex-direction:column; }.category p { text-align:left; } }
</style>
