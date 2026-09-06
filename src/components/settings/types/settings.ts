export type Theme = "system" | "light" | "dark";

export type SettingDefinition = {
  id: string;
  category: "general" | "appearance" | "accessibility" | "about";
  label: string;
  description: string;
  keywords: string[];
  defaultValue: unknown;
  persistence: "application" | "session";
  applyMode: "immediate" | "grouped";
  resettable: boolean;
};

export const settingDefinitions: SettingDefinition[] = [
  {
    id: "general.hideToTray",
    category: "general",
    label: "Hide to tray when closing",
    description: "Keep the app running in the system tray when you close its window, so you can reopen it without relaunching.",
    keywords: ["tray", "close", "background", "quit"],
    defaultValue: false,
    persistence: "application",
    applyMode: "immediate",
    resettable: true,
  },
  {
    id: "general.restoreWindowState",
    category: "general",
    label: "Restore last window size and position",
    description: "Open the app where you left it, using the last window size and screen position.",
    keywords: ["window", "size", "position", "layout", "remember"],
    defaultValue: true,
    persistence: "application",
    applyMode: "immediate",
    resettable: true,
  },
  {
    id: "general.launchAtLogin",
    category: "general",
    label: "Launch at login",
    description: "Start the app automatically when you sign in to your computer.",
    keywords: ["startup", "login", "autostart", "boot"],
    defaultValue: false,
    persistence: "application",
    applyMode: "immediate",
    resettable: true,
  },
  {
    id: "general.startMinimized",
    category: "general",
    label: "Start minimized",
    description: "Start in the background with the window minimized, especially useful when launching at login.",
    keywords: ["startup", "minimize", "background", "login"],
    defaultValue: false,
    persistence: "application",
    applyMode: "immediate",
    resettable: true,
  },
  {
    id: "appearance.theme",
    category: "appearance",
    label: "Theme",
    description: "Choose whether the app follows your desktop theme or always uses a light or dark appearance.",
    keywords: ["system", "light", "dark", "color", "appearance"],
    defaultValue: "system",
    persistence: "application",
    applyMode: "immediate",
    resettable: true,
  },
  {
    id: "accessibility.reducedMotion",
    category: "accessibility",
    label: "Reduce motion",
    description: "Minimize animations and transitions. The system preference is respected by default.",
    keywords: ["animation", "motion", "accessibility"],
    defaultValue: false,
    persistence: "application",
    applyMode: "immediate",
    resettable: true,
  },
];
