import { describe, expect, it } from "vitest";
import {
  TAU,
  degToRad,
  hitTestIcon,
  itemCenter,
  itemCenterAngleRad,
  normalizeAngle,
  selectSector,
} from "./geometry";

const START = degToRad(-90); // first item at 12 o'clock

describe("normalizeAngle", () => {
  it("keeps positive in-range angles unchanged", () => {
    expect(normalizeAngle(0)).toBe(0);
    expect(normalizeAngle(Math.PI)).toBeCloseTo(Math.PI, 12);
  });

  it("wraps negative angles", () => {
    expect(normalizeAngle(-TAU / 4)).toBeCloseTo((3 * TAU) / 4, 12);
    expect(normalizeAngle(-90 * (Math.PI / 180))).toBeCloseTo((270 * Math.PI) / 180, 12);
  });

  it("wraps angles >= TAU", () => {
    expect(normalizeAngle(TAU)).toBe(0);
    expect(normalizeAngle(TAU + 0.5)).toBeCloseTo(0.5, 12);
    expect(normalizeAngle(4 * TAU + 1)).toBeCloseTo(1, 12);
  });
});

describe("itemCenterAngleRad / itemCenter", () => {
  it("places the first item at the start angle", () => {
    const angle = itemCenterAngleRad(0, 6, START);
    expect(angle).toBeCloseTo(normalizeAngle(START), 12);
  });

  it("places N items evenly, clockwise from the start angle", () => {
    const center = { x: 100, y: 100 };
    const radius = 80;
    // 4 items starting at 12 o'clock: up, right, down, left.
    const up = itemCenter(0, 4, radius, START, center);
    const right = itemCenter(1, 4, radius, START, center);
    const down = itemCenter(2, 4, radius, START, center);
    const left = itemCenter(3, 4, radius, START, center);
    expect(up.x).toBeCloseTo(100, 6);
    expect(up.y).toBeCloseTo(20, 6);
    expect(right.x).toBeCloseTo(180, 6);
    expect(right.y).toBeCloseTo(100, 6);
    expect(down.x).toBeCloseTo(100, 6);
    expect(down.y).toBeCloseTo(180, 6);
    expect(left.x).toBeCloseTo(20, 6);
    expect(left.y).toBeCloseTo(100, 6);
  });

  it("computes exact positions for 6 items", () => {
    const center = { x: 0, y: 0 };
    const radius = 100;
    const p0 = itemCenter(0, 6, radius, START, center);
    expect(p0.x).toBeCloseTo(0, 6);
    expect(p0.y).toBeCloseTo(-100, 6);
    const p1 = itemCenter(1, 6, radius, START, center);
    expect(p1.x).toBeCloseTo(100 * Math.cos(degToRad(-30)), 6);
    expect(p1.y).toBeCloseTo(100 * Math.sin(degToRad(-30)), 6);
  });
});

describe("selectSector", () => {
  const base = {
    count: 4,
    deadZoneRadius: 30,
    startAngleRad: START,
    hysteresisRad: 0,
    current: null,
  };

  it("returns null for empty menus", () => {
    expect(selectSector({ ...base, count: 0, dx: 0, dy: -100 })).toBeNull();
  });

  it("selects the single item whenever outside the dead zone", () => {
    expect(selectSector({ ...base, count: 1, dx: 50, dy: 0 })).toBe(0);
    expect(selectSector({ ...base, count: 1, dx: 0, dy: 50 })).toBe(0);
  });

  it("selects the four compass directions correctly", () => {
    expect(selectSector({ ...base, dx: 0, dy: -100 })).toBe(0); // up
    expect(selectSector({ ...base, dx: 100, dy: 0 })).toBe(1); // right
    expect(selectSector({ ...base, dx: 0, dy: 100 })).toBe(2); // down
    expect(selectSector({ ...base, dx: -100, dy: 0 })).toBe(3); // left
  });

  it("handles 6, 8 and 12 item menus", () => {
    // 8 items: item 2 is at -90 + 2*45 = 0° (right).
    expect(selectSector({ ...base, count: 8, dx: 100, dy: 0 })).toBe(2);
    // 12 items: item 3 is at -90 + 3*30 = 0° (right).
    expect(selectSector({ ...base, count: 12, dx: 100, dy: 0 })).toBe(3);
    // 12 items: item 6 is straight down.
    expect(selectSector({ ...base, count: 12, dx: 0, dy: 100 })).toBe(6);
  });

  it("treats the cursor exactly at the center as dead zone", () => {
    expect(selectSector({ ...base, dx: 0, dy: 0 })).toBeNull();
  });

  it("respects the dead-zone boundary", () => {
    expect(selectSector({ ...base, dx: 29.9, dy: 0 })).toBeNull();
    expect(selectSector({ ...base, dx: 30.1, dy: 0 })).toBe(1);
    // Dead zone of zero still selects when directly at center? dx=0,dy=0 -> dist 0 <= 0 -> null.
    expect(selectSector({ ...base, deadZoneRadius: 0, dx: 0, dy: 0 })).toBeNull();
  });

  it("resolves sector boundaries deterministically", () => {
    // Between item 0 (up, -90°) and item 1 (right, 0°): boundary at -45°.
    // A boundary value itself belongs to the clockwise-next sector.
    const boundary = Math.cos(degToRad(-45)) * 100;
    const boundaryY = Math.sin(degToRad(-45)) * 100;
    expect(selectSector({ ...base, dx: boundary, dy: boundaryY })).toBe(1);
    // Just below the boundary belongs to the previous sector.
    const below = Math.cos(degToRad(-46)) * 100;
    const belowY = Math.sin(degToRad(-46)) * 100;
    expect(selectSector({ ...base, dx: below, dy: belowY })).toBe(0);
    // Just above the boundary belongs to the next sector.
    const above = Math.cos(degToRad(-44)) * 100;
    const aboveY = Math.sin(degToRad(-44)) * 100;
    expect(selectSector({ ...base, dx: above, dy: aboveY })).toBe(1);
  });

  it("handles movement between adjacent items", () => {
    const s0 = selectSector({ ...base, dx: 0, dy: -100, current: 0 });
    expect(s0).toBe(0);
    const s1 = selectSector({ ...base, dx: 100, dy: 0, current: 0 });
    expect(s1).toBe(1);
    const back = selectSector({ ...base, dx: 0, dy: -100, current: 1 });
    expect(back).toBe(0);
  });

  it("wraps around the last and first sectors", () => {
    // Moving from item 0 (up) to item 3 (left) crosses the -180° boundary.
    const s = selectSector({ ...base, dx: -100, dy: -1, current: 0 });
    expect(s).toBe(3);
    const back = selectSector({ ...base, dx: 0, dy: -100, current: 3 });
    expect(back).toBe(0);
  });
});

