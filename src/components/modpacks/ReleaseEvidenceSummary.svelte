<script lang="ts">
  import ArrowSquareOutIcon from "phosphor-svelte/lib/ArrowSquareOutIcon";
  import ArrowClockwiseIcon from "phosphor-svelte/lib/ArrowClockwiseIcon";
  import CheckIcon from "phosphor-svelte/lib/CheckIcon";
  import ClockIcon from "phosphor-svelte/lib/ClockIcon";
  import ProhibitIcon from "phosphor-svelte/lib/ProhibitIcon";
  import PushPinIcon from "phosphor-svelte/lib/PushPinIcon";
  import PushPinSlashIcon from "phosphor-svelte/lib/PushPinSlashIcon";
  import XIcon from "phosphor-svelte/lib/XIcon";
  import Button from "../ui/Button.svelte";
  import Tooltip from "../ui/Tooltip.svelte";
  import type { CapturedEntry, Evidence, InventoryEntry, ModpackFingerprint, ReleaseWorkspaceCandidate, ReleaseWorkspaceObservation } from "../../lib/domain";

  type Source = "snapshot" | "current" | "external";
  type Filter = "all" | Source;
  type ChangeType = "updated file" | "added mod" | "removed mod" | "current mod";
  type EvidenceRow = {
    id: string;
    source: Source;
    changeType: ChangeType;
    current?: InventoryEntry;
    candidate?: ReleaseWorkspaceCandidate;
    observation?: { scope: string; freshness: string; status: string; active: boolean };
    candidateId?: string;
  };

  let {
    candidates = [],
    inventory = [],
    baselineEntries = [],
    baselineFingerprint = null,
    observations = [],
    evidenceLabel,
    busy = false,
    ondecision,
    ondiscoverUpdates,
    onopenPage,
    onopenLink,
    onpin,
    pinningEntry = null,
  }: {
    candidates?: ReleaseWorkspaceCandidate[];
    inventory?: InventoryEntry[];
    baselineEntries?: CapturedEntry[];
    baselineFingerprint?: ModpackFingerprint | null;
    observations?: ReleaseWorkspaceObservation[];
    evidenceLabel: <T>(value: Evidence<T>) => string;
    busy?: boolean;
    ondecision?: (candidateId: string, decision: "selected" | "skipped" | "deferred" | "blocked") => void | Promise<void>;
    ondiscoverUpdates?: () => void | Promise<void>;
    onopenPage?: (entry: InventoryEntry) => void | Promise<void>;
    onopenLink?: (url: string) => void | Promise<void>;
    onpin?: (entry: InventoryEntry, pin: boolean) => void;
    pinningEntry?: string | null;
  } = $props();

  let filter = $state<Filter>("all");
  let query = $state("");

  function normalized(value: Evidence<string>): string | null {
    if (typeof value !== "object" || value === null || !("observed" in value)) return null;
    const result = value.observed.trim().toLowerCase();
    return result || null;
  }

  function normalizedScope(scope: string): string {
    return scope.trim().replaceAll(String.fromCharCode(92), "/").toLowerCase();
  }

  function changedFromBaseline(entry: InventoryEntry, baseline: CapturedEntry | undefined): boolean {
    if (!baseline) return true;
    return [entry.local_id, entry.metadata_path, evidenceLabel(entry.name), evidenceLabel(entry.version), evidenceLabel(entry.provider), evidenceLabel(entry.side), evidenceLabel(entry.source_url)].join("\u0000")
      !== [baseline.local_id, baseline.metadata_path, evidenceLabel(baseline.name), evidenceLabel(baseline.version), evidenceLabel(baseline.provider), evidenceLabel(baseline.side), evidenceLabel(baseline.source_url)].join("\u0000");
  }

  function fingerprintMatchesBaseline(fingerprint: ModpackFingerprint | null): boolean {
    if (fingerprint === null || baselineFingerprint === null || !fingerprint.complete || !baselineFingerprint.complete || fingerprint.root !== baselineFingerprint.root) return false;
    if (fingerprint.entries.length !== baselineFingerprint.entries.length) return false;
    return fingerprint.entries.every((entry, index) => {
      const baselineEntry = baselineFingerprint.entries[index];
      return baselineEntry !== undefined
        && entry.relative_path === baselineEntry.relative_path
        && entry.kind === baselineEntry.kind
        && entry.size === baselineEntry.size
        && entry.content_hash === baselineEntry.content_hash;
    });
  }

  function candidateTargetStatus(row: EvidenceRow): "verified" | "unverified" | null {
    if (!row.candidate || !row.current || row.candidate.decision !== "selected") return null;
    const available = evidenceLabel(row.candidate.candidate.available_version).trim().toLowerCase();
    const observed = evidenceLabel(row.current.version).trim().toLowerCase();
    const artifact = available.endsWith(".jar") ? available.slice(0, -4) : available;
    return available === observed || artifact.endsWith(observed) ? "verified" : "unverified";
  }

  function observationStatus(status: string): string {
    return status === "app owned apply overlap"
      ? "Updated by CM Modpack Util"
      : status;
  }

  const modRows = $derived.by<EvidenceRow[]>(() => {
    const unmatchedInventory = new Set(inventory.map((entry) => entry.local_id));
    const baselineById = new Map(baselineEntries.map((entry) => [entry.local_id, entry]));
    const rows: EvidenceRow[] = [];
    for (const candidate of candidates) {
      const candidateName = normalized(candidate.candidate.identity);
      const candidatePath = normalized(candidate.candidate.local_path);
      const current = inventory.find((entry) => {
        if (!unmatchedInventory.has(entry.local_id)) return false;
        const nameMatch = candidateName !== null && candidateName === normalized(entry.name);
        const pathMatch = candidatePath !== null && candidatePath === normalized({ observed: entry.local_id });
        return nameMatch || pathMatch;
      });
      if (current) unmatchedInventory.delete(current.local_id);
      rows.push({
        id: `mod-${candidate.source_candidate_id}`,
        source: "snapshot",
        changeType: current
          ? changedFromBaseline(current, baselineById.get(current.local_id)) ? "updated file" : "current mod"
          : "removed mod",
        current,
        candidate,
        candidateId: candidate.source_candidate_id,
      });
    }
    for (const entry of inventory) {
      if (unmatchedInventory.has(entry.local_id)) {
        const baseline = baselineById.get(entry.local_id);
        rows.push({
          id: `mod-current-${entry.local_id}`,
          source: "current",
          changeType: !baseline ? "added mod" : changedFromBaseline(entry, baseline) ? "updated file" : "current mod",
          current: entry,
        });
      }
    }
    return rows;
  });

  const rows = $derived.by<EvidenceRow[]>(() => {
    const externalRows = new Map<string, EvidenceRow>();
    for (const observation of observations) {
      for (const scope of observation.changed_scope) {
        const key = normalizedScope(scope);
        if (!key) continue;
        externalRows.set(key, {
          id: `external-${key}`,
          source: "external",
          changeType: key.endsWith(".pw.toml") ? "removed mod" : "updated file",
          observation: {
            scope,
            freshness: observation.evidence_freshness,
            status: observation.blocking_reason?.replaceAll("_", " ") ?? "observed",
            active: (observation.blocking_reason !== null || observation.evidence_freshness !== "current") && !fingerprintMatchesBaseline(observation.fingerprint),
          },
        });
      }
    }
    const mergedModPaths = new Set<string>();
    const mergedMods = modRows.map((row) => {
      const path = row.current?.local_id ?? (row.candidate ? normalized(row.candidate.candidate.local_path) : null);
      if (!path) return row;
      const observation = externalRows.get(normalizedScope(path));
      if (!observation?.observation?.active) return row;
      mergedModPaths.add(normalizedScope(path));
      return {
        ...row,
        observation: observation.observation,
      };
    });
    const standaloneExternal = [...externalRows.values()].filter((row) => row.observation?.active && !mergedModPaths.has(normalizedScope(row.observation.scope)));
    return [...mergedMods, ...standaloneExternal];
  });

  const filteredRows = $derived(rows.filter((row) => {
    if (filter !== "all" && row.source !== filter && !(filter === "snapshot" && row.candidate) && !(filter === "current" && row.current) && !(filter === "external" && row.observation)) return false;
    if (!query.trim()) return true;
    const needle = query.trim().toLowerCase();
    const current = row.current ? `${evidenceLabel(row.current.name)} ${evidenceLabel(row.current.version)} ${row.current.local_id} ${evidenceLabel(row.current.provider)} ${evidenceLabel(row.current.side)}` : "";
    const candidate = row.candidate ? `${evidenceLabel(row.candidate.candidate.identity)} ${evidenceLabel(row.candidate.candidate.current_version)} ${evidenceLabel(row.candidate.candidate.available_version)} ${evidenceLabel(row.candidate.candidate.local_path)} ${row.candidate.decision}` : "";
    const external = row.observation ? `${row.observation.scope} ${row.observation.freshness} ${row.observation.status}` : "";
    return `${current} ${candidate} ${external}`.toLowerCase().includes(needle);
  }));
