import { useEffect, useState, type CSSProperties } from "react";
import { emit } from "@tauri-apps/api/event";
import { onMenuHide, onMenuSelection, onMenuShow } from "../lib/events";
import { RadialMenu } from "../components/RadialMenu";
import type { MenuPayload, SelectionPayload } from "../types/config";

/**
 * Root component of the transparent overlay window.
 *
 * The overlay is click-through at the OS level; the native mouse hook owns
 * all input. This component is purely a renderer: it receives menu payloads
 * and selection updates as events and paints the radial menu around the
 * cursor.
 */
export function Overlay() {
  const [payload, setPayload] = useState<MenuPayload | null>(null);
  const [selection, setSelection] = useState<SelectionPayload | null>(null);

  useEffect(() => {
    const unlisteners: Array<() => void> = [];
    let disposed = false;

    onMenuShow((p) => {
      setPayload(p);
      setSelection(null);
    }).then(add);
    onMenuSelection((s) => setSelection(s)).then(add);
    onMenuHide(() => {
      setPayload(null);
      setSelection(null);
    }).then(add);

    function add(u: () => void) {
      if (disposed) u();
      else unlisteners.push(u);
    }

    // Tell the backend this webview is alive before the first trigger.
    void emit("menu://ready");

    return () => {
      disposed = true;
      unlisteners.forEach((u) => u());
    };
  }, []);

  // Acknowledge renders so Rust only makes the window visible once content
  // is committed (prevents any stale/empty frame flash on open).
  useEffect(() => {
    if (payload) void emit("menu://rendered");
  }, [payload]);

  if (!payload) return <div className="overlay-root" />;

  return (
    <div
      className="overlay-root"
      data-theme={payload.theme}
      style={{ "--menu-opacity": payload.menuOpacity } as CSSProperties}
    >
      <RadialMenu
        apps={payload.apps}
        radius={payload.radius}
        iconSize={payload.iconSize}
        startAngleDeg={payload.startAngleDeg}
        animationSpeed={payload.animationSpeed}
        theme={payload.theme}
        menuOpacity={payload.menuOpacity}
        selectionGlow={payload.selectionGlow}
        selectedIndex={selection?.index ?? null}
        cursor={selection?.cursor ?? null}
        emptyHint={payload.emptyHint}
      />
    </div>
  );
}
