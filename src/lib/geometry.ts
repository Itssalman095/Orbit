/**
 * Pure radial-menu geometry and selection math.
 *
 * This module is the single source of truth for the *frontend* (overlay
 * rendering and the settings preview). The native layer keeps an equivalent,
 * unit-tested implementation in `src-tauri/src/selection.rs`; both must
 * agree, since real selection while the trigger is held is computed by the
 * global mouse hook in Rust.
 *
 * Coordinate system: screen coordinates, y grows downward.
 * Angles: radians, measured clockwise from the positive x-axis
 * (because y is down, atan2(dy, dx) naturally produces clockwise angles).
 */

export const TAU = Math.PI * 2;

export interface Point {
  x: number;
  y: number;
}

export function degToRad(deg: number): number {
  return (deg * Math.PI) / 180;
}

export function radToDeg(rad: number): number {
  return (rad * 180) / Math.PI;
}

/** Normalizes any angle into [0, TAU). */
export function normalizeAngle(a: number): number {
  const wrapped = ((a % TAU) + TAU) % TAU;
  // Guard against `-0` or tiny negative results from float modulo.
  return wrapped === TAU ? 0 : wrapped;
}

/**
 * Center angle (radians) of item `index` out of `count`, starting at
 * `startAngleRad` and proceeding clockwise.
 */
export function itemCenterAngleRad(index: number, count: number, startAngleRad: number): number {
  return normalizeAngle(startAngleRad + (TAU * index) / count);
}

/**
 * Position of item `index` around `center`, using
 *   angle = startAngleRad + TAU * index / count
 *   x = centerX + radius * cos(angle)
 *   y = centerY + radius * sin(angle)
 */
export function itemCenter(
  index: number,
  count: number,
  radius: number,
  startAngleRad: number,
  center: Point,
): Point {
  const angle = itemCenterAngleRad(index, count, startAngleRad);
  return {
    x: center.x + radius * Math.cos(angle),
    y: center.y + radius * Math.sin(angle),
  };
}

export interface SelectSectorParams {
  /** Cursor x relative to the menu center. */
  dx: number;
  /** Cursor y relative to the menu center. */
  dy: number;
  /** Number of selectable items. */
  count: number;
  /** Distance from center below which nothing is selected. */
  deadZoneRadius: number;
  /** Angle of the first item's sector center (radians). */
  startAngleRad: number;
  /** Hysteresis margin in radians; see `current`. */
  hysteresisRad: number;
  /** Previously selected index (or null) — used to damp flicker at sector boundaries. */
  current: number | null;
}

/**
 * Determines which radial sector contains the cursor direction.
 *
 * Rules:
 *  - `count <= 0` → null (empty menu).
 *  - Inside the dead zone → null.
 *  - Exactly one item → 0 for any direction outside the dead zone.
 *  - Otherwise the sector index whose wedge [start + i*TAU/n, start + (i+1)*TAU/n)
 *    contains atan2(dy, dx).
 *  - Hysteresis: when the raw index differs from `current` and `current` is an
 *    adjacent sector, the cursor must be at least `hysteresisRad` beyond the
 *    shared boundary before the selection switches. Jumps of more than one
 *    sector switch immediately.
 */
export function selectSector(p: SelectSectorParams): number | null {
  const { dx, dy, count, deadZoneRadius, startAngleRad, hysteresisRad } = p;
  if (count <= 0) return null;

  const dead = Math.max(0, deadZoneRadius);
  const distSq = dx * dx + dy * dy;
  if (distSq <= dead * dead) return null;
  if (count === 1) return 0;

  const sectorWidth = TAU / count;
  const cursorAngle = normalizeAngle(Math.atan2(dy, dx));
  // Sectors are centered on each item's center angle, so shift by half a
  // sector: item i covers angles (start + (i - 1/2)w, start + (i + 1/2)w].
  const rel = normalizeAngle(cursorAngle - startAngleRad + sectorWidth / 2);

  let idx = Math.floor(rel / sectorWidth);
  if (idx >= count) idx = count - 1; // floating point guard (rel === TAU - eps)

  let result = idx;
  const current = p.current;
  const hasCurrent = current !== null && Number.isInteger(current) && current >= 0 && current < count;
  if (hysteresisRad > 0 && hasCurrent && current !== idx) {
    const frac = rel / sectorWidth - idx; // position inside sector, [0, 1)
    const prev = (idx - 1 + count) % count;
    const next = (idx + 1) % count;
    if (current === prev) {
      // Just crossed the boundary shared with the previous sector (frac 0 side).
      if (frac * sectorWidth < hysteresisRad) result = current;
    } else if (current === next) {
      // Just crossed the boundary shared with the next sector (frac 1 side).
      if ((1 - frac) * sectorWidth < hysteresisRad) result = current;
    }
    // Non-adjacent jump: deliberate movement, switch immediately.
  }
  return result;
}

/**
 * True when the point (x, y) lies within `tolerance` of the icon at
 * `center` (used by the sticky-menu click hit test and the settings preview).
 */
export function hitTestIcon(px: number, py: number, center: Point, iconSize: number, tolerance = 6): boolean {
  const half = iconSize / 2 + tolerance;
  const dx = px - center.x;
  const dy = py - center.y;
  return dx * dx + dy * dy <= half * half;
}
