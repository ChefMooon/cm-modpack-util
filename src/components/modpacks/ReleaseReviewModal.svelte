<script lang="ts">
  import Button from "../ui/Button.svelte";
  import Modal from "../ui/Modal.svelte";
  import ReleaseEvidenceSummary from "./ReleaseEvidenceSummary.svelte";
  import type { ChangelogArtifact, ChangelogRevision, Evidence, InventoryEntry, ModpackRecord, ReleaseWorkspace, ReleaseWorkspaceWatcherStatus, SnapshotRecord } from "../../lib/domain";

  let {
    open = false,
    modpack,
    snapshot,
    workspace,
    changelogArtifacts = [],
    changelogRevisions = [],
    selectedChangelogRevision = null,
    changelogDraft = $bindable(""),
    changelogBusy = false,
    inventory = [],
    observations = [],
    watcherStatus = "stopped",
    busy = false,
    error = "",
    onclose,
    ondecision,
    onapply,
    onabandon,
    onunlinkSnapshot,
    onrebase,
    onfinalize,
    onpublish,
    ongenerateChangelog,
    oncreateBlankChangelog,
    onselectChangelog,
    onsaveChangelogRevision,
    onstartWatcher,
    onstopWatcher,
    ondiscoverUpdates,
    onopenPage,
    onopenLink,
    onpin,
    pinningEntry = null,
  }: {
    open?: boolean;
    modpack: ModpackRecord | null;
    snapshot: SnapshotRecord | null;
    workspace: ReleaseWorkspace | null;
    changelogArtifacts?: ChangelogArtifact[];
    changelogRevisions?: ChangelogRevision[];
    selectedChangelogRevision?: ChangelogRevision | null;
    changelogDraft?: string;
    changelogBusy?: boolean;
    inventory?: InventoryEntry[];
    observations?: import("../../lib/domain").ReleaseWorkspaceObservation[];
    watcherStatus?: ReleaseWorkspaceWatcherStatus;
    busy?: boolean;
    error?: string;
    onclose?: () => void;
    ondecision: (candidateId: string, decision: "selected" | "skipped" | "deferred" | "blocked" | "pinned" | "uncertain") => void | Promise<void>;
    onapply: () => void | Promise<void>;
    onabandon: () => void | Promise<void>;
    onunlinkSnapshot: () => void | Promise<void>;
    onrebase: () => void | Promise<void>;
    onfinalize: () => void | Promise<void>;
    onpublish: () => void | Promise<void>;
    ongenerateChangelog: () => void | Promise<void>;
    oncreateBlankChangelog: () => void | Promise<void>;
    onselectChangelog: (revisionId: string) => void | Promise<void>;
    onsaveChangelogRevision: () => void | Promise<void>;
    onstartWatcher: () => void | Promise<void>;
    onstopWatcher: () => void | Promise<void>;
    ondiscoverUpdates: () => void | Promise<void>;
    onopenPage?: (entry: InventoryEntry) => void | Promise<void>;
    onopenLink?: (url: string) => void | Promise<void>;
    onpin?: (entry: InventoryEntry, pin: boolean) => void;
    pinningEntry?: string | null;
  } = $props();

  let showAbandon = $state(false);

  const terminalLifecycles = ["finalized", "published", "withdrawn", "abandoned"];

  function watcherStatusLabel(status: ReleaseWorkspaceWatcherStatus): string {
    return {
      stopped: "Not observing",
      starting: "Starting observer",
      observing: "Observing files",
      stopping: "Stopping observer",
      error: "Observer unavailable",
    }[status];
  }

  function evidenceLabel<T>(value: Evidence<T>): string {
    if (typeof value === "object" && value !== null) {
      if ("observed" in value) return String(value.observed);
      if ("malformed" in value) return `Malformed: ${value.malformed.message}`;
    }
    return value;
  }
</script>

