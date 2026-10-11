import { useCallback, useRef, useState } from "react";
import { degToRad, itemCenter, selectSector, type Point } from "../lib/geometry";
import type { MenuApp, ThemeName } from "../types/config";
import { RadialItem } from "./RadialItem";
import { SelectionIndicator } from "./SelectionIndicator";

export interface RadialMenuProps {
  apps: MenuApp[];
  radius: number;
  iconSize: number;
  startAngleDeg: number;
  animationSpeed: number;
  theme: ThemeName;
  menuOpacity: number;
  selectionGlow: boolean;
  /** Index of the highlighted item (owned by the native layer in overlay mode). */
  selectedIndex: number | null;
  /** Cursor position relative to the menu center (CSS px), for the cursor dot. */
  cursor?: Point | null;
  emptyHint?: string;
  /**
   * Preview mode: the menu computes selection from local browser mouse events
   * (settings window only — the real overlay is click-through and selection
   * is computed by the native hook).
   */
  interactive?: boolean;
  onHoverIndex?: (index: number | null) => void;
  onPickIndex?: (index: number) => void;
  /** Draw a faint circle showing the dead zone (preview aid). */
  showDeadZone?: boolean;
  deadZoneRadius?: number;
}

export function RadialMenu({
  apps,
  radius,
  iconSize,
  startAngleDeg,
  animationSpeed,
  theme,
  menuOpacity,
  selectionGlow,
  selectedIndex,
  cursor,
  emptyHint,
  interactive,
  onHoverIndex,
  onPickIndex,
  showDeadZone,
  deadZoneRadius,
}: RadialMenuProps) {
  const startAngleRad = degToRad(startAngleDeg);
  const count = apps.length;
  const [hoverIndex, setHoverIndex] = useState<number | null>(null);
  const selectionRef = useRef<number | null>(null);

  // In interactive (preview) mode we own selection locally, mirroring the
  // native algorithm exactly.
  const effectiveSelected = interactive ? hoverIndex : selectedIndex;

  const handleMouseMove = useCallback(
    (e: React.MouseEvent<HTMLDivElement>) => {
      if (!interactive || count === 0) return;
      const rect = e.currentTarget.getBoundingClientRect();
      const dx = e.clientX - (rect.left + rect.width / 2);
      const dy = e.clientY - (rect.top + rect.height / 2);
      const next = selectSector({
        dx,
        dy,
        count,
        deadZoneRadius: deadZoneRadius ?? 0,
        startAngleRad,
        hysteresisRad: degToRad(2.5),
        current: selectionRef.current,
      });
      if (next !== selectionRef.current) {
        selectionRef.current = next;
        setHoverIndex(next);
        onHoverIndex?.(next);
      }
    },
    [interactive, count, deadZoneRadius, startAngleRad, onHoverIndex],
  );

  const handleMouseLeave = useCallback(() => {
    if (!interactive) return;
    selectionRef.current = null;
    setHoverIndex(null);
    onHoverIndex?.(null);
  }, [interactive, onHoverIndex]);

  const stagger = count > 0 ? Math.min(animationSpeed / count, 40) : 0;

  return (
    <div
      className={"radial-menu" + (interactive ? " is-interactive" : "")}
      data-theme={theme}
      onMouseMove={handleMouseMove}
      onMouseLeave={handleMouseLeave}
      style={{ "--menu-opacity": menuOpacity } as React.CSSProperties}
    >
      <div
        className="radial-backdrop"
        style={{ width: 2 * (radius + iconSize), height: 2 * (radius + iconSize) }}
        aria-hidden="true"
      />
      {showDeadZone && deadZoneRadius ? (
        <div
          className="radial-deadzone"
          style={{ width: deadZoneRadius * 2, height: deadZoneRadius * 2 }}
          aria-hidden="true"
        />
      ) : null}

      {count === 0 ? (
        <div className="radial-empty">{emptyHint ?? "No apps configured"}</div>
      ) : (
        <>
          <SelectionIndicator
            position={
              effectiveSelected !== null
                ? itemCenter(effectiveSelected, count, radius, startAngleRad, { x: 0, y: 0 })
                : null
            }
            iconSize={iconSize}
            animationSpeed={animationSpeed}
            glow={selectionGlow}
            cursor={cursor}
          />
          {apps.map((app, i) => {
            const pos = itemCenter(i, count, radius, startAngleRad, { x: 0, y: 0 });
            return (
              <RadialItem
                key={app.id}
                app={app}
                index={i}
                x={pos.x}
                y={pos.y}
                iconSize={iconSize}
                selected={effectiveSelected === i}
                animationSpeed={animationSpeed}
                openDelayMs={i * stagger}
                interactive={interactive}
                onHover={(idx) => {
                  // Hover only re-highlights; movement drives selection.
                  void idx;
                }}
                onClick={(idx) => onPickIndex?.(idx)}
              />
            );
          })}
        </>
      )}
    </div>
  );
}
