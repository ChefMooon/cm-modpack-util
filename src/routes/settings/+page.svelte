<script lang="ts">
  import Modal from "../../components/ui/Modal.svelte";
  import ThemeSelect from "../../components/settings/ThemeSelect.svelte";
  import { settingDefinitions, type Theme } from "../../components/settings/types/settings";
  import { onMount } from "svelte";
  import AppShell from "../../components/layout/AppShell.svelte";
  import MagnifyingGlassIcon from "phosphor-svelte/lib/MagnifyingGlassIcon";
  import XIcon from "phosphor-svelte/lib/XIcon";
  import { enable as enableAutostart, disable as disableAutostart, isEnabled as isAutostartEnabled } from "@tauri-apps/plugin-autostart";
  import { restoreStateCurrent, StateFlags } from "@tauri-apps/plugin-window-state";
  import { useToast } from "../../components/ui/toast/toast.svelte";
  import Button from "../../components/ui/Button.svelte";
  import Tooltip from "../../components/ui/Tooltip.svelte";
  import { loadSettings, resetStoredSettings, saveSetting } from "../../lib/settings";
  import { commandErrorMessage } from "../../lib/errors";
  import { appVersion, formatAppVersion, loadAppVersion } from "../../lib/appVersion.svelte";
  import { checkForUpdates, closeInstallPrompt, closeUpdateDetails, deferUpdateReview, downloadUpdate, installAndRestart, openUpdateDetails, updateSnapshot } from "../../lib/updates.svelte";
  import { openUrl } from "@tauri-apps/plugin-opener";

  let theme = $state<Theme>("system");
  let reducedMotion = $state(false);
  let hideToTray = $state(false);
  let restoreWindowState = $state(true);
  let launchAtLogin = $state(false);
  let startMinimized = $state(false);
  let pendingSetting = $state<string | null>(null);
  let showReset = $state(false);
  let resetMessage = $state("");
  let resetPending = $state(false);
  let search = $state("");
  let activeCategory = $state("general");
  const { toast } = useToast();

  const visibleDefinitions = $derived(settingDefinitions.filter((setting) => {
    const query = search.trim().toLowerCase();
    return !query || [setting.label, setting.description, ...setting.keywords].join(" ").toLowerCase().includes(query);
  }));
  const hasMatch = (id: string) => visibleDefinitions.some((setting) => setting.id === id);
  const description = (id: string) => settingDefinitions.find((setting) => setting.id === id)?.description ?? "";

  onMount(() => {
    void loadAppVersion();
    const updateActiveCategory = () => {
      const category = window.location.hash.slice(1);
      if (["general", "appearance", "accessibility", "about"].includes(category)) {
        activeCategory = category;
      }
    };
    updateActiveCategory();
    window.addEventListener("hashchange", updateActiveCategory);

    void (async () => {
      try {
        const settings = await loadSettings();
        const savedTheme = JSON.parse(settings["appearance.theme"] ?? '"system"');
        const savedMotion = JSON.parse(settings["accessibility.reducedMotion"] ?? "false");
        const savedHideToTray = JSON.parse(settings["general.hideToTray"] ?? "false");
        const savedRestoreWindowState = JSON.parse(settings["general.restoreWindowState"] ?? "true");
        const savedStartMinimized = JSON.parse(settings["general.startMinimized"] ?? "false");
        if (savedTheme === "system" || savedTheme === "light" || savedTheme === "dark") theme = savedTheme;
        if (typeof savedMotion === "boolean") reducedMotion = savedMotion;
        if (typeof savedHideToTray === "boolean") hideToTray = savedHideToTray;
        if (typeof savedRestoreWindowState === "boolean") restoreWindowState = savedRestoreWindowState;
        if (typeof savedStartMinimized === "boolean") startMinimized = savedStartMinimized;
        launchAtLogin = await isAutostartEnabled();
        // Native setup restores the window before it is revealed.
      } catch {
        // Defaults remain usable if the database is unavailable.
      }
    })();

    return () => window.removeEventListener("hashchange", updateActiveCategory);
  });

  async function setBooleanSetting(key: string, value: boolean) {
    const previous = { hideToTray, restoreWindowState, launchAtLogin, startMinimized }[key.split(".")[1] as "hideToTray" | "restoreWindowState" | "launchAtLogin" | "startMinimized"];
    if (key === "general.hideToTray") hideToTray = value;
    if (key === "general.restoreWindowState") restoreWindowState = value;
    if (key === "general.launchAtLogin") launchAtLogin = value;
    if (key === "general.startMinimized") startMinimized = value;
    pendingSetting = key;
    try {
      if (key === "general.launchAtLogin") value ? await enableAutostart() : await disableAutostart();
      await saveSetting(key, value);
      if (key === "general.restoreWindowState" && value) await restoreStateCurrent(StateFlags.ALL);
    } catch {
      if (key === "general.launchAtLogin") {
        try { previous ? await enableAutostart() : await disableAutostart(); } catch { /* keep the original error state */ }
      }
      toast({ title: "Setting could not be saved", description: "Your change was reverted.", severity: "error" });
      if (key === "general.hideToTray") hideToTray = Boolean(previous);
      if (key === "general.restoreWindowState") restoreWindowState = Boolean(previous);
      if (key === "general.launchAtLogin") launchAtLogin = Boolean(previous);
      if (key === "general.startMinimized") startMinimized = Boolean(previous);
    } finally { pendingSetting = null; }
  }

  async function setReducedMotion(value: boolean) {
    const previous = reducedMotion;
    reducedMotion = value;
    try {
      await saveSetting("accessibility.reducedMotion", value);
      document.documentElement.dataset.reducedMotion = value ? "true" : "false";
    } catch {
      reducedMotion = previous;
      toast({ title: "Accessibility setting could not be saved", severity: "error" });
    }
  }

  async function resetSettings() {
    resetPending = true;
    resetMessage = "";
    try {
      await resetStoredSettings();
      theme = "system";
      reducedMotion = false;
      hideToTray = false;
      restoreWindowState = true;
      launchAtLogin = false;
      startMinimized = false;
      await disableAutostart();
      document.documentElement.dataset.theme = "system";
      document.documentElement.dataset.reducedMotion = "false";
      resetMessage = "";
      showReset = false;
      toast({ title: "Settings reset", description: "Your preferences were restored to their defaults.", severity: "success" });
    } catch (error) {
      resetMessage = "";
      toast({ title: "Settings could not be reset", description: commandErrorMessage(error), severity: "error" });
    } finally {
      resetPending = false;
    }
  }

  async function checkUpdates() {
    await checkForUpdates(true);
  }

  function formatCheckTime(value: string | null) {
    return value ? new Date(value).toLocaleString() : "Not checked yet";
  }

  async function viewRelease() {
    if (!updateSnapshot.releaseUrl) return;
    try {
      await openUrl(updateSnapshot.releaseUrl);
    } catch {
      toast({ title: "Release page could not be opened", description: "Open the project releases page in your browser.", severity: "error" });
    }
  }
