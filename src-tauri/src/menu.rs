//! Radial menu runtime: the state machine driven by the global mouse hook.
//!
//! All input decisions (open, selection updates, release behavior, sticky
//! clicks) are made here against a snapshot of geometry taken when the menu
//! opened. Side effects (window operations) are delegated to the main thread
//! via events; this module only produces decisions and payloads.

use serde::Serialize;

use crate::geometry::clamp_center_to_area;
use crate::selection;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Idle,
    /// Trigger held: menu follows selection.
    Open,
    /// Menu stays visible after release; clicks select.
    Sticky,
}

#[derive(Debug, Clone)]
pub struct MenuItem {
    pub id: String,
    pub name: String,
}

/// Geometry snapshot in logical (CSS) px + the DPI scale at open time.
#[derive(Debug, Clone, Copy)]
pub struct Geometry {
    pub radius: f64,
    pub icon_size: f64,
    pub dead_zone: f64,
    pub start_angle_rad: f64,
    pub hysteresis_rad: f64,
    pub scale: f64,
}

impl Geometry {
    pub fn radius_phys(&self) -> f64 {
        self.radius * self.scale
    }
    pub fn icon_size_phys(&self) -> f64 {
        self.icon_size * self.scale
    }
    pub fn dead_zone_phys(&self) -> f64 {
        self.dead_zone * self.scale
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct PendingShow {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

#[derive(Debug)]
pub struct MenuRuntime {
    pub phase: Phase,
    /// Menu center in physical px (clamped).
    pub center: (i32, i32),
    pub half: (i32, i32),
    pub geometry: Geometry,
    pub items: Vec<MenuItem>,
    pub selection: Option<usize>,
    /// Window geometry awaiting the webview's rendered-ack.
    pub pending_show: Option<PendingShow>,
    pub last_cursor_emit_ms: u128,
}

impl Default for MenuRuntime {
    fn default() -> Self {
        Self {
            phase: Phase::Idle,
            center: (0, 0),
            half: (0, 0),
            geometry: Geometry {
                radius: 140.0,
                icon_size: 56.0,
                dead_zone: 30.0,
                start_angle_rad: -std::f64::consts::FRAC_PI_2,
                hysteresis_rad: selection::deg_to_rad(2.5),
                scale: 1.0,
            },
            items: Vec::new(),
            selection: None,
            pending_show: None,
            last_cursor_emit_ms: 0,
        }
    }
}

impl MenuRuntime {
    /// Number of logical px of headroom around the ring.
    pub const MARGIN: i32 = 28;

    /// Opens (or re-opens) the menu at the cursor, clamped into the work area.
    pub fn open(
        &mut self,
        cursor: (i32, i32),
        work: crate::geometry::WorkArea,
        geometry: Geometry,
        items: Vec<MenuItem>,
    ) -> PendingShow {
        let half_logical_w = geometry.radius + geometry.icon_size + Self::MARGIN as f64;
        let half_logical_h = half_logical_w;
        let half_w = (half_logical_w * geometry.scale).ceil() as i32;
        let half_h = (half_logical_h * geometry.scale).ceil() as i32;

        let (cx, cy) = clamp_center_to_area(cursor, work, half_w, half_h);
        self.center = (cx, cy);
        self.half = (half_w, half_h);
        self.geometry = geometry;
        self.items = items;
        self.selection = None;
        self.phase = Phase::Open;

        let pending = PendingShow {
            x: cx - half_w,
            y: cy - half_h,
            w: half_w * 2,
            h: half_h * 2,
        };
        self.pending_show = Some(pending);
        pending
    }

    /// Updates selection from a physical cursor position while Open.
    /// Returns Some(payload) when the selection (or throttled cursor dot)
    /// should be pushed to the overlay.
    pub fn update_selection(
        &mut self,
        cursor_phys: (i32, i32),
        now_ms: u128,
    ) -> Option<SelectionDto> {
        if self.phase != Phase::Open {
            return None;
        }
        let dx = cursor_phys.0 as f64 - self.center.0 as f64;
        let dy = cursor_phys.1 as f64 - self.center.1 as f64;
        let g = self.geometry;
        let next = selection::select_sector(&selection::SectorParams {
            dx,
            dy,
            count: self.items.len(),
            dead_zone_radius: g.dead_zone_phys(),
            start_rad: g.start_angle_rad,
            hysteresis_rad: g.hysteresis_rad,
            current: self.selection,
        });

        let changed = next != self.selection;
        // The cursor dot follows the pointer, throttled to ~60 Hz; selection
        // changes always emit immediately.
        let throttle_elapsed = now_ms.saturating_sub(self.last_cursor_emit_ms) >= 15;
        if !changed && !throttle_elapsed {
            return None;
        }

        self.selection = next;
        self.last_cursor_emit_ms = now_ms;
        Some(SelectionDto {
            index: next,
            cursor: CursorDto {
                x: dx / g.scale,
                y: dy / g.scale,
            },
        })
    }

    /// Cursor dot update while Sticky (selection frozen).
    pub fn sticky_cursor_update(
        &mut self,
        cursor_phys: (i32, i32),
        now_ms: u128,
    ) -> Option<SelectionDto> {
        if self.phase != Phase::Sticky {
            return None;
        }
        if now_ms.saturating_sub(self.last_cursor_emit_ms) < 15 {
            return None;
        }
        self.last_cursor_emit_ms = now_ms;
        let dx = cursor_phys.0 as f64 - self.center.0 as f64;
        let dy = cursor_phys.1 as f64 - self.center.1 as f64;
        let g = self.geometry;
        Some(SelectionDto {
            index: self.selection,
            cursor: CursorDto {
                x: dx / g.scale,
                y: dy / g.scale,
            },
        })
    }

    /// Item center in physical px, relative to the menu center.
    pub fn item_center_phys(&self, index: usize) -> Option<(f64, f64)> {
        if index >= self.items.len() {
            return None;
        }
        Some(selection::item_center(
            index,
            self.items.len(),
            self.geometry.radius_phys(),
            self.geometry.start_angle_rad,
        ))
    }

    pub fn close(&mut self) {
        self.phase = Phase::Idle;
        self.selection = None;
        self.items.clear();
        self.pending_show = None;
    }

    pub fn go_sticky(&mut self) {
        self.phase = Phase::Sticky;
    }
}

// ---------------------------------------------------------------------------
// Release / click decisions (pure, unit tested)
// ---------------------------------------------------------------------------

pub struct ReleaseDecision {
    pub launch: Option<usize>,
    pub sticky: bool,
}

/// What happens when the trigger is released over the current selection.
/// `launch: Some(i)` implies closing after launching.
pub fn decide_release(
    selection: Option<usize>,
    launch_on_release: bool,
    sticky_when_no_selection: bool,
) -> ReleaseDecision {
    if !launch_on_release {
        // Release never launches; menu stays for a click.
        return ReleaseDecision {
            launch: None,
            sticky: true,
        };
    }
    match selection {
        Some(i) => ReleaseDecision {
            launch: Some(i),
            sticky: false,
        },
        None => {
            if sticky_when_no_selection {
                ReleaseDecision {
                    launch: None,
                    sticky: true,
                }
            } else {
                ReleaseDecision {
                    launch: None,
                    sticky: false,
                }
            }
        }
    }
}

#[derive(Debug)]
pub enum ClickOutcome {
    /// Clicked an item: launch it.
    Launch(usize),
    /// Clicked inside the window but not on an item: dismiss.
    Dismiss,
    /// Clicked outside the window: dismiss and let the click pass through.
    DismissAndPassThrough,
}

/// Sticky-mode click routing (click position in physical px).
pub fn decide_sticky_click(runtime: &MenuRuntime, px: f64, py: f64) -> ClickOutcome {
    let (cx, cy) = (runtime.center.0 as f64, runtime.center.1 as f64);
    let half = runtime.half.0 as f64;
    let half_h = runtime.half.1 as f64;
    let inside_window =
        px >= cx - half && px <= cx + half && py >= cy - half_h && py <= cy + half_h;

    if !inside_window {
        return ClickOutcome::DismissAndPassThrough;
    }

    let icon = runtime.geometry.icon_size_phys();
    let tolerance = 8.0 * runtime.geometry.scale;
    for i in 0..runtime.items.len() {
        if let Some((ix, iy)) = runtime.item_center_phys(i) {
            if selection::hit_test_icon(px, py, cx + ix, cy + iy, icon, tolerance) {
                return ClickOutcome::Launch(i);
            }
        }
    }
    ClickOutcome::Dismiss
}

// ---------------------------------------------------------------------------
// IPC payloads
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MenuAppDto {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MenuShowDto {
    pub apps: Vec<MenuAppDto>,
    pub radius: f64,
    pub icon_size: f64,
    pub start_angle_deg: f64,
    pub animation_speed: f64,
    pub theme: String,
    pub menu_opacity: f64,
    pub selection_glow: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub empty_hint: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectionDto {
    pub index: Option<usize>,
    pub cursor: CursorDto,
}

#[derive(Debug, Clone, Serialize)]
pub struct CursorDto {
    pub x: f64,
    pub y: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn geometry(scale: f64) -> Geometry {
        Geometry {
            radius: 140.0,
            icon_size: 56.0,
            dead_zone: 30.0,
            start_angle_rad: -std::f64::consts::FRAC_PI_2,
            hysteresis_rad: 0.0,
            scale,
        }
    }

    fn items(n: usize) -> Vec<MenuItem> {
        (0..n)
            .map(|i| MenuItem {
                id: format!("a{i}"),
                name: format!("App {i}"),
            })
            .collect()
    }

    fn work() -> crate::geometry::WorkArea {
        crate::geometry::WorkArea {
            left: 0,
            top: 0,
            right: 1920,
            bottom: 1040,
        }
    }

    #[test]
    fn open_clamps_center() {
        let mut rt = MenuRuntime::default();
        let pending = rt.open((10, 10), work(), geometry(1.0), items(4));
        let half = 140.0 + 56.0 + MenuRuntime::MARGIN as f64;
        assert_eq!(rt.center, (half as i32, half as i32));
        assert_eq!(pending.x, 0);
        assert_eq!(pending.w, (half as i32) * 2);
        assert_eq!(rt.phase, Phase::Open);
        assert_eq!(rt.selection, None);
    }

    #[test]
    fn open_scales_with_dpi() {
        let mut rt = MenuRuntime::default();
        let pending = rt.open((960, 520), work(), geometry(2.0), items(4));
        let half = ((140.0f64 + 56.0 + 28.0) * 2.0).ceil() as i32;
        assert_eq!(pending.w, half * 2);
        assert_eq!(rt.half, (half, half));
    }

    #[test]
    fn selection_follows_cursor_and_dead_zone() {
        let mut rt = MenuRuntime::default();
        rt.open((960, 520), work(), geometry(1.0), items(4));
        let center = rt.center;

        // Up (item 0).
        let payload = rt.update_selection((center.0, center.1 - 100), 0).unwrap();
        assert_eq!(payload.index, Some(0));
        // Dead zone.
        let payload = rt.update_selection((center.0 + 5, center.1), 1).unwrap();
        assert_eq!(payload.index, None);
        assert_eq!(rt.selection, None);
        // Right (item 1).
        let payload = rt.update_selection((center.0 + 100, center.1), 2).unwrap();
        assert_eq!(payload.index, Some(1));
    }

    #[test]
    fn throttle_suppresses_emission() {
        let mut rt = MenuRuntime::default();
        rt.open((960, 520), work(), geometry(1.0), items(4));
        let c = rt.center;
        assert!(rt.update_selection((c.0, c.0 - 100), 0).is_some());
        // Same position, within throttle window -> suppressed.
        assert!(rt.update_selection((c.0, c.0 - 100), 5).is_none());
        // After the throttle window -> emitted.
        assert!(rt.update_selection((c.0, c.0 - 100), 1000).is_some());
    }

    #[test]
    fn close_resets() {
        let mut rt = MenuRuntime::default();
        rt.open((960, 520), work(), geometry(1.0), items(4));
        rt.close();
        assert_eq!(rt.phase, Phase::Idle);
        assert!(rt.pending_show.is_none());
        assert!(rt.items.is_empty());
    }

    #[test]
    fn release_launches_selected() {
        let d = decide_release(Some(2), true, false);
        assert_eq!(d.launch, Some(2));
        assert!(!d.sticky);
    }

    #[test]
    fn release_none_closes_by_default() {
        let d = decide_release(None, true, false);
        assert_eq!(d.launch, None);
        assert!(!d.sticky);
    }

    #[test]
    fn release_none_sticky_when_configured() {
        let d = decide_release(None, true, true);
        assert_eq!(d.launch, None);
        assert!(d.sticky);
    }

    #[test]
    fn release_never_launches_when_launch_on_release_off() {
        let d = decide_release(Some(1), false, false);
        assert_eq!(d.launch, None);
        assert!(d.sticky);
    }

    #[test]
    fn sticky_click_hit_tests_items() {
        let mut rt = MenuRuntime::default();
        rt.open((500, 500), work(), geometry(1.0), items(4));
        let (cx, cy) = (rt.center.0 as f64, rt.center.1 as f64);
        // Item 1 sits to the right at distance 140.
        match decide_sticky_click(&rt, cx + 140.0, cy) {
            ClickOutcome::Launch(i) => assert_eq!(i, 1),
            other => panic!("expected Launch, got {other:?}"),
        }
        // Center click: inside window, no item -> Dismiss.
        assert!(matches!(
            decide_sticky_click(&rt, cx, cy),
            ClickOutcome::Dismiss
        ));
        // Far outside the window -> pass through.
        assert!(matches!(
            decide_sticky_click(&rt, cx + 5000.0, cy),
            ClickOutcome::DismissAndPassThrough
        ));
    }
}