</script>

<section class="evidence-summary" aria-labelledby="release-evidence-title">
  <div class="summary-heading">
    <div>
      <p class="eyebrow">Release evidence</p>
      <h3 id="release-evidence-title">Current installs, snapshot updates, and external changes</h3>
    </div>
    <div class="summary-actions">
      <span class="count" aria-live="polite">{filteredRows.length} of {rows.length} shown</span>
      {#if ondiscoverUpdates}<Tooltip text="Discover updates" alignment="end"><Button size="icon" variant="secondary" type="button" aria-label="Discover updates" disabled={busy} onclick={ondiscoverUpdates}><ArrowClockwiseIcon size={17} aria-hidden="true" /></Button></Tooltip>{/if}
    </div>
  </div>
  <div class="summary-controls">
    <label>
      <span>Find evidence</span>
      <input bind:value={query} type="search" placeholder="Name, path, status" />
    </label>
    <label>
      <span>Show</span>
      <select bind:value={filter}>
        <option value="all">All evidence</option>
        <option value="snapshot">Snapshot updates</option>
        <option value="current">Current mods</option>
        <option value="external">External changes</option>
      </select>
    </label>
  </div>
  {#if !rows.length}
    <p class="empty">No release evidence has been observed yet.</p>
  {:else if !filteredRows.length}
    <p class="empty">No evidence matches the current filter.</p>
  {:else}
    <div class="evidence-list" role="list" aria-label="Release evidence entries">
      {#each filteredRows as row (row.id)}
        <article class="evidence-row" role="listitem">
          {#if row.observation && row.source === "external"}
            <span class="source" data-source="external">{row.changeType}</span><div><strong>{row.observation.scope}</strong><small>{row.observation.freshness} evidence</small></div><small>{observationStatus(row.observation.status)}</small>
          {:else}
            <span class="source" data-source={row.source} data-change={row.changeType}>{row.changeType}</span>
            <div class="mod-details"><div class="mod-column"><strong>{row.current ? evidenceLabel(row.current.name) : row.candidate ? evidenceLabel(row.candidate.candidate.identity) : "Unknown mod"}</strong>{#if row.current}<small>Installed: {evidenceLabel(row.current.version)} · {evidenceLabel(row.current.provider)} · {evidenceLabel(row.current.side)}</small><small>{row.current.local_id}</small>{:else}<small>Current install: Not present</small>{/if}{#if candidateTargetStatus(row) === "verified"}<small class="target-status verified">Target version verified</small>{:else if candidateTargetStatus(row) === "unverified"}<small class="target-status unverified">Selected target not verified</small>{:else if row.observation}<small class="external-status">{row.observation.status}</small>{/if}</div>{#if row.candidate}<div class="mod-column snapshot-column"><strong>Update</strong><small>{evidenceLabel(row.candidate.candidate.current_version)} → {evidenceLabel(row.candidate.candidate.available_version)} · {row.candidate.decision}</small><small>{evidenceLabel(row.candidate.candidate.local_path)}</small></div>{/if}</div>
            <div class="evidence-status"><div class="row-actions">{#if row.current?.page_link && onopenPage}<Tooltip text="Open mod page"><Button variant="ghost" size="icon" type="button" aria-label="Open mod page" onclick={() => onopenPage?.(row.current!)}><ArrowSquareOutIcon size={16} aria-hidden="true" /></Button></Tooltip>{:else if row.candidate?.candidate.page_link && onopenLink}<Tooltip text="Open mod page"><Button variant="ghost" size="icon" type="button" aria-label="Open mod page" onclick={() => onopenLink?.(row.candidate!.candidate.page_link!.url)}><ArrowSquareOutIcon size={16} aria-hidden="true" /></Button></Tooltip>{/if}{#if row.current && onpin}{#if evidenceLabel(row.current.pin) === "true"}<Tooltip text="Unpin mod"><Button variant="ghost" size="icon" type="button" aria-label="Unpin mod" disabled={pinningEntry !== null} loading={pinningEntry === row.current.local_id} onclick={() => onpin?.(row.current!, false)}><PushPinSlashIcon size={16} aria-hidden="true" /></Button></Tooltip>{:else if evidenceLabel(row.current.pin) === "false"}<Tooltip text="Pin mod"><Button variant="ghost" size="icon" type="button" aria-label="Pin mod" disabled={pinningEntry !== null} loading={pinningEntry === row.current.local_id} onclick={() => onpin?.(row.current!, true)}><PushPinIcon size={16} aria-hidden="true" /></Button></Tooltip>{/if}{/if}</div>{#if row.candidateId && ondecision}<div class="decision-actions"><Tooltip text={row.candidate?.decision === "selected" ? "Selected update" : "Select update"}><Button class="decision-selected-{row.candidate?.decision === "selected"}" variant="ghost" size="icon" type="button" aria-label={row.candidate?.decision === "selected" ? "Selected update" : "Select update"} aria-pressed={row.candidate?.decision === "selected"} disabled={busy} onclick={() => ondecision?.(row.candidateId!, "selected")}><CheckIcon size={16} aria-hidden="true" /></Button></Tooltip><Tooltip text={row.candidate?.decision === "skipped" ? "Skipped update" : "Skip update"}><Button class="decision-selected-{row.candidate?.decision === "skipped"}" variant="ghost" size="icon" type="button" aria-label={row.candidate?.decision === "skipped" ? "Skipped update" : "Skip update"} aria-pressed={row.candidate?.decision === "skipped"} disabled={busy} onclick={() => ondecision?.(row.candidateId!, "skipped")}><XIcon size={16} aria-hidden="true" /></Button></Tooltip><Tooltip text={row.candidate?.decision === "deferred" ? "Deferred update" : "Defer update"}><Button class="decision-selected-{row.candidate?.decision === "deferred"}" variant="ghost" size="icon" type="button" aria-label={row.candidate?.decision === "deferred" ? "Deferred update" : "Defer update"} aria-pressed={row.candidate?.decision === "deferred"} disabled={busy} onclick={() => ondecision?.(row.candidateId!, "deferred")}><ClockIcon size={16} aria-hidden="true" /></Button></Tooltip><Tooltip text={row.candidate?.decision === "blocked" ? "Blocked update" : "Block update"}><Button class="decision-selected-{row.candidate?.decision === "blocked"}" variant="ghost" size="icon" type="button" aria-label={row.candidate?.decision === "blocked" ? "Blocked update" : "Block update"} aria-pressed={row.candidate?.decision === "blocked"} disabled={busy} onclick={() => ondecision?.(row.candidateId!, "blocked")}><ProhibitIcon size={16} aria-hidden="true" /></Button></Tooltip></div>{/if}</div>
          {/if}
        </article>
      {/each}
    </div>
  {/if}
</section>

<style>
  .evidence-summary { display:grid; gap:12px; margin:18px 0 4px; padding:14px; border:1px solid var(--color-border); background:var(--color-surface-raised); }
  .summary-heading,.summary-controls,.evidence-row { display:flex; align-items:center; gap:12px; }
  .summary-heading { justify-content:space-between; }
  .summary-actions,.row-actions,.decision-actions { display:flex; align-items:center; gap:4px; flex-wrap:wrap; justify-content:flex-end; }
  h3,p { margin:0; }
  .eyebrow,.summary-controls label span,.source { color:var(--color-accent-strong); font:700 10px var(--font-mono); letter-spacing:.1em; text-transform:uppercase; }
  .eyebrow { margin-bottom:5px; }
  .count,.summary-controls label span,.evidence-row small { color:var(--color-text-muted); font:11px var(--font-mono); }
  .summary-controls { align-items:flex-end; flex-wrap:wrap; }
  .summary-controls label { display:grid; gap:6px; min-width:180px; }
  input,select { min-height:34px; box-sizing:border-box; padding:7px 9px; border:1px solid var(--color-border); color:var(--color-text); background:var(--color-surface); font:12px var(--font-mono); }
  .evidence-list { display:grid; gap:1px; border:1px solid var(--color-border); background:var(--color-border); }
  .evidence-row { display:grid; grid-template-columns:76px minmax(0,1fr) auto; align-items:center; padding:14px; background:var(--color-surface); }
  .evidence-row > div { display:grid; gap:4px; min-width:0; flex:1; }
  .mod-details { display:grid; grid-template-columns:minmax(0,0.85fr) minmax(0,1.15fr); gap:18px; }.mod-column { display:grid; gap:5px; min-width:0; }.mod-column strong { font-size:13px; }.snapshot-column { padding-left:18px; border-left:1px solid var(--color-border); }.mod-column small { overflow-wrap:anywhere; }
  .evidence-row strong { overflow-wrap:anywhere; }
  .evidence-row small { overflow-wrap:anywhere; }
  .source { min-width:0; color:var(--color-text-muted); }
  .source[data-source="external"] { color:var(--color-warning); }
  .source[data-source="snapshot"] { color:var(--color-accent-strong); }
  .source[data-change="added mod"] { color:var(--color-success, #2f7d4a); }
  .source[data-change="removed mod"] { color:var(--color-danger); }
  .external-status { color:var(--color-warning) !important; text-transform:uppercase; }
  .target-status { font-weight:700; text-transform:uppercase; }.target-status.verified { color:var(--color-success) !important; }.target-status.unverified { color:var(--color-danger) !important; }
  .decision-actions :global(.decision-selected-true) { color:var(--color-on-accent); background:var(--color-accent); border-color:var(--color-accent); }
  .decision-actions :global(.decision-selected-true:hover:not(:disabled)) { filter:brightness(1.08); }
  .empty { color:var(--color-text-muted); line-height:1.6; }
  @media (max-width:760px) { .evidence-row { grid-template-columns:1fr; align-items:flex-start; gap:10px; }.summary-heading,.evidence-row { align-items:flex-start; }.summary-controls label { width:100%; }.source { min-width:0; }.summary-actions,.row-actions,.decision-actions { justify-content:flex-start; }.evidence-status { width:100%; display:flex; justify-content:space-between; gap:8px; }.mod-details { width:100%; grid-template-columns:1fr; }.snapshot-column { padding-left:0; border-top:1px solid var(--color-border); border-left:0; padding-top:8px; } }
</style>