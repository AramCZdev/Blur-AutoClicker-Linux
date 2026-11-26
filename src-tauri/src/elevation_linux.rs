//! Linux no-op counterpart to the Windows `elevation` module.
//!
//! Windows UAC elevation has no X11 equivalent: processes do not carry an
//! "elevated" flag and there is no `runas` verb to relaunch through. Keeping
//! the same API lets the shared UI/state code compile unchanged while every
//! elevation guard is inert on this platform.

pub const MAX_SUPPRESSED_WINDOWS: usize = 16;
pub const MAX_LABEL_CHARS: usize = 120;

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ElevationState {
    pub elevated: bool,
    pub can_elevate: bool,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockedWindow {
    #[serde(skip_serializing)]
    pub handle: isize,
    #[serde(skip_serializing)]
    pub pid: u32,
    pub name: String,
    pub title: String,
}

pub fn process_is_elevated(_pid: u32) -> bool {
    false
}

pub fn current_process_elevated() -> bool {
    false
}

pub fn can_elevate() -> bool {
    false
}

pub fn state() -> ElevationState {
    ElevationState {
        elevated: false,
        can_elevate: false,
    }
}

pub fn clean_label(raw: &str) -> String {
    let value = raw.trim_matches('\0').trim();
    if value.chars().count() <= MAX_LABEL_CHARS {
        return value.to_string();
    }
    value
        .chars()
        .take(MAX_LABEL_CHARS.saturating_sub(1))
        .chain(std::iter::once('…'))
        .collect()
}

pub fn window_label(description: Option<String>, stem: Option<String>, title: &str) -> String {
    let picked = description
        .map(|value| clean_label(&value))
        .filter(|value| !value.is_empty())
        .or_else(|| {
            stem.map(|value| clean_label(&value))
                .filter(|value| !value.is_empty())
        });
    picked.unwrap_or_else(|| title.to_string())
}

pub fn describe_window(handle: isize, pid: u32) -> BlockedWindow {
    BlockedWindow {
        handle,
        pid,
        name: String::new(),
        title: String::new(),
    }
}

pub fn remember_suppressed(list: &mut Vec<(isize, u32)>, entry: (isize, u32)) {
    if list.contains(&entry) {
        return;
    }
    if list.len() >= MAX_SUPPRESSED_WINDOWS {
        list.remove(0);
    }
    list.push(entry);
}

pub fn foreground_needs_prompt(_suppressed: &[(isize, u32)]) -> Option<(isize, u32)> {
    None
}

pub fn elevated_parameters<I: IntoIterator<Item = String>>(
    args: I,
    keep_autostart: bool,
) -> String {
    args.into_iter()
        .filter(|arg| keep_autostart || arg != "--autostart")
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn relaunch_elevated(_keep_autostart: bool) -> bool {
    false
}

pub fn relaunch_unelevated(_keep_autostart: bool) -> bool {
    false
}
