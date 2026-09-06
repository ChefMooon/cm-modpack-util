<script lang="ts">
  import { onMount } from "svelte";
  import FolderSimpleIcon from "phosphor-svelte/lib/FolderSimpleIcon";
  import PlusIcon from "phosphor-svelte/lib/PlusIcon";
  import ArrowClockwiseIcon from "phosphor-svelte/lib/ArrowClockwiseIcon";
  import LinkBreakIcon from "phosphor-svelte/lib/LinkBreakIcon";
  import LinkIcon from "phosphor-svelte/lib/LinkIcon";
  import ArchiveIcon from "phosphor-svelte/lib/ArchiveIcon";
  import PencilSimpleIcon from "phosphor-svelte/lib/PencilSimpleIcon";
  import MagnifyingGlassIcon from "phosphor-svelte/lib/MagnifyingGlassIcon";
  import ArrowSquareOutIcon from "phosphor-svelte/lib/ArrowSquareOutIcon";
  import XIcon from "phosphor-svelte/lib/XIcon";
  import CheckCircleIcon from "phosphor-svelte/lib/CheckCircleIcon";
  import WarningCircleIcon from "phosphor-svelte/lib/WarningCircleIcon";
  import Header from "../components/header/Header.svelte";
  import Button from "../components/ui/Button.svelte";
  import Modal from "../components/ui/Modal.svelte";
  import Tooltip from "../components/ui/Tooltip.svelte";
  import { useToast } from "../components/ui/toast/toast.svelte";
  import { commandErrorMessage } from "../lib/errors";
  import type {
    ApplicationProjectMetadata,
    Evidence,
    InventoryEntry,
    ProjectOverview,
    Observation,
    ProjectRecord,
    RegistrationPreview,
    ValidationResult,
  } from "../lib/domain";
  import {
    archiveProject,
    chooseProjectDirectory,
    disconnectProject,
    listProjects,
    getProjectInventory,
    getProjectOverview,
    openTrustedPage,
    previewProject,
    reconnectProject,
    refreshProject,
    registerProject,
    restoreProject,
    updateProjectMetadata,
  } from "../lib/projects";

  let projects = $state<ProjectRecord[]>([]);
  let loading = $state(true);
  let busy = $state(false);
  let error = $state("");
  let selected = $state<RegistrationPreview | null>(null);
  let previewPath = $state("");
  let reconnecting = $state<ProjectRecord | null>(null);
  let editing = $state<ProjectRecord | null>(null);
  let metadataDraft = $state<ApplicationProjectMetadata | null>(null);
  let tagsText = $state("");
  let showPreview = $state(false);
  let showReconnect = $state(false);
  let showEdit = $state(false);
  let showArchived = $state(false);
  let inspected = $state<ProjectRecord | null>(null);
  let inventory = $state<InventoryEntry[]>([]);
  let overview = $state<ProjectOverview | null>(null);
  let inventoryFilter = $state("all");
  let inspecting = $state(false);
  const { toast } = useToast();

  onMount(loadProjects);

  async function loadProjects() {
    loading = true;
    error = "";
    try {
      projects = await listProjects();
    } catch (cause) {
      error = commandErrorMessage(cause);
    } finally {
      loading = false;
    }
  }

  async function selectProject() {
    error = "";
    const path = await chooseProjectDirectory();
    if (!path) return;
    busy = true;
    try {
      selected = await previewProject(path);
      previewPath = path;
      tagsText = selected.application_defaults.tags.join(", ");
      showPreview = true;
    } catch (cause) {
      error = commandErrorMessage(cause);
    } finally {
      busy = false;
    }
  }

  async function confirmRegistration() {
    if (!selected) return;
    busy = true;
    try {
      await registerProject(previewPath, selected.application_defaults);
      showPreview = false;
      selected = null;
      await loadProjects();
      toast({
        title: "Project registered",
        description: "The external Packwiz directory was not modified.",
        severity: "success",
      });
    } catch (cause) {
      error = commandErrorMessage(cause);
    } finally {
      busy = false;
    }
  }

  async function refresh(project: ProjectRecord) {
    busy = true;
    try {
      replace(await refreshProject(project.id));
      if (inspected?.id === project.id) await inspectProject(project);
      toast({ title: "Project refreshed", severity: "success" });
    } catch (cause) {
      error = commandErrorMessage(cause);
    } finally {
      busy = false;
    }
  }

  async function inspectProject(project: ProjectRecord) {
    inspected = project;
    inspecting = true;
    error = "";
    try {
      [inventory, overview] = await Promise.all([
        getProjectInventory(project.id),
        getProjectOverview(project.id),
      ]);
    } catch (cause) {
      inventory = [];
      overview = null;
      error = commandErrorMessage(cause);
    } finally {
      inspecting = false;
    }
  }

  async function openPage(entry: InventoryEntry) {
    if (!entry.page_link) return;
    try {
      await openTrustedPage(entry.page_link.url);
    } catch (cause) {
      error = commandErrorMessage(cause);
    }
  }

  function beginEdit(project: ProjectRecord) {
    editing = project;
    metadataDraft = {
      ...project.application,
      tags: [...project.application.tags],
    };
    tagsText = project.application.tags.join(", ");
    showEdit = true;
  }

  async function saveMetadata() {
    if (!editing || !metadataDraft) return;
    busy = true;
    try {
      replace(await updateProjectMetadata(editing.id, {
        ...metadataDraft,
        tags: tagsText.split(",").map((tag) => tag.trim()).filter(Boolean),
      }));
      showEdit = false;
      editing = null;
      metadataDraft = null;
      toast({ title: "Project details updated", severity: "success" });
    } catch (cause) {
      error = commandErrorMessage(cause);
    } finally {
      busy = false;
    }
  }

  async function changeLifecycle(
    project: ProjectRecord,
    action: "archive" | "restore" | "disconnect",
  ) {
    busy = true;
    try {
      const result =
        action === "archive"
          ? await archiveProject(project.id)
          : action === "restore"
            ? await restoreProject(project.id)
            : await disconnectProject(project.id);
      replace(result);
    } catch (cause) {
      error = commandErrorMessage(cause);
    } finally {
      busy = false;
    }
  }

  async function beginReconnect(project: ProjectRecord) {
    reconnecting = project;
    showReconnect = true;
  }

  async function confirmReconnect() {
    if (!reconnecting) return;
    const path = await chooseProjectDirectory();
    if (!path) return;
    busy = true;
    try {
      replace(await reconnectProject(reconnecting.id, path));
      showReconnect = false;
      reconnecting = null;
      toast({ title: "Project reconnected", severity: "success" });
    } catch (cause) {
      error = commandErrorMessage(cause);
    } finally {
      busy = false;
    }
  }

  function replace(project: ProjectRecord) {
    projects = projects.map((candidate) =>
      candidate.id === project.id ? project : candidate,
    );
  }

  function observed(value: Observation<string>): string {
    return typeof value === "string" ? "Unavailable" : value.observed;
  }

  function hasErrors(results: ValidationResult[]) {
    return results.some((result) => result.severity === "error");
  }

  function evidenceLabel<T>(value: Evidence<T>): string {
    if (typeof value === "string") return value === "unknown" ? "Unknown" : "Unavailable";
    if ("observed" in value) return String(value.observed);
    return `Malformed: ${value.malformed.message}`;
  }

  const filteredInventory = $derived(
    inventory.filter((entry) => {
      if (inventoryFilter === "all") return true;
      if (inventoryFilter === "pinned") return evidenceLabel(entry.pin) === "true";
      if (inventoryFilter === "unknown") {
        return [entry.provider, entry.side, entry.source_url, entry.version].some(
          (value) => evidenceLabel(value).startsWith("Unknown") || evidenceLabel(value).startsWith("Unavailable") || evidenceLabel(value).startsWith("Malformed"),
        );
      }
      return evidenceLabel(entry.side).toLowerCase() === inventoryFilter;
    }),
  );
  function statusLabel(project: ProjectRecord) {
    return project.application.lifecycle === "disconnected"
      ? "Disconnected"
      : project.application.lifecycle === "archived"
        ? "Archived"
        : hasErrors(project.validation)
          ? "Needs attention"
          : "Ready";
  }

  const visibleProjects = $derived(
    showArchived
      ? projects
      : projects.filter((project) => project.application.lifecycle !== "archived"),
  );
