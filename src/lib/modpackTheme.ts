export const MODPACK_DEFAULT_THEME = "cyan";

export type ModpackPaletteKey =
  | "cyan"
  | "amber"
  | "coral"
  | "green"
  | "violet"
  | "blue"
  | "rose"
  | "slate";

export type ModpackThemeValue = ModpackPaletteKey | `#${string}`;

type ModpackPaletteEntry = {
  key: ModpackPaletteKey;
  label: string;
  color: string;
  foreground: string;
};

export const MODPACK_PALETTE: ModpackPaletteEntry[] = [
  { key: "cyan", label: "Cyan", color: "#007f88", foreground: "#ffffff" },
  { key: "amber", label: "Amber", color: "#8b6d00", foreground: "#ffffff" },
  { key: "coral", label: "Coral", color: "#a83f35", foreground: "#ffffff" },
  { key: "green", label: "Green", color: "#2e6f54", foreground: "#ffffff" },
  { key: "violet", label: "Violet", color: "#5f528d", foreground: "#ffffff" },
  { key: "blue", label: "Blue", color: "#28658f", foreground: "#ffffff" },
  { key: "rose", label: "Rose", color: "#9a4668", foreground: "#ffffff" },
  { key: "slate", label: "Slate", color: "#526168", foreground: "#ffffff" },
];

const paletteByKey = new Map(MODPACK_PALETTE.map((entry) => [entry.key, entry]));
const HEX_COLOR_PATTERN = /^#[0-9a-f]{6}$/i;

export function isCustomModpackTheme(value: string | null | undefined): value is `#${string}` {
  return Boolean(value && HEX_COLOR_PATTERN.test(value));
}

export function isKnownModpackTheme(value: string | null | undefined): value is ModpackPaletteKey {
  return Boolean(value && paletteByKey.has(value as ModpackPaletteKey));
}

export function normalizeModpackTheme(value: string | null | undefined): string {
  if (isKnownModpackTheme(value)) return value;
  if (isCustomModpackTheme(value)) return value.toLowerCase();
  return MODPACK_DEFAULT_THEME;
}

export function getModpackTheme(value: string | null | undefined): ModpackPaletteEntry & { value: string; custom: boolean } {
  const normalized = normalizeModpackTheme(value);
  const paletteEntry = paletteByKey.get(normalized as ModpackPaletteKey);
  if (paletteEntry) return { ...paletteEntry, value: normalized, custom: false };
  return { key: "slate", label: "Custom color", color: normalized, foreground: "#ffffff", value: normalized, custom: true };
}

export function isValidCustomModpackTheme(value: string): boolean {
  return HEX_COLOR_PATTERN.test(value);
}
