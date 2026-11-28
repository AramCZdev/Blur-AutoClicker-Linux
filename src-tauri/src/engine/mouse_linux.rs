use super::cycle::{execute_click_cycle, ClickCycleKind, ClickCyclePlan};
use super::worker::{sleep_interruptible, RunControl};
use super::SendResult;
use std::time::{Duration, Instant};
use x11rb::protocol::xproto::{
    ConnectionExt as XprotoExt, BUTTON_PRESS_EVENT, BUTTON_RELEASE_EVENT, MOTION_NOTIFY_EVENT,
};

// Keep the upstream engine's flag-shaped interface while translating each
// operation to an XTest event. These values intentionally match Win32's
// MOUSEEVENTF_* constants so the click-cycle logic remains platform-neutral.
const MOUSEEVENTF_MOVE: u32 = 0x0001;
const MOUSEEVENTF_LEFTDOWN: u32 = 0x0002;
const MOUSEEVENTF_LEFTUP: u32 = 0x0004;
const MOUSEEVENTF_RIGHTDOWN: u32 = 0x0008;
const MOUSEEVENTF_RIGHTUP: u32 = 0x0010;
const MOUSEEVENTF_MIDDLEDOWN: u32 = 0x0020;
const MOUSEEVENTF_MIDDLEUP: u32 = 0x0040;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VirtualScreenRect {
    pub left: i32,
    pub top: i32,
    pub width: i32,
    pub height: i32,
}

impl VirtualScreenRect {
    #[inline]
    pub fn new(left: i32, top: i32, width: i32, height: i32) -> Self {
        Self {
            left,
            top,
            width,
            height,
        }
    }

    #[inline]
    pub fn right(self) -> i32 {
        self.left + self.width
    }

    #[inline]
    pub fn bottom(self) -> i32 {
        self.top + self.height
    }

    #[inline]
    pub fn contains(self, x: i32, y: i32) -> bool {
        x >= self.left && x < self.right() && y >= self.top && y < self.bottom()
    }

    #[inline]
    pub fn offset_from(self, origin: VirtualScreenRect) -> Self {
        Self::new(
            self.left - origin.left,
            self.top - origin.top,
            self.width,
            self.height,
        )
    }
}

pub fn current_cursor_position() -> Option<(i32, i32)> {
    let server = crate::x11::get()?;
    let pointer = server
        .connection
        .query_pointer(server.root)
        .ok()?
        .reply()
        .ok()?;
    Some((pointer.root_x as i32, pointer.root_y as i32))
}

pub fn current_virtual_screen_rect() -> Option<VirtualScreenRect> {
    let server = crate::x11::get()?;
    let geometry = server
        .connection
        .get_geometry(server.root)
        .ok()?
        .reply()
        .ok()?;
    if geometry.width == 0 || geometry.height == 0 {
        return None;
    }
    Some(VirtualScreenRect::new(
        0,
        0,
        geometry.width as i32,
        geometry.height as i32,
    ))
}

/// RandR monitor enumeration is a follow-up task. The root window geometry is
/// the complete X screen and is a safe fallback for stop-zone calculations.
pub fn current_monitor_rects() -> Option<Vec<VirtualScreenRect>> {
    current_virtual_screen_rect().map(|screen| vec![screen])
}

#[inline]
pub fn get_cursor_pos() -> (i32, i32) {
    current_cursor_position().unwrap_or((0, 0))
}

#[inline]
pub fn move_mouse(target_x: i32, target_y: i32) {
    let x = target_x.clamp(i16::MIN as i32, i16::MAX as i32) as i16;
    let y = target_y.clamp(i16::MIN as i32, i16::MAX as i32) as i16;
    if !crate::x11::send_events(&[(MOTION_NOTIFY_EVENT, 0, x, y)]) {
        log::warn!("[X11] Failed to move pointer; check DISPLAY and XTest availability");
    }
}

#[inline]
pub fn get_button_flags(button: i32) -> (u32, u32) {
    match button {
        2 => (MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP),
        3 => (MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP),
        _ => (MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP),
    }
}

fn button_for_flag(flag: u32) -> Option<(u8, u8)> {
    if flag & MOUSEEVENTF_LEFTDOWN != 0 || flag & MOUSEEVENTF_LEFTUP != 0 {
        Some((
            1,
            if flag & MOUSEEVENTF_LEFTDOWN != 0 {
                BUTTON_PRESS_EVENT
            } else {
                BUTTON_RELEASE_EVENT
            },
        ))
    } else if flag & MOUSEEVENTF_RIGHTDOWN != 0 || flag & MOUSEEVENTF_RIGHTUP != 0 {
        Some((
            3,
            if flag & MOUSEEVENTF_RIGHTDOWN != 0 {
                BUTTON_PRESS_EVENT
            } else {
                BUTTON_RELEASE_EVENT
            },
        ))
    } else if flag & MOUSEEVENTF_MIDDLEDOWN != 0 || flag & MOUSEEVENTF_MIDDLEUP != 0 {
        Some((
            2,
            if flag & MOUSEEVENTF_MIDDLEDOWN != 0 {
                BUTTON_PRESS_EVENT
            } else {
                BUTTON_RELEASE_EVENT
            },
        ))
    } else {
        None
    }
}