<Modal open={open} onclose={onclose} size="wide" title={workspace ? `Release Review · ${workspace.metadata.name}` : `Start release · ${modpack?.application.display_name ?? "Modpack"}`}>
  {#if error}<p class="error" role="alert">{error}</p>{/if}
  {#if !workspace}
    <p class="lede">This workspace starts from the immutable snapshot baseline and remains recoverable across navigation. Packwiz files are not changed until an explicit apply action.</p>
  {:else}
    <header class="workspace-header">
      <div><p class="eyebrow">{workspace.phase} phase</p><h2>{workspace.metadata.name}</h2><p>{workspace.primary_next_action.replaceAll("_", " ")}</p></div>
      <div class="state"><strong>{workspace.lifecycle.replaceAll("_", " ")}</strong><span>{workspace.evidence_freshness} evidence</span></div>
    </header>
    {#if workspace.blocking_reason}<div class="blocking" role="alert"><strong>Action required</strong><span>{workspace.blocking_reason.replaceAll("_", " ")}</span></div>{/if}
    <div class="facts" role="status"><span><b>Baseline</b>{workspace.baseline_origin.replaceAll("_", " ")}</span><span><b>Source</b>{workspace.source_snapshot_id ?? workspace.baseline_release_id ?? "Current project"}</span><span><b>Evidence</b>{workspace.evidence_status}</span><span><b>Publication</b>{workspace.publication_status}</span><span><b>Activity</b>{workspace.activity.length}</span></div>
    <ReleaseEvidenceSummary candidates={workspace.candidates} {inventory} baselineEntries={workspace.baseline_capture?.state.entries ?? []} baselineFingerprint={workspace.baseline_capture?.state.source_fingerprint ?? null} {observations} {evidenceLabel} {busy} ondecision={ondecision} ondiscoverUpdates={ondiscoverUpdates} {onopenPage} {onopenLink} {onpin} {pinningEntry} />
    <section class="workspace-section" aria-labelledby="recovery-title"><div class="section-heading"><div><p class="eyebrow">Evidence coordination</p><h3 id="recovery-title">Terminal edits and recovery</h3></div><div class="watch-actions"><span class:watching={watcherStatus === "observing"} class:watch-error={watcherStatus === "error"} class="watch-status" role="status" aria-live="polite">{watcherStatusLabel(watcherStatus)}</span><Button size="sm" variant="quiet" type="button" disabled={busy || terminalLifecycles.includes(workspace.lifecycle) || watcherStatus === "starting" || watcherStatus === "observing"} loading={watcherStatus === "starting"} onclick={onstartWatcher}>Observe files</Button><Button size="sm" variant="quiet" type="button" disabled={busy || terminalLifecycles.includes(workspace.lifecycle) || watcherStatus === "stopped" || watcherStatus === "stopping" || watcherStatus === "error"} loading={watcherStatus === "stopping"} onclick={onstopWatcher}>Stop observing</Button></div></div>{#if workspace.activity.length}<ul class="activity">{#each workspace.activity.slice(-5) as item (item.id)}<li><span>{item.event_type.replaceAll("_", " ")}</span><small>{item.message}</small></li>{/each}</ul>{:else}<p class="muted">No workspace activity has been recorded yet.</p>{/if}</section>
    <section class="workspace-section" aria-labelledby="changelog-title"><div class="section-heading"><div><p class="eyebrow">Proposed changelog</p><h3 id="changelog-title">Choose the revision to finalize</h3></div><div class="watch-actions"><Button size="sm" variant="quiet" type="button" disabled={busy || changelogBusy} loading={changelogBusy} onclick={ongenerateChangelog}>Generate proposal</Button><Button size="sm" variant="quiet" type="button" disabled={busy || changelogBusy} onclick={oncreateBlankChangelog}>Blank proposal</Button></div></div>{#if changelogRevisions.length}<div class="revision-list">{#each changelogRevisions as revision (revision.id)}<label class:selected={selectedChangelogRevision?.id === revision.id}><input type="radio" name="workspace-changelog" checked={selectedChangelogRevision?.id === revision.id} onchange={() => onselectChangelog(revision.id)} /><span><strong>Proposed revision</strong><small>{revision.created_at} · {revision.content.length} characters</small></span></label>{/each}</div>{:else}<p class="muted">No proposed revision exists yet. Generate one from workspace evidence or start with a blank proposal.</p>{/if}{#if selectedChangelogRevision}<div class="revision-editor"><div class="revision-preview"><span class="proposal-label">PROPOSED · NOT FINAL</span><pre>{selectedChangelogRevision.content}</pre></div><label class="editor-field"><span>Edit proposed revision</span><textarea rows="8" bind:value={changelogDraft}></textarea></label><Button size="sm" variant="quiet" type="button" disabled={busy || changelogBusy} onclick={onsaveChangelogRevision}>Save new proposed revision</Button></div>{/if}</section>
    <div class="actions"><Button variant="quiet" type="button" onclick={onclose}>Keep Release Review open</Button>{#if workspace.source_snapshot_id && !["finalized", "published", "withdrawn", "abandoned"].includes(workspace.lifecycle)}<Button variant="quiet" type="button" disabled={busy} onclick={onunlinkSnapshot}>Unlink snapshot</Button>{/if}{#if !["finalized", "published", "withdrawn", "abandoned"].includes(workspace.lifecycle)}<Button variant="quiet" type="button" disabled={busy} onclick={onrebase}>Rebase from current files</Button>{/if}{#if ["draft", "recovery_required"].includes(workspace.lifecycle) && workspace.candidates.some((candidate) => candidate.decision === "selected")}<Button variant="primary" type="button" disabled={busy} loading={busy} onclick={onapply}>Apply selected updates</Button>{/if}{#if workspace.lifecycle === "ready_to_finalize" || workspace.lifecycle === "provisional"}<Button variant="primary" type="button" disabled={busy || !selectedChangelogRevision} loading={busy} onclick={onfinalize}>Finalize validated release</Button>{/if}{#if workspace.lifecycle === "finalized"}<Button variant="primary" type="button" disabled={busy} loading={busy} onclick={onpublish}>Publish read-only record</Button>{/if}{#if !["finalized", "published", "withdrawn", "abandoned"].includes(workspace.lifecycle)}<Button variant="danger" type="button" disabled={busy} onclick={() => (showAbandon = true)}>Abandon Release Review</Button>{/if}</div>
  {/if}
</Modal>

<Modal bind:open={showAbandon} title="Abandon Release Review" onclose={() => (showAbandon = false)}><p class="lede">Abandoning preserves the snapshot, decisions, and activity history. It does not revert Packwiz files.</p><div class="actions"><Button variant="quiet" type="button" onclick={() => (showAbandon = false)}>Keep Release Review open</Button><Button variant="danger" type="button" loading={busy} onclick={() => { showAbandon = false; void onabandon(); }}>Abandon</Button></div></Modal>

<style>
  .lede,.muted { color:var(--color-text-muted); line-height:1.6; }
  .error { padding:10px 12px; border-left:3px solid var(--color-danger); color:var(--color-danger); background:var(--color-surface-raised); }
  .workspace-header,.section-heading,.actions,.watch-actions,.facts { display:flex; align-items:center; gap:12px; }
  .workspace-header,.section-heading { justify-content:space-between; }
  .workspace-header { padding-bottom:18px; border-bottom:1px solid var(--color-border); }
  h2,h3,p { margin:0; }.workspace-header p:not(.eyebrow) { color:var(--color-text-muted); font-size:12px; }
  .eyebrow { color:var(--color-accent-strong); font:700 10px var(--font-mono); letter-spacing:.12em; text-transform:uppercase; }
  .state { display:grid; gap:5px; min-width:130px; text-align:right; }.state span,.facts,.activity small { color:var(--color-text-muted); font:11px var(--font-mono); }
  .blocking { display:grid; gap:4px; margin:16px 0; padding:12px; border-left:3px solid var(--color-warning); background:var(--color-surface-raised); }.blocking span { color:var(--color-text-muted); }
  .facts { flex-wrap:wrap; margin:16px 0; padding:10px; background:var(--color-surface-raised); }.facts span { display:grid; gap:4px; min-width:130px; }.facts b { color:var(--color-text-subtle); font:10px var(--font-mono); text-transform:uppercase; }
  .workspace-section { display:grid; gap:12px; margin-top:20px; }
  .revision-list { display:grid; gap:1px; border:1px solid var(--color-border); background:var(--color-border); }.revision-list label { display:flex; gap:10px; align-items:flex-start; padding:12px; background:var(--color-surface); cursor:pointer; }.revision-list label.selected { outline:2px solid var(--color-accent-strong); outline-offset:-2px; }.revision-list span { display:grid; gap:4px; }.revision-list small,.proposal-label { color:var(--color-text-muted); font:11px var(--font-mono); }.revision-editor { display:grid; gap:10px; padding:12px; background:var(--color-surface-raised); }.revision-preview { display:grid; gap:7px; }.revision-preview pre { max-height:180px; overflow:auto; margin:0; white-space:pre-wrap; font:12px/1.5 var(--font-mono); }.editor-field { display:grid; gap:6px; }.editor-field span { font:700 11px var(--font-mono); text-transform:uppercase; }.editor-field textarea { width:100%; box-sizing:border-box; resize:vertical; color:var(--color-text); background:var(--color-surface); border:1px solid var(--color-border); padding:10px; font:12px/1.5 var(--font-mono); }
  .activity { display:grid; gap:8px; margin:0; padding:0; list-style:none; }.activity li { display:grid; gap:3px; padding:9px 10px; background:var(--color-surface-raised); }.activity span { font:700 10px var(--font-mono); text-transform:uppercase; }.actions { justify-content:flex-end; flex-wrap:wrap; margin-top:20px; }.watch-actions { flex-wrap:wrap; }.watch-status { color:var(--color-text-muted); font:700 10px var(--font-mono); text-transform:uppercase; }.watch-status.watching { color:var(--color-success); }.watch-status.watch-error { color:var(--color-danger); }
  @media (max-width:640px) { .workspace-header,.section-heading { align-items:flex-start; flex-direction:column; }.state { text-align:left; } }
</style>
