<script lang="ts">
  import ClockCounterClockwiseIcon from "phosphor-svelte/lib/ClockCounterClockwiseIcon";
  import Header from "../../components/header/Header.svelte";
  import { onMount } from "svelte";
  import { getModpackOperationHistory, listModpacks, listModpackSnapshots } from "../../lib/modpacks";
  import type { OperationAttempt, ModpackRecord, SnapshotRecord } from "../../lib/domain";

  let snapshots = $state<SnapshotRecord[]>([]);
  let operations = $state<OperationAttempt[]>([]);
  let modpacks = $state<ModpackRecord[]>([]);
  let loading = $state(true);
  let error = $state("");

  async function loadHistory() {
    loading = true;
    error = "";
    try {
      modpacks = await listModpacks();
      const results = await Promise.all(modpacks.map((modpack) => listModpackSnapshots(modpack.id)));
      snapshots = results.flat().sort((left, right) => right.created_at.localeCompare(left.created_at));
      const operationResults = await Promise.all(modpacks.map((modpack) => getModpackOperationHistory(modpack.id)));
      operations = operationResults.flat().sort((left, right) => right.created_at.localeCompare(left.created_at));
    } catch (cause) {
      error = cause instanceof Error ? cause.message : "Snapshot history could not be loaded.";
    } finally {
      loading = false;
    }
  }

  onMount(() => void loadHistory());

  function modpackName(modpackId: string) {
    return modpacks.find((modpack) => modpack.id === modpackId)?.application.display_name ?? modpackId;
  }
</script>

<svelte:head>
  <title>Activity · CM Modpack Util</title>
  <meta name="description" content="Review CM Modpack Util activity." />
</svelte:head>

<main class="activity-shell">
  <Header />
  <section class="page-heading" aria-labelledby="activity-title">
    <p class="eyebrow">Activity</p>
    <h1 id="activity-title">Operation history</h1>
    <p>Durable discovery snapshots and their safety outcomes.</p>
  </section>
  {#if error}<section class="error-state" role="alert"><p>{error}</p><button type="button" onclick={loadHistory}>Retry</button></section>{/if}
  {#if loading}
    <section class="empty-state" aria-live="polite"><p>Loading snapshot history...</p></section>
  {:else if snapshots.length === 0 && operations.length === 0}
  <section class="empty-state" aria-labelledby="empty-title">
    <div class="empty-icon" aria-hidden="true"><ClockCounterClockwiseIcon size={34} weight="duotone" /></div>
    <p class="eyebrow">No operations yet</p>
    <h2 id="empty-title">No snapshots yet</h2>
    <p>Run a safe discovery check from a registered modpack to create its first durable review record.</p>
  </section>
  {:else}
    {#if operations.length}<section class="snapshot-list" aria-labelledby="operation-title">
      <h2 id="operation-title">Mutation attempts</h2>
      {#each operations as operation (operation.id)}
        <article class="snapshot-item operation-item">
          <div class="snapshot-summary"><strong>{modpackName(operation.modpack_id)}</strong><span>{operation.kind}</span><span data-outcome={operation.outcome ?? "pending"}>{operation.outcome ?? operation.status}</span></div>
          <div class="snapshot-meta"><span>{operation.verification?.verified ? "Verified after re-read" : "Verification not complete"}</span><small>{operation.created_at}</small></div>
          <p>{operation.error?.message ?? operation.verification?.observed_state ?? "No additional diagnostic"}</p>
        </article>
      {/each}
    </section>{/if}
    <section class="snapshot-list" aria-labelledby="snapshot-title">
      <h2 id="snapshot-title">Snapshot history</h2>
      {#each snapshots as snapshot (snapshot.id)}
        <article class="snapshot-item">
          <div class="snapshot-summary"><strong>{modpackName(snapshot.modpack_id)}</strong><span>{snapshot.outcome.replaceAll("_", " ")}</span><span>{snapshot.lifecycle}</span></div>
          <div class="snapshot-meta"><span>{snapshot.candidates.length} candidates</span><span>{snapshot.decisions.length} decisions</span><small>{snapshot.created_at}</small></div>
          <p>{snapshot.id}</p>
          {#if snapshot.predecessor_id}<a href={`/?snapshot=${encodeURIComponent(snapshot.predecessor_id)}`}>View predecessor</a>{/if}
          <a class="review-link" href={`/?snapshot=${encodeURIComponent(snapshot.id)}`}>Open review</a>
        </article>
      {/each}
    </section>
  {/if}
</main>

<style>
  .activity-shell { min-height: 100vh; padding: 0 42px 42px; background: var(--color-bg); }
  .page-heading, .empty-state { max-width: 1060px; margin-right: auto; margin-left: auto; }.page-heading { margin-top: 58px; margin-bottom: 34px; }.eyebrow { margin: 0 0 10px; color: var(--color-accent-strong); font: 700 10px var(--font-mono); letter-spacing: .12em; text-transform: uppercase; }h1, h2 { margin: 0; }h1 { font-size: clamp(28px, 5vw, 44px); line-height: 1.05; }.page-heading > p:last-child { margin: 14px 0 0; color: var(--color-text-muted); font-size: 15px; line-height: 1.55; }
  .empty-state { display: grid; justify-items: center; padding: 72px 28px; border: 1px solid var(--color-border); background: var(--color-surface); box-shadow: 5px 5px 0 var(--color-text); text-align: center; }.empty-icon { display: grid; width: 68px; height: 68px; margin-bottom: 24px; place-items: center; border: 1px solid var(--color-accent); color: var(--color-accent-strong); background: color-mix(in srgb, var(--color-accent) 12%, var(--color-surface)); }.empty-state h2 { margin-bottom: 12px; font-size: 20px; }.empty-state > p:last-child { max-width: 580px; margin: 0; color: var(--color-text-muted); line-height: 1.6; }
  .snapshot-list { max-width: 1060px; margin: 0 auto; }.snapshot-list h2 { margin-bottom: 14px; }.snapshot-item { display: flex; justify-content: space-between; gap: 16px; padding: 16px; border-top: 1px solid var(--color-border); background: var(--color-surface); }.snapshot-item div { display: flex; gap: 12px; }.snapshot-item span, .snapshot-item p, .snapshot-item small { color: var(--color-text-muted); }.snapshot-item p { margin: 0; font-family: var(--font-mono); font-size: 12px; }
  .snapshot-summary, .snapshot-meta { display: flex; flex-wrap: wrap; align-items: center; gap: 10px; }.snapshot-summary span { text-transform: capitalize; }.snapshot-meta { font-size: 12px; }.snapshot-item a { color: var(--color-accent-strong); font-size: 12px; }.review-link { font-weight: 700; }.error-state { max-width: 1060px; margin: 0 auto 18px; padding: 14px 16px; border-left: 3px solid var(--color-danger); background: var(--color-surface); }.error-state p { display: inline; margin: 0 16px 0 0; }.error-state button { border: 0; color: var(--color-accent-strong); background: transparent; font: inherit; font-weight: 700; cursor: pointer; }
    [data-outcome="complete"] { color: var(--color-success); }
    .operation-item [data-outcome="failed"], .operation-item [data-outcome="partial"], .operation-item [data-outcome="cancelled"], .operation-item [data-outcome="indeterminate"] { color: var(--color-danger); }
  @media (max-width: 700px) { .activity-shell { padding: 0 20px 28px; }.page-heading { margin-top: 36px; }.empty-state { padding: 52px 20px; } }
</style>
