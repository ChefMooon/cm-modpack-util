<script lang="ts">
  import { onMount } from "svelte";
  import FolderSimpleIcon from "phosphor-svelte/lib/FolderSimpleIcon";
  import PlusIcon from "phosphor-svelte/lib/PlusIcon";
  import ArrowClockwiseIcon from "phosphor-svelte/lib/ArrowClockwiseIcon";
  import LinkBreakIcon from "phosphor-svelte/lib/LinkBreakIcon";
  import LinkIcon from "phosphor-svelte/lib/LinkIcon";
  import ArchiveIcon from "phosphor-svelte/lib/ArchiveIcon";
  import PencilSimpleIcon from "phosphor-svelte/lib/PencilSimpleIcon";
  import ArrowSquareOutIcon from "phosphor-svelte/lib/ArrowSquareOutIcon";
  import XIcon from "phosphor-svelte/lib/XIcon";
  import CheckCircleIcon from "phosphor-svelte/lib/CheckCircleIcon";
  import WarningCircleIcon from "phosphor-svelte/lib/WarningCircleIcon";
  import CaretDoubleLeftIcon from "phosphor-svelte/lib/CaretDoubleLeftIcon";
  import CaretDoubleRightIcon from "phosphor-svelte/lib/CaretDoubleRightIcon";
  import DotsThreeIcon from "phosphor-svelte/lib/DotsThreeIcon";
  import Header from "../components/header/Header.svelte";
  import Button from "../components/ui/Button.svelte";
  import Modal from "../components/ui/Modal.svelte";
  import Tooltip from "../components/ui/Tooltip.svelte";
  import SnapshotReviewModal from "../components/modpacks/SnapshotReviewModal.svelte";
  import SnapshotHistory from "../components/modpacks/SnapshotHistory.svelte";
  import ModpackSettings from "../components/modpacks/ModpackSettings.svelte";
  import ModpackList from "../components/modpacks/ModpackList.svelte";
  import ModpackSummary from "../components/modpacks/ModpackSummary.svelte";
  import { useToast } from "../components/ui/toast/toast.svelte";
  import { commandErrorMessage } from "../lib/errors";
  import type {
    ApplicationModpackMetadata,
    ApplyOperationReport,
    Evidence,
    InventoryEntry,
    ModpackOverview,
    Observation,
    ModpackRecord,
    RegistrationPreview,
    ValidationResult,
    DiscoveryOutcomeKind,
    DiscoveryProgress,
    DiscoveryResult,
    ProcessEvidence,
    SnapshotRecord,
    ChangelogArtifact,
    ChangelogRevision,
    ChangelogExport,
    ChangelogProgress,
  } from "../lib/domain";
  import {
    archiveModpack,
    chooseModpackDirectory,
    disconnectModpack,
    listModpacks,
    getModpackInventory,
    getModpackOverview,
    openTrustedPage,
    previewModpack,
    reconnectModpack,
    refreshModpack,
    registerModpack,
    restoreModpack,
    updateModpackMetadata,
    cancelUpdateCheck,
    listenUpdateCheckProcess,
    listenUpdateCheckProgress,
    checkForUpdates,
    listModpackSnapshots,
    getSnapshot,
    setSnapshotDecision,
    saveModpackSnapshotNote,
    closeSnapshot,
    recheckSnapshot,
    linkSnapshotRetry,
    pinModpackEntry,
    unpinModpackEntry,
    applyModpack,
    cancelOperation,
    generateChangelog,
    chooseChangelogDestination,
    exportChangelog,
    createChangelogRevision,
    cancelChangelogGeneration,
    listenChangelogProgress,
    listChangelogArtifacts,
    listChangelogRevisions,
    listChangelogExports,
  } from "../lib/projects";

  let projects = $state<ModpackRecord[]>([]);
  type DetailTab = "summary" | "versions" | "settings";
  let focusedProject = $state<ModpackRecord | null>(null);
  let activeTab = $state<DetailTab>("summary");
  let listCollapsed = $state(false);
  let paneWidth = $state(300);
  let resizingPane = $state(false);
  let resizeStartX = 0;
  let resizeStartWidth = 300;
  let loading = $state(true);
  let busy = $state(false);
  let error = $state("");
  let selected = $state<RegistrationPreview | null>(null);
  let previewPath = $state("");
  let reconnecting = $state<ModpackRecord | null>(null);
  let editing = $state<ModpackRecord | null>(null);
  let metadataDraft = $state<ApplicationModpackMetadata | null>(null);
  let tagsText = $state("");
  let showPreview = $state(false);
  let showReconnect = $state(false);
  let showArchived = $state(false);
  let inspected = $state<ModpackRecord | null>(null);
  let inventory = $state<InventoryEntry[]>([]);
  let overview = $state<ModpackOverview | null>(null);
  let inventoryFilter = $state("all");
  let inspecting = $state(false);
  let overviewLoading = $state(false);
  let inventoryLoading = $state(false);
  let overviewError = $state("");
  let inventoryError = $state("");
  let summaryRequest = 0;
  let pinningEntry = $state<string | null>(null);
  let pinConfirmation = $state<{ entry: InventoryEntry; pin: boolean } | null>(null);
  let showPinConfirmation = $state(false);
  let discoveryProject = $state<ModpackRecord | null>(null);
  let showReviewModal = $state(false);
  let discovery = $state<DiscoveryResult | null>(null);
  let discoveryProgress = $state<DiscoveryProgress | null>(null);
  let discoveryProcess = $state<ProcessEvidence | null>(null);
  let discoveryBusy = $state(false);
  let discoverySnapshotId = $state<string | null>(null);
  let discoverySnapshot = $state<SnapshotRecord | null>(null);
  let snapshots = $state<SnapshotRecord[]>([]);
  let snapshotsLoading = $state(false);
  let snapshotsError = $state("");
  let versionFilter = $state<"all" | "releases" | "snapshots">("all");
  let showSnapshotNote = $state(false);
  let showApplyConfirmation = $state(false);
  let applyBusy = $state(false);
  let applyOperationId = $state<string | null>(null);
  let applyReport = $state<ApplyOperationReport | null>(null);
  let changelog = $state<ChangelogArtifact | null>(null);
  let changelogRevision = $state<ChangelogRevision | null>(null);
  let changelogRevisions = $state<ChangelogRevision[]>([]);
  let changelogExports = $state<ChangelogExport[]>([]);
  let changelogIntroduction = $state("");
  let changelogOffline = $state(false);
  let changelogBusy = $state(false);
  let changelogProgress = $state<ChangelogProgress | null>(null);
  let changelogPartialChoice = $state(false);
  let changelogDraft = $state("");
  let snapshotNoteDraft = $state("");
  let discoveryUnlisten: (() => void)[] = [];
  const { toast } = useToast();

  onMount(() => {
    void loadProjects();
    void hydrateDiscoveryEvents();
    const closeProjectMenus = (event: PointerEvent) => {
      if (event.target instanceof Element && event.target.closest(".project-menu")) return;
      document.querySelectorAll<HTMLDetailsElement>(".project-menu[open]").forEach((menu) => { menu.open = false; });
    };
    const handleProjectMenuKeydown = (event: KeyboardEvent) => {
      if (event.key !== "Escape") return;
      document.querySelectorAll<HTMLDetailsElement>(".project-menu[open]").forEach((menu) => { menu.open = false; });
    };
    const handleBeforeUnload = (event: BeforeUnloadEvent) => {
      if (!settingsDirty()) return;
      event.preventDefault();
      event.returnValue = "";
    };
    document.addEventListener("pointerdown", closeProjectMenus);
    document.addEventListener("keydown", handleProjectMenuKeydown);
    window.addEventListener("beforeunload", handleBeforeUnload);
    return () => {
      document.removeEventListener("pointerdown", closeProjectMenus);
      document.removeEventListener("keydown", handleProjectMenuKeydown);
      window.removeEventListener("beforeunload", handleBeforeUnload);
      discoveryUnlisten.forEach((unlisten) => unlisten());
    };
  });

  async function hydrateDiscoveryEvents() {
    discoveryUnlisten = await Promise.all([
      listenUpdateCheckProgress((progress) => {
        if (!discoveryProject || progress.modpack_id === discoveryProject.id) discoveryProgress = progress;
      }),
      listenUpdateCheckProcess((process) => {
        discoveryProcess = process;
      }),
      listenChangelogProgress((progress) => {
        if (!changelogBusy || progress.attempt_id === changelog?.attempt_id) changelogProgress = progress;
      }),
    ]);
  }

  async function loadProjects() {
    loading = true;
    error = "";
    try {
      projects = await listModpacks();
      const snapshotId = new URLSearchParams(window.location.search).get("snapshot");
      if (snapshotId) await openSnapshot(snapshotId);
    } catch (cause) {
      error = commandErrorMessage(cause);
    } finally {
      loading = false;
    }
  }

  async function openSnapshot(snapshotId: string) {
    const snapshot = await getSnapshot(snapshotId);
    const project = projects.find((candidate) => candidate.id === snapshot.modpack_id);
    if (!project) return;
    focusedProject = project;
    activeTab = "versions";
    void loadSnapshots(project);
    discoveryProject = project;
    discoverySnapshot = snapshot;
    discoverySnapshotId = snapshot.id;
    discovery = snapshot.result;
    showReviewModal = true;
    const artifacts = await listChangelogArtifacts(project.id);
    changelog = artifacts.find((artifact) => artifact.snapshot_id === snapshot.id) ?? null;
    changelogRevisions = changelog ? await listChangelogRevisions(changelog.id) : [];
    changelogExports = changelog ? await listChangelogExports(changelog.id) : [];
    changelogRevision = changelogRevisions.find((revision) => revision.is_current) ?? null;
    changelogDraft = changelog?.content ?? "";
    if (changelogRevision) changelogDraft = changelogRevision.content;
  }

  async function selectProject() {
    error = "";
    const path = await chooseModpackDirectory();
    if (!path) return;
    busy = true;
    try {
      selected = await previewModpack(path);
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
      await registerModpack(previewPath, selected.application_defaults);
      showPreview = false;
      selected = null;
      await loadProjects();
      toast({
        title: "Modpack registered",
        description: "The external Packwiz directory was not modified.",
        severity: "success",
      });
    } catch (cause) {
      error = commandErrorMessage(cause);
    } finally {
      busy = false;
    }
  }

  async function refresh(project: ModpackRecord) {
    busy = true;
    try {
      replace(await refreshModpack(project.id));
      if (inspected?.id === project.id) await inspectProject(project);
      toast({ title: "Modpack refreshed", severity: "success" });
    } catch (cause) {
      error = commandErrorMessage(cause);
    } finally {
      busy = false;
    }
  }

  async function inspectProject(project: ModpackRecord) {
    const requestId = ++summaryRequest;
    focusedProject = project;
    inspected = project;
    inspecting = true;
    error = "";
    overviewLoading = true;
    inventoryLoading = true;
    overviewError = "";
    inventoryError = "";
    void getModpackOverview(project.id).then((result) => {
      if (requestId !== summaryRequest) return;
      overview = result;
    }).catch((cause) => {
      if (requestId !== summaryRequest) return;
      overview = null;
      overviewError = commandErrorMessage(cause);
    }).finally(() => {
      if (requestId === summaryRequest) overviewLoading = false;
    });
    void getModpackInventory(project.id).then((result) => {
      if (requestId !== summaryRequest) return;
      inventory = result;
    }).catch((cause) => {
      if (requestId !== summaryRequest) return;
      inventory = [];
      inventoryError = commandErrorMessage(cause);
    }).finally(() => {
      if (requestId === summaryRequest) {
        inventoryLoading = false;
        inspecting = false;
      }
    });
  }

  function focusProject(project: ModpackRecord) {
    if (focusedProject?.id !== project.id && !canLeaveSettings()) return;
    focusedProject = project;
    activeTab = "summary";
    void inspectProject(project);
    void loadSnapshots(project);
  }

  async function loadSnapshots(project: ModpackRecord) {
    snapshotsLoading = true;
    snapshotsError = "";
    try {
      snapshots = await listModpackSnapshots(project.id);
    } catch (cause) {
      snapshots = [];
      snapshotsError = commandErrorMessage(cause);
    } finally {
      snapshotsLoading = false;
    }
  }

  function settingsDirty() {
    if (!editing || !metadataDraft) return false;
    return JSON.stringify({ ...metadataDraft, tags: tagsText.split(",").map((tag) => tag.trim()).filter(Boolean) }) !== JSON.stringify(editing.application);
  }

  function canLeaveSettings() {
    if (busy && settingsDirty()) return false;
    if (!settingsDirty()) return true;
    const initiatingControl = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    const discard = window.confirm("Discard unsaved modpack settings?");
    if (discard) {
      metadataDraft = null;
      editing = null;
      return true;
    }
    queueMicrotask(() => initiatingControl?.focus());
    return false;
  }


  async function checkProjectForUpdates(project: ModpackRecord) {
    if (discoveryBusy) return;
    discoveryProject = project;
    showReviewModal = true;
    discovery = null;
    discoverySnapshot = null;
    discoveryProcess = null;
    discoveryProgress = null;
    discoveryBusy = true;
    error = "";
    try {
      discovery = await checkForUpdates(project.id);
      const snapshots = await listModpackSnapshots(project.id);
      discoverySnapshotId = snapshots[0]?.id ?? null;
      discoverySnapshot = discoverySnapshotId ? await getSnapshot(discoverySnapshotId) : null;
    } catch (cause) {
      discovery = null;
      error = commandErrorMessage(cause);
    } finally {
      discoveryBusy = false;
    }
  }

  async function cancelUpdateCheckForProject() {
    try {
      await cancelUpdateCheck();
    } catch (cause) {
      error = commandErrorMessage(cause);
    }
  }

  async function decideCandidate(candidateId: string, decision: "selected" | "skipped" | "blocked" | "deferred") {
    if (!discoverySnapshotId) return;
    try {
      await setSnapshotDecision(discoverySnapshotId, candidateId, decision);
      discoverySnapshot = await getSnapshot(discoverySnapshotId);
      toast({ title: "Review decision saved", severity: "success" });
    } catch (cause) {
      error = commandErrorMessage(cause);
    }
  }

  async function finishSnapshot(cancelled: boolean) {
    if (!discoverySnapshotId) return;
    try {
      discoverySnapshot = await closeSnapshot(discoverySnapshotId, cancelled);
      discovery = discoverySnapshot.result;
      const project = discoveryProject;
      if (project) await loadSnapshots(project);
      discoveryProcess = null;
      discoveryProgress = null;
      toast({ title: cancelled ? "Review cancelled" : "Review closed", severity: "success" });
    } catch (cause) {
      error = commandErrorMessage(cause);
    }
  }

  async function saveReviewNote() {
    if (!discoveryProject || !discoverySnapshotId || !snapshotNoteDraft.trim()) return;
    try {
      await saveModpackSnapshotNote(discoveryProject.id, "snapshot", snapshotNoteDraft, discoverySnapshotId);
      discoverySnapshot = await getSnapshot(discoverySnapshotId);
      snapshotNoteDraft = "";
      showSnapshotNote = false;
      toast({ title: "Review note saved", severity: "success" });
    } catch (cause) {
      error = commandErrorMessage(cause);
    }
  }

  async function recheckReview() {
    if (!discoverySnapshotId) return;
    try {
      discoverySnapshot = await recheckSnapshot(discoverySnapshotId);
      discovery = discoverySnapshot.result;
      toast({ title: discoverySnapshot.lifecycle === "stale" ? "Review marked stale" : "Review is current", severity: discoverySnapshot.lifecycle === "stale" ? "warning" : "success" });
    } catch (cause) {
      error = commandErrorMessage(cause);
    }
  }

  async function retryReview() {
    if (!discoveryProject || !discoverySnapshotId || discoveryBusy) return;
    const predecessorId = discoverySnapshotId;
    await checkProjectForUpdates(discoveryProject);
    if (!discoverySnapshotId || discoverySnapshotId === predecessorId) return;
    try {
      discoverySnapshot = await linkSnapshotRetry(predecessorId, discoverySnapshotId);
      discovery = discoverySnapshot.result;
      toast({ title: "Linked retry created", severity: "success" });
    } catch (cause) {
      error = commandErrorMessage(cause);
    }
  }

  async function generateSnapshotChangelog() {
    if (!discoveryProject || !discoverySnapshot || !["reviewable", "closed"].includes(discoverySnapshot.lifecycle) || changelogBusy) return;
    changelogBusy = true;
    changelogProgress = { attempt_id: "", completed: 0, total: discoverySnapshot.candidates.length, message: "Preparing changelog lookups", cancellable: false };
    error = "";
    try {
      changelog = await generateChangelog({
        modpack_id: discoveryProject.id,
        snapshot_id: discoverySnapshot.id,
        introduction: changelogIntroduction.trim() || null,
        offline: changelogOffline,
        source: discoverySnapshot.lifecycle === "closed" ? "applied_operation" : "discovery_candidates",
        request_fingerprint: crypto.randomUUID(),
      });
      changelogProgress = null;
      changelogRevision = null;
      changelogDraft = changelog.content;
      changelogPartialChoice = changelog.status === "partial" || changelog.status === "cancelled";
      toast({ title: "Changelog generated", description: "Review provider evidence before exporting.", severity: changelog.status === "complete" ? "success" : "warning" });
    } catch (cause) {
      error = commandErrorMessage(cause);
    } finally {
      changelogBusy = false;
      if (!changelog) changelogProgress = null;
    }
  }

  async function cancelSnapshotChangelog() {
    if (!changelogBusy) return;
    try {
      await cancelChangelogGeneration();
      toast({ title: "Stopping changelog generation", description: "The completed lookups will be preserved as a partial result.", severity: "warning" });
    } catch (cause) {
      error = commandErrorMessage(cause);
    }
  }

  function discardPartialChangelog() {
    changelog = null;
    changelogRevision = null;
    changelogDraft = "";
    changelogPartialChoice = false;
  }

  function keepPartialChangelog() {
    changelogPartialChoice = false;
  }

  async function saveChangelogRevision() {
    if (!changelog || changelogDraft === changelog.content) return;
    try {
      changelogRevision = await createChangelogRevision({ artifact_id: changelog.id, prior_revision_id: changelogRevision?.id ?? null, content: changelogDraft, introduction: changelog.introduction });
      changelogRevisions = [changelogRevision, ...changelogRevisions];
      toast({ title: "Revision saved", description: "The generated artifact remains unchanged.", severity: "success" });
    } catch (cause) {
      error = commandErrorMessage(cause);
    }
  }

  async function exportSnapshotChangelog() {
    if (!changelog) return;
    const destination = await chooseChangelogDestination();
    if (!destination) return;
    try {
      const exported = await exportChangelog({ artifact_id: changelog.id, revision_id: changelogRevision?.id ?? null, destination, content: changelogDraft });
      changelogExports = [exported, ...changelogExports];
      toast({ title: exported.status === "exported" ? "Changelog exported" : "Export unavailable", description: exported.diagnostic?.message, severity: exported.status === "exported" ? "success" : "warning" });
    } catch (cause) {
      error = commandErrorMessage(cause);
    }
  }

  function selectedCandidateIds(): string[] {
    return (discoverySnapshot?.candidates ?? [])
      .filter((candidate) => latestDecision(candidate.id) === "selected")
      .map((candidate) => candidate.id);
  }

  async function applySelectedUpdates() {
    if (!discoverySnapshot || discoverySnapshot.lifecycle !== "reviewable") return;
    const candidateIds = selectedCandidateIds();
    if (!candidateIds.length) return;
    showApplyConfirmation = false;
    applyBusy = true;
    applyOperationId = crypto.randomUUID();
    applyReport = null;
    error = "";
    try {
      applyReport = await applyModpack({
        operation_id: applyOperationId,
        snapshot_id: discoverySnapshot.id,
        candidate_ids: candidateIds,
      });
      const severity = applyReport.outcome === "complete" ? "success" : applyReport.outcome === "partial" ? "warning" : "error";
      toast({ title: `Update apply ${applyReport.outcome}`, description: "Each selected mod was re-read after its Packwiz command.", severity });
      if (inspected) inventory = await getModpackInventory(inspected.id);
    } catch (cause) {
      error = commandErrorMessage(cause);
    } finally {
      applyBusy = false;
      applyOperationId = null;
    }
  }

  async function cancelApply() {
    if (!applyOperationId) return;
    try {
      await cancelOperation(applyOperationId);
    } catch (cause) {
      error = commandErrorMessage(cause);
    }
  }

  function latestDecision(candidateId: string) {
    return discoverySnapshot?.decisions.filter((decision) => decision.candidate_id === candidateId).at(-1)?.decision;
  }

  async function openPage(entry: InventoryEntry) {
    if (!entry.page_link) return;
    try {
      await openTrustedPage(entry.page_link.url);
    } catch (cause) {
      error = commandErrorMessage(cause);
    }
  }

  async function changePin(entry: InventoryEntry, pin: boolean) {
    if (!inspected || pinningEntry) return;
    pinningEntry = entry.local_id;
    error = "";
    try {
      const request = { modpack_id: inspected.id, entry_id: entry.local_id };
      const attempt = pin ? await pinModpackEntry(request) : await unpinModpackEntry(request);
      if (attempt.verification?.verified) {
        inventory = await getModpackInventory(inspected.id);
        toast({ title: pin ? "Mod pinned" : "Mod unpinned", description: "Packwiz metadata was verified after the operation.", severity: "success" });
      } else {
        error = attempt.error?.message ?? "Packwiz pin state could not be verified.";
      }
    } catch (cause) {
      error = commandErrorMessage(cause);
    } finally {
      pinningEntry = null;
    }
  }

  async function confirmPinChange() {
    if (!pinConfirmation) return;
    const change = pinConfirmation;
    pinConfirmation = null;
    showPinConfirmation = false;
    await changePin(change.entry, change.pin);
  }

  async function openCandidatePage(url: string) {
    try {
      await openTrustedPage(url);
    } catch (cause) {
      error = commandErrorMessage(cause);
    }
  }

  function beginEdit(project: ModpackRecord) {
    editing = project;
    metadataDraft = {
      ...project.application,
      tags: [...project.application.tags],
    };
    tagsText = project.application.tags.join(", ");
  }

  function openSettings(project: ModpackRecord) {
    if (!canLeaveSettings()) return;
    focusProject(project);
    beginEdit(project);
    activeTab = "settings";
  }

  async function saveMetadata() {
    if (!editing || !metadataDraft) return;
    busy = true;
    try {
      const updated = await updateModpackMetadata(editing.id, {
        ...metadataDraft,
        tags: tagsText.split(",").map((tag) => tag.trim()).filter(Boolean),
      });
      replace(updated);
      focusedProject = updated;
      inspected = updated;
      editing = updated;
      metadataDraft = { ...updated.application, tags: [...updated.application.tags] };
      tagsText = updated.application.tags.join(", ");
      toast({ title: "Modpack details updated", severity: "success" });
    } catch (cause) {
      error = commandErrorMessage(cause);
    } finally {
      busy = false;
    }
  }

  async function changeLifecycle(
    project: ModpackRecord,
    action: "archive" | "restore" | "disconnect",
  ) {
    busy = true;
    try {
      const result =
        action === "archive"
          ? await archiveModpack(project.id)
          : action === "restore"
            ? await restoreModpack(project.id)
            : await disconnectModpack(project.id);
      replace(result);
    } catch (cause) {
      error = commandErrorMessage(cause);
    } finally {
      busy = false;
    }
  }

  async function beginReconnect(project: ModpackRecord) {
    reconnecting = project;
    showReconnect = true;
  }

  async function confirmReconnect() {
    if (!reconnecting) return;
    const path = await chooseModpackDirectory();
    if (!path) return;
    busy = true;
    try {
      replace(await reconnectModpack(reconnecting.id, path));
      showReconnect = false;
      reconnecting = null;
      toast({ title: "Modpack reconnected", severity: "success" });
    } catch (cause) {
      error = commandErrorMessage(cause);
    } finally {
      busy = false;
    }
  }

  function replace(project: ModpackRecord) {
    projects = projects.map((candidate) =>
      candidate.id === project.id ? project : candidate,
    );
  }

  function startPaneResize(event: PointerEvent) {
    if (event.button !== 0) return;
    resizingPane = true;
    resizeStartX = event.clientX;
    resizeStartWidth = paneWidth;
    (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
    event.preventDefault();
  }

  function movePaneResize(event: PointerEvent) {
    if (!resizingPane) return;
    paneWidth = Math.min(600, Math.max(240, resizeStartWidth + event.clientX - resizeStartX));
  }

  function endPaneResize(event: PointerEvent) {
    if (!resizingPane) return;
    resizingPane = false;
    const target = event.currentTarget as HTMLElement;
    if (target.hasPointerCapture(event.pointerId)) target.releasePointerCapture(event.pointerId);
  }

  function handlePaneResizeKey(event: KeyboardEvent) {
    const step = event.shiftKey ? 32 : 8;
    if (event.key === "ArrowLeft") paneWidth = Math.max(240, paneWidth - step);
    else if (event.key === "ArrowRight") paneWidth = Math.min(600, paneWidth + step);
    else return;
    event.preventDefault();
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

  function freshnessLabel(timestamp: string | null) {
    if (!timestamp) return "Not read yet";
    const seconds = Number(timestamp);
    if (!Number.isFinite(seconds) || seconds <= 0) return "Read time unavailable";
    const ageMinutes = Math.max(0, Math.floor((Date.now() - seconds * 1000) / 60000));
    if (ageMinutes < 1) return "Read just now";
    if (ageMinutes < 60) return `Read ${ageMinutes}m ago`;
    const ageHours = Math.floor(ageMinutes / 60);
    if (ageHours < 24) return `Read ${ageHours}h ago`;
    const ageDays = Math.floor(ageHours / 24);
    if (ageDays < 7) return `Read ${ageDays}d ago`;
    return `Read ${new Date(seconds * 1000).toLocaleDateString(undefined, { month: "short", day: "numeric" })}`;
  }

  function freshnessTitle(timestamp: string | null) {
    if (!timestamp) return "Evidence has not been read yet";
    const seconds = Number(timestamp);
    if (!Number.isFinite(seconds) || seconds <= 0) return "The evidence read time is unavailable";
    return `Evidence read ${new Date(seconds * 1000).toLocaleString()}`;
  }

  function outcomeMessage(outcome: DiscoveryOutcomeKind) {
    return {
      unsupported: "The installed Packwiz executable is outside the tested compatibility profile.",
      unsafe: "The project changed or cancellation safety could not be proven; candidates are withheld.",
      indeterminate: "The evidence was incomplete or ambiguous; candidates are withheld.",
      cancelled: "The update check was cancelled before a safe result was available.",
      failed: "The update check failed before a safe result was available.",
      normal: "The project was unchanged after the cancellation probe.",
    }[outcome];
  }

  function snapshotStatus(snapshot: SnapshotRecord) {
    return snapshot.lifecycle === "stale" ? "Stale" : snapshot.lifecycle === "cancelled" ? "Cancelled" : snapshot.outcome.replaceAll("_", " ");
  }

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
      ><PlusIcon size={17} weight="bold" /> Register modpack</Button
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
      <p>Loading registered modpacks...</p>
    </section>
  {:else if projects.length === 0}<section
      class="state"
      aria-labelledby="empty-title"
    >
      <div class="empty-icon" aria-hidden="true">
        <FolderSimpleIcon size={34} weight="duotone" />
      </div>
      <p class="eyebrow">No registered modpacks</p>
      <h2 id="empty-title">Start with a local Packwiz directory</h2>
      <p>
        Choose a project folder to preview its Packwiz evidence before anything
        is registered.
      </p>
      <Button variant="secondary" type="button" onclick={selectProject}
        ><PlusIcon size={16} /> Choose directory</Button
      >
    </section>
  {:else}<section class="workspace" class:collapsed={listCollapsed} style={`--list-width: ${paneWidth}px`}>
      <aside class="list-pane" class:collapsed={listCollapsed} aria-label="Project list">
        <div class="workspace-tools">
          <Tooltip text={listCollapsed ? "Expand projects" : "Collapse projects"} position="bottom">
            <Button
              variant="ghost"
              size="icon"
              type="button"
              aria-label={listCollapsed ? "Expand projects" : "Collapse projects"}
              aria-expanded={!listCollapsed}
              aria-controls="project-list"
              onpointerdown={(event) => event.preventDefault()}
              onclick={() => { if (canLeaveSettings()) listCollapsed = !listCollapsed; }}
            >{#if listCollapsed}<CaretDoubleRightIcon size={17} weight="bold" />{:else}<CaretDoubleLeftIcon size={17} weight="bold" />{/if}</Button>
          </Tooltip>
        </div>
        <ModpackList
          projects={projects}
          bind:showArchived
          collapsed={listCollapsed}
          focusedModpackId={focusedProject?.id ?? null}
          busy={busy}
          inspecting={inspecting}
          discoveryBusy={discoveryBusy}
          discoveryModpackId={discoveryProject?.id ?? null}
          onfocus={focusProject}
          oncheck={checkProjectForUpdates}
          onreconnect={beginReconnect}
          onrefresh={refresh}
          onedit={openSettings}
          onlifecycle={changeLifecycle}
          observed={observed}
          freshnessLabel={freshnessLabel}
          freshnessTitle={freshnessTitle}
          hasErrors={hasErrors}
        />
      </aside>
      <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
      <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
      <div
        class="workspace-divider"
        role="separator"
        tabindex="0"
              aria-label="Resize modpack list"
        aria-valuemin="240"
        aria-valuemax="600"
        aria-valuenow={paneWidth}
        onpointerdown={startPaneResize}
        onpointermove={movePaneResize}
        onpointerup={endPaneResize}
        onpointercancel={endPaneResize}
        onkeydown={handlePaneResizeKey}
      ></div>
      <div class="detail-pane">
        <div class="detail-tabs" role="tablist" aria-label="Modpack details">
          <button class:active={activeTab === "summary"} id="summary-tab" role="tab" aria-selected={activeTab === "summary"} aria-controls="summary-panel" tabindex={activeTab === "summary" ? 0 : -1} onclick={() => { if (activeTab !== "summary" && canLeaveSettings()) activeTab = "summary"; }}>Summary</button>
          <button class:active={activeTab === "versions"} id="versions-tab" role="tab" aria-selected={activeTab === "versions"} aria-controls="versions-panel" tabindex={activeTab === "versions" ? 0 : -1} onclick={() => { if (activeTab !== "versions" && canLeaveSettings()) activeTab = "versions"; }}>Versions</button>
          <button class:active={activeTab === "settings"} id="settings-tab" role="tab" aria-selected={activeTab === "settings"} aria-controls="settings-panel" tabindex={activeTab === "settings" ? 0 : -1} onclick={() => { if (activeTab !== "settings" && canLeaveSettings()) { if (focusedProject && (!metadataDraft || editing?.id !== focusedProject.id)) beginEdit(focusedProject); activeTab = "settings"; } }}>Settings</button>
        </div>
        {#if !focusedProject}
          <section class="detail-empty" role="status"><p class="eyebrow">No focused modpack</p><h2>Select a modpack to begin</h2><p>Choose a modpack card to load its local evidence and modpack tools.</p></section>
        {:else if activeTab === "summary"}
          <ModpackSummary
            inspected={inspected}
            overview={overview}
            inventory={inventory}
            bind:inventoryFilter
            overviewLoading={overviewLoading}
            inventoryLoading={inventoryLoading}
            overviewError={overviewError}
            inventoryError={inventoryError}
            inspecting={inspecting}
            pinningEntry={pinningEntry}
            evidenceLabel={evidenceLabel}
            freshnessLabel={freshnessLabel}
            freshnessTitle={freshnessTitle}
            onclose={() => { inspected = null; focusedProject = null; }}
            onreread={inspectProject}
            onopenPage={openPage}
            onpin={(entry, pin) => { pinConfirmation = { entry, pin }; showPinConfirmation = true; }}
            onretry={inspectProject}
          />
        {:else if activeTab === "versions"}
          <SnapshotHistory
            bind:filter={versionFilter}
            snapshots={snapshots}
            loading={snapshotsLoading}
            error={snapshotsError}
            onretry={() => { if (focusedProject) void loadSnapshots(focusedProject); }}
            onopen={(snapshotId) => { if (canLeaveSettings()) { activeTab = "versions"; void openSnapshot(snapshotId); } }}
            snapshotStatus={snapshotStatus}
          />
        {:else}
          <ModpackSettings
            metadataDraft={metadataDraft}
            bind:tagsText
            busy={busy}
            onsave={saveMetadata}
          />
        {/if}
      </div>
    </section>
  {/if}
</main>

<SnapshotReviewModal
  bind:open={showReviewModal}
  project={discoveryProject}
  discovery={discovery}
  discoveryProgress={discoveryProgress}
  discoveryProcess={discoveryProcess}
  snapshot={discoverySnapshot}
  snapshotId={discoverySnapshotId}
  discoveryBusy={discoveryBusy}
  applyBusy={applyBusy}
  applyReport={applyReport}
  changelog={changelog}
  changelogRevision={changelogRevision}
  changelogRevisions={changelogRevisions}
  changelogExports={changelogExports}
  bind:changelogIntroduction
  bind:changelogOffline
  changelogBusy={changelogBusy}
  changelogProgress={changelogProgress}
  changelogPartialChoice={changelogPartialChoice}
  bind:changelogDraft
  bind:noteDraft={snapshotNoteDraft}
  selectedCandidateIds={selectedCandidateIds}
  latestDecision={latestDecision}
  evidenceLabel={evidenceLabel}
  oncancelDiscovery={cancelUpdateCheckForProject}
  oncloseReview={finishSnapshot}
  ondecideCandidate={decideCandidate}
  onsaveNote={saveReviewNote}
  onrecheck={recheckReview}
  onretry={retryReview}
  onapply={() => (showApplyConfirmation = false, applySelectedUpdates())}
  oncancelApply={cancelApply}
  onopenPage={openCandidatePage}
  ongenerator={generateSnapshotChangelog}
  oncancelChangelog={cancelSnapshotChangelog}
  onkeepPartial={keepPartialChangelog}
  ondiscardPartial={discardPartialChangelog}
  onsaveRevision={saveChangelogRevision}
  onexport={exportSnapshotChangelog}
/>

<Modal bind:open={showSnapshotNote} title="Add review note" onclose={() => (showSnapshotNote = false)}>
  <label class="field"><span>Note</span><textarea bind:value={snapshotNoteDraft} rows="5" maxlength="4000" placeholder="Explain this review or its outcome"></textarea></label>
  <div class="modal-actions"><Button variant="quiet" type="button" onclick={() => (showSnapshotNote = false)}>Cancel</Button><Button variant="primary" type="button" onclick={saveReviewNote}>Save note</Button></div>
</Modal>

<Modal bind:open={showApplyConfirmation} title="Apply selected updates" onclose={() => (showApplyConfirmation = false)}>
  <p class="modal-lede">Packwiz will update {selectedCandidateIds().length} selected mod{selectedCandidateIds().length === 1 ? "" : "s"} one at a time. The application will continue after an individual failure and show a partial report when needed.</p>
  <div class="modal-actions"><Button variant="secondary" type="button" onclick={() => (showApplyConfirmation = false)}>Cancel</Button><Button variant="primary" type="button" onclick={applySelectedUpdates}>Apply updates</Button></div>
</Modal>

<Modal bind:open={showPinConfirmation} title={pinConfirmation?.pin ? "Pin Packwiz entry" : "Unpin Packwiz entry"} onclose={() => { pinConfirmation = null; showPinConfirmation = false; }}>
  {#if pinConfirmation}
    <p class="modal-lede">This runs the native Packwiz command for the exact current inventory entry. The metadata is re-read before success is shown.</p>
    <div class="preview-grid"><div><span class="label">Entry</span><strong>{evidenceLabel(pinConfirmation.entry.name)}</strong></div><div><span class="label">Metadata</span><code>{pinConfirmation.entry.metadata_path}</code></div></div>
    <div class="modal-actions"><Button variant="secondary" type="button" onclick={() => { pinConfirmation = null; showPinConfirmation = false; }}>Cancel</Button><Button variant={pinConfirmation.pin ? "primary" : "danger"} type="button" loading={pinningEntry === pinConfirmation.entry.local_id} onclick={confirmPinChange}>{pinConfirmation.pin ? "Pin entry" : "Unpin entry"}</Button></div>
  {/if}
</Modal>

<Modal
  bind:open={showPreview}
  title="Review modpack registration"
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
  bind:open={showReconnect}
  title="Reconnect modpack"
  onclose={() => (showReconnect = false)}
  ><p>
    Select the modpack directory again. CM Modpack Util will validate it before
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
  .workspace {
    display: grid;
    grid-template-columns: minmax(240px, var(--list-width)) 28px minmax(0, 1fr);
    grid-template-rows: minmax(0, 1fr);
    width: min(1180px, 100%);
    height: min(720px, calc(100vh - 190px));
    min-height: 480px;
    margin: 0 auto;
    border: 1px solid var(--color-border);
    background: var(--color-surface);
    transition: grid-template-columns 0.24s ease;
  }
  .workspace.collapsed {
    grid-template-columns: 56px 28px minmax(0, 1fr);
  }
  .list-pane,
  .detail-pane {
    height: 100%;
    min-width: 0;
    min-height: 0;
    overflow-x: hidden;
    overflow-y: auto;
  }
  .list-pane {
    position: relative;
    z-index: 4;
    container-type: inline-size;
  }
  .list-pane.collapsed {
    overflow: visible;
  }
  .list-pane.collapsed .workspace-tools {
    justify-items: center;
    padding: 0;
  }
  .workspace-tools {
    display: grid;
    gap: 0;
    position: relative;
    z-index: 5;
    height: 58px;
    min-height: 58px;
    padding: 0;
    border-bottom: 1px solid var(--color-border);
    background: var(--color-surface-raised);
  }
  .workspace-tools > :global(.tooltip) {
    display: flex;
    width: 100%;
    height: 100%;
  }
  .workspace-tools :global(.tooltip-content) {
    z-index: 10;
  }
  .workspace-tools :global(.button) {
    width: 100%;
    height: 58px;
    justify-content: flex-start;
    padding: 0 12px;
  }
  .list-pane.collapsed .workspace-tools :global(.button) {
    justify-content: center;
    padding: 0;
  }
  .workspace-divider {
    position: relative;
    z-index: 2;
    width: 28px;
    height: 100%;
    padding: 0;
    border: 0;
    cursor: col-resize;
    background: transparent;
    touch-action: none;
  }
  .workspace-divider::before {
    display: block;
    width: 8px;
    height: 100%;
    margin: 0 auto;
    background: var(--color-border);
    content: "";
  }
  .workspace-divider:hover::before,
  .workspace-divider:focus-visible::before,
  .workspace-divider:active::before {
    background: var(--color-accent);
  }
  .workspace-divider:focus-visible {
    box-shadow: var(--focus-ring);
    outline: none;
  }
  .detail-pane {
    background: var(--color-bg);
  }
  .detail-tabs {
    position: sticky;
    top: 0;
    z-index: 1;
    display: flex;
    gap: 4px;
    padding: 12px 18px 0;
    border-bottom: 1px solid var(--color-border);
    background: var(--color-bg);
  }
  .detail-tabs button {
    min-height: 40px;
    padding: 0 14px;
    border: 0;
    border-bottom: 3px solid transparent;
    color: var(--color-text-muted);
    background: transparent;
    font: 700 12px var(--font-mono);
    cursor: pointer;
  }
  .detail-tabs button.active {
    border-bottom-color: var(--color-accent);
    color: var(--color-text);
  }
  .detail-tabs button:focus-visible {
    outline: none;
    box-shadow: var(--focus-ring);
  }
  .detail-empty {
    display: grid;
    align-content: center;
    min-height: 360px;
    padding: 32px;
  }
  .detail-empty p:not(.eyebrow) {
    max-width: 440px;
    color: var(--color-text-muted);
    line-height: 1.6;
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
  h2 {
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
  .state {
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
  .label {
    color: var(--color-text-muted);
    font: 11px var(--font-mono);
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
  @media (max-width: 700px) {
    .projects-shell {
      padding: 0 20px 28px;
    }
    .hero {
      align-items: flex-start;
      flex-direction: column;
      margin-top: 36px;
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
  }
  @media (max-width: 820px) {
    .workspace {
      display: block;
      height: auto;
      min-height: 0;
    }
    .list-pane,
    .detail-pane {
      max-height: none;
    }
    .list-pane.collapsed {
      display: none;
    }
    .workspace-divider {
      display: none;
    }
    .detail-tabs {
      position: static;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .workspace,
    .loader {
      animation: none;
      transition: none;
    }
  }
</style>
