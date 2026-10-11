import type { CSSProperties } from "react";
import type { MenuApp } from "../types/config";
import { AppIcon } from "./AppIcon";

export interface RadialItemProps {
  app: MenuApp;
  index: number;
  /** Item position relative to the menu center. */
  x: number;
  y: number;
  iconSize: number;
  selected: boolean;
  animationSpeed: number;
  /** Stagger delay multiplier for the opening animation. */
  openDelayMs: number;
  /** When true the item renders inline SVG-free click targets (settings preview). */
  interactive?: boolean;
  onHover?: (index: number | null) => void;
  onClick?: (index: number) => void;
  iconSrc?: string | null;
}

/**
 * A single application tile positioned on the circle.
 * Purely presentational: selection state is owned by the parent.
 */
export function RadialItem({
  app,
  index,
  x,
  y,
  iconSize,
  selected,
  animationSpeed,
  openDelayMs,
  interactive,
  onHover,
  onClick,
  iconSrc,
}: RadialItemProps) {
  const style: CSSProperties = {
    width: iconSize,
    height: iconSize,
    left: `calc(50% + ${x}px)`,
    top: `calc(50% + ${y}px)`,
    animationDuration: `${animationSpeed}ms`,
    animationDelay: `${openDelayMs}ms`,
  };

  return (
    <div
      className={
        "radial-item" + (selected ? " is-selected" : "") + (interactive ? " is-interactive" : "")
      }
      style={style}
      role={interactive ? "button" : undefined}
      tabIndex={interactive ? 0 : undefined}
      aria-label={app.name}
      data-title={app.name}
      onMouseEnter={interactive ? () => onHover?.(index) : undefined}
      onMouseLeave={interactive ? () => onHover?.(null) : undefined}
      onClick={interactive ? () => onClick?.(index) : undefined}
      onKeyDown={
        interactive
          ? (e) => {
              if (e.key === "Enter" || e.key === " ") {
                e.preventDefault();
                onClick?.(index);
              }
            }
          : undefined
      }
    >
      <AppIcon name={app.name} src={iconSrc ?? app.icon} size={iconSize} />
      <div className="radial-item-label" aria-hidden="true">
        {app.name}
      </div>
    </div>
  );
}
