<script lang="ts">
  import { onMount } from "svelte";
  import { Archive, ArrowCounterClockwise, DotsThree, Eye, PencilSimple } from "phosphor-svelte";
  import DOMPurify from "dompurify";
  import { marked } from "marked";
  import Button from "../ui/Button.svelte";
  import Modal from "../ui/Modal.svelte";
  import type { ChangelogArtifact, ChangelogRevision, ChangelogProgress } from "../../lib/domain";

  let {
    artifacts = [],
    revisions = [],
    selectedRevision = null,
    draft = $bindable(""),
    busy = false,
    changelogBusy = false,
    changelogProgress = null,
    oncancel,
    ongenerate,
    oncreateBlank,
    onselect,
    onselectionchange,
    onsave,
    onarchive,
  }: {
    artifacts?: ChangelogArtifact[];
    revisions?: ChangelogRevision[];
    selectedRevision?: ChangelogRevision | null;
    draft?: string;
    busy?: boolean;
    changelogBusy?: boolean;
    changelogProgress?: ChangelogProgress | null;
    oncancel?: () => void | Promise<void>;
    ongenerate: () => void | Promise<void>;
    oncreateBlank: () => void | Promise<void>;
    onselect: (revisionId: string) => void | Promise<void>;
    onselectionchange?: (versionIds: string[]) => void | Promise<void>;
    onsave: () => void | Promise<void>;
    onarchive?: (revision: ChangelogRevision) => void | Promise<void>;
  } = $props();

  let preview = $state(false);
  let showArchived = $state(false);
  let pendingArchive = $state<ChangelogRevision | null>(null);
  let archiveModalOpen = $state(false);

  const artifactById = $derived(new Map(artifacts.map((artifact) => [artifact.id, artifact])));
  const visibleArtifacts = $derived(artifacts.filter((artifact) => revisions.some((revision) => revision.artifact_id === artifact.id)));
  const selectedArtifact = $derived(selectedRevision ? artifactById.get(selectedRevision.artifact_id) ?? null : null);
  const selectedIsArchived = $derived(Boolean(selectedRevision?.archived_at));
  const hasDraftChanges = $derived(Boolean(selectedRevision && draft !== selectedRevision.content));
  const foundVersions = $derived(
    selectedArtifact?.entries.flatMap((entry) =>
      entry.versions.filter((version) => version.included && version.content_status === "available"),
    ) ?? [],
  );
  let selectedVersionIds = $state<string[]>([]);

  $effect(() => {
    if (!selectedArtifact) {
      selectedVersionIds = [];
      return;
    }
    selectedVersionIds = selectedRevision?.selected_version_ids.length
      ? selectedRevision.selected_version_ids
      : foundVersions.map((version) => version.version.version_id);
  });

  function toggleVersion(versionId: string, checked: boolean): void {
    selectedVersionIds = checked
      ? [...new Set([...selectedVersionIds, versionId])]
      : selectedVersionIds.filter((id) => id !== versionId);
    void onselectionchange?.(selectedVersionIds);
  }

  function selectAllVersions(include: boolean): void {
    selectedVersionIds = include
      ? foundVersions.map((version) => version.version.version_id)
      : [];
    void onselectionchange?.(selectedVersionIds);
  }

  onMount(() => {
    function closeMenus(event: MouseEvent): void {
      const target = event.target;
      if (target instanceof Element && target.closest(".revision-menu")) return;
      document.querySelectorAll<HTMLDetailsElement>(".revision-menu[open]").forEach((menu) => (menu.open = false));
    }

    function closeMenusOnEscape(event: KeyboardEvent): void {
      if (event.key !== "Escape") return;
      document.querySelectorAll<HTMLDetailsElement>(".revision-menu[open]").forEach((menu) => (menu.open = false));
    }

    document.addEventListener("click", closeMenus);
    document.addEventListener("keydown", closeMenusOnEscape);
    return () => {
      document.removeEventListener("click", closeMenus);
      document.removeEventListener("keydown", closeMenusOnEscape);
    };
  });

  function revisionsForArtifact(artifactId: string): ChangelogRevision[] {
    return revisions
      .filter((revision) => revision.artifact_id === artifactId && (showArchived || !selectedIsArchivedRevision(revision)))
      .sort((left, right) => Number(right.created_at) - Number(left.created_at));
  }

  function selectedIsArchivedRevision(revision: ChangelogRevision): boolean {
    return Boolean(revision.archived_at);
  }

  function formatTimestamp(value: string): string {
    const numericValue = Number(value);
    const date = Number.isFinite(numericValue) ? new Date(numericValue * 1000) : new Date(value);
    if (Number.isNaN(date.getTime())) return value;
    return new Intl.DateTimeFormat(undefined, { dateStyle: "medium", timeStyle: "short" }).format(date);
  }

  function artifactLabel(artifact: ChangelogArtifact): string {
    if (artifact.status === "complete") return artifact.content.trim() ? "Generated proposal" : "Blank proposal";
    return `${artifact.status.replaceAll("_", " ")} proposal`;
  }

  function revisionLabel(revision: ChangelogRevision, artifactRevisions: ChangelogRevision[]): string {
    const position = artifactRevisions.findIndex((item) => item.id === revision.id);
    return `Revision ${artifactRevisions.length - position}`;
  }

  function renderMarkdown(source: string): string {
    return DOMPurify.sanitize(marked.parse(source, { async: false }));
  }

  function requestArchive(revision: ChangelogRevision): void {
    if (revision.archived_at) {
      void onarchive?.(revision);
      return;
    }
    pendingArchive = revision;
    archiveModalOpen = true;
  }

  function confirmArchive(): void {
    if (!pendingArchive) return;
    const revision = pendingArchive;
    pendingArchive = null;
    archiveModalOpen = false;
    void onarchive?.(revision);
  }

  function handleRevisionAction(event: MouseEvent, revision: ChangelogRevision): void {
    const details = (event.currentTarget as HTMLElement).closest("details");
    if (details) details.open = false;
    requestArchive(revision);
  }
