/**
 * Typed wrappers around Tauri IPC commands.
 *
 * All window/global-input work happens in Rust; the frontend only exchanges
 * configuration and rendering payloads through this narrow surface.
 */

import { invoke } from "@tauri-apps/api/core";
import type { Config, ConfigResponse } from "../types/config";

export const ipc = {
  loadConfig: (): Promise<ConfigResponse> => invoke("load_config"),
  saveConfig: (config: Config): Promise<ConfigResponse> => invoke("save_config", { config }),
  reloadConfig: (): Promise<ConfigResponse> => invoke("reload_config"),

  setLauncherEnabled: (enabled: boolean): Promise<void> => invoke("set_launcher_enabled", { enabled }),
  setStartWithWindows: (enabled: boolean): Promise<void> => invoke("set_start_with_windows", { enabled }),

  /** Opens the native Windows file picker. Returns null when cancelled. */
  pickExecutable: (): Promise<string | null> => invoke("pick_executable"),
  /** Native picker for custom icon images (png/jpg/ico/bmp). */
  pickIcon: (): Promise<string | null> => invoke("pick_icon"),
  /** Native folder picker for working directories. */
  pickDirectory: (): Promise<string | null> => invoke("pick_directory"),

  /** Extracts a Windows icon as a PNG data: URL (cached in Rust). */
  getAppIcon: (path: string): Promise<string | null> => invoke("get_app_icon", { path }),

  /** Launches a configured application by id (validated backend-side). */
  launchApp: (appId: string): Promise<void> => invoke("launch_application", { appId }),

  /** Checks existence of each path, used to flag missing executables. */
  checkPaths: (paths: string[]): Promise<boolean[]> => invoke("check_paths", { paths }),

  quitApp: (): Promise<void> => invoke("quit_app"),
};
