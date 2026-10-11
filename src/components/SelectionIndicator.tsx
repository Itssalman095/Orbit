import type { Point } from "../lib/geometry";

export interface SelectionIndicatorProps {
  /** Position of the selected item, relative to the menu center. */
  position: Point | null;
  iconSize: number;
  animationSpeed: number;
  glow: boolean;
  /** Live cursor position relative to the menu center, if available. */
  cursor?: Point | null;
}

/**
 * A glowing ring that slides between items to mark the current selection,
 * plus an optional subtle dot that follows the physical cursor.
 */
export function SelectionIndicator({
  position,
  iconSize,
  animationSpeed,
  glow,
  cursor,
}: SelectionIndicatorProps) {
  const anim = Math.min(Math.max(animationSpeed, 0), 180);
  return (
    <>
      <div
        className={"selection-indicator" + (glow ? " has-glow" : "")}
        style={{
          width: iconSize * 1.32,
          height: iconSize * 1.32,
          transitionDuration: `${anim}ms`,
          opacity: position ? 1 : 0,
          transform: position
            ? `translate3d(calc(-50% + ${position.x}px), calc(-50% + ${position.y}px), 0)`
            : "translate3d(-50%, -50%, 0)",
        }}
        aria-hidden="true"
      />
      {cursor ? (
        <div
          className="cursor-dot"
          style={{
            transform: `translate3d(calc(-50% + ${cursor.x}px), calc(-50% + ${cursor.y}px), 0)`,
          }}
          aria-hidden="true"
        />
      ) : null}
    </>
  );
}
