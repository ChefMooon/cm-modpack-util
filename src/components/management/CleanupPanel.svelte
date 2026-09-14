<script lang="ts">
  import Button from "../ui/Button.svelte";
  import type { CleanupPlan, CleanupResult, CleanupScope } from "../../lib/domain";

  let {
    scope,
    plan,
    result,
    busy = false,
    onscopechange,
    onpreview,
    onexecute,
    ondelete,
  }: {
    scope: CleanupScope;
    plan: CleanupPlan | null;
    result: CleanupResult | null;
    busy?: boolean;
    onscopechange: (scope: CleanupScope) => void;
    onpreview: () => void | Promise<void>;
    onexecute: () => void | Promise<void>;
    ondelete: (recordType: CleanupPlan["candidates"][number]["record_type"], recordId: string) => void | Promise<void>;
  } = $props();

  function formatBytes(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  }
</script>

<section class="cleanup-card" aria-labelledby="cleanup-title">
  <div class="section-heading"><div><p class="eyebrow">Cleanup</p><h2 id="cleanup-title">Remove eligible application records</h2></div><span class="scope-label">Preview first</span></div>
  <div class="cleanup-controls">
    <p class="description">Only unreferenced provider-cache rows and obsolete export records can be physically removed. External destination files stay untouched.</p>
    <div>
      <label class="scope" for="cleanup-scope">Cleanup scope
        <select id="cleanup-scope" value={scope} onchange={(event) => onscopechange(event.currentTarget.value as CleanupScope)}>
          <option value="provider_cache">Provider cache</option>
          <option value="obsolete_exports">Obsolete exports</option>
          <option value="provider_cache_and_obsolete_exports">Both eligible categories</option>
        </select>
      </label>
      <div class="actions"><Button variant="secondary" type="button" loading={busy} onclick={onpreview}>Preview cleanup</Button>{#if plan}<Button variant="danger" type="button" disabled={!plan.removable_bytes || plan.blocked_reasons.length > 0} loading={busy} onclick={onexecute}>Run scoped cleanup</Button>{/if}</div>
    </div>
  </div>
  {#if plan}
    <div class="plan-summary" role="status"><strong>{plan.candidates.filter((candidate) => !candidate.protected).length} removable records</strong><span>{formatBytes(plan.removable_bytes)}</span><span>{plan.protected_count} protected</span></div>
    {#if plan.blocked_reasons.length}<div class="blocked" role="alert">{#each plan.blocked_reasons as reason}<p>{reason}</p>{/each}</div>{/if}
    <ul class="candidate-list">{#each plan.candidates as candidate}<li class:protected={candidate.protected}><div><strong>{candidate.record_type}</strong><span>{candidate.record_id}</span></div><div class="candidate-action">{#if candidate.protected}<span>Protected from physical deletion</span>{:else}<span>{formatBytes(candidate.bytes)}</span><Button variant="ghost" size="sm" type="button" disabled={busy} onclick={() => ondelete(candidate.record_type, candidate.record_id)}>Review permanent deletion</Button>{/if}</div><p>{candidate.reason}</p></li>{/each}</ul>
  {/if}
  {#if result}<div class:result-warning={result.status !== "succeeded"} class="result" role={result.status === "succeeded" ? "status" : "alert"} aria-live="polite"><strong>Cleanup {result.status}</strong><span>{result.removed_count} application-owned records removed · {formatBytes(result.removed_bytes)}</span>{#if result.status === "partial"}<p>Some eligible records could not be removed. Protected and external data remains intact.</p>{/if}{#each result.partial_failures as failure}<p>{failure}</p>{/each}</div>{/if}
</section>

<style>
  .cleanup-card { min-width:0; height:100%; padding:20px; border:1px solid var(--color-border); background:var(--color-surface); }
  .section-heading { display:flex; justify-content:space-between; gap:14px; align-items:flex-start; }
  .section-heading > div { min-width:0; }
  .eyebrow { margin:0 0 8px; color:var(--color-accent-strong); font:700 10px var(--font-mono); letter-spacing:.12em; text-transform:uppercase; }
  .cleanup-card h2 { margin:0; font-size:20px; overflow-wrap:anywhere; }
  .scope-label { flex:0 0 auto; color:var(--color-info); font:700 11px var(--font-mono); }
  .cleanup-controls { display:grid; grid-template-columns:minmax(0,1.2fr) minmax(280px,.8fr); gap:18px; align-items:end; }
  .description { color:var(--color-text-muted); font-size:13px; line-height:1.5; }
  .scope { display:grid; gap:7px; margin:0 0 14px; color:var(--color-text-muted); font:11px var(--font-mono); }
  .scope select { width:100%; min-width:0; min-height:40px; padding:0 10px; border:1px solid var(--color-border); color:var(--color-text); background:var(--color-bg); font:inherit; }
  .actions { display:flex; gap:10px; flex-wrap:wrap; }
  .plan-summary { display:flex; flex-wrap:wrap; gap:14px; margin-top:18px; padding:12px; color:var(--color-text-muted); background:var(--color-surface-raised); font-size:12px; }
  .plan-summary strong { color:var(--color-text); }
  .candidate-list { display:grid; gap:8px; min-width:0; margin:16px 0 0; padding:0; list-style:none; }
  .candidate-list li { display:grid; grid-template-columns:minmax(0,1fr) minmax(0,auto); gap:4px 12px; min-width:0; padding:10px 0; border-top:1px solid var(--color-border); font-size:12px; }
  .candidate-list li > div { display:grid; gap:3px; min-width:0; }
  .candidate-list li div span { color:var(--color-text-muted); font:11px var(--font-mono); overflow-wrap:anywhere; }
  .candidate-action { display:flex !important; align-items:center; justify-content:flex-end; gap:8px; min-width:0; flex-wrap:wrap; }
  .candidate-action :global(.button) { max-width:100%; white-space:normal; text-align:left; }
  .candidate-list p { grid-column:1 / -1; min-width:0; margin:0; color:var(--color-text-muted); overflow-wrap:anywhere; }
  .candidate-list .protected { color:var(--color-warning); }
  .blocked,.result { margin-top:16px; padding:11px; border-left:3px solid var(--color-warning); background:color-mix(in srgb,var(--color-warning) 10%,var(--color-surface)); font-size:12px; overflow-wrap:anywhere; }
  .blocked p,.result p { margin:4px 0 0; }
  .result { border-color:var(--color-success); background:color-mix(in srgb,var(--color-success) 10%,var(--color-surface)); }
  .result-warning { border-color:var(--color-warning); background:color-mix(in srgb,var(--color-warning) 10%,var(--color-surface)); }
  .result strong { margin-right:12px; text-transform:capitalize; }
  @media (max-width:600px) {
    .cleanup-card { padding:16px; }
    .cleanup-controls { grid-template-columns:1fr; gap:0; }
    .scope { margin-top:18px; }
    .section-heading { flex-direction:column; }
    .candidate-list li { grid-template-columns:1fr; }
    .candidate-action { align-items:flex-start; justify-content:flex-start; flex-direction:column; }
  }
</style>
