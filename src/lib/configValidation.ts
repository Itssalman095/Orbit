/**
 * Configuration validation and sanitization.
 *
 * Loaded JSON is untrusted: it may be missing fields, contain values of the
 * wrong type, or carry out-of-range numbers (e.g. from hand editing).
 * `sanitizeConfig` always returns a fully valid `Config`.
 *
 * The Rust layer performs the same normalization on load (see
 * `src-tauri/src/config.rs`); this module is used for optimistic UI edits
 * and for validating user input before saving.
 */

import type { Config, LauncherApp, Settings, TriggerButton } from "../types/config";

export const DEFAULT_SETTINGS: Settings = {
  enabled: true,
  trigger: "middle",
  radius: 140,
  iconSize: 56,
  deadZoneRadius: 30,
  startAngleDeg: -90,
  animationSpeed: 120,
  theme: "dark",
  menuOpacity: 0.9,
  selectionGlow: true,
  hysteresisDeg: 2.5,
  launchOnRelease: true,
  stickyWhenNoSelection: false,
  startWithWindows: false,
  startMinimized: true,
};

const TRIGGERS: TriggerButton[] = ["middle", "x1", "x2", "right"];

function isRecord(v: unknown): v is Record<string, unknown> {
  return typeof v === "object" && v !== null && !Array.isArray(v);
}

function clampNumber(v: unknown, min: number, max: number, fallback: number): number {
  const n = typeof v === "number" ? v : typeof v === "string" ? Number(v) : NaN;
  if (!Number.isFinite(n)) return fallback;
  return Math.min(max, Math.max(min, n));
}

function clampBool(v: unknown, fallback: boolean): boolean {
  return typeof v === "boolean" ? v : fallback;
}

function clampString(v: unknown, fallback: string): string {
  return typeof v === "string" && v.trim().length > 0 ? v.trim() : fallback;
}

function clampOptionalString(v: unknown): string | undefined {
  if (typeof v !== "string") return undefined;
  const s = v.trim();
  return s.length > 0 ? s : undefined;
}

function clampTrigger(v: unknown): TriggerButton {
  return TRIGGERS.includes(v as TriggerButton) ? (v as TriggerButton) : "middle";
}

/** Extracts a display name from an executable path ("C:\x\Notepad.exe" -> "Notepad"). */
export function nameFromPath(path: string): string {
  const clean = path.replace(/[\\/]+$/, "");
  const base = clean.split(/[\\/]/).pop() ?? clean;
  const withoutExt = base.replace(/\.(exe|lnk|bat|cmd)$/i, "");
  return withoutExt.length > 0 ? withoutExt : base;
}

/**
 * Splits a raw command-line string into argument tokens, honoring double
 * quotes:  `--profile "C:\My Dir"` -> ["--profile", "C:\My Dir"].
 */
export function splitCommandLine(input: string): string[] {
  const out: string[] = [];
  let cur = "";
  let inQuotes = false;
  for (const ch of input) {
    if (ch === '"') {
      inQuotes = !inQuotes;
    } else if (ch === " " || ch === "\t") {
      if (!inQuotes && cur.length > 0) {
        out.push(cur);
        cur = "";
      } else if (inQuotes) {
        cur += ch;
      }
    } else {
      cur += ch;
    }
  }
  if (cur.length > 0) out.push(cur);
  return out;
}

/** Maps a raw LauncherApp-ish value onto a valid LauncherApp, or null if unusable. */
export function sanitizeApp(raw: unknown): LauncherApp | null {
  if (!isRecord(raw)) return null;
  const id = clampString(raw.id, "");
  const executablePath = clampString(raw.executablePath, "");
  if (id.length === 0 || executablePath.length === 0) return null;

  const rawArgs = Array.isArray(raw.arguments)
    ? raw.arguments.filter((a): a is string => typeof a === "string")
    : [];

  return {
    id,
    name: clampString(raw.name, nameFromPath(executablePath)),
    executablePath,
    iconPath: clampOptionalString(raw.iconPath),
    arguments: rawArgs.map((a) => a.trim()).filter((a) => a.length > 0),
    workingDirectory: clampOptionalString(raw.workingDirectory),
    enabled: clampBool(raw.enabled, true),
  };
}

export function sanitizeSettings(raw: unknown): Settings {
  if (!isRecord(raw)) return { ...DEFAULT_SETTINGS };
  return {
    enabled: clampBool(raw.enabled, DEFAULT_SETTINGS.enabled),
    trigger: clampTrigger(raw.trigger),
    radius: clampNumber(raw.radius, 60, 400, DEFAULT_SETTINGS.radius),
    iconSize: clampNumber(raw.iconSize, 24, 128, DEFAULT_SETTINGS.iconSize),
    deadZoneRadius: clampNumber(raw.deadZoneRadius, 0, 200, DEFAULT_SETTINGS.deadZoneRadius),
    startAngleDeg: clampNumber(raw.startAngleDeg, -360, 360, DEFAULT_SETTINGS.startAngleDeg),
    animationSpeed: clampNumber(raw.animationSpeed, 0, 600, DEFAULT_SETTINGS.animationSpeed),
    theme: raw.theme === "light" ? "light" : "dark",
    menuOpacity: clampNumber(raw.menuOpacity, 0.2, 1, DEFAULT_SETTINGS.menuOpacity),
    selectionGlow: clampBool(raw.selectionGlow, DEFAULT_SETTINGS.selectionGlow),
    hysteresisDeg: clampNumber(raw.hysteresisDeg, 0, 15, DEFAULT_SETTINGS.hysteresisDeg),
    launchOnRelease: clampBool(raw.launchOnRelease, DEFAULT_SETTINGS.launchOnRelease),
    stickyWhenNoSelection: clampBool(raw.stickyWhenNoSelection, DEFAULT_SETTINGS.stickyWhenNoSelection),
    startWithWindows: clampBool(raw.startWithWindows, DEFAULT_SETTINGS.startWithWindows),
    startMinimized: clampBool(raw.startMinimized, DEFAULT_SETTINGS.startMinimized),
  };
}

/** Always returns a valid Config, repairing any corrupted input. */
export function sanitizeConfig(raw: unknown): Config {
  const root = isRecord(raw) ? raw : {};
  const appsRaw = Array.isArray(root.applications) ? root.applications : [];

  const seen = new Set<string>();
  const applications: LauncherApp[] = [];
  for (const item of appsRaw) {
    const app = sanitizeApp(item);
    if (!app) continue;
    // Deduplicate ids, preserving order: first occurrence wins.
    if (seen.has(app.id)) continue;
    seen.add(app.id);
    applications.push(app);
  }

  return {
    version: 1,
    settings: sanitizeSettings(root.settings),
    applications,
  };
}

/** Shape-level validation used by the settings editor; returns user-facing error strings. */
export function validateLauncherApp(app: LauncherApp): string[] {
  const errors: string[] = [];
  if (app.id.length === 0) errors.push("Missing id.");
  if (app.executablePath.trim().length === 0) {
    errors.push("Executable path is required.");
  } else if (!/\.(exe|lnk)$/i.test(app.executablePath)) {
    errors.push("Executable path should point to an .exe or .lnk file.");
  }
  if (app.name.trim().length === 0) errors.push("Display name is required.");
  return errors;
}