describe("selectSector hysteresis", () => {
  const hyst = degToRad(3);
  const params = {
    count: 4,
    deadZoneRadius: 30,
    startAngleRad: START,
    hysteresisRad: hyst,
  };

  it("keeps the current item just inside the boundary margin", () => {
    // Boundary between item 0 and 1 is at -45° (rel 45°). 1° past it must not switch.
    const dx = Math.cos(degToRad(-44)) * 100;
    const dy = Math.sin(degToRad(-44)) * 100;
    expect(selectSector({ ...params, dx, dy, current: 0 })).toBe(0);
  });

  it("switches once the cursor passes the margin", () => {
    const dx = Math.cos(degToRad(-41)) * 100;
    const dy = Math.sin(degToRad(-41)) * 100;
    expect(selectSector({ ...params, dx, dy, current: 0 })).toBe(1);
  });

  it("applies hysteresis across the wrap boundary too", () => {
    // Item 3 (left, 180°) shares its upper boundary with item 0 at -135° (== 225°).
    // Cursor 1° past that boundary into item 0's sector, coming from item 3: keep 3.
    const dx = Math.cos(degToRad(-134)) * 100;
    const dy = Math.sin(degToRad(-134)) * 100;
    expect(selectSector({ ...params, dx, dy, current: 3 })).toBe(3);
    // 5° past the boundary: switch.
    const dx2 = Math.cos(degToRad(-130)) * 100;
    const dy2 = Math.sin(degToRad(-130)) * 100;
    expect(selectSector({ ...params, dx: dx2, dy: dy2, current: 3 })).toBe(0);
  });

  it("switches immediately on non-adjacent jumps", () => {
    // From item 0 (up) straight to item 2 (down) — two sectors away.
    expect(selectSector({ ...params, dx: 0, dy: 100, current: 0 })).toBe(2);
  });

  it("selects immediately when there is no current selection", () => {
    const dx = Math.cos(degToRad(-44.5)) * 100;
    const dy = Math.sin(degToRad(-44.5)) * 100;
    expect(selectSector({ ...params, dx, dy, current: null })).toBe(1);
  });

  it("re-enters the dead zone without sticky hysteresis", () => {
    expect(selectSector({ ...params, dx: 10, dy: 0, current: 1 })).toBeNull();
  });
});

describe("hitTestIcon", () => {
  it("hits within the icon bounds plus tolerance", () => {
    const center = { x: 200, y: 200 };
    expect(hitTestIcon(200, 200, center, 56)).toBe(true);
    expect(hitTestIcon(230, 200, center, 56)).toBe(true); // edge
    expect(hitTestIcon(236, 200, center, 56)).toBe(false); // beyond half (28) + 6
    expect(hitTestIcon(200, 174, center, 56)).toBe(true); // 26px above center
    expect(hitTestIcon(200, 165, center, 56)).toBe(false); // 35px above center
  });
});
