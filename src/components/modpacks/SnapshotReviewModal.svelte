<script lang="ts">
  import ArrowSquareOutIcon from "phosphor-svelte/lib/ArrowSquareOutIcon";
  import XIcon from "phosphor-svelte/lib/XIcon";
  import Button from "../ui/Button.svelte";
  import Modal from "../ui/Modal.svelte";
  import type {
    ApplyOperationReport,
    ChangelogArtifact,
    ChangelogExport,
    ChangelogProgress,
    ChangelogRevision,
    DiscoveryProgress,
    DiscoveryResult,
    ProcessEvidence,
    ModpackRecord,
    SnapshotRecord,
    UpdateCandidate,
  } from "../../lib/domain";

  type Decision = "selected" | "skipped" | "blocked" | "deferred";

  let {
    open = false,
    onclose,
    modpack,
    discovery,
    discoveryProgress,
    discoveryProcess,
    snapshot,
    snapshotId,
    discoveryBusy,
    applyBusy,
    applyReport,
    changelog,
    changelogRevision,
    changelogRevisions,
    changelogExports,
    changelogIntroduction = $bindable(""),
    changelogOffline = $bindable(false),
    changelogBusy,
    changelogProgress,
    changelogPartialChoice,
    changelogDraft = $bindable(""),
    noteDraft = $bindable(""),
    selectedCandidateIds,
    latestDecision,
    evidenceLabel,
    oncancelDiscovery,
    oncloseReview,
    ondecideCandidate,
    onsaveNote,
    onrecheck,
    onretry,
    onapply,
    oncancelApply,
    onopenPage,
    ongenerator,
    oncancelChangelog,
    onkeepPartial,
    ondiscardPartial,
    onsaveRevision,
    onexport,
    onstartReleaseWorkspace,
  }: {
    open?: boolean;
    onclose?: () => void;
    modpack: ModpackRecord | null;
    discovery: DiscoveryResult | null;
    discoveryProgress: DiscoveryProgress | null;
    discoveryProcess: ProcessEvidence | null;
    snapshot: SnapshotRecord | null;
    snapshotId: string | null;
    discoveryBusy: boolean;
    applyBusy: boolean;
    applyReport: ApplyOperationReport | null;
    changelog: ChangelogArtifact | null;
    changelogRevision: ChangelogRevision | null;
    changelogRevisions: ChangelogRevision[];
    changelogExports: ChangelogExport[];
    changelogIntroduction?: string;
    changelogOffline?: boolean;
    changelogBusy: boolean;
    changelogProgress: ChangelogProgress | null;
    changelogPartialChoice: boolean;
    changelogDraft?: string;
    noteDraft?: string;
    selectedCandidateIds: () => string[];
    latestDecision: (candidateId: string) => string | undefined;
    evidenceLabel: <T>(value: import("../../lib/domain").Evidence<T>) => string;
    oncancelDiscovery: () => void | Promise<void>;
    oncloseReview: (cancelled: boolean) => void | Promise<void>;
    ondecideCandidate: (candidateId: string, decision: Decision) => void | Promise<void>;
    onsaveNote: () => void | Promise<void>;
    onrecheck: () => void | Promise<void>;
    onretry: () => void | Promise<void>;
    onapply: () => void | Promise<void>;
    oncancelApply: () => void | Promise<void>;
    onopenPage: (url: string) => void | Promise<void>;
    ongenerator: () => void | Promise<void>;
    oncancelChangelog: () => void | Promise<void>;
    onkeepPartial: () => void;
    ondiscardPartial: () => void;
    onsaveRevision: () => void | Promise<void>;
    onexport: () => void | Promise<void>;
    onstartReleaseWorkspace: () => void | Promise<void>;
  } = $props();

  let showNote = $state(false);
  function candidateRecords(): { id: string; candidate: UpdateCandidate }[] {
    if (snapshot) return snapshot.candidates;
    return (discovery?.candidates ?? []).map((candidate, index) => ({ id: `${snapshotId ?? "discovery"}-candidate-${index}`, candidate, observed_at: "" }));
  }
