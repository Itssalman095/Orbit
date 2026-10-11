//! Pure radial selection math — the native twin of `src/lib/geometry.ts`.
//!
//! The global mouse hook owns selection while the menu is open, so this
//! implementation is the one that matters at runtime. The TypeScript copy
//! exists so the settings preview renders identically; the unit tests of
//! both mirror each other.
//!
//! Coordinate system: screen coordinates, y grows downward. Angles are
//! radians measured clockwise from +x.

pub const TAU: f64 = std::f64::consts::TAU;

#[inline]
pub fn deg_to_rad(deg: f64) -> f64 {
    deg * std::f64::consts::PI / 180.0
}

/// Normalizes an angle into [0, TAU).
#[inline]
pub fn normalize_angle(a: f64) -> f64 {
    let wrapped = ((a % TAU) + TAU) % TAU;
    if wrapped == TAU {
        0.0
    } else {
        wrapped
    }
}

/// Center angle (radians) of item `index` out of `count`, starting at
/// `start_rad`, clockwise.
#[inline]
pub fn item_center_angle(index: usize, count: usize, start_rad: f64) -> f64 {
    normalize_angle(start_rad + TAU * index as f64 / count as f64)
}

/// Position of item `index` relative to the menu center.
#[inline]
pub fn item_center(index: usize, count: usize, radius: f64, start_rad: f64) -> (f64, f64) {
    let angle = item_center_angle(index, count, start_rad);
    (radius * angle.cos(), radius * angle.sin())
}

pub struct SectorParams {
    pub dx: f64,
    pub dy: f64,
    pub count: usize,
    pub dead_zone_radius: f64,
    pub start_rad: f64,
    pub hysteresis_rad: f64,
    pub current: Option<usize>,
}

/// Determines which sector contains the cursor direction:
/// - `count == 0` → None
/// - inside the dead zone → None
/// - one item → Some(0)
/// - hysteresis: with an existing adjacent selection, the cursor must pass
///   `hysteresis_rad` beyond the shared boundary before switching.
///   Non-adjacent jumps switch immediately.
pub fn select_sector(p: &SectorParams) -> Option<usize> {
    let count = p.count;
    if count == 0 {
        return None;
    }

    let dead = p.dead_zone_radius.max(0.0);
    let dist_sq = p.dx * p.dx + p.dy * p.dy;
    if dist_sq <= dead * dead {
        return None;
    }
    if count == 1 {
        return Some(0);
    }

    let sector_width = TAU / count as f64;
    let cursor_angle = normalize_angle(p.dy.atan2(p.dx));
    // Sectors are centered on each item: item i covers
    // (start + (i - 1/2)w, start + (i + 1/2)w].
    let rel = normalize_angle(cursor_angle - p.start_rad + sector_width / 2.0);

    let mut idx = (rel / sector_width).floor() as isize;
    if idx >= count as isize {
        idx = count as isize - 1; // fp guard
    }
    let idx = idx as usize;

    let mut result = idx;
    let has_current = matches!(p.current, Some(c) if c < count);
    if p.hysteresis_rad > 0.0 && has_current {
        let current = p.current.unwrap();
        if current != idx {
            let frac = rel / sector_width - idx as f64; // [0, 1)
            let prev = (idx + count - 1) % count;
            let next = (idx + 1) % count;
            if current == prev {
                if frac * sector_width < p.hysteresis_rad {
                    result = current;
                }
            } else if current == next {
                if (1.0 - frac) * sector_width < p.hysteresis_rad {
                    result = current;
                }
            }
            // Non-adjacent jumps switch immediately.
        }
    }
    Some(result)
}

/// True when the physical point lies within an icon circle (used for the
/// sticky-mode click hit test).
pub fn hit_test_icon(
    px: f64,
    py: f64,
    center_x: f64,
    center_y: f64,
    icon_size: f64,
    tolerance: f64,
) -> bool {
    let half = icon_size / 2.0 + tolerance;
    let dx = px - center_x;
    let dy = py - center_y;
    dx * dx + dy * dy <= half * half
}

#[cfg(test)]
mod tests {
    use super::*;

    const START: f64 = -std::f64::consts::FRAC_PI_2; // -90°, first item at 12 o'clock

    fn sel(
        dx: f64,
        dy: f64,
        count: usize,
        dead: f64,
        current: Option<usize>,
        hyst: f64,
    ) -> Option<usize> {
        select_sector(&SectorParams {
            dx,
            dy,
            count,
            dead_zone_radius: dead,
            start_rad: START,
            hysteresis_rad: hyst,
            current,
        })
    }

    fn at_angle(deg: f64, r: f64) -> (f64, f64) {
        let a = deg_to_rad(deg);
        (r * a.cos(), r * a.sin())
    }

    #[test]
    fn normalize_wraps() {
        assert_eq!(normalize_angle(0.0), 0.0);
        assert!((normalize_angle(-TAU / 4.0) - 3.0 * TAU / 4.0).abs() < 1e-12);
        assert!((normalize_angle(TAU + 0.5) - 0.5).abs() < 1e-12);
        assert_eq!(normalize_angle(TAU), 0.0);
    }

