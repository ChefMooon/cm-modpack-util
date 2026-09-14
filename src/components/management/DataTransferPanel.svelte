<script lang="ts">
  import Button from "../ui/Button.svelte";
  import Modal from "../ui/Modal.svelte";
  import { useToast } from "../ui/toast/toast.svelte";
  import { commandErrorMessage, translateCommandError } from "../../lib/errors";
  import {
    chooseApplicationDataDestination,
    chooseApplicationDataSource,
    exportApplicationData,
    importApplicationData,
    previewApplicationData,
  } from "../../lib/data-transfer";
  import type {
    DataTransferCompression,
    DataTransferExportResult,
    DataTransferPreview,
    DataTransferRecordPreview,
  } from "../../lib/data-transfer";

  let { onmerge }: { onmerge: () => void | Promise<void> } = $props();

  let compression = $state<DataTransferCompression>("json");
  let preview = $state<DataTransferPreview | null>(null);
  let selected = $state<string[]>([]);
  let exportResult = $state<DataTransferExportResult | null>(null);
  let busy = $state(false);
  let error = $state("");
  let errorCode = $state("");
  let statusMessage = $state("");
  const { toast } = useToast();

  const safeStatuses = new Set(["addition", "unavailable"]);

  function safeRecords(value: DataTransferPreview): DataTransferRecordPreview[] {
    return value.records.filter((record) => safeStatuses.has(record.status));
  }

  function isSafe(record: DataTransferRecordPreview): boolean {
    return safeStatuses.has(record.status);
  }

  function resetFeedback() {
    error = "";
    errorCode = "";
    statusMessage = "";
  }

  function showError(cause: unknown) {
    const commandError = translateCommandError(cause);
    errorCode = commandError.code;
    error = commandErrorMessage(cause);
  }

  async function exportData() {
    resetFeedback();
    busy = true;
    try {
      const destination = await chooseApplicationDataDestination(compression);
      if (!destination) {
        statusMessage = "Export cancelled.";
        return;
      }
      exportResult = await exportApplicationData(destination, compression);
      statusMessage = `Exported ${exportResult.record_count} application records.`;
      toast({ title: "Application data exported", severity: "success" });
    } catch (cause) {
      showError(cause);
      toast({ title: "Application data export failed", description: error, severity: "error" });
    } finally {
      busy = false;
    }
  }

  function toggleSelected(identity: string) {
    selected = selected.includes(identity)
      ? selected.filter((value) => value !== identity)
      : [...selected, identity];
  }

  function selectAllSafe() {
    if (!preview) return;
    selected = safeRecords(preview).map((record) => record.identity);
  }

  function clearSelection() {
    selected = [];
  }

  async function confirmImport() {
    if (!preview || selected.length === 0) return;
    resetFeedback();
    busy = true;
    try {
      const result = await importApplicationData({
        source: previewSource,
        preview_fingerprint: preview.fingerprint,
        selected,
        cancelled: false,
      });
      preview = null;
      selected = [];
      statusMessage = `Imported ${result.imported} records; ${result.skipped} records were skipped.`;
      toast({ title: "Application data merged", description: statusMessage, severity: "success" });
      await onmerge();
    } catch (cause) {
      showError(cause);
      toast({ title: "Application data import failed", description: error, severity: "error" });
    } finally {
      busy = false;
    }
  }

  let previewSource = $state("");

  async function chooseImportWithSource() {
    resetFeedback();
    busy = true;
    try {
      const source = await chooseApplicationDataSource();
      if (!source) {
        statusMessage = "Import cancelled.";
        return;
      }
      previewSource = source;
      preview = await previewApplicationData(source);
      selected = safeRecords(preview).map((record) => record.identity);
    } catch (cause) {
      preview = null;
      selected = [];
      showError(cause);
      toast({ title: "Import preview failed", description: error, severity: "error" });
    } finally {
      busy = false;
    }
  }

  function statusLabel(status: DataTransferRecordPreview["status"]): string {
    return status.replaceAll("_", " ");
  }

  function statusVariant(status: DataTransferRecordPreview["status"]): string {
    return isSafe({ family: "", identity: "", status, reason: null }) ? "safe" : "attention";
  }
</script>

