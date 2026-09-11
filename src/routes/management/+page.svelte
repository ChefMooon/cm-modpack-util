<script lang="ts">
  import { onMount } from "svelte";
  import AppShell from "../../components/layout/AppShell.svelte";
  import Button from "../../components/ui/Button.svelte";
  import Modal from "../../components/ui/Modal.svelte";
  import CleanupPanel from "../../components/management/CleanupPanel.svelte";
  import ImpactPreview from "../../components/management/ImpactPreview.svelte";
  import StorageSummary from "../../components/management/StorageSummary.svelte";
  import { commandErrorMessage } from "../../lib/errors";
  import {
    applyLifecycleAction,
    executeCleanup,
    getStorageReport,
    listModpacks,
    previewCleanup,
    previewLifecycleAction,
  } from "../../lib/modpacks";
  import type {
    CleanupPlan,
    CleanupResult,
    CleanupScope,
    ImpactPreview as ImpactPreviewRecord,
    LifecycleAction,
    LifecycleRecordType,
    ModpackRecord,
    StorageReport,
  } from "../../lib/domain";

  let modpacks = $state<ModpackRecord[]>([]);
  let storage = $state<StorageReport | null>(null);
  let cleanupPlan = $state<CleanupPlan | null>(null);
  let cleanupResult = $state<CleanupResult | null>(null);
  let scope = $state<CleanupScope>("provider_cache_and_obsolete_exports");
  let preview = $state<ImpactPreviewRecord | null>(null);
  let confirmation = $state("");
  let loading = $state(true);
  let busy = $state(false);
  let cleanupBusy = $state(false);
  let error = $state("");
  let statusMessage = $state("");
  let previewTarget = $state<{ type: LifecycleRecordType; id: string; action: LifecycleAction } | null>(null);

  onMount(() => {
    void (async () => {
      await loadManagement();
      const params = new URLSearchParams(window.location.search);
      const target = params.get("target");
      const action = params.get("action");
      if (target && (action === "archive" || action === "restore")) {
        await openPreview("modpack", target, action);
      }
    })();
  });

  async function loadManagement() {
    loading = true;
    error = "";
    statusMessage = "";
    try {
      [modpacks, storage] = await Promise.all([listModpacks(), getStorageReport()]);
    } catch (cause) {
      error = commandErrorMessage(cause);
    } finally {
      loading = false;
    }
  }

  async function openPreview(type: LifecycleRecordType, id: string, action: LifecycleAction) {
    busy = true;
    error = "";
    statusMessage = "";
    confirmation = "";
    previewTarget = { type, id, action };
    try {
      preview = await previewLifecycleAction(type, id, action);
    } catch (cause) {
      preview = null;
      error = commandErrorMessage(cause);
    } finally {
      busy = false;
    }
  }

  async function applyPreview() {
    if (!preview || !previewTarget) return;
    busy = true;
    error = "";
    try {
      const result = await applyLifecycleAction({
        target_type: previewTarget.type,
        target_id: previewTarget.id,
        action: previewTarget.action,
        scope: preview.scope,
        preview_fingerprint: preview.fingerprint,
        confirmation: confirmation || null,
      });
      preview = null;
      previewTarget = null;
      confirmation = "";
      await loadManagement();
      if (result.status === "succeeded") {
        statusMessage = result.message;
      } else {
        error = result.message;
      }
    } catch (cause) {
      error = commandErrorMessage(cause);
    } finally {
      busy = false;
    }
  }

  async function previewCleanupScope() {
    cleanupBusy = true;
    error = "";
    statusMessage = "";
    try {
      cleanupPlan = await previewCleanup(scope);
      cleanupResult = null;
    } catch (cause) {
      error = commandErrorMessage(cause);
    } finally {
      cleanupBusy = false;
    }
  }

  async function runCleanup() {
    if (!cleanupPlan) return;
    cleanupBusy = true;
    error = "";
    statusMessage = "";
    try {
      cleanupResult = await executeCleanup({ scope, preview_fingerprint: cleanupPlan.fingerprint });
      cleanupPlan = null;
      storage = await getStorageReport();
      statusMessage = `Cleanup ${cleanupResult.status}: ${cleanupResult.removed_count} application-owned records removed.`;
    } catch (cause) {
      error = commandErrorMessage(cause);
    } finally {
      cleanupBusy = false;
    }
  }

  function modpackLabel(modpack: ModpackRecord): string {
    return `${modpack.application.display_name} · ${modpack.application.lifecycle}`;
  }
</script>

<svelte:head>
  <title>Management · CM Modpack Util</title>
  <meta name="description" content="Manage application-owned records and removable data safely." />
</svelte:head>