</script>

<svelte:head><title>Settings · CM Modpack Util</title></svelte:head>

<AppShell>

  <header class="settings-header">
    <div><p class="eyebrow">Preferences</p><h1>Settings</h1></div>
  </header>

  <div class="search-wrap">
    <label for="settings-search">Search settings</label>
    <div class="search-box"><MagnifyingGlassIcon size={18} weight="regular" aria-hidden="true" /><input id="settings-search" bind:value={search} placeholder="Search settings" onkeydown={(event) => event.key === "Escape" && (search = "")} />{#if search}<Tooltip text="Clear search"><Button variant="ghost" size="icon" type="button" aria-label="Clear settings search" onclick={() => (search = "")}><XIcon size={16} weight="bold" aria-hidden="true" /></Button></Tooltip>{/if}</div>
  </div>

  <div class="settings-layout">
    <nav class="sidebar" aria-label="Settings categories">
      <a class:active={activeCategory === "general"} class="category" href="#general" aria-current={activeCategory === "general" ? "location" : undefined}>General</a>
      <a class:active={activeCategory === "appearance"} class="category" href="#appearance" aria-current={activeCategory === "appearance" ? "location" : undefined}>Appearance</a>
      <a class:active={activeCategory === "accessibility"} class="category" href="#accessibility" aria-current={activeCategory === "accessibility" ? "location" : undefined}>Accessibility</a>
      <a class:active={activeCategory === "about"} class="category" href="#about" aria-current={activeCategory === "about" ? "location" : undefined}>About</a>
    </nav>

    <div class="detail-pane">
      {#if hasMatch("general.hideToTray") || hasMatch("general.restoreWindowState") || hasMatch("general.launchAtLogin") || hasMatch("general.startMinimized")}
        <section id="general" class="settings-section" aria-labelledby="general-title"><p class="eyebrow">General</p><h2 id="general-title">Control how the app starts and closes</h2>
          {#if hasMatch("general.hideToTray")}<div class="setting-card setting-row"><div class="setting-copy"><h3>Hide to tray when closing</h3><p>{description("general.hideToTray")}</p></div><label class="switch"><input type="checkbox" checked={hideToTray} disabled={pendingSetting === "general.hideToTray"} onchange={(event) => setBooleanSetting("general.hideToTray", event.currentTarget.checked)} /><span aria-hidden="true"></span><b>{hideToTray ? "On" : "Off"}</b></label></div>{/if}
          {#if hasMatch("general.restoreWindowState")}<div class="setting-card setting-row"><div class="setting-copy"><h3>Restore last window size and position</h3><p>{description("general.restoreWindowState")}</p></div><label class="switch"><input type="checkbox" checked={restoreWindowState} disabled={pendingSetting === "general.restoreWindowState"} onchange={(event) => setBooleanSetting("general.restoreWindowState", event.currentTarget.checked)} /><span aria-hidden="true"></span><b>{restoreWindowState ? "On" : "Off"}</b></label></div>{/if}
          {#if hasMatch("general.launchAtLogin")}<div class="setting-card setting-row"><div class="setting-copy"><h3>Launch at login</h3><p>{description("general.launchAtLogin")}</p></div><label class="switch"><input type="checkbox" checked={launchAtLogin} disabled={pendingSetting === "general.launchAtLogin"} onchange={(event) => setBooleanSetting("general.launchAtLogin", event.currentTarget.checked)} /><span aria-hidden="true"></span><b>{launchAtLogin ? "On" : "Off"}</b></label></div>{/if}
          {#if hasMatch("general.startMinimized")}<div class="setting-card setting-row"><div class="setting-copy"><h3>Start minimized</h3><p>{description("general.startMinimized")}</p></div><label class="switch"><input type="checkbox" checked={startMinimized} disabled={pendingSetting === "general.startMinimized"} onchange={(event) => setBooleanSetting("general.startMinimized", event.currentTarget.checked)} /><span aria-hidden="true"></span><b>{startMinimized ? "On" : "Off"}</b></label></div>{/if}
        </section>
      {/if}
      {#if hasMatch("appearance.theme")}
        <section id="appearance" class="settings-section" aria-labelledby="appearance-title"><p class="eyebrow">Appearance</p><h2 id="appearance-title">How the app looks</h2><div class="setting-card"><div class="setting-copy"><h3>Theme</h3><p>{settingDefinitions[0].description}</p></div><ThemeSelect bind:value={theme} /></div></section>
      {/if}
      {#if hasMatch("accessibility.reducedMotion")}
        <section id="accessibility" class="settings-section" aria-labelledby="accessibility-title"><p class="eyebrow">Accessibility</p><h2 id="accessibility-title">Make the app easier to use</h2><div class="setting-card setting-row"><div class="setting-copy"><h3>Reduce motion</h3><p>{settingDefinitions[1].description}</p></div><label class="switch"><input type="checkbox" checked={reducedMotion} onchange={(event) => setReducedMotion(event.currentTarget.checked)} /><span aria-hidden="true"></span><b>{reducedMotion ? "On" : "Off"}</b></label></div></section>
      {/if}
      {#if search && !hasMatch("appearance.theme") && !hasMatch("accessibility.reducedMotion")}<p class="empty-state">No settings matched “{search}”. <Button variant="quiet" size="sm" type="button" onclick={() => (search = "")}>Clear search</Button></p>{/if}

      <section id="about" class="settings-section" aria-labelledby="about-title"><p class="eyebrow">About</p><h2 id="about-title">CM Modpack Util</h2><div class="setting-card about-card"><p>A local-first foundation for registered modpacks.</p><p class="muted">Current version {formatAppVersion(appVersion.value ?? updateSnapshot.currentVersion)}</p><div class="update-status" role="status" aria-live="polite"><div><h3>Updates</h3><p class="muted">{updateSnapshot.state === "up_to_date" ? "You are up to date." : updateSnapshot.downloadReady ? "The update is downloaded and ready to install." : updateSnapshot.availableVersion ? `Version ${updateSnapshot.availableVersion} is available.` : updateSnapshot.error?.message ?? "Check for the latest stable release."}</p><p class="muted">Last checked: {formatCheckTime(updateSnapshot.lastCheckedAt)}</p></div><div class="update-actions">{#if updateSnapshot.downloadReady}<Button variant="primary" size="sm" type="button" onclick={installAndRestart}>Install and restart</Button>{:else if updateSnapshot.availableVersion}<Button variant="quiet" size="sm" type="button" onclick={openUpdateDetails}>View update</Button>{/if}<Button variant="secondary" size="sm" type="button" disabled={updateSnapshot.state === "checking" || updateSnapshot.state === "downloading" || updateSnapshot.state === "installing"} onclick={checkUpdates}>{updateSnapshot.state === "checking" ? "Checking…" : "Check for updates"}</Button></div></div></div></section>
      <section class="settings-section danger-section" aria-labelledby="reset-title"><p class="eyebrow">Advanced</p><h2 id="reset-title">Reset settings</h2><div class="setting-card reset-card"><div><h3>Restore defaults</h3><p>Remove all saved preferences from the local SQLite database.</p></div><Button variant="danger" type="button" disabled={resetPending} onclick={() => (showReset = true)}>Reset settings</Button></div>{#if resetMessage}<p class="status" role="status">{resetMessage}</p>{/if}</section>
    </div>
  </div>
</AppShell>

<Modal open={updateSnapshot.detailsOpen} title={`Update to ${updateSnapshot.availableVersion ?? "latest version"}`} onclose={closeUpdateDetails}>
  <div class="update-details">
    <p class="lede">A stable update is available for CM Modpack Util.</p>
    <dl class="update-facts"><div><dt>Current version</dt><dd>{formatAppVersion(appVersion.value ?? updateSnapshot.currentVersion)}</dd></div><div><dt>Available version</dt><dd>{updateSnapshot.availableVersion ?? "Unavailable"}</dd></div><div><dt>Release date</dt><dd>{updateSnapshot.releaseDate ? new Date(updateSnapshot.releaseDate).toLocaleDateString() : "Not provided"}</dd></div></dl>
    <section aria-labelledby="release-notes-title"><h3 id="release-notes-title">Release notes</h3>{#if updateSnapshot.releaseNotes}<div class="release-notes">{updateSnapshot.releaseNotes}</div>{:else}<p class="muted">No release notes were provided. Review the release page for details.</p>{/if}</section>
    <p class="muted">The signed Windows installer will download only after you choose <strong>Download update</strong>. Windows will close and restart only after you choose <strong>Install and restart</strong>.</p>
  </div>
  <div class="modal-actions"><Button variant="ghost" type="button" onclick={viewRelease}>View release on GitHub</Button><Button variant="secondary" type="button" onclick={deferUpdateReview}>Later</Button><Button variant="primary" type="button" onclick={downloadUpdate}>Download update</Button></div>
</Modal>

<Modal open={updateSnapshot.installPromptOpen} title="Update ready to install" onclose={closeInstallPrompt}>
  <p>The update has been downloaded and is ready to install. Choose whether to restart now or continue working and install it later.</p>
  <div class="modal-actions"><Button variant="secondary" type="button" onclick={closeInstallPrompt}>Later</Button><Button variant="primary" type="button" onclick={installAndRestart}>Install and restart</Button></div>
</Modal>

<Modal bind:open={showReset} title="Reset all settings?" onclose={() => (showReset = false)}>
  <p>This removes saved preferences and restores the default theme and accessibility settings. This action cannot be undone.</p>
  <div class="modal-actions"><Button variant="secondary" type="button" onclick={() => (showReset = false)}>Cancel</Button><Button variant="danger" type="button" disabled={resetPending} onclick={resetSettings}>Reset settings</Button></div>
</Modal>

<style>
  .settings-header { display: flex; align-items: flex-start; gap: 34px; margin: 25px 0 24px; }.eyebrow { margin: 0 0 7px; color: var(--color-cyan); font-family: var(--font-mono); font-size: 10px; font-weight: 700; letter-spacing: .12em; text-transform: uppercase; }h1, h2, h3 { margin: 0; }h1 { font-size: clamp(24px, 4vw, 36px); line-height: 1.04; }h2 { font-size: 20px; }h3 { font-size: 15px; }
  .search-wrap { width: 100%; margin-bottom: 32px; }.search-wrap label { display: block; margin-bottom: 8px; color: var(--color-text-muted); font-size: 12px; }.search-box { display: flex; align-items: center; gap: 8px; padding: 0 12px; border: 1px solid var(--color-line); border-radius: var(--radius-sm); background: var(--color-panel); }.search-box input { width: 100%; padding: 8px 0; border: 0; outline: 0; color: var(--color-ink); background: transparent; }.settings-layout { display: grid; grid-template-columns: 190px minmax(0, 1fr); gap: 48px; }.sidebar { display: flex; flex-direction: column; gap: 4px; }.category { padding: 10px 12px; border-left: 2px solid transparent; color: var(--color-text-muted); text-decoration: none; }.category:hover, .category.active { border-left-color: var(--color-cyan); color: var(--color-ink); background: var(--color-panel-muted); }.detail-pane { min-width: 0; }.settings-section { margin-bottom: 48px; scroll-margin-top: 24px; }.settings-section > h2 { margin-bottom: 18px; }.setting-card { padding: 22px; border: 1px solid var(--color-line); border-radius: var(--radius-sm); background: var(--color-panel); }.setting-copy p, .about-card p, .reset-card p { max-width: 620px; margin: 8px 0 20px; color: var(--color-text-muted); font-size: 13px; line-height: 1.55; }.about-card p { margin: 0; }.about-card .muted { margin-top: 8px; color: var(--color-text-subtle); }.setting-row, .reset-card { display: flex; align-items: center; justify-content: space-between; gap: 20px; }.setting-row .setting-copy p, .reset-card p { margin-bottom: 0; }
  .switch { display: flex; align-items: center; gap: 10px; color: var(--color-text-muted); cursor: pointer; }.switch input { position: absolute; opacity: 0; }.switch span { width: 42px; height: 24px; padding: 3px; border-radius: 20px; background: var(--color-border); }.switch span::after { display: block; width: 18px; height: 18px; border-radius: 50%; background: var(--color-text-muted); content: ""; transition: transform .15s; }.switch input:checked + span { background: var(--color-accent-strong); }.switch input:checked + span::after { background: var(--color-on-accent); transform: translateX(18px); }.switch input:focus-visible + span { box-shadow: var(--focus-ring); }.switch b { font-size: 12px; }.status, .empty-state { color: var(--color-text-muted); font-size: 12px; }  .update-status { display: flex; align-items: flex-start; justify-content: space-between; gap: 16px; margin-top: 18px; padding-top: 18px; border-top: 1px solid var(--color-line); }.update-status p { margin: 5px 0 0; }.update-actions { display: flex; flex-wrap: wrap; justify-content: flex-end; gap: 8px; }.modal-actions { display: flex; flex-wrap: wrap; justify-content: flex-end; gap: 10px; margin-top: 22px; }.update-details { color: var(--color-text-muted); }.update-details .lede { margin-top: 0; }.update-facts { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 12px; margin: 18px 0 24px; }.update-facts div { padding: 12px; border: 1px solid var(--color-line); border-radius: var(--radius-sm); }.update-facts dt { color: var(--color-text-subtle); font-size: 11px; }.update-facts dd { margin: 5px 0 0; color: var(--color-text); font-weight: 700; }.release-notes { max-height: 260px; overflow: auto; margin-top: 8px; padding: 12px; border: 1px solid var(--color-line); border-radius: var(--radius-sm); color: var(--color-text); white-space: pre-wrap; overflow-wrap: anywhere; }
  @media (max-width: 700px) { .settings-header { gap: 18px; flex-direction: column; margin: 17px 0; }.settings-layout { display: block; }.sidebar { flex-direction: row; margin-bottom: 32px; overflow-x: auto; }.setting-row, .reset-card { align-items: flex-start; flex-direction: column; }.update-status { flex-direction: column; }.update-actions { justify-content: flex-start; }.update-facts { grid-template-columns: 1fr; } }
</style>
