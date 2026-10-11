/**
 * Shared configuration types for Orbit.
 *
 * The same shape is mirrored on the Rust side (src-tauri/src/config.rs)
 * and persisted as JSON in the app data directory.
 */

export type TriggerButton = "middle" | "x1" | "x2" | "right";
export type ThemeName = "dark" | "light";

export interface LauncherApp {
  id: string;
  name: string;
  executablePath: string;
  iconPath?: string;
  arguments?: string[];
  workingDirectory?: string;
  enabled: boolean;
}

export interface Settings {
  /** Master switch: is the global trigger active? */
  enabled: boolean;
  /** Which physical mouse button opens the menu. */
  trigger: TriggerButton;
  /** Menu radius in logical (CSS) pixels. */
  radius: number;
  /** Icon size in logical (CSS) pixels. */
  iconSize: number;
  /** No item is selected while the cursor is within this radius of the center. */
  deadZoneRadius: number;
  /** Angle (degrees) of the first item; -90 = 12 o'clock. */
  startAngleDeg: number;
  /** Open/close animation duration in milliseconds. */
  animationSpeed: number;
  theme: ThemeName;
  /** Overlay background opacity multiplier (0..1). */
  menuOpacity: number;
  /** Draw a glow around the selected item. */
  selectionGlow: boolean;
  /** Extra angular margin (degrees) required to move to an adjacent sector; reduces flicker at boundaries. */
  hysteresisDeg: number;
  /** Launch the highlighted app when the trigger button is released. */
  launchOnRelease: boolean;
  /** Keep the menu open (click to choose) instead of closing when the trigger is released with no selection. */
  stickyWhenNoSelection: boolean;
  /** Register the app to start with Windows (HKCU Run key, no admin rights). */
  startWithWindows: boolean;
  /** When the app auto-starts, keep it minimized to the tray. */
  startMinimized: boolean;
}

export interface Config {
  version: number;
  settings: Settings;
  applications: LauncherApp[];
}

/** Runtime menu item sent to the overlay. `icon` is a data: URL produced by the Rust icon extractor. */
export interface MenuApp {
  id: string;
  name: string;
  icon?: string | null;
}

/** Payload of the `menu://show` event. All sizes in logical (CSS) pixels. */
export interface MenuPayload {
  apps: MenuApp[];
  radius: number;
  iconSize: number;
  startAngleDeg: number;
  animationSpeed: number;
  theme: ThemeName;
  menuOpacity: number;
  selectionGlow: boolean;
  emptyHint?: string;
}

/** Payload of `menu://selection`, cursor relative to the overlay window (CSS px). */
export interface SelectionPayload {
  index: number | null;
  cursor: { x: number; y: number };
}

/** Result of load/save/reload commands; `missing` maps app id -> executable missing. */
export interface ConfigResponse {
  config: Config;
  missing: Record<string, boolean>;
}

export interface LaunchErrorPayload {
  appId: string;
  appName: string;
  message: string;
}

/** An installed app discovered in the Start Menu / Desktop (a .lnk shortcut). */
export interface AvailableApp {
  id: string;
  name: string;
  path: string;
}
