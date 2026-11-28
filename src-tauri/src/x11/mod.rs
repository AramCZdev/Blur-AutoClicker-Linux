//! Shared X11 connection used by the Linux input backend.
//!
//! The connection is initialized lazily so unit tests and headless processes can
//! still start without a DISPLAY. XTest is checked during initialization rather
//! than letting the first click fail with an opaque protocol error.

use std::sync::OnceLock;

use x11rb::connection::Connection as _;
use x11rb::protocol::xproto::Window;
use x11rb::protocol::xtest::ConnectionExt as _;
use x11rb::rust_connection::RustConnection;

pub struct X11Connection {
    pub connection: RustConnection,
    pub root: Window,
}

static CONNECTION: OnceLock<Option<X11Connection>> = OnceLock::new();

/// Returns the shared X11 connection, or None when no usable X11/XWayland
/// display is available or the server does not provide the XTest extension.
pub fn get() -> Option<&'static X11Connection> {
    CONNECTION
        .get_or_init(|| {
            let (connection, screen_num) = x11rb::connect(None).ok()?;
            let root = connection.setup().roots.get(screen_num)?.root;

            // Confirm XTest exists before exposing this connection to callers.
            connection.xtest_get_version(2, 2).ok()?.reply().ok()?;

            Some(X11Connection { connection, root })
        })
        .as_ref()
}

/// Queue XTest events and flush them as one batch. XTest uses device id 0 for
/// the core pointer and NONE to target the pointer's current root window.
pub fn send_events(events: &[(u8, u8, i16, i16)]) -> bool {
    use x11rb::{CURRENT_TIME, NONE};

    let Some(server) = get() else {
        return false;
    };

    for &(event_type, detail, x, y) in events {
        if server
            .connection
            .xtest_fake_input(event_type, detail, CURRENT_TIME, NONE, x, y, 0)
            .is_err()
        {
            return false;
        }
    }

    server.connection.flush().is_ok()
}
