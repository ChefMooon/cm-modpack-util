<script lang="ts">
  import Button from "../ui/Button.svelte";
  import Modal from "../ui/Modal.svelte";
  import ProposedChangelogPanel from "./ProposedChangelogPanel.svelte";
  import ReleaseEvidenceSummary from "./ReleaseEvidenceSummary.svelte";
  import type { ChangelogArtifact, ChangelogProgress, ChangelogRevision, Evidence, InventoryEntry, ModpackRecord, ReleaseWorkspace, ReleaseWorkspaceWatcherStatus, SnapshotRecord } from "../../lib/domain";

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
    changelogProgress = null,
    inventory = [],
    observations = [],
    activity = [],
    activityLoading = false,
    activityHasMore = false,
    activityError = "",
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
    onwithdraw,
    ongenerateChangelog,
    oncreateBlankChangelog,
    onselectChangelog,
    onselectionChange,
    oncancelChangelog,
    onsaveChangelogRevision,
    onarchiveChangelogRevision,
    onstartWatcher,
    onstopWatcher,
    ondiscoverUpdates,
    onloadMoreActivity,
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
    changelogProgress?: ChangelogProgress | null;
    inventory?: InventoryEntry[];
    observations?: import("../../lib/domain").ReleaseWorkspaceObservation[];
    activity?: import("../../lib/domain").ReleaseWorkspaceActivity[];
    activityLoading?: boolean;
    activityHasMore?: boolean;
    activityError?: string;
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
    onwithdraw: () => void | Promise<void>;
    ongenerateChangelog: () => void | Promise<void>;
    oncreateBlankChangelog: () => void | Promise<void>;
    onselectChangelog: (revisionId: string) => void | Promise<void>;
    onselectionChange?: (versionIds: string[]) => void | Promise<void>;
    oncancelChangelog?: () => void | Promise<void>;
    onsaveChangelogRevision: () => void | Promise<void>;
    onarchiveChangelogRevision?: (revision: ChangelogRevision) => void | Promise<void>;
    onstartWatcher: () => void | Promise<void>;
    onstopWatcher: () => void | Promise<void>;
    ondiscoverUpdates: () => void | Promise<void>;
    onloadMoreActivity: () => void | Promise<void>;
    onopenPage?: (entry: InventoryEntry) => void | Promise<void>;
    onopenLink?: (url: string) => void | Promise<void>;
    onpin?: (entry: InventoryEntry, pin: boolean) => void;
    pinningEntry?: string | null;
  } = $props();

  type ReviewTab = "evidence" | "changelog" | "coordination";
  const reviewTabs: ReviewTab[] = ["evidence", "changelog", "coordination"];
  let showAbandon = $state(false);
  let showWithdraw = $state(false);
  let activeTab = $state<ReviewTab>("evidence");
  let previousWorkspaceId = $state<string | null>(null);
  let wasOpen = $state(false);

  const terminalLifecycles = ["finalized", "published", "withdrawn", "abandoned"];
  const hasStaleObservations = $derived(
    workspace !== null
      && !["ready_to_finalize", "finalized", "published", "withdrawn"].includes(workspace.lifecycle)
      && observations.some((observation) =>
        observation.evidence_freshness !== "current" || observation.blocking_reason !== null
      ),
  );

  $effect(() => {
    const workspaceId = workspace?.id ?? null;
    if (open && (!wasOpen || workspaceId !== previousWorkspaceId)) activeTab = "evidence";
    wasOpen = open;
    previousWorkspaceId = workspaceId;
  });

  function selectTab(tab: ReviewTab, focus = false): void {
    activeTab = tab;
    if (!focus) return;
    requestAnimationFrame(() => document.getElementById(`${tab}-tab`)?.focus());
  }

  function handleTabKeydown(event: KeyboardEvent, tab: ReviewTab): void {
    const currentIndex = reviewTabs.indexOf(tab);
    let nextIndex = currentIndex;
    if (event.key === "ArrowRight") nextIndex = (currentIndex + 1) % reviewTabs.length;
    else if (event.key === "ArrowLeft") nextIndex = (currentIndex - 1 + reviewTabs.length) % reviewTabs.length;
    else if (event.key === "Home") nextIndex = 0;
    else if (event.key === "End") nextIndex = reviewTabs.length - 1;
    else return;
    event.preventDefault();
    selectTab(reviewTabs[nextIndex], true);
  }

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

  function formatTimestamp(value: string): string {
    const numericValue = Number(value);
    const date = Number.isFinite(numericValue)
      ? new Date(numericValue * 1000)
      : new Date(value);
    if (Number.isNaN(date.getTime())) return value;
    return new Intl.DateTimeFormat(undefined, { dateStyle: "medium", timeStyle: "medium" }).format(date);
  }

  async function advanceToChangelog(): Promise<void> {
    selectTab("changelog", true);
    if (!changelogArtifacts.length && !changelogBusy) await ongenerateChangelog();
  }