<section class="transfer-card" aria-labelledby="transfer-title">
  <div class="heading">
    <div>
      <p class="eyebrow">Backup and transfer</p>
      <h2 id="transfer-title">Application data</h2>
    </div>
    <span class="scope-label">Preview before merge</span>
  </div>
  <p class="description">Export application-owned metadata for recovery or transfer. Packwiz project files, provider cache, and external changelog files remain outside the bundle.</p>
  <div class="actions">
    <label for="data-compression">Export format
      <select id="data-compression" bind:value={compression} disabled={busy}>
        <option value="json">Canonical JSON</option>
        <option value="gzip">Gzip JSON</option>
      </select>
    </label>
    <div class="button-row">
      <Button variant="secondary" type="button" loading={busy} onclick={exportData}>Export application data</Button>
      <Button variant="primary" type="button" loading={busy} onclick={chooseImportWithSource}>Import application data</Button>
    </div>
  </div>
  {#if exportResult}
    <div class="result" role="status" aria-live="polite">
      <strong>Export complete</strong>
      <span>{exportResult.destination} · SHA-256 {exportResult.payload_sha256}</span>
    </div>
  {/if}
  {#if error}
    <div class="error" role="alert" aria-live="assertive">
      <strong>{errorCode || "Transfer failed"}</strong>
      <span>{error}</span>
    </div>
  {/if}
  {#if statusMessage}<p class="status" role="status" aria-live="polite">{statusMessage}</p>{/if}
</section>

<Modal open={preview !== null} title="Review application-data import" size="wide" onclose={() => { if (!busy) { preview = null; selected = []; } }}>
  {#if preview}
    <div class="preview-content">
      <p class="preview-intro">Only additions and unavailable/disconnected modpack records can be selected. Conflicts and dependent records remain untouched.</p>
      <div class="preview-summary" role="status" aria-live="polite">
        <strong>{preview.safe_record_count} safe records</strong>
        <span>{preview.record_count} total</span>
        <span>{preview.records.filter((record) => record.status === "conflict").length} conflicts</span>
        <span>{preview.records.filter((record) => record.status === "dependent_skip").length} dependent skips</span>
      </div>
      <div class="selection-actions">
        <Button variant="quiet" size="sm" type="button" disabled={busy} onclick={selectAllSafe}>Select all safe records</Button>
        <Button variant="quiet" size="sm" type="button" disabled={busy} onclick={clearSelection}>Clear selection</Button>
      </div>
      <ul class="record-list">
        {#each preview.records as record (`${record.family}:${record.identity}`)}
          <li class:unsafe={!isSafe(record)}>
            {#if isSafe(record)}
              <label class="record-choice">
                <input type="checkbox" checked={selected.includes(record.identity)} disabled={busy} onchange={() => toggleSelected(record.identity)} />
                <span><strong>{record.family}</strong><small>{record.identity}</small></span>
              </label>
            {:else}
              <div class="record-choice"><span><strong>{record.family}</strong><small>{record.identity}</small></span></div>
            {/if}
            <span class={`record-status ${statusVariant(record.status)}`}>{statusLabel(record.status)}</span>
            {#if record.reason}<p>{record.reason}</p>{/if}
          </li>
        {/each}
      </ul>
      {#if preview.excluded.length}<p class="excluded"><strong>Excluded:</strong> {preview.excluded.join(", ")}</p>{/if}
      {#if error}<div class="error" role="alert" aria-live="assertive"><strong>{errorCode || "Import failed"}</strong><span>{error}</span></div>{/if}
      <div class="modal-actions">
        <Button variant="ghost" type="button" disabled={busy} onclick={() => { preview = null; selected = []; }}>Cancel import</Button>
        <Button variant="primary" type="button" disabled={selected.length === 0} loading={busy} onclick={confirmImport}>Merge selected safe records</Button>
      </div>
    </div>
  {/if}
</Modal>

<style>
  .transfer-card { min-width:0; height:100%; padding:20px; border:1px solid var(--color-border); background:var(--color-surface); }
  .heading { display:flex; justify-content:space-between; gap:14px; align-items:flex-start; }.eyebrow { margin:0 0 8px; color:var(--color-accent-strong); font:700 10px var(--font-mono); letter-spacing:.12em; text-transform:uppercase; }.heading h2 { margin:0; font-size:20px; }.scope-label { color:var(--color-info); font:700 11px var(--font-mono); }.description,.preview-intro { color:var(--color-text-muted); font-size:13px; line-height:1.5; }.actions { display:grid; gap:14px; margin-top:18px; }.actions label { display:grid; gap:7px; color:var(--color-text-muted); font:11px var(--font-mono); }.actions select { min-height:40px; padding:0 10px; border:1px solid var(--color-border); color:var(--color-text); background:var(--color-bg); font:inherit; }.button-row,.modal-actions,.selection-actions { display:flex; flex-wrap:wrap; gap:10px; }.result,.error,.status { display:flex; gap:12px; flex-wrap:wrap; margin-top:16px; padding:11px; font-size:12px; overflow-wrap:anywhere; }.result,.status { border-left:3px solid var(--color-success); background:color-mix(in srgb,var(--color-success) 10%,var(--color-surface)); }.error { border-left:3px solid var(--color-danger); background:color-mix(in srgb,var(--color-danger) 10%,var(--color-surface)); }.preview-content { min-width:0; }.preview-summary { display:flex; flex-wrap:wrap; gap:14px; margin:16px 0; padding:12px; background:var(--color-surface-raised); font-size:12px; }.record-list { display:grid; gap:0; max-height:420px; margin:14px 0; padding:0; overflow:auto; list-style:none; border-top:1px solid var(--color-border); }.record-list li { display:grid; grid-template-columns:minmax(0,1fr) auto; gap:4px 12px; padding:11px 0; border-bottom:1px solid var(--color-border); }.record-list li.unsafe { color:var(--color-text-muted); }.record-choice { display:flex; gap:10px; align-items:flex-start; min-width:0; }.record-choice input { margin-top:3px; }.record-choice span { display:grid; gap:3px; min-width:0; }.record-choice small { color:var(--color-text-muted); font:11px var(--font-mono); overflow-wrap:anywhere; }.record-status { align-self:start; padding:3px 6px; font:10px var(--font-mono); text-transform:capitalize; }.record-status.safe { color:var(--color-success); background:color-mix(in srgb,var(--color-success) 12%,var(--color-surface)); }.record-status.attention { color:var(--color-warning); background:color-mix(in srgb,var(--color-warning) 12%,var(--color-surface)); }.record-list p { grid-column:1 / -1; margin:0; color:var(--color-text-muted); font-size:12px; }.excluded { color:var(--color-text-muted); font-size:12px; }.modal-actions { justify-content:flex-end; margin-top:18px; padding-top:14px; border-top:1px solid var(--color-border); }.status { margin-bottom:0; }
  @media (max-width:600px) { .heading { flex-direction:column; }.button-row,.modal-actions { flex-direction:column; align-items:stretch; } }
</style>
