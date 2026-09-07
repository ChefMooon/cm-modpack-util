<script lang="ts">
  import Button from "../ui/Button.svelte";
  import type { ApplicationProjectMetadata } from "../../lib/domain";

  let {
    metadataDraft,
    tagsText = $bindable(""),
    busy,
    onsave,
  }: {
    metadataDraft: ApplicationProjectMetadata | null;
    tagsText?: string;
    busy: boolean;
    onsave: () => void | Promise<void>;
  } = $props();
</script>

<div id="settings-panel" class="settings-panel" role="tabpanel" aria-labelledby="settings-tab">
  <div class="detail-heading"><p class="eyebrow">Settings</p><h2>Application-owned metadata</h2><p>Packwiz evidence remains read-only. Save these fields together for the focused project.</p></div>
  {#if metadataDraft}
    <div class="owned-fields">
      <label class="field">Display name<input bind:value={metadataDraft.display_name} /></label>
      <label class="field">Theme / color<input value={metadataDraft.theme ?? ""} oninput={(event) => (metadataDraft!.theme = event.currentTarget.value || null)} /></label>
      <label class="field wide">Tags<input value={tagsText} placeholder="client, favorite" oninput={(event) => (tagsText = event.currentTarget.value)} /></label>
      <label class="field wide">Description<textarea rows="4" value={metadataDraft.description ?? ""} oninput={(event) => (metadataDraft!.description = event.currentTarget.value || null)}></textarea></label>
      <label class="favorite"><input type="checkbox" bind:checked={metadataDraft.favorite} /> Favorite project</label>
    </div>
    <div class="modal-actions"><Button variant="primary" type="button" loading={busy} onclick={onsave}>Save details</Button></div>
  {/if}
</div>

<style>
  .settings-panel { display:grid; align-content:center; min-height:360px; padding:32px; }
  .detail-heading { padding:28px 24px 10px; }.detail-heading p:not(.eyebrow) { margin-bottom:0; color:var(--color-text-muted); line-height:1.6; }.eyebrow { margin:0 0 10px; color:var(--color-accent-strong); font:700 10px var(--font-mono); letter-spacing:.12em; text-transform:uppercase; }.owned-fields { display:grid; grid-template-columns:1fr 1fr; gap:14px; margin:12px 24px 0; }.wide { grid-column:1 / -1; }.field { display:grid; gap:7px; color:var(--color-text); font-size:12px; }.field input { min-height:40px; padding:0 10px; border:1px solid var(--color-border); border-radius:var(--radius-sm); color:var(--color-text); background:var(--color-bg); font:inherit; }.field textarea { min-height:72px; padding:9px 10px; border:1px solid var(--color-border); border-radius:var(--radius-sm); color:var(--color-text); background:var(--color-bg); font:inherit; resize:vertical; }.favorite { display:flex; align-items:center; gap:8px; color:var(--color-text); font-size:12px; }.modal-actions { display:flex; justify-content:flex-end; gap:10px; margin:24px; }
  @media (max-width:700px) { .owned-fields { grid-template-columns:1fr; }.wide { grid-column:auto; } }
</style>