</script>

<Modal open={open} onclose={onclose} size="wide" fillHeight title={workspace ? `Release Review · ${workspace.metadata.name}` : `Start release · ${modpack?.application.display_name ?? "Modpack"}`}>
  {#if error}<p class="error" role="alert">{error}</p>{/if}
  {#if !workspace}
    <p class="lede">This workspace starts from the immutable snapshot baseline and remains recoverable across navigation. Packwiz files are not changed until an explicit apply action.</p>
  {:else}
    <header class="workspace-header">
      <div><p class="eyebrow">{workspace.phase} phase</p><h2>{workspace.metadata.name}</h2><p>{workspace.primary_next_action.replaceAll("_", " ")}</p></div>
      <div class="state"><strong>{workspace.lifecycle.replaceAll("_", " ")}</strong><span>{workspace.evidence_freshness} evidence</span></div>
    </header>
    {#if workspace.blocking_reason}<div class="blocking" role="alert"><strong>Action required</strong><span>{workspace.blocking_reason.replaceAll("_", " ")}</span></div>{/if}
    {#if hasStaleObservations}<div class="blocking external-blocking" role="alert"><strong>Project changed after the release baseline</strong><span>If the selected update now matches the installed target, choose <b>Apply selected updates</b> to verify and record that result. Unrelated or ambiguous changes remain blocked; use <b>Rebase from current files</b> only when those changes should become a new baseline.</span></div>{/if}
    {#if workspace.lifecycle === "draft" && !workspace.candidates.length}<div class="blocking"><strong>No update candidates</strong><span>Discover updates to add candidates, or rebase the release baseline if the current project is already the intended starting point. A release with no changed result cannot be finalized.</span></div>{:else if workspace.lifecycle === "draft" && !workspace.candidates.some((candidate) => candidate.decision === "selected")}<div class="blocking"><strong>Select an update to continue</strong><span>Choose at least one candidate in Release evidence, or use Discover updates, rebase the baseline, or abandon this review.</span></div>{:else if workspace.lifecycle === "ready_to_finalize" && !selectedChangelogRevision}<div class="blocking"><strong>Proposed changelog required</strong><span>Generate and review a proposed changelog before finalizing this release. Use the footer action to open the changelog workflow.</span></div>{:else if workspace.lifecycle === "provisional"}<div class="blocking"><strong>Verification required</strong><span>Review the fresh evidence and resolve any recovery outcome before finalization becomes available.</span></div>{/if}
    <div class="facts" role="status"><span><b>Baseline</b>{workspace.baseline_origin.replaceAll("_", " ")}</span><span><b>Source</b>{workspace.source_snapshot_id ?? workspace.baseline_release_id ?? "Current project"}</span><span><b>Evidence</b>{workspace.evidence_status}</span><span><b>Publication</b>{workspace.publication_status}</span><span><b>Activity</b>{workspace.activity_count}</span><div class="observer-fact"><b>Observer</b><span class:watching={watcherStatus === "observing"} class:watch-error={watcherStatus === "error"} class="watch-status" role="status" aria-live="polite">{watcherStatusLabel(watcherStatus)}</span><div class="watch-actions"><Button size="sm" variant="quiet" type="button" disabled={busy || terminalLifecycles.includes(workspace.lifecycle) || watcherStatus === "starting" || watcherStatus === "observing"} loading={watcherStatus === "starting"} onclick={onstartWatcher}>Observe files</Button><Button size="sm" variant="quiet" type="button" disabled={busy || terminalLifecycles.includes(workspace.lifecycle) || watcherStatus === "stopped" || watcherStatus === "stopping" || watcherStatus === "error"} loading={watcherStatus === "stopping"} onclick={onstopWatcher}>Stop observing</Button></div></div></div>
    <div class="review-tabs" role="tablist" aria-label="Release review sections">
      <button class:active={activeTab === "evidence"} id="evidence-tab" role="tab" aria-selected={activeTab === "evidence"} aria-controls="evidence-panel" tabindex={activeTab === "evidence" ? 0 : -1} onclick={() => selectTab("evidence")} onkeydown={(event) => handleTabKeydown(event, "evidence")}>Release evidence</button>
      <button class:active={activeTab === "changelog"} id="changelog-tab" role="tab" aria-selected={activeTab === "changelog"} aria-controls="changelog-panel" tabindex={activeTab === "changelog" ? 0 : -1} onclick={() => selectTab("changelog")} onkeydown={(event) => handleTabKeydown(event, "changelog")}>Proposed changelog</button>
      <button class:active={activeTab === "coordination"} id="coordination-tab" role="tab" aria-selected={activeTab === "coordination"} aria-controls="coordination-panel" tabindex={activeTab === "coordination" ? 0 : -1} onclick={() => selectTab("coordination")} onkeydown={(event) => handleTabKeydown(event, "coordination")}>Evidence coordination</button>
    </div>
    <div id="evidence-panel" class="review-panel" role="tabpanel" aria-labelledby="evidence-tab" hidden={activeTab !== "evidence"}>
      <ReleaseEvidenceSummary candidates={workspace.candidates} {inventory} baselineEntries={workspace.baseline_capture?.state.entries ?? []} baselineFingerprint={workspace.baseline_capture?.state.source_fingerprint ?? null} {observations} {evidenceLabel} {busy} ondecision={ondecision} ondiscoverUpdates={ondiscoverUpdates} {onopenPage} {onopenLink} {onpin} {pinningEntry} />
    </div>
    <div id="changelog-panel" class="review-panel" role="tabpanel" aria-labelledby="changelog-tab" hidden={activeTab !== "changelog"}>
      <ProposedChangelogPanel artifacts={changelogArtifacts} revisions={changelogRevisions} selectedRevision={selectedChangelogRevision} bind:draft={changelogDraft} {busy} {changelogBusy} {changelogProgress} oncancel={oncancelChangelog} ongenerate={ongenerateChangelog} oncreateBlank={oncreateBlankChangelog} onselect={onselectChangelog} onselectionchange={onselectionChange} onsave={onsaveChangelogRevision} onarchive={onarchiveChangelogRevision} />
    </div>
    <div id="coordination-panel" class="review-panel" role="tabpanel" aria-labelledby="coordination-tab" hidden={activeTab !== "coordination"}>
      <div class="panel-heading"><div><p class="eyebrow">Evidence coordination</p><h3>Terminal edits and recovery</h3></div><span class="history-summary">{workspace.activity_count} recorded</span></div>
      {#if activityError}<p class="error" role="alert">{activityError}</p>{/if}{#if activity.length}<ul class="activity">{#each activity as item (item.id)}<li><time datetime={item.occurred_at} title={item.occurred_at}>{formatTimestamp(item.occurred_at)}</time><span>{item.event_type.replaceAll("_", " ")}</span><small>{item.message}</small></li>{/each}</ul>{:else if activityLoading}<p class="muted">Loading workspace activity...</p>{:else}<p class="muted">No workspace activity has been recorded yet.</p>{/if}{#if activityHasMore}<Button size="sm" variant="quiet" type="button" disabled={activityLoading || busy} loading={activityLoading} onclick={onloadMoreActivity}>Show more history</Button>{/if}
    </div>
  {/if}
  {#snippet footer()}
    {#if workspace}<div class="actions"><Button variant="quiet" type="button" onclick={onclose}>Keep Release Review open</Button>{#if workspace.source_snapshot_id && !["finalized", "published", "withdrawn", "abandoned"].includes(workspace.lifecycle)}<Button variant="quiet" type="button" disabled={busy} onclick={onunlinkSnapshot}>Unlink snapshot</Button>{/if}{#if !["finalized", "published", "withdrawn", "abandoned"].includes(workspace.lifecycle)}<Button variant="quiet" type="button" disabled={busy} onclick={onrebase}>Rebase from current files</Button>{/if}{#if ["draft", "recovery_required"].includes(workspace.lifecycle) && workspace.candidates.some((candidate) => candidate.decision === "selected")}<Button variant="primary" type="button" disabled={busy} loading={busy} onclick={onapply}>Apply selected updates</Button>{/if}{#if workspace.lifecycle === "ready_to_finalize"}<Button variant="secondary" type="button" disabled={busy || changelogBusy} loading={changelogBusy} onclick={advanceToChangelog}>{selectedChangelogRevision ? "Review proposed changelog" : "Generate proposed changelog"}</Button><Button variant="primary" type="button" disabled={busy || !selectedChangelogRevision} loading={busy} onclick={onfinalize}>Finalize validated release</Button>{/if}{#if workspace.lifecycle === "finalized"}<Button variant="primary" type="button" disabled={busy} loading={busy} onclick={onpublish}>Publish read-only record</Button>{/if}{#if workspace.lifecycle === "published"}<Button variant="danger" type="button" disabled={busy} onclick={() => (showWithdraw = true)}>Withdraw release</Button>{/if}{#if !["finalized", "published", "withdrawn", "abandoned"].includes(workspace.lifecycle)}<Button variant="danger" type="button" disabled={busy} onclick={() => (showAbandon = true)}>Abandon Release Review</Button>{/if}</div>{/if}
  {/snippet}
</Modal>

<Modal bind:open={showAbandon} title="Abandon Release Review" onclose={() => (showAbandon = false)}><p class="lede">Abandoning preserves the snapshot, decisions, and activity history. It does not revert Packwiz files.</p><div class="actions"><Button variant="quiet" type="button" onclick={() => (showAbandon = false)}>Keep Release Review open</Button><Button variant="danger" type="button" loading={busy} onclick={() => { showAbandon = false; void onabandon(); }}>Abandon</Button></div></Modal>
<Modal bind:open={showWithdraw} title="Withdraw published release" onclose={() => (showWithdraw = false)}><p class="lede">Withdrawal keeps the captured release and publication history, but marks it as no longer published. This cannot be undone from the release review.</p><div class="actions"><Button variant="quiet" type="button" onclick={() => (showWithdraw = false)}>Keep published</Button><Button variant="danger" type="button" loading={busy} onclick={() => { showWithdraw = false; void onwithdraw(); }}>Withdraw release</Button></div></Modal>

<style>
  .lede,.muted { color:var(--color-text-muted); line-height:1.6; }
  .error { padding:10px 12px; border-left:3px solid var(--color-danger); color:var(--color-danger); background:var(--color-surface-raised); }
  .workspace-header,.actions,.watch-actions,.facts { display:flex; align-items:center; gap:12px; }
  .workspace-header { justify-content:space-between; }
  .workspace-header { padding-bottom:18px; border-bottom:1px solid var(--color-border); }
  h2,h3,p { margin:0; }.workspace-header p:not(.eyebrow) { color:var(--color-text-muted); font-size:12px; }
  .eyebrow { color:var(--color-accent-strong); font:700 10px var(--font-mono); letter-spacing:.12em; text-transform:uppercase; }
  .state { display:grid; gap:5px; min-width:130px; text-align:right; }.state span,.facts,.activity small { color:var(--color-text-muted); font:11px var(--font-mono); }
  .blocking { display:grid; gap:4px; margin:16px 0; padding:12px; border-left:3px solid var(--color-warning); background:var(--color-surface-raised); }.blocking span { color:var(--color-text-muted); }
  .facts { flex-wrap:wrap; margin:16px 0; padding:10px; background:var(--color-surface-raised); }.facts span { display:grid; gap:4px; min-width:130px; }.facts b { color:var(--color-text-subtle); font:10px var(--font-mono); text-transform:uppercase; }
  .review-tabs { display:flex; gap:1px; margin-top:20px; border:1px solid var(--color-border); background:var(--color-border); }
  .review-tabs button { flex:1; min-height:42px; padding:9px 12px; border:0; color:var(--color-text-muted); background:var(--color-surface-raised); font:700 10px var(--font-mono); letter-spacing:.08em; text-transform:uppercase; cursor:pointer; }
  .review-tabs button:hover,.review-tabs button.active { color:var(--color-accent-strong); background:var(--color-surface); }
  .review-tabs button.active { box-shadow:inset 0 -3px 0 var(--color-accent-strong); }
  .review-panel { display:grid; gap:12px; margin-top:0; padding:14px; border:1px solid var(--color-border); background:var(--color-surface-raised); }
  #evidence-panel { padding:0; border:0; background:transparent; }
  #evidence-panel :global(.evidence-summary) { margin-top:0; }
  .review-panel[hidden] { display:none; }
  .panel-heading { display:flex; align-items:center; justify-content:space-between; gap:12px; }
  .panel-heading h3 { margin:0; }
  .history-summary { color:var(--color-text-muted); font:11px var(--font-mono); }
  .activity { display:grid; gap:1px; margin:0; padding:0; border:1px solid var(--color-border); background:var(--color-border); list-style:none; }.activity li { display:grid; gap:3px; padding:10px 12px; background:var(--color-surface); }.activity time { color:var(--color-text-muted); font:11px var(--font-mono); }.activity span { color:var(--color-accent-strong); font:700 10px var(--font-mono); text-transform:uppercase; }.actions { justify-content:flex-end; flex-wrap:wrap; margin-top:20px; }.watch-actions { flex-wrap:wrap; }.watch-status { color:var(--color-text-muted); font:700 10px var(--font-mono); text-transform:uppercase; }.watch-status.watching { color:var(--color-success); }.watch-status.watch-error { color:var(--color-danger); }.observer-fact { display:grid; gap:4px; min-width:230px; }.observer-fact > .watch-actions { margin-top:2px; }
  @media (max-width:640px) { .workspace-header,.panel-heading { align-items:flex-start; flex-direction:column; }.state { text-align:left; }.review-tabs { flex-direction:column; }.review-tabs button { flex:none; text-align:left; }.review-tabs button.active { box-shadow:inset 3px 0 0 var(--color-accent-strong); } }
</style>
