/**
 * Typed subscriptions for events emitted by the Rust layer.
 * Every helper returns an unsubscribe function for React effects.
 */

import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { LaunchErrorPayload, MenuPayload, SelectionPayload } from "../types/config";

export function onMenuShow(cb: (payload: MenuPayload) => void): Promise<UnlistenFn> {
  return listen<MenuPayload>("menu://show", (e) => cb(e.payload));
}

export function onMenuSelection(cb: (payload: SelectionPayload) => void): Promise<UnlistenFn> {
  return listen<SelectionPayload>("menu://selection", (e) => cb(e.payload));
}

export function onMenuHide(cb: () => void): Promise<UnlistenFn> {
  return listen("menu://hide", () => cb());
}

export function onNavigate(cb: (tab: string) => void): Promise<UnlistenFn> {
  return listen<{ tab: string }>("app://navigate", (e) => cb(e.payload.tab));
}

export function onLaunchError(cb: (payload: LaunchErrorPayload) => void): Promise<UnlistenFn> {
  return listen<LaunchErrorPayload>("app://launch-error", (e) => cb(e.payload));
}

export function onConfigReloaded(cb: () => void): Promise<UnlistenFn> {
  return listen("app://config-reloaded", () => cb());
}