#[inline]
pub fn send_mouse_event(flags: u32) -> bool {
    if flags & MOUSEEVENTF_MOVE != 0 {
        return false;
    }
    let Some((button, event_type)) = button_for_flag(flags) else {
        return false;
    };
    crate::x11::send_events(&[(event_type, button, 0, 0)])
}

pub fn send_batch(down: u32, up: u32, count: usize) -> bool {
    let Some((button, down_type)) = button_for_flag(down) else {
        return false;
    };
    let Some((up_button, up_type)) = button_for_flag(up) else {
        return false;
    };
    if button != up_button || count == 0 {
        return count == 0;
    }

    let mut events = Vec::with_capacity(count.saturating_mul(2));
    for _ in 0..count {
        events.push((down_type, button, 0, 0));
        events.push((up_type, button, 0, 0));
    }
    crate::x11::send_events(&events)
}

pub fn send_clicks(
    down: u32,
    up: u32,
    count: usize,
    plan: ClickCyclePlan,
    control: &RunControl,
    should_abort: &dyn Fn() -> bool,
) -> SendResult {
    let result = send_clicks_inner(down, up, count, plan, control, should_abort);
    if result.is_blocked() {
        let _ = send_mouse_event(up);
    }
    result
}

fn send_clicks_inner(
    down: u32,
    up: u32,
    count: usize,
    plan: ClickCyclePlan,
    control: &RunControl,
    should_abort: &dyn Fn() -> bool,
) -> SendResult {
    if count == 0 || should_abort() {
        return SendResult::Sent;
    }

    if plan.kind == ClickCycleKind::Single && plan.first_hold_ms == 0 {
        if count > 1 {
            return batch_result(send_batch(down, up, count));
        }
        if !control.is_active() {
            return SendResult::Sent;
        }
        return batch_result(send_batch(down, up, 1));
    }

    let blocked = std::cell::Cell::new(false);
    let is_active = || control.is_active() && !should_abort();
    let mut sleep_for = |duration| sleep_interruptible(duration, control, should_abort);

    for _ in 0..count {
        if should_abort() {
            break;
        }
        let mut press = || {
            if !send_mouse_event(down) {
                blocked.set(true);
            }
        };
        let mut release = || {
            if !send_mouse_event(up) {
                blocked.set(true);
            }
        };
        if !execute_click_cycle(plan, &mut press, &mut release, &mut sleep_for, &is_active) {
            break;
        }
    }

    if blocked.get() {
        SendResult::Blocked
    } else {
        SendResult::Sent
    }
}

#[inline]
fn batch_result(sent: bool) -> SendResult {
    if sent {
        SendResult::Sent
    } else {
        SendResult::Blocked
    }
}

#[inline]
pub fn ease_in_out_quad(t: f64) -> f64 {
    if t < 0.5 {
        2.0 * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
    }
}

#[inline]
pub fn cubic_bezier(t: f64, p0: f64, p1: f64, p2: f64, p3: f64) -> f64 {
    let u = 1.0 - t;
    u * u * u * p0 + 3.0 * u * u * t * p1 + 3.0 * u * t * t * p2 + t * t * t * p3
}

pub fn smooth_move(
    start_x: i32,
    start_y: i32,
    end_x: i32,
    end_y: i32,
    duration_ms: u64,
    _rng: &mut crate::engine::rng::SmallRng,
) {
    if duration_ms < 3 || (start_x == end_x && start_y == end_y) {
        move_mouse(end_x, end_y);
        return;
    }

    let steps = ((duration_ms / 8) as usize).clamp(4, 75);
    let tick = Duration::from_millis(duration_ms) / steps as u32;
    let start = Instant::now();

    for step in 0..=steps {
        let t = ease_in_out_quad(step as f64 / steps as f64);
        let x = (start_x as f64 + (end_x - start_x) as f64 * t).round() as i32;
        let y = (start_y as f64 + (end_y - start_y) as f64 * t).round() as i32;
        move_mouse(x, y);

        if step < steps {
            let expected = tick * (step + 1) as u32;
            if expected > start.elapsed() {
                std::thread::sleep(expected - start.elapsed());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn screen_rect_uses_half_open_bounds() {
        let rect = VirtualScreenRect::new(0, 0, 1920, 1080);
        assert!(rect.contains(0, 0));
        assert!(rect.contains(1919, 1079));
        assert!(!rect.contains(1920, 1079));
        assert!(!rect.contains(1919, 1080));
    }

    #[test]
    fn button_mapping_preserves_ui_button_choices() {
        assert_eq!(
            get_button_flags(1),
            (MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP)
        );
        assert_eq!(
            get_button_flags(2),
            (MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP)
        );
        assert_eq!(
            get_button_flags(3),
            (MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP)
        );
    }

    #[test]
    fn x11_button_numbers_are_correct() {
        assert_eq!(
            button_for_flag(MOUSEEVENTF_LEFTDOWN),
            Some((1, BUTTON_PRESS_EVENT))
        );
        assert_eq!(
            button_for_flag(MOUSEEVENTF_RIGHTUP),
            Some((3, BUTTON_RELEASE_EVENT))
        );
        assert_eq!(
            button_for_flag(MOUSEEVENTF_MIDDLEDOWN),
            Some((2, BUTTON_PRESS_EVENT))
        );
    }

    #[test]
    fn headless_connection_is_optional() {
        // This test deliberately does not require an X server. The first call
        // may return None in CI and must not panic.
        let _ = current_cursor_position();
    }
}