</script>

<svelte:head>
  <title>Projects · CM Modpack Util</title>
  <meta
    name="description"
    content="Register and manage local Packwiz projects."
  />
</svelte:head>

<main class="projects-shell">
  <Header />
  <section class="hero" aria-labelledby="projects-title">
    <div>
      <p class="eyebrow">Projects</p>
      <h1 id="projects-title">Your local modpack workspace</h1>
      <p class="lede">
        Register a Packwiz directory to review its local evidence and manage it
        without changing the external files.
      </p>
    </div>
    <Button
      variant="primary"
      type="button"
      loading={busy}
      onclick={selectProject}
      ><PlusIcon size={17} weight="bold" /> Register project</Button
    >
  </section>

  {#if error}<div class="error-banner" role="alert">
      <WarningCircleIcon size={18} aria-hidden="true" /><span>{error}</span
      ><Button
        variant="ghost"
        size="sm"
        type="button"
        onclick={() => (error = "")}>Dismiss</Button
      >
    </div>{/if}
  {#if loading}<section class="state" role="status">
      <div class="loader" aria-hidden="true"></div>
      <p>Loading registered projects...</p>
    </section>
  {:else if projects.length === 0}<section
      class="state"
      aria-labelledby="empty-title"
    >
      <div class="empty-icon" aria-hidden="true">
        <FolderSimpleIcon size={34} weight="duotone" />
      </div>
      <p class="eyebrow">No registered projects</p>
      <h2 id="empty-title">Start with a local Packwiz directory</h2>
      <p>
        Choose a project folder to preview its Packwiz evidence before anything
        is registered.
      </p>
      <Button variant="secondary" type="button" onclick={selectProject}
        ><PlusIcon size={16} /> Choose directory</Button
      >
    </section>
  {:else}<section class="project-list" aria-labelledby="list-title">
      <div class="list-heading">
        <div>
          <p class="eyebrow">Registered projects</p>
          <h2 id="list-title">Local project records</h2>
        </div>
        <div class="list-controls">
          <span class="count"
            >{visibleProjects.length} active
            {visibleProjects.length === 1 ? "project" : "projects"}</span
          >
          <Button
            variant="ghost"
            size="sm"
            type="button"
            onclick={() => (showArchived = !showArchived)}
            >{showArchived ? "Hide archived" : "Show archived"}</Button
          >
        </div>
      </div>
      {#if visibleProjects.length === 0}<p class="no-results">
        Archived projects are hidden from the active list.
      </p>{/if}
      {#each visibleProjects as project (project.id)}<article
          class:disconnected={project.application.lifecycle === "disconnected"}
          class:archived={project.application.lifecycle === "archived"}
          class="project-row"
        >
          <div class="project-mark" aria-hidden="true">
            {#if project.application.lifecycle === "disconnected"}<LinkBreakIcon
                size={22}
              />{:else if hasErrors(project.validation)}<WarningCircleIcon
                size={22}
              />{:else}<CheckCircleIcon size={22} />{/if}
          </div>
          <div class="project-main">
            <div class="project-title">
              <h3>{project.application.display_name}</h3>
              <span class="status" data-status={project.application.lifecycle}
                >{statusLabel(project)}</span
              >
            </div>
            <p class="path">{project.canonical_path}</p>
            <div class="evidence">
              <span>Packwiz name: {observed(project.packwiz.name)}</span><span
                >Version: {observed(project.packwiz.version)}</span
              ><span
                >Loader data: {project.packwiz.declared_versions.length
                  ? "Observed"
                  : "Unavailable"}</span
              >
            </div>
            {#if hasErrors(project.validation)}<ul class="issues">
                {#each project.validation.filter((item) => item.severity === "error") as issue}<li
                  >
                    {issue.message}
                  </li>{/each}
              </ul>{/if}
          </div>
          <div class="actions">
            <Button
              variant="secondary"
              size="sm"
              type="button"
              disabled={busy || inspecting}
              onclick={() => inspectProject(project)}
              ><MagnifyingGlassIcon size={15} /> Inspect</Button
            >
            <Tooltip text="Edit application-owned project details"
              ><Button
                variant="ghost"
                size="icon"
                type="button"
                aria-label={`Edit ${project.application.display_name}`}
                disabled={busy}
                onclick={() => beginEdit(project)}
                ><PencilSimpleIcon size={17} /></Button
              ></Tooltip
            >
            {#if project.application.lifecycle === "disconnected"}<Button
                variant="secondary"
                size="sm"
                type="button"
                disabled={busy}
                onclick={() => beginReconnect(project)}
                ><LinkIcon size={15} /> Reconnect</Button
              >{:else}<Tooltip text="Read local Packwiz evidence again"
                ><Button
                  variant="ghost"
                  size="icon"
                  type="button"
                  aria-label={`Refresh ${project.application.display_name}`}
                  disabled={busy}
                  onclick={() => refresh(project)}
                  ><ArrowClockwiseIcon size={17} /></Button
                ></Tooltip
              >{#if project.application.lifecycle === "archived"}<Button
                  variant="secondary"
                  size="sm"
                  type="button"
                  disabled={busy}
                  onclick={() => changeLifecycle(project, "restore")}
                  >Restore</Button
                >{:else}<Button
                  variant="ghost"
                  size="sm"
                  type="button"
                  disabled={busy}
                  onclick={() => changeLifecycle(project, "archive")}
                  ><ArchiveIcon size={15} /> Archive</Button
                >{/if}<Button
                variant="ghost"
                size="sm"
                type="button"
                disabled={busy}
                onclick={() => changeLifecycle(project, "disconnect")}
                >Disconnect</Button
              >{/if}
          </div>
        </article>{/each}
    </section>
    {#if inspected}
      <section class="inspection" aria-labelledby="inspection-title" aria-live="polite">
        <div class="inspection-heading">
          <div>
            <p class="eyebrow">Current local evidence</p>
            <h2 id="inspection-title">{inspected.application.display_name}</h2>
            <p class="path">Read-only observation from the registered project boundary.</p>
          </div>
          <div class="inspection-actions">
            <Button variant="ghost" size="icon" type="button" aria-label="Close inspection" onclick={() => (inspected = null)}><XIcon size={17} /></Button>
            <Button variant="secondary" size="sm" type="button" loading={inspecting} onclick={() => inspectProject(inspected!)}><ArrowClockwiseIcon size={15} /> Re-read</Button>
          </div>
        </div>
        {#if inspecting}
          <div class="inspection-state" role="status"><div class="loader" aria-hidden="true"></div><p>Reading local Packwiz evidence...</p></div>
        {:else if !overview}
          <div class="inspection-state" role="status"><WarningCircleIcon size={20} /><p>Current inventory is unavailable. The registered project record is still preserved.</p></div>
        {:else}
          <div class="overview-grid">
            <div><span class="label">Minecraft</span><strong>{evidenceLabel(overview.minecraft_version)}</strong></div>
            <div><span class="label">Loader</span><strong>{evidenceLabel(overview.loader)}</strong></div>
            <div><span class="label">Mods</span><strong>{overview.inventory_counts.total}</strong></div>
            <div><span class="label">Git</span><strong>{overview.git.state.replaceAll("_", " ")}</strong></div>
            <div><span class="label">Providers</span><strong>{overview.inventory_counts.provider_modrinth} Modrinth · {overview.inventory_counts.provider_curseforge} CurseForge · {overview.inventory_counts.provider_unknown} unknown</strong></div>
            <div><span class="label">Sides</span><strong>{overview.inventory_counts.side_client} client · {overview.inventory_counts.side_server} server · {overview.inventory_counts.side_both} both · {overview.inventory_counts.side_unknown} unknown</strong></div>
          </div>
          <div class="validation-summary">
            <span class="label">Validation evidence</span>
            {#if overview.validation.length === 0}<span class="valid-text"><CheckCircleIcon size={15} /> Required local evidence is valid.</span>{:else}{#each overview.validation as item}<span class:item-warning={item.severity === "warning"} class:item-error={item.severity === "error"}><strong>{item.severity}</strong> {item.message}</span>{/each}{/if}
          </div>
          <div class="inventory-heading">
            <div><p class="eyebrow">Mod inventory</p><h3>{filteredInventory.length} of {inventory.length} entries</h3></div>
            <label class="filter-label">Filter<select bind:value={inventoryFilter}><option value="all">All entries</option><option value="pinned">Pinned</option><option value="client">Client</option><option value="server">Server</option><option value="both">Both sides</option><option value="unknown">Unknown evidence</option></select></label>
          </div>
          <div class="inventory-table" role="table" aria-label="Current mod inventory">
            <div class="inventory-row inventory-header" role="row"><span>Local entry</span><span>Provider</span><span>Side</span><span>Pin</span><span>Page</span></div>
            {#each filteredInventory as entry (entry.local_id)}
              <div class="inventory-row" role="row">
                <div><strong>{evidenceLabel(entry.name)}</strong><span class="path">{entry.local_id}</span><span class="metadata-path">{entry.metadata_path}</span></div>
                <span>{evidenceLabel(entry.provider)}</span>
                <span>{evidenceLabel(entry.side)}</span>
                <span>{evidenceLabel(entry.pin)}</span>
                {#if entry.page_link}<Button variant="quiet" size="sm" type="button" onclick={() => openPage(entry)}><ArrowSquareOutIcon size={14} /> Open</Button>{:else}<span class="unavailable">Unavailable</span>{/if}
              </div>
            {/each}
          </div>
          {#if overview.activity.length}
            <div class="activity-section">
              <p class="eyebrow">Recent activity</p>
              <div class="activity-list">
                {#each overview.activity as event}
                  <div class="activity-item"><span class="label">{event.occurred_at}</span><strong>{event.event_type.replaceAll("_", " ")}</strong><span>{event.message}</span></div>
                {/each}
              </div>
            </div>
          {/if}
        {/if}
      </section>
    {/if}
  {/if}
</main>

<Modal
  bind:open={showPreview}
  title="Review project registration"
  onclose={() => (showPreview = false)}
>
  {#if selected}<p class="modal-lede">
      Review the local evidence before registering <strong
        >{selected.application_defaults.display_name}</strong
      >.
    </p>
    <div class="preview-grid">
      <div>
        <span class="label">Directory</span><code
          >{selected.canonical_path}</code
        >
      </div>
      <div>
        <span class="label">Packwiz name</span><strong
          >{observed(selected.packwiz.name)}</strong
        >
      </div>
      <div>
        <span class="label">Author</span><strong
          >{observed(selected.packwiz.author)}</strong
        >
      </div>
      <div>
        <span class="label">Pack format</span><strong
          >{observed(selected.packwiz.pack_format)}</strong
        >
      </div>
    </div>
    <div class="owned-fields">
      <label class="field"
        >Display name<input
          bind:value={selected.application_defaults.display_name}
        /></label
      >
      <label class="field"
        >Theme / color<input
          value={selected.application_defaults.theme ?? ""}
          oninput={(event) =>
            (selected!.application_defaults.theme = event.currentTarget.value || null)}
        /></label
      >
      <label class="field wide"
        >Tags<input
          value={tagsText}
          placeholder="client, favorite"
          oninput={(event) => (tagsText = event.currentTarget.value)}
        /></label
      >
      <label class="field wide"
        >Description<textarea
          rows="3"
          value={selected.application_defaults.description ?? ""}
          oninput={(event) =>
            (selected!.application_defaults.description = event.currentTarget.value || null)}
        ></textarea></label
      >
      <label class="favorite"
        ><input
          type="checkbox"
          bind:checked={selected.application_defaults.favorite}
        /> Favorite project</label
      >
    </div>
    <div class="validation" aria-live="polite">
      <span class="label">Validation evidence</span
      >{#if selected.validation.length === 0}<p class="valid-line">
          <CheckCircleIcon size={16} /> Required local evidence is valid.
        </p>{:else}{#each selected.validation as item}<p
            class:warning-line={item.severity === "warning"}
            class:error-line={item.severity === "error"}
          >
            <span class="severity">{item.severity}</span>{item.message}
          </p>{/each}{/if}
    </div>
    <div class="modal-actions">
      <Button
        variant="secondary"
        type="button"
        onclick={() => (showPreview = false)}>Cancel</Button
      ><Button
        variant="primary"
        type="button"
        disabled={hasErrors(selected.validation)}
        loading={busy}
        onclick={confirmRegistration}>Register</Button
      >
    </div>{/if}
</Modal>

<Modal
  bind:open={showEdit}
  title="Edit project details"
  onclose={() => (showEdit = false)}
>
  {#if editing && metadataDraft}
    <p class="modal-lede">
      Update application-owned details for <strong
        >{editing.application.display_name}</strong
      >. Packwiz evidence remains read-only.
    </p>
    <div class="owned-fields">
      <label class="field"
        >Display name<input bind:value={metadataDraft.display_name} /></label
      >
      <label class="field"
        >Theme / color<input
          value={metadataDraft.theme ?? ""}
          oninput={(event) =>
            (metadataDraft!.theme = event.currentTarget.value || null)}
        /></label
      >
      <label class="field wide"
        >Tags<input
          value={tagsText}
          placeholder="client, favorite"
          oninput={(event) => (tagsText = event.currentTarget.value)}
        /></label
      >
      <label class="field wide"
        >Description<textarea
          rows="3"
          value={metadataDraft.description ?? ""}
          oninput={(event) =>
            (metadataDraft!.description = event.currentTarget.value || null)}
        ></textarea></label
      >
      <label class="favorite"
        ><input type="checkbox" bind:checked={metadataDraft.favorite} /> Favorite
        project</label
      >
    </div>
    <div class="modal-actions">
      <Button
        variant="secondary"
        type="button"
        onclick={() => (showEdit = false)}>Cancel</Button
      ><Button
        variant="primary"
        type="button"
        loading={busy}
        onclick={saveMetadata}>Save details</Button
      >
    </div>
  {/if}
</Modal>

<Modal
  bind:open={showReconnect}
  title="Reconnect project"
  onclose={() => (showReconnect = false)}
  ><p>
    Select the project directory again. CM Modpack Util will validate it before
    reconnecting this existing record.
  </p>
  <div class="modal-actions">
    <Button
      variant="secondary"
      type="button"
      onclick={() => (showReconnect = false)}>Cancel</Button
    ><Button
      variant="primary"
      type="button"
      loading={busy}
      onclick={confirmReconnect}><LinkIcon size={16} /> Choose directory</Button
    >
  </div></Modal
>

<style>
  .projects-shell {
    min-height: 100vh;
    padding: 0 42px 42px;
    background: var(--color-bg);
  }
  .hero {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: 24px;
    max-width: 1060px;
    margin: 58px auto 34px;
  }
  .eyebrow {
    margin: 0 0 10px;
    color: var(--color-accent-strong);
    font: 700 10px var(--font-mono);
    letter-spacing: 0.12em;
    text-transform: uppercase;
  }
  h1,
  h2,
  h3 {
    margin: 0;
  }
  h1 {
    max-width: 680px;
    font-size: clamp(28px, 5vw, 46px);
    line-height: 1.04;
  }
  h2 {
    font-size: 20px;
  }
  h3 {
    font-size: 16px;
  }
  .lede {
    max-width: 620px;
    margin: 14px 0 0;
    color: var(--color-text-muted);
    font-size: 15px;
    line-height: 1.55;
  }
  .error-banner {
    display: flex;
    align-items: center;
    gap: 10px;
    max-width: 1060px;
    margin: 0 auto 20px;
    padding: 12px 14px;
    border: 1px solid var(--color-danger);
    color: var(--color-danger);
    background: color-mix(
      in srgb,
      var(--color-danger) 8%,
      var(--color-surface)
    );
  }
  .error-banner span {
    flex: 1;
  }
  .state,
  .project-list {
    max-width: 1060px;
    margin: 0 auto;
    border: 1px solid var(--color-border);
    background: var(--color-surface);
  }
  .state {
    display: grid;
    justify-items: center;
    padding: 72px 28px;
    text-align: center;
    box-shadow: 5px 5px 0 var(--color-text);
  }
  .state > p:not(.eyebrow) {
    max-width: 540px;
    margin: 0 0 22px;
    color: var(--color-text-muted);
    line-height: 1.6;
  }
  .empty-icon {
    display: grid;
    width: 68px;
    height: 68px;
    margin-bottom: 24px;
    place-items: center;
    border: 1px solid var(--color-accent);
    color: var(--color-accent-strong);
    background: color-mix(
      in srgb,
      var(--color-accent) 12%,
      var(--color-surface)
    );
  }
  .loader {
    width: 22px;
    height: 22px;
    margin-bottom: 16px;
    border: 2px solid var(--color-border);
    border-top-color: var(--color-accent);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  .list-heading {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    padding: 24px;
    border-bottom: 1px solid var(--color-border);
  }
  .list-controls {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .count,
  .path,
  .evidence,
  .label {
    color: var(--color-text-muted);
    font: 11px var(--font-mono);
  }
  .project-row {
    display: flex;
    gap: 16px;
    padding: 22px 24px;
    border-bottom: 1px solid var(--color-border);
  }
  .project-row:last-child {
    border-bottom: 0;
  }
  .project-row.disconnected,
  .project-row.archived {
    background: color-mix(
      in srgb,
      var(--color-surface-raised) 55%,
      var(--color-surface)
    );
  }
  .project-mark {
    color: var(--color-success);
    padding-top: 2px;
  }
  .disconnected .project-mark {
    color: var(--color-warning);
  }
  .project-main {
    min-width: 0;
    flex: 1;
  }
  .project-title {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
  }
  .status {
    padding: 3px 7px;
    border: 1px solid var(--color-border);
    font: 10px var(--font-mono);
    text-transform: uppercase;
  }
  .status[data-status="disconnected"] {
    color: var(--color-warning);
  }
  .status[data-status="archived"] {
    color: var(--color-text-muted);
  }
  .path {
    margin: 7px 0;
    overflow-wrap: anywhere;
  }
  .evidence {
    display: flex;
    gap: 14px;
    flex-wrap: wrap;
    font-size: 10px;
  }
  .actions {
    display: flex;
    align-items: center;
    gap: 4px;
    flex-wrap: wrap;
    justify-content: flex-end;
  }
  .issues {
    margin: 12px 0 0;
    padding-left: 18px;
    color: var(--color-danger);
    font-size: 12px;
  }
  .modal-lede {
    margin: 0 0 18px;
  }
  .preview-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 14px;
    margin-bottom: 20px;
  }
  .preview-grid > div {
    display: grid;
    gap: 5px;
    padding-bottom: 10px;
    border-bottom: 1px solid var(--color-border);
  }
  .owned-fields {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 14px;
  }
  .wide {
    grid-column: 1 / -1;
  }
  code {
    overflow-wrap: anywhere;
    color: var(--color-text);
    font: 11px var(--font-mono);
  }
  .field {
    display: grid;
    gap: 7px;
    color: var(--color-text);
    font-size: 12px;
  }
  .field input {
    min-height: 40px;
    padding: 0 10px;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    color: var(--color-text);
    background: var(--color-bg);
    font: inherit;
  }
  .field textarea {
    min-height: 72px;
    padding: 9px 10px;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    color: var(--color-text);
    background: var(--color-bg);
    font: inherit;
    resize: vertical;
  }
  .favorite {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--color-text);
    font-size: 12px;
  }
  .no-results {
    padding: 24px;
    color: var(--color-text-muted);
    font-size: 13px;
  }
  .validation {
    margin-top: 20px;
  }
  .validation > .label {
    display: block;
    margin-bottom: 8px;
  }
  .validation p {
    display: flex;
    gap: 7px;
    margin: 7px 0;
    color: var(--color-info);
    font-size: 12px;
  }
  .validation .warning-line {
    color: var(--color-warning);
  }
  .validation .error-line {
    color: var(--color-danger);
  }
  .severity {
    min-width: 52px;
    font: 10px var(--font-mono);
    text-transform: uppercase;
  }
  .valid-line {
    align-items: center;
  }
  .modal-actions {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
    margin-top: 24px;
  }
  .inspection {
    max-width: 1060px;
    margin: 22px auto 0;
    padding: 24px;
    border: 1px solid var(--color-border);
    background: var(--color-surface);
  }
  .inspection-heading,
  .inventory-heading {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 18px;
  }
  .inspection-heading { padding-bottom: 20px; border-bottom: 1px solid var(--color-border); }
  .inspection-actions { display: flex; align-items: center; gap: 8px; }
  .overview-grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 1px;
    margin: 20px 0 26px;
    border: 1px solid var(--color-border);
    background: var(--color-border);
  }
  .overview-grid > div { display: grid; gap: 7px; min-height: 72px; padding: 13px; background: var(--color-surface); }
  .overview-grid strong { font-size: 13px; line-height: 1.4; }
  .validation-summary { display: grid; gap: 8px; margin: -8px 0 26px; color: var(--color-text-muted); font-size: 12px; }
  .validation-summary > span:not(.label) { display: flex; gap: 6px; align-items: flex-start; }
  .validation-summary .label { margin-bottom: 2px; }
  .valid-text { color: var(--color-success); }
  .item-warning { color: var(--color-warning); }
  .item-error { color: var(--color-danger); }
  .inventory-heading { align-items: center; margin-bottom: 12px; }
  .inventory-heading h3 { margin-top: 4px; }
  .filter-label { display: flex; align-items: center; gap: 8px; color: var(--color-text-muted); font: 11px var(--font-mono); }
  .filter-label select { min-height: 34px; padding: 0 8px; border: 1px solid var(--color-border); color: var(--color-text); background: var(--color-bg); font: inherit; }
  .inventory-table { border: 1px solid var(--color-border); }
  .inventory-row { display: grid; grid-template-columns: minmax(180px, 1.7fr) repeat(3, minmax(80px, .7fr)) 82px; gap: 12px; align-items: center; padding: 12px 13px; border-bottom: 1px solid var(--color-border); color: var(--color-text-muted); font-size: 12px; }
  .inventory-row:last-child { border-bottom: 0; }
  .inventory-row > div { display: grid; gap: 4px; min-width: 0; }
  .inventory-row strong { overflow-wrap: anywhere; color: var(--color-text); font-size: 12px; }
  .inventory-row .path { margin: 0; font-size: 10px; }
  .metadata-path { overflow-wrap: anywhere; color: var(--color-text-subtle); font: 10px var(--font-mono); }
  .inventory-header { color: var(--color-text-muted); background: var(--color-surface-raised); font: 10px var(--font-mono); text-transform: uppercase; }
  .unavailable { color: var(--color-text-subtle); font: 10px var(--font-mono); }
  .inspection-state { display: grid; justify-items: center; gap: 10px; padding: 34px 12px 12px; color: var(--color-text-muted); text-align: center; }
  .inspection-state p { margin: 0; }
  .activity-section { margin-top: 26px; }
  .activity-section > .eyebrow { margin-bottom: 10px; }
  .activity-list { border-top: 1px solid var(--color-border); }
  .activity-item { display: grid; grid-template-columns: 155px 150px 1fr; gap: 12px; align-items: center; padding: 10px 0; border-bottom: 1px solid var(--color-border); color: var(--color-text-muted); font-size: 12px; }
  .activity-item strong { color: var(--color-text); font: 11px var(--font-mono); text-transform: uppercase; }
  @media (max-width: 700px) {
    .projects-shell {
      padding: 0 20px 28px;
    }
    .hero {
      align-items: flex-start;
      flex-direction: column;
      margin-top: 36px;
    }
    .project-row {
      align-items: flex-start;
      flex-direction: column;
    }
    .actions {
      justify-content: flex-start;
    }
    .preview-grid {
      grid-template-columns: 1fr;
    }
    .owned-fields {
      grid-template-columns: 1fr;
    }
    .wide {
      grid-column: auto;
    }
    .list-heading {
      align-items: flex-start;
      gap: 14px;
      flex-direction: column;
    }
    .list-controls {
      align-items: flex-start;
      flex-direction: column;
      gap: 6px;
    }
    .inspection { padding: 18px; }
    .inspection-heading, .inventory-heading { align-items: flex-start; flex-direction: column; }
    .overview-grid { grid-template-columns: 1fr 1fr; }
    .inventory-table { overflow-x: auto; }
    .inventory-row { min-width: 650px; }
    .activity-item { grid-template-columns: 1fr; gap: 4px; }
  }
  @media (prefers-reduced-motion: reduce) {
    .loader {
      animation: none;
    }
  }
</style>