    #[test]
    fn empty_menu_selects_nothing() {
        assert_eq!(sel(0.0, -100.0, 0, 30.0, None, 0.0), None);
    }

    #[test]
    fn single_item_always_selected_outside_dead_zone() {
        assert_eq!(sel(50.0, 0.0, 1, 30.0, None, 0.0), Some(0));
    }

    #[test]
    fn compass_directions() {
        assert_eq!(sel(0.0, -100.0, 4, 30.0, None, 0.0), Some(0)); // up
        assert_eq!(sel(100.0, 0.0, 4, 30.0, None, 0.0), Some(1)); // right
        assert_eq!(sel(0.0, 100.0, 4, 30.0, None, 0.0), Some(2)); // down
        assert_eq!(sel(-100.0, 0.0, 4, 30.0, None, 0.0), Some(3)); // left
    }

    #[test]
    fn eight_and_twelve_items() {
        assert_eq!(sel(100.0, 0.0, 8, 30.0, None, 0.0), Some(2));
        assert_eq!(sel(100.0, 0.0, 12, 30.0, None, 0.0), Some(3));
        assert_eq!(sel(0.0, 100.0, 12, 30.0, None, 0.0), Some(6));
    }

    #[test]
    fn dead_zone_boundaries() {
        assert_eq!(sel(0.0, 0.0, 4, 30.0, None, 0.0), None);
        assert_eq!(sel(29.0, 0.0, 4, 30.0, None, 0.0), None);
        assert_eq!(sel(31.0, 0.0, 4, 30.0, None, 0.0), Some(1));
    }

    #[test]
    fn sector_boundaries() {
        // Boundary between items 0 and 1 sits at -45°; the boundary value
        // itself belongs to the next (clockwise) sector.
        assert_eq!(
            sel(
                at_angle(-45.0, 100.0).0,
                at_angle(-45.0, 100.0).1,
                4,
                30.0,
                None,
                0.0
            ),
            Some(1)
        );
        assert_eq!(
            sel(
                at_angle(-46.0, 100.0).0,
                at_angle(-46.0, 100.0).1,
                4,
                30.0,
                None,
                0.0
            ),
            Some(0)
        );
        assert_eq!(
            sel(
                at_angle(-44.0, 100.0).0,
                at_angle(-44.0, 100.0).1,
                4,
                30.0,
                None,
                0.0
            ),
            Some(1)
        );
    }

    #[test]
    fn adjacent_movement_without_hysteresis() {
        assert_eq!(sel(0.0, -100.0, 4, 30.0, Some(0), 0.0), Some(0));
        assert_eq!(sel(100.0, 0.0, 4, 30.0, Some(0), 0.0), Some(1));
        assert_eq!(sel(0.0, -100.0, 4, 30.0, Some(1), 0.0), Some(0));
    }

    #[test]
    fn hysteresis_keeps_current_near_boundary() {
        let hyst = deg_to_rad(3.0);
        // 1° past the -45° boundary coming from item 0: keep 0.
        let (x, y) = at_angle(-44.0, 100.0);
        assert_eq!(sel(x, y, 4, 30.0, Some(0), hyst), Some(0));
        // 5° past: switch.
        let (x, y) = at_angle(-40.0, 100.0);
        assert_eq!(sel(x, y, 4, 30.0, Some(0), hyst), Some(1));
    }

    #[test]
    fn hysteresis_wraps_between_last_and_first() {
        let hyst = deg_to_rad(3.0);
        // Boundary between item 3 (left) and item 0 (up) is at -135°.
        let (x, y) = at_angle(-134.0, 100.0);
        assert_eq!(sel(x, y, 4, 30.0, Some(3), hyst), Some(3));
        let (x, y) = at_angle(-129.0, 100.0);
        assert_eq!(sel(x, y, 4, 30.0, Some(3), hyst), Some(0));
    }

    #[test]
    fn non_adjacent_jump_switches_immediately() {
        assert_eq!(sel(0.0, 100.0, 4, 30.0, Some(0), deg_to_rad(10.0)), Some(2));
    }

    #[test]
    fn dead_zone_resets_selection() {
        assert_eq!(sel(10.0, 0.0, 4, 30.0, Some(1), deg_to_rad(3.0)), None);
    }

    #[test]
    fn item_positions() {
        let (x, y) = item_center(0, 4, 80.0, START);
        assert!((x - 0.0).abs() < 1e-9 && (y - -80.0).abs() < 1e-9);
        let (x, y) = item_center(1, 4, 80.0, START);
        assert!((x - 80.0).abs() < 1e-9 && (y - 0.0).abs() < 1e-9);
    }

    #[test]
    fn hit_test() {
        assert!(hit_test_icon(200.0, 200.0, 200.0, 200.0, 56.0, 6.0));
        assert!(hit_test_icon(230.0, 200.0, 200.0, 200.0, 56.0, 6.0));
        assert!(!hit_test_icon(240.0, 200.0, 200.0, 200.0, 56.0, 6.0));
    }
}