<AppShell>
  <section class="page-heading" aria-labelledby="management-title">
    <p class="eyebrow">Management</p>
    <h1 id="management-title">Data management and cleanup</h1>
    <p>Preview every application-owned action. Registered Packwiz directories, Git history, and user export files remain protected.</p>
  </section>

  {#if error}<section class="error-state" role="alert"><strong>Action needs attention</strong><p>{error}</p><Button variant="ghost" size="sm" type="button" onclick={() => (error = "")}>Dismiss</Button></section>{/if}
  {#if statusMessage}<section class="success-state" role="status" aria-live="polite"><p>{statusMessage}</p></section>{/if}
  {#if loading}<section class="state" role="status" aria-live="polite"><p>Loading management data...</p></section>
  {:else}
    <section class="management-grid">
      {#if storage}<StorageSummary report={storage} />{/if}
      <CleanupPanel scope={scope} plan={cleanupPlan} result={cleanupResult} busy={cleanupBusy} onscopechange={(value) => { scope = value; cleanupPlan = null; }} onpreview={previewCleanupScope} onexecute={runCleanup} ondelete={(recordType, recordId) => openPreview(recordType, recordId, "permanent_delete")} />
    </section>

    <section class="records" aria-labelledby="records-title">
      <div class="section-heading"><div><p class="eyebrow">Application records</p><h2 id="records-title">Lifecycle actions</h2></div><p>Archived records are hidden from normal workflows but remain available here for status review.</p></div>
      {#if modpacks.length === 0}<p class="empty" role="status">No registered application records are available.</p>{:else}<div class="record-list">{#each modpacks as modpack (modpack.id)}<article class="record"><div><h3>{modpack.application.display_name}</h3><p>{modpackLabel(modpack)}</p><small>{modpack.id}</small></div><div class="record-actions">{#if modpack.application.lifecycle === "archived"}<Button variant="secondary" size="sm" type="button" disabled={busy} onclick={() => openPreview("modpack", modpack.id, "restore")}>Preview restore</Button>{:else}<Button variant="secondary" size="sm" type="button" disabled={busy} onclick={() => openPreview("modpack", modpack.id, "archive")}>Preview archive</Button>{/if}<Button variant="ghost" size="sm" type="button" disabled={busy} onclick={() => openPreview("modpack", modpack.id, "detach")}>Preview detach</Button></div></article>{/each}</div>{/if}
    </section>
  {/if}
</AppShell>

<Modal open={preview !== null} title="Review management action" size="wide" onclose={() => { preview = null; previewTarget = null; }}>
  {#if preview}<ImpactPreview preview={preview} {confirmation} busy={busy} onconfirmationchange={(value) => (confirmation = value)} onconfirm={applyPreview} oncancel={() => { preview = null; previewTarget = null; }} />{/if}
</Modal>

<style>
  .page-heading,.management-grid,.records { max-width:1060px; margin-right:auto; margin-left:auto; }.page-heading { margin-top:25px; margin-bottom:24px; }.eyebrow { margin:0 0 7px; color:var(--color-accent-strong); font:700 10px var(--font-mono); letter-spacing:.12em; text-transform:uppercase; }.page-heading h1 { margin:0; font-size:clamp(24px,4vw,36px); line-height:1.04; }.page-heading > p:last-child { max-width:680px; margin:10px 0 0; color:var(--color-text-muted); font-size:13px; line-height:1.45; }.management-grid { display:grid; grid-template-columns:repeat(2,minmax(0,1fr)); gap:18px; align-items:stretch; }.records { margin-top:22px; padding:20px; border:1px solid var(--color-border); background:var(--color-surface); }.section-heading { display:flex; justify-content:space-between; gap:18px; align-items:flex-start; }.section-heading h2 { margin:0; font-size:20px; }.section-heading > p { max-width:390px; margin:0; color:var(--color-text-muted); font-size:12px; line-height:1.5; }.record-list { margin-top:18px; border-top:1px solid var(--color-border); }.record { display:flex; justify-content:space-between; gap:16px; padding:15px 0; border-bottom:1px solid var(--color-border); }.record h3 { margin:0 0 5px; font-size:15px; }.record p,.record small { margin:0; color:var(--color-text-muted); font:11px var(--font-mono); }.record small { display:block; margin-top:6px; overflow-wrap:anywhere; }.record-actions { display:flex; flex-wrap:wrap; justify-content:flex-end; gap:8px; align-items:center; }.error-state,.success-state { max-width:1060px; margin:0 auto 18px; padding:13px 16px; background:var(--color-surface); }.error-state { border-left:3px solid var(--color-danger); }.success-state { border-left:3px solid var(--color-success); }.error-state p,.success-state p { display:inline; margin:0 14px; color:var(--color-text-muted); }.state,.empty { max-width:1060px; margin:0 auto; padding:30px; color:var(--color-text-muted); background:var(--color-surface); text-align:center; }.empty { margin-top:16px; }@media (max-width:760px) { .page-heading { margin-top:25px; }.management-grid { grid-template-columns:1fr; }.section-heading,.record { flex-direction:column; }.record-actions { justify-content:flex-start; } }
</style>