</script>

<Modal open={open} onclose={onclose} size="wide" title={modpack ? `Update review · ${modpack.application.display_name}` : "Update review"}>
  {#if discoveryBusy}
    <div class="progress" role="status"><div class="loader" aria-hidden="true"></div><span>{discoveryProgress?.message ?? "Preparing the Packwiz safety probe..."}</span><Button variant="danger" size="sm" type="button" onclick={oncancelDiscovery}><XIcon size={15} /> Cancel</Button></div>
  {/if}
  {#if discovery}
    <div class="outcome" data-outcome={discovery.outcome}><strong>{discovery.outcome === "normal" ? "Update check completed" : discovery.outcome.replaceAll("_", " ")}</strong><span>{discovery.diagnostics.messages[0] ?? "Packwiz output was observed safely."}</span></div>
    {#if discovery.outcome === "normal"}
      {#if !candidateRecords().length}<p class="empty">No updates were presented by Packwiz.</p>{:else}<div class="candidate-list" aria-label="Available updates">
        {#each candidateRecords() as record (record.id)}
          {@const candidate = record.candidate}
          <article class="candidate">
            <div class="candidate-evidence">
              <div class="identity"><strong>{evidenceLabel(candidate.identity)}</strong><span class="path">{evidenceLabel(candidate.local_path)}</span></div>
              <div><span class="label">Version</span><strong>{evidenceLabel(candidate.current_version)} <span class="arrow">→</span> {evidenceLabel(candidate.available_version)}</strong></div>
              <div><span class="label">Change</span><strong>{candidate.version_change}</strong></div>
              <div><span class="label">Provider / side</span><strong>{evidenceLabel(candidate.provider)} / {evidenceLabel(candidate.side)}</strong></div>
              <div><span class="label">Pin</span><strong>{evidenceLabel(candidate.pin)}</strong></div>
              <div><span class="label">Severity</span><strong>{evidenceLabel(candidate.severity)}</strong></div>
            </div>
            <div class="candidate-actions">
              <div class="decisions" aria-label="Historical snapshot decision"><span class="decision">{latestDecision(record.id) ?? "undecided"}</span><span class="historical-label">Historical evidence</span></div>
              {#if candidate.page_link}<Button variant="quiet" size="sm" type="button" onclick={() => onopenPage(candidate.page_link!.url)}><ArrowSquareOutIcon size={14} /> Open source</Button>{/if}
            </div>
          </article>
        {/each}
      </div>{/if}
      {#if snapshot}<div class="actions"><Button variant="secondary" size="sm" type="button" onclick={() => oncloseReview(false)}>Close review</Button>{#if snapshot.lifecycle === "reviewable"}<Button variant="primary" size="sm" type="button" onclick={onstartReleaseWorkspace}>Start Release Review</Button>{/if}</div>{/if}
    {:else}<p class="empty">{discovery.diagnostics.messages[0] ?? "No safe update candidates are available."}</p>{/if}
    <p class="historical-note">This snapshot is immutable discovery evidence. Candidate decisions, update application, recovery, and changelog work are owned by the release workspace.</p>
    {#if changelogBusy && changelogProgress}<div class="changelog-progress" role="status" aria-live="polite"><span class="loader" aria-hidden="true"></span><div><strong>Generating changelog</strong><span>{changelogProgress.message}</span></div><span>{changelogProgress.completed} / {changelogProgress.total}</span>{#if changelogProgress.cancellable}<Button variant="danger" size="sm" type="button" onclick={oncancelChangelog}>Cancel</Button>{/if}</div>{/if}
    {#if changelog}<section class="changelog-result"><div class="result-heading"><div><p class="eyebrow">Stored artifact</p><h3>Generation {changelog.status}</h3></div><span>{changelog.entries.length} entries</span></div>{#if changelogPartialChoice}<div class="status-panel" role="alert"><strong>This generation stopped before all lookups finished.</strong><p>The fetched entries are stored as a partial artifact.</p><div class="actions"><Button variant="primary" size="sm" type="button" onclick={onkeepPartial}>Keep partial result</Button><Button variant="quiet" size="sm" type="button" onclick={ondiscardPartial}>Discard from review</Button></div></div>{/if}<div class="candidate-list">{#each changelog.entries as entry (entry.local.entry_id)}<article class="candidate"><div class="identity"><strong>{evidenceLabel(entry.local.local_identity)}</strong><span>{entry.retrieval.replaceAll("_", " ")}</span><span>{entry.match_evidence.confidence} confidence</span></div><p>{entry.changelog ?? entry.match_evidence.reason}</p></article>{/each}</div><div class="actions"><Button variant="primary" size="sm" type="button" onclick={onexport}>Export Markdown</Button></div><label class="field"><span>Editable revision</span><textarea bind:value={changelogDraft} rows="10"></textarea></label><div class="actions"><Button variant="quiet" size="sm" type="button" disabled={changelogDraft === changelog.content} onclick={onsaveRevision}>Save revision</Button></div><details class="evidence"><summary>Original generated Markdown</summary><pre>{changelog.content}</pre></details><details class="evidence"><summary>Revision history ({changelogRevisions.length})</summary>{#each changelogRevisions as revision (revision.id)}<p><strong>{revision.is_current ? "Current revision" : "Revision"}</strong> {revision.created_at}</p>{/each}</details><details class="evidence"><summary>Export history ({changelogExports.length})</summary>{#each changelogExports as exportRecord (exportRecord.id)}<p><strong>{exportRecord.status}</strong> {exportRecord.destination}</p>{/each}</details></section>{/if}
    {#if applyReport}<section class="report"><p class="eyebrow">Verified operation report</p><h3>Apply {applyReport.outcome}</h3>{#each applyReport.attempts as attempt (attempt.id)}<article><strong>{attempt.verification?.intended_state ?? "Selected mod"}</strong><span>{attempt.outcome ?? attempt.status}</span><span>{attempt.verification?.verified ? "Verified after re-read" : attempt.error?.message ?? "Verification incomplete"}</span></article>{/each}</section>{/if}
    {#if snapshot?.lifecycle === "stale"}<p class="stale" role="status">This review is stale. Start a fresh retry before relying on these decisions.</p>{/if}
    <details class="evidence"><summary>Safety evidence</summary><div class="evidence-grid"><span>Prompt <strong>{discoveryProcess?.prompt ?? "Unavailable"}</strong></span><span>Cancellation <strong>{discoveryProcess?.cancellation ?? "Unavailable"}</strong></span><span>Fingerprint <strong>{discovery.diagnostics.fingerprint?.unchanged ? "Unchanged" : "Not proven unchanged"}</strong></span><span>Output <strong>{discoveryProcess?.output_truncated ? "Truncated" : "Complete"}</strong></span><span>Exit code <strong>{discoveryProcess?.exit_code ?? "Unavailable"}</strong></span></div>{#if discoveryProcess}<pre>{discoveryProcess.stdout || "(empty)"}{"\n\n"}{discoveryProcess.stderr || "(empty)"}</pre>{/if}</details>
  {/if}
</Modal>

<Modal bind:open={showNote} title="Add review note"><label class="field"><span>Note</span><textarea bind:value={noteDraft} rows="5" maxlength="4000" placeholder="Explain this review or its outcome"></textarea></label><div class="actions"><Button variant="quiet" type="button" onclick={() => (showNote = false)}>Cancel</Button><Button variant="primary" type="button" onclick={() => { showNote = false; void onsaveNote(); }}>Save note</Button></div></Modal>
<style>
  .changelog-progress { display:flex; align-items:center; justify-content:space-between; gap:10px; margin:12px 0; padding:10px 12px; border-left:3px solid var(--color-accent-strong); background:var(--color-surface-raised); }
  .changelog-progress > div { display:grid; gap:3px; min-width:0; flex:1; }
  .changelog-progress > div span { color:var(--color-text-muted); font:11px var(--font-mono); }
  .progress,.actions,.decisions,.result-heading,.candidate-actions { display:flex; align-items:center; gap:8px; flex-wrap:wrap; }.progress { justify-content:space-between; padding:12px 0; }.loader { width:18px; height:18px; border:2px solid var(--color-border); border-top-color:var(--color-accent); border-radius:50%; animation:spin .8s linear infinite; }.outcome { display:grid; gap:5px; margin:12px 0; padding:14px; border-left:3px solid var(--color-info); }.outcome[data-outcome="normal"] { border-color:var(--color-success); }.outcome strong,.candidate strong,.report strong { color:var(--color-text); }.empty,.stale { color:var(--color-text-muted); }.candidate-list { display:grid; gap:1px; margin:16px 0; border:1px solid var(--color-border); background:var(--color-border); }.candidate { display:grid; gap:14px; padding:16px; background:var(--color-surface); font-size:12px; }.candidate-evidence { display:grid; grid-template-columns:minmax(180px,1.5fr) repeat(5,minmax(90px,1fr)); gap:14px; }.candidate-evidence > div,.identity { display:grid; gap:5px; min-width:0; }.candidate-actions { justify-content:space-between; padding-top:12px; border-top:1px solid var(--color-border); }.decision { min-width:84px; color:var(--color-text); font:700 10px var(--font-mono); text-transform:uppercase; }.label,.path { color:var(--color-text-muted); font:10px var(--font-mono); }.path { overflow-wrap:anywhere; }.arrow { color:var(--color-text-subtle); }.eyebrow { margin:0 0 8px; color:var(--color-accent-strong); font:700 10px var(--font-mono); letter-spacing:.12em; text-transform:uppercase; }.field { display:grid; gap:7px; color:var(--color-text); font-size:12px; }.field textarea { min-height:72px; padding:9px 10px; border:1px solid var(--color-border); border-radius:var(--radius-sm); color:var(--color-text); background:var(--color-bg); font:inherit; resize:vertical; }.result-heading { justify-content:space-between; }.status-panel { padding:12px; border-left:3px solid var(--color-warning); }.status-panel p { color:var(--color-text-muted); }.report article { display:grid; grid-template-columns:1.2fr .7fr 2fr; gap:12px; padding:10px 0; border-top:1px solid var(--color-border); color:var(--color-text-muted); font-size:12px; }.evidence { margin-top:18px; border-top:1px solid var(--color-border); }.evidence summary { padding:12px 0; color:var(--color-text); font-weight:700; cursor:pointer; }.evidence-grid { display:grid; grid-template-columns:repeat(5,1fr); gap:10px; }.evidence-grid span { display:grid; gap:4px; }.evidence-grid strong { color:var(--color-text); font:10px var(--font-mono); text-transform:uppercase; }.evidence pre { max-height:180px; overflow:auto; padding:10px; color:var(--color-console-text); background:var(--color-console); font:11px/1.5 var(--font-mono); white-space:pre-wrap; overflow-wrap:anywhere; }.evidence p { color:var(--color-text-muted); font-size:12px; }.historical-note { margin-top:18px; padding:12px; border-left:3px solid var(--color-info); color:var(--color-text-muted); }@keyframes spin { to { transform:rotate(360deg); } }@media (max-width:780px) { .candidate-evidence { grid-template-columns:1fr 1fr; }.identity { grid-column:1 / -1; }.evidence-grid { grid-template-columns:1fr 1fr; }.report article { grid-template-columns:1fr; gap:4px; }}@media (prefers-reduced-motion:reduce) { .loader { animation:none; }}
</style>