</script>

<div class="changelog-panel">
  <div class="panel-heading">
    <div><p class="eyebrow">Proposed changelog</p><h3>Choose the revision to finalize</h3></div>
    <div class="panel-actions">
      <Button size="sm" variant="quiet" type="button" disabled={busy || changelogBusy} loading={changelogBusy} onclick={ongenerate}>Generate proposal</Button>
      <Button size="sm" variant="quiet" type="button" disabled={busy || changelogBusy} onclick={oncreateBlank}>Blank proposal</Button>
    </div>
  </div>

  {#if changelogBusy && changelogProgress}
    <div class="generation-progress" role="status" aria-live="polite">
      <div><strong>Generating changelog</strong><span>{changelogProgress.message}</span></div>
      <span class="progress-count">{changelogProgress.completed} / {changelogProgress.total}</span>
      {#if changelogProgress.cancellable && oncancel}<Button variant="danger" size="sm" type="button" onclick={oncancel}>Cancel</Button>{/if}
    </div>
  {/if}

  {#if revisions.length === 0}
    <p class="muted">No proposed revision exists yet. Generate one from workspace evidence or start with a blank proposal.</p>
  {:else}
    <div class="artifact-list">
      {#each visibleArtifacts as artifact (artifact.id)}
        {@const artifactRevisions = revisionsForArtifact(artifact.id)}
        <section class="artifact-group" aria-labelledby={`artifact-${artifact.id}`}>
          <header class="artifact-heading">
            <div>
              <p class="eyebrow">{artifactLabel(artifact)}</p>
              <h4 id={`artifact-${artifact.id}`}>{formatTimestamp(artifact.created_at)}</h4>
            </div>
            <span class="artifact-meta">{artifactRevisions.length} {artifactRevisions.length === 1 ? "revision" : "revisions"}</span>
          </header>
          <div class="revision-list">
            {#each artifactRevisions as revision (revision.id)}
              <label class:selected={selectedRevision?.id === revision.id}>
                <input type="radio" name="workspace-changelog" checked={selectedRevision?.id === revision.id} onchange={() => onselect(revision.id)} />
                <span class="revision-copy"><strong>{revisionLabel(revision, artifactRevisions)}</strong><small>{formatTimestamp(revision.created_at)} · {revision.content.length} characters{revision.is_current ? " · Current" : ""}{revision.frozen ? " · Frozen" : ""}</small></span>
                {#if onarchive}
                  <details class="revision-menu">
                    <summary aria-label={`More actions for ${revisionLabel(revision, artifactRevisions)}`}><DotsThree size={18} weight="bold" /></summary>
                    <div class="revision-menu-items">
                      <Button variant="ghost" size="sm" type="button" disabled={busy || changelogBusy || revision.frozen} onclick={(event) => handleRevisionAction(event, revision)}>{#if revision.archived_at}<ArrowCounterClockwise size={15} /> Restore{:else}<Archive size={15} /> Mark as old{/if}</Button>
                    </div>
                  </details>
                {/if}
              </label>
            {/each}
          </div>
        </section>
      {/each}
    </div>
    {#if revisions.some(selectedIsArchivedRevision)}
      <Button variant="ghost" size="sm" type="button" onclick={() => (showArchived = !showArchived)}>{showArchived ? "Hide old revisions" : "Show old revisions"}</Button>
    {/if}
  {/if}

  {#if selectedArtifact && foundVersions.length}
    <section class="found-changelogs" aria-labelledby="found-changelogs-heading">
      <header class="panel-heading"><div><p class="eyebrow">Found changelogs</p><h3 id="found-changelogs-heading">Choose releases to include</h3></div><div class="panel-actions"><Button variant="quiet" size="sm" type="button" disabled={busy || changelogBusy} onclick={() => selectAllVersions(true)}>Add all</Button><Button variant="ghost" size="sm" type="button" disabled={busy || changelogBusy} onclick={() => selectAllVersions(false)}>Remove all</Button></div></header>
      <p class="muted">{selectedVersionIds.length} of {foundVersions.length} changelogs selected.</p>
      <div class="found-list">
        {#each selectedArtifact.entries as entry (entry.local.entry_id)}
          {#if entry.versions.length}
            <div class="found-mod"><strong>{entry.local.local_identity === "unknown" ? "Unknown mod" : typeof entry.local.local_identity === "object" && "observed" in entry.local.local_identity ? entry.local.local_identity.observed : "Mod"}</strong>
              {#each entry.versions.filter((version) => version.included && version.content_status === "available") as version (version.version.version_id)}
                <label class:unavailable={version.content_status !== "available"}><input type="checkbox" checked={selectedVersionIds.includes(version.version.version_id)} disabled={busy || changelogBusy || version.content_status !== "available"} onchange={(event) => toggleVersion(version.version.version_id, event.currentTarget.checked)} /><span><b>{version.version.version_number ?? "Provider release"}</b><small>{version.content_status === "available" ? "Changelog available" : "No changelog text was published for this version; it will not be added to the generated Markdown."}</small></span></label>
              {/each}
            </div>
          {/if}
        {/each}
      </div>
    </section>
  {/if}

  {#if selectedRevision}
    <section class="revision-editor" aria-label="Selected proposed revision">
      <header class="editor-heading">
        <div><span class="proposal-label">{selectedIsArchived ? "OLD · NOT FINAL" : "PROPOSED · NOT FINAL"}</span><strong>{selectedArtifact ? artifactLabel(selectedArtifact) : "Selected revision"}</strong></div>
        <Button variant="ghost" size="sm" type="button" disabled={busy || changelogBusy} aria-pressed={preview} onclick={() => (preview = !preview)}>{#if preview}<PencilSimple size={15} /> Edit source{:else}<Eye size={15} /> Preview{/if}</Button>
      </header>
      {#if preview}
        <div class="revision-preview markdown-preview">{@html renderMarkdown(draft)}</div>
      {:else}
        <label class="editor-field"><span>Edit proposed revision</span><textarea rows="12" bind:value={draft} disabled={busy || changelogBusy || selectedIsArchived}></textarea></label>
        <div class="editor-footer"><span class="draft-meta">{draft.length} characters{hasDraftChanges ? " · Unsaved changes" : ""}</span><Button size="sm" variant="quiet" type="button" disabled={busy || changelogBusy || selectedIsArchived || !hasDraftChanges} onclick={onsave}>Save new proposed revision</Button></div>
      {/if}
    </section>
  {/if}
</div>

<Modal bind:open={archiveModalOpen} title="Mark revision as old" onclose={() => { archiveModalOpen = false; pendingArchive = null; }}>
  {#if pendingArchive}
    <p class="muted">This removes the revision from the active proposal list but keeps it in revision history so it can be restored later.</p>
    <div class="confirm-actions"><Button variant="quiet" type="button" onclick={() => { archiveModalOpen = false; pendingArchive = null; }}>Keep revision active</Button><Button variant="warning" type="button" onclick={confirmArchive}>Mark as old</Button></div>
  {/if}
</Modal>

<style>
  .changelog-panel { display:grid; gap:12px; }
  .generation-progress { display:flex; align-items:center; gap:12px; padding:10px 12px; border-left:3px solid var(--color-accent-strong); background:var(--color-surface-raised); }
  .generation-progress > div { display:grid; gap:3px; min-width:0; flex:1; }.generation-progress span { color:var(--color-text-muted); font:11px var(--font-mono); }.progress-count { white-space:nowrap; }
  .panel-heading,.artifact-heading,.editor-heading,.editor-footer { display:flex; align-items:center; justify-content:space-between; gap:12px; }
  h3,h4,p { margin:0; }
  .panel-actions,.editor-footer { display:flex; align-items:center; gap:12px; flex-wrap:wrap; }
  .eyebrow,.proposal-label { color:var(--color-accent-strong); font:700 10px var(--font-mono); letter-spacing:.12em; text-transform:uppercase; }
  .artifact-list { display:grid; gap:12px; }
  .artifact-group { display:grid; gap:8px; padding:12px; border:1px solid var(--color-border); background:var(--color-surface); }
  .artifact-heading { align-items:flex-start; }
  .artifact-heading h4 { margin-top:4px; font:700 13px var(--font-mono); }
  .artifact-meta,.revision-list small,.draft-meta,.muted { color:var(--color-text-muted); font:11px var(--font-mono); }
  .revision-list { display:grid; gap:1px; border:1px solid var(--color-border); background:var(--color-border); }
  .revision-list label { display:flex; align-items:center; gap:10px; min-width:0; padding:11px 12px; background:var(--color-surface-raised); cursor:pointer; }
  .revision-list label.selected { outline:2px solid var(--color-accent-strong); outline-offset:-2px; }
  .revision-copy { display:grid; gap:4px; min-width:0; flex:1; }
  .revision-copy small { overflow:hidden; text-overflow:ellipsis; white-space:nowrap; }
  .revision-menu { position:relative; flex:none; }
  .revision-menu summary { display:grid; place-items:center; width:30px; height:30px; color:var(--color-text-muted); cursor:pointer; list-style:none; }
  .revision-menu summary::-webkit-details-marker { display:none; }
  .revision-menu-items { position:absolute; z-index:2; top:32px; right:0; min-width:150px; padding:5px; border:1px solid var(--color-border); background:var(--color-surface); box-shadow:var(--shadow-md); }
  .revision-menu-items :global(.button) { width:100%; justify-content:flex-start; }
  .revision-editor { display:grid; gap:12px; padding:14px; border:1px solid var(--color-border); background:var(--color-surface); }
  .found-changelogs { display:grid; gap:10px; padding:14px; border:1px solid var(--color-border); background:var(--color-surface); }
  .found-list,.found-mod { display:grid; gap:1px; }
  .found-mod { border:1px solid var(--color-border); background:var(--color-border); }
  .found-mod > strong { padding:9px 10px; background:var(--color-surface-raised); font:700 12px var(--font-mono); }
  .found-mod label { display:flex; align-items:flex-start; gap:9px; padding:9px 10px; background:var(--color-surface); cursor:pointer; }
  .found-mod label.unavailable { color:var(--color-text-muted); cursor:not-allowed; }
  .found-mod label span { display:grid; gap:3px; }.found-mod small { color:var(--color-text-muted); font:11px var(--font-mono); line-height:1.4; }
  .editor-heading { align-items:flex-start; }
  .editor-heading > div { display:grid; gap:5px; }
  .editor-heading strong { font-size:13px; }
  .editor-field { display:grid; gap:6px; }
  .editor-field span { color:var(--color-accent-strong); font:700 11px var(--font-mono); text-transform:uppercase; }
  textarea { width:100%; box-sizing:border-box; resize:vertical; color:var(--color-text); background:var(--color-surface); border:1px solid var(--color-border); padding:10px; font:12px/1.5 var(--font-mono); }
  .editor-footer { justify-content:space-between; }
  .revision-preview { min-height:260px; max-height:460px; overflow:auto; padding:12px; border:1px solid var(--color-border); background:var(--color-surface-raised); }
  .markdown-preview :global(h2),.markdown-preview :global(h3),.markdown-preview :global(h4) { margin:0 0 10px; }
  .markdown-preview :global(p) { margin:0 0 10px; line-height:1.6; }
  .markdown-preview :global(li) { margin-left:20px; line-height:1.6; }
  .confirm-actions { display:flex; justify-content:flex-end; gap:12px; margin-top:20px; }
  @media (max-width:640px) { .panel-heading,.artifact-heading,.editor-heading { align-items:flex-start; flex-direction:column; }.panel-actions { width:100%; }.editor-footer { align-items:flex-start; flex-direction:column; }.editor-footer :global(.button) { align-self:flex-end; }.revision-copy small { white-space:normal; } }
</style>
