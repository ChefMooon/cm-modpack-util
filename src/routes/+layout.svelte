<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { saveWindowState, StateFlags } from "@tauri-apps/plugin-window-state";
  import "../app.css";
  import ToastViewport from "../components/ui/toast/ToastViewport.svelte";

  let { children } = $props();

  type Theme = "system" | "light" | "dark";
  let theme = $state<Theme>("system");

  function applyTheme(value: Theme) {
    theme = value;
    document.documentElement.dataset.theme = value;
  }

  onMount(() => {
    applyTheme("system");
    let mediaQuery: MediaQueryList | undefined;
    let saveTimer: ReturnType<typeof setTimeout> | undefined;
    let unlistenMoved: (() => void) | undefined;
    let unlistenResized: (() => void) | undefined;
    const handleSystemTheme = () => {
      if (theme === "system") applyTheme("system");
    };
    const scheduleWindowSave = () => {
      if (saveTimer) clearTimeout(saveTimer);
      saveTimer = setTimeout(() => {
        void saveWindowState(StateFlags.ALL);
      }, 250);
    };
    try {
      void (async () => {
        const currentWindow = getCurrentWindow();
        const settings = await invoke<Record<string, string>>("get_settings");
        const saved = JSON.parse(settings["appearance.theme"] ?? '"system"');
        const motion = JSON.parse(settings["accessibility.reducedMotion"] ?? "false");
        if (saved === "system" || saved === "light" || saved === "dark") applyTheme(saved);
        document.documentElement.dataset.reducedMotion = motion === true ? "true" : "false";
        unlistenMoved = await currentWindow.onMoved(scheduleWindowSave);
        unlistenResized = await currentWindow.onResized(scheduleWindowSave);
      })().catch(() => undefined);
      mediaQuery = window.matchMedia("(prefers-color-scheme: dark)");
      mediaQuery.addEventListener("change", handleSystemTheme);
    } catch {
      // The static preview can run without the Tauri runtime.
    }
    return () => {
      mediaQuery?.removeEventListener("change", handleSystemTheme);
      if (saveTimer) clearTimeout(saveTimer);
      unlistenMoved?.();
      unlistenResized?.();
    };
  });
</script>

<ToastViewport />
{@render children()}
