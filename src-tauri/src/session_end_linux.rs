//! Linux no-op counterpart to the Windows session-end guard.
//!
//! Linux desktop sessions do not send `WM_ENDSESSION`; normal process exit
//! teardown is sufficient.

pub fn shutdown_requested() -> bool {
    false
}

pub fn install() {}
