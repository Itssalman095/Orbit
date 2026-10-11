//! Screen-geometry helpers: keeping the overlay inside the monitor work
//! area, in physical pixels.

/// A monitor work area in physical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkArea {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl WorkArea {
    pub fn width(&self) -> i32 {
        self.right - self.left
    }
    pub fn height(&self) -> i32 {
        self.bottom - self.top
    }
    pub fn center(&self) -> (i32, i32) {
        ((self.left + self.right) / 2, (self.top + self.bottom) / 2)
    }
}

/// Clamps a desired overlay center (usually the cursor) so that a window of
/// half-size (`half_w`, `half_h`) stays inside the work area. If the area is
/// smaller than the window, the area center is used.
pub fn clamp_center_to_area(
    cursor: (i32, i32),
    area: WorkArea,
    half_w: i32,
    half_h: i32,
) -> (i32, i32) {
    let (mut x, mut y) = cursor;

    if area.width() >= 2 * half_w {
        x = x.clamp(area.left + half_w, area.right - half_w);
    } else {
        x = area.center().0;
    }
    if area.height() >= 2 * half_h {
        y = y.clamp(area.top + half_h, area.bottom - half_h);
    } else {
        y = area.center().1;
    }
    (x, y)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area(l: i32, t: i32, r: i32, b: i32) -> WorkArea {
        WorkArea {
            left: l,
            top: t,
            right: r,
            bottom: b,
        }
    }

    #[test]
    fn cursor_inside_area_unchanged() {
        let a = area(0, 0, 1920, 1040);
        assert_eq!(clamp_center_to_area((960, 520), a, 200, 200), (960, 520));
    }

    #[test]
    fn clamps_near_left_and_top() {
        let a = area(0, 0, 1920, 1040);
        assert_eq!(clamp_center_to_area((10, 10), a, 200, 180), (200, 180));
    }

    #[test]
    fn clamps_near_right_and_bottom() {
        let a = area(0, 0, 1920, 1040);
        assert_eq!(clamp_center_to_area((1915, 1035), a, 200, 180), (1720, 860));
    }

    #[test]
    fn clamps_on_secondary_monitor_offset() {
        // A monitor to the right of the primary with an offset origin.
        let a = area(1920, 200, 4480, 1240);
        assert_eq!(
            clamp_center_to_area((1930, 1200), a, 150, 150),
            (2070, 1090)
        );
    }

    #[test]
    fn work_area_smaller_than_window_centers() {
        let a = area(0, 0, 300, 200);
        assert_eq!(clamp_center_to_area((500, 500), a, 400, 400), (150, 100));
    }

    #[test]
    fn exact_fit_stays_at_edge() {
        let a = area(0, 0, 400, 400);
        assert_eq!(clamp_center_to_area((200, 200), a, 200, 200), (200, 200));
        assert_eq!(clamp_center_to_area((390, 390), a, 200, 200), (200, 200));
    }
}
