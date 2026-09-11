<script lang="ts">
  import WarningCircleIcon from "phosphor-svelte/lib/WarningCircleIcon";
  import Button from "../ui/Button.svelte";
  import type { ImpactPreview } from "../../lib/domain";

  let {
    preview,
    confirmation = "",
    busy = false,
    onconfirmationchange,
    onconfirm,
    oncancel,
  }: {
    preview: ImpactPreview;
    confirmation?: string;
    busy?: boolean;
    onconfirmationchange: (value: string) => void;
    onconfirm: () => void | Promise<void>;
    oncancel: () => void;
  } = $props();

  const requiresConfirmation = $derived(preview.requires_confirmation);
</script>

<section class="preview" aria-labelledby="impact-title">
  <div class="preview-heading">
    <div>
      <p class="eyebrow">Impact preview</p>
      <h2 id="impact-title">{preview.action.replaceAll("_", " ")}</h2>
      <p class="record-id">{preview.target_type} · {preview.target_id}</p>
    </div>
    <span class:blocked={!preview.eligible} class:ready={preview.eligible} class="status" role="status">
      {preview.eligible ? "Ready" : "Blocked"}
    </span>
  </div>

  {#if preview.blocked_reasons.length}
    <div class="callout blocked-callout" role="alert">
      <WarningCircleIcon size={18} aria-hidden="true" />
      <div><strong>This action is blocked.</strong>{#each preview.blocked_reasons as reason}<p>{reason}</p>{/each}</div>
    </div>
  {/if}

  <div class="impact-grid">
    <div><span>Direct references</span><strong>{preview.direct_references.length}</strong></div>
    <div><span>Retained evidence</span><strong>{preview.retained_evidence.length}</strong></div>
    <div><span>Removable records</span><strong>{preview.removable_records.length}</strong></div>
    <div><span>External boundaries</span><strong>{preview.external_boundaries.length}</strong></div>
  </div>

  {#if preview.direct_references.length}
    <section class="detail-section" aria-labelledby="references-title">
      <h3 id="references-title">Protected relationships</h3>
      <ul>{#each preview.direct_references as reference}<li><strong>{reference.relationship}</strong> · {reference.reason}<small>{reference.source_type} · {reference.source_id}</small></li>{/each}</ul>
    </section>
  {/if}
  {#if preview.retained_evidence.length}
    <section class="detail-section" aria-labelledby="evidence-title">
      <h3 id="evidence-title">Evidence that stays retained</h3>
      <ul>{#each preview.retained_evidence as item}<li>{item}</li>{/each}</ul>
    </section>
  {/if}
  {#if preview.external_boundaries.length}
    <section class="detail-section protected" aria-labelledby="external-title">
      <h3 id="external-title">External resources never touched</h3>
      <ul>{#each preview.external_boundaries as item}<li>{item}</li>{/each}</ul>
    </section>
  {/if}
  {#if preview.orphan}
    <div class="callout warning" role="status"><strong>Orphaned application record</strong><p>{preview.orphan.reason}</p><p>Missing {preview.orphan.missing_parent_type} · {preview.orphan.missing_parent_id}</p></div>
  {/if}
  {#if preview.unavailable_destination}
    <div class="callout warning" role="status"><strong>Export destination unavailable</strong><p>{preview.unavailable_destination.destination}</p><p>{preview.unavailable_destination.diagnostic ?? preview.unavailable_destination.status}</p></div>
  {/if}

  {#if requiresConfirmation}
    <label class="confirmation">Type <code>DELETE {preview.target_id}</code> to confirm permanent deletion
      <input value={confirmation} oninput={(event) => onconfirmationchange(event.currentTarget.value)} autocomplete="off" spellcheck="false" aria-describedby="confirmation-help" aria-label={`Type DELETE ${preview.target_id} to confirm permanent deletion`} />
    </label>
    <p id="confirmation-help" class="help">The confirmation is target-specific and is checked again by Rust before mutation.</p>
  {/if}

  <div class="actions">
    <Button variant="ghost" type="button" onclick={oncancel}>Cancel</Button>
    <Button variant={requiresConfirmation ? "danger" : "primary"} type="button" disabled={!preview.eligible || (requiresConfirmation && confirmation !== `DELETE ${preview.target_id}`)} loading={busy} onclick={onconfirm}>{requiresConfirmation ? "Permanently delete" : "Apply action"}</Button>
  </div>
</section>

<style>
  .preview { display:grid; gap:18px; }.preview-heading { display:flex; justify-content:space-between; gap:16px; align-items:flex-start; }.eyebrow { margin:0 0 8px; color:var(--color-accent-strong); font:700 10px var(--font-mono); letter-spacing:.12em; text-transform:uppercase; }.preview h2 { margin:0; font-size:22px; text-transform:capitalize; }.record-id,.help { margin:8px 0 0; color:var(--color-text-muted); font:12px var(--font-mono); overflow-wrap:anywhere; }.status { padding:5px 8px; color:var(--color-success); background:color-mix(in srgb,var(--color-success) 14%,var(--color-surface)); font:700 11px var(--font-mono); text-transform:uppercase; }.status.blocked { color:var(--color-danger); background:color-mix(in srgb,var(--color-danger) 12%,var(--color-surface)); }.impact-grid { display:grid; grid-template-columns:repeat(4,1fr); gap:1px; background:var(--color-border); border:1px solid var(--color-border); }.impact-grid div { display:grid; gap:6px; padding:12px; background:var(--color-surface); }.impact-grid span { color:var(--color-text-muted); font-size:11px; }.impact-grid strong { font-size:18px; }.detail-section { border-top:1px solid var(--color-border); padding-top:14px; }.detail-section h3 { margin:0 0 9px; font-size:14px; }.detail-section ul { display:grid; gap:8px; margin:0; padding-left:18px; }.detail-section li { color:var(--color-text-muted); font-size:13px; }.detail-section small { display:block; margin-top:3px; color:var(--color-text-subtle); font:11px var(--font-mono); }.protected li { color:var(--color-warning); }.callout { display:flex; gap:10px; align-items:flex-start; padding:12px; border-left:3px solid var(--color-warning); color:var(--color-text-muted); background:color-mix(in srgb,var(--color-warning) 10%,var(--color-surface)); }.callout strong { color:var(--color-text); }.callout p { margin:5px 0 0; }.blocked-callout { border-color:var(--color-danger); background:color-mix(in srgb,var(--color-danger) 10%,var(--color-surface)); }.confirmation { display:grid; gap:8px; color:var(--color-text); font-size:13px; }.confirmation code { font-family:var(--font-mono); }.confirmation input { min-height:42px; padding:0 10px; border:1px solid var(--color-border); color:var(--color-text); background:var(--color-bg); font:13px var(--font-mono); }.actions { display:flex; justify-content:flex-end; gap:10px; }.help { font-size:11px; }.warning { display:block; }.warning p { overflow-wrap:anywhere; }@media (max-width:700px) { .impact-grid { grid-template-columns:1fr 1fr; }.preview-heading { flex-direction:column; }.actions { flex-wrap:wrap; } }
</style>
