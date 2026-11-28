//! Build-only no-op definitions for Win32 symbols still referenced by the
//! upstream engine.
//!
//! The Linux port has not rewritten the input/hook stack yet (see
//! `docs/linux-port-plan.md`, phases P1-P5). Those modules still call Win32
//! APIs directly; on Linux the linker needs local definitions for those
//! symbols. These stubs make the tree build but deliberately provide no
//! Windows behaviour. They must be replaced with real X11 implementations as
//! each phase lands.

use core::ffi::c_void;
use windows_sys::core::{BOOL, PCWSTR, PWSTR};
use windows_sys::Win32::Foundation::{
    HANDLE, HINSTANCE, HWND, LPARAM, LRESULT, POINT, WAIT_EVENT, WIN32_ERROR, WPARAM,
};
use windows_sys::Win32::Graphics::Gdi::{BITMAPINFO, DIB_USAGE, HBITMAP, HBRUSH, HDC, HGDIOBJ};
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CREATE_TOOLHELP_SNAPSHOT_FLAGS, PROCESSENTRY32W,
};
use windows_sys::Win32::System::StationsAndDesktops::{
    DESKTOP_ACCESS_FLAGS, DESKTOP_CONTROL_FLAGS, HDESK, USER_OBJECT_INFORMATION_INDEX,
};
use windows_sys::Win32::System::Threading::{PROCESS_ACCESS_RIGHTS, PROCESS_NAME_FORMAT};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{INPUT, MAP_VIRTUAL_KEY_TYPE};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    DI_FLAGS, GET_ANCESTOR_FLAGS, HHOOK, HICON, HOOKPROC, ICONINFO, MESSAGEBOX_RESULT,
    MESSAGEBOX_STYLE, MSG, MSG_WAIT_FOR_MULTIPLE_OBJECTS_EX_FLAGS, PEEK_MESSAGE_REMOVE_TYPE,
    QUEUE_STATUS_FLAGS, SYSTEM_METRICS_INDEX, SYSTEM_PARAMETERS_INFO_ACTION,
    SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, WINDOWS_HOOK_ID, WNDENUMPROC,
};

#[no_mangle]
pub unsafe extern "system" fn CallNextHookEx(
    _hhook: HHOOK,
    _ncode: i32,
    _wparam: WPARAM,
    _lparam: LPARAM,
) -> LRESULT {
    0
}

#[no_mangle]
pub unsafe extern "system" fn CloseDesktop(_hdesktop: HDESK) -> BOOL {
    0
}

#[no_mangle]
pub unsafe extern "system" fn GetAncestor(_hwnd: HWND, _gaflags: GET_ANCESTOR_FLAGS) -> HWND {
    core::ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "system" fn GetAsyncKeyState(_vkey: i32) -> i16 {
    0
}

#[no_mangle]
pub unsafe extern "system" fn GetCurrentProcessId() -> u32 {
    std::process::id()
}

#[no_mangle]
pub unsafe extern "system" fn GetCurrentThreadId() -> u32 {
    // The upstream code only compares thread IDs for equality.
    unsafe { libc::gettid() as u32 }
}

#[no_mangle]
pub unsafe extern "system" fn GetCursorPos(lppoint: *mut POINT) -> BOOL {
    if !lppoint.is_null() {
        *lppoint = POINT { x: 0, y: 0 };
    }
    0
}

#[no_mangle]
pub unsafe extern "system" fn GetDoubleClickTime() -> u32 {
    500
}

#[no_mangle]
pub unsafe extern "system" fn GetLastError() -> WIN32_ERROR {
    0
}

#[no_mangle]
pub unsafe extern "system" fn GetMessageW(
    _lpmsg: *mut MSG,
    _hwnd: HWND,
    _wmsgfiltermin: u32,
    _wmsgfiltermax: u32,
) -> BOOL {
    // WM_QUIT: let Win32-style message loops terminate on Linux.
    -1
}

#[no_mangle]
pub unsafe extern "system" fn GetThreadDesktop(_dwthreadid: u32) -> HDESK {
    core::ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "system" fn GetUserObjectInformationW(
    _hobj: HANDLE,
    _nindex: USER_OBJECT_INFORMATION_INDEX,
    pvinfo: *mut c_void,
    nlength: u32,
    lpnlengthneeded: *mut u32,
) -> BOOL {
    if !lpnlengthneeded.is_null() {
        *lpnlengthneeded = 0;
    }
    if !pvinfo.is_null() && nlength > 0 {
        unsafe { core::ptr::write_bytes(pvinfo, 0, nlength as usize) };
    }
    0
}

#[no_mangle]
pub unsafe extern "system" fn MsgWaitForMultipleObjectsEx(
    _ncount: u32,
    _phandles: *const HANDLE,
    _dwmilliseconds: u32,
    _dwwakemask: QUEUE_STATUS_FLAGS,
    _dwflags: MSG_WAIT_FOR_MULTIPLE_OBJECTS_EX_FLAGS,
) -> WAIT_EVENT {
    // WAIT_TIMEOUT.
    0x102
}

#[no_mangle]
pub unsafe extern "system" fn OpenInputDesktop(
    _dwflags: DESKTOP_CONTROL_FLAGS,
    _finherit: BOOL,
    _dwdesiredaccess: DESKTOP_ACCESS_FLAGS,
) -> HDESK {
    core::ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "system" fn PeekMessageW(
    _lpmsg: *mut MSG,
    _hwnd: HWND,
    _wmsgfiltermin: u32,
    _wmsgfiltermax: u32,
    _wremovemsg: PEEK_MESSAGE_REMOVE_TYPE,
) -> BOOL {
    0
}

#[no_mangle]
pub unsafe extern "system" fn SetWindowsHookExW(
    _idhook: WINDOWS_HOOK_ID,
    _lpfn: HOOKPROC,
    _hmod: HINSTANCE,
    _dwthreadid: u32,
) -> HHOOK {
    core::ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "system" fn SystemParametersInfoW(
    _uiaction: SYSTEM_PARAMETERS_INFO_ACTION,
    _uiparam: u32,
    _pvparam: *mut c_void,
    _fwinini: SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS,
) -> BOOL {
    0
}

#[no_mangle]
pub unsafe extern "system" fn UnhookWindowsHookEx(_hhook: HHOOK) -> BOOL {
    0
}

#[no_mangle]
pub unsafe extern "system" fn VkKeyScanW(_ch: u16) -> i16 {
    -1
}

#[no_mangle]
pub unsafe extern "system" fn WindowFromPoint(_point: POINT) -> HWND {
    core::ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "system" fn CloseHandle(_hobject: HANDLE) -> BOOL {
    0
}

#[no_mangle]
pub unsafe extern "system" fn CreateCompatibleDC(_hdc: HDC) -> HDC {
    core::ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "system" fn CreateDIBSection(
    _hdc: HDC,
    _pbmi: *const BITMAPINFO,
    _usage: DIB_USAGE,
    ppvbits: *mut *mut c_void,
    _hsection: HANDLE,
    _offset: u32,
) -> HBITMAP {
    if !ppvbits.is_null() {
        *ppvbits = core::ptr::null_mut();
    }
    core::ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "system" fn CreateToolhelp32Snapshot(
    _dwflags: CREATE_TOOLHELP_SNAPSHOT_FLAGS,
    _th32processid: u32,
) -> HANDLE {
    // INVALID_HANDLE_VALUE signals "no snapshot" to callers.
    usize::MAX as HANDLE
}

#[no_mangle]
pub unsafe extern "system" fn DeleteDC(_hdc: HDC) -> BOOL {
    0
}

#[no_mangle]
pub unsafe extern "system" fn DeleteObject(_ho: HGDIOBJ) -> BOOL {
    0
}

#[no_mangle]
pub unsafe extern "system" fn DestroyIcon(_hicon: HICON) -> BOOL {
    0
}

#[no_mangle]
pub unsafe extern "system" fn EnumWindows(_lpenumfunc: WNDENUMPROC, _lparam: LPARAM) -> BOOL {
    0
}

#[no_mangle]
pub unsafe extern "system" fn ExtractIconExW(
    _lpszfile: PCWSTR,
    _niconindex: i32,
    phiconlarge: *mut HICON,
    phiconsmall: *mut HICON,
    _nicons: u32,
) -> u32 {
    if !phiconlarge.is_null() {
        *phiconlarge = core::ptr::null_mut();
    }
    if !phiconsmall.is_null() {
        *phiconsmall = core::ptr::null_mut();
    }
    0
}

#[no_mangle]
pub unsafe extern "system" fn GetIconInfo(_hicon: HICON, piconinfo: *mut ICONINFO) -> BOOL {
    if !piconinfo.is_null() {
        *piconinfo = core::mem::zeroed::<ICONINFO>();
    }
    0
}

#[no_mangle]
pub unsafe extern "system" fn GetKeyState(_nvirtkey: i32) -> i16 {
    0
}

#[no_mangle]
pub unsafe extern "system" fn GetObjectW(_h: HGDIOBJ, _c: i32, _pv: *mut c_void) -> i32 {
    0
}

#[no_mangle]
pub unsafe extern "system" fn GetSystemMetrics(_nindex: SYSTEM_METRICS_INDEX) -> i32 {
    0
}

#[no_mangle]
pub unsafe extern "system" fn GetWindowThreadProcessId(
    _hwnd: HWND,
    lpdwprocessid: *mut u32,
) -> u32 {
    if !lpdwprocessid.is_null() {
        *lpdwprocessid = 0;
    }
    0
}

#[no_mangle]
pub unsafe extern "system" fn MapVirtualKeyW(_ucode: u32, _umaptype: MAP_VIRTUAL_KEY_TYPE) -> u32 {
    0
}

#[no_mangle]
pub unsafe extern "system" fn PostThreadMessageW(
    _idthread: u32,
    _msg: u32,
    _wparam: WPARAM,
    _lparam: LPARAM,
) -> BOOL {
    0
}

#[no_mangle]
pub unsafe extern "system" fn Process32FirstW(
    _hsnapshot: HANDLE,
    _lppe: *mut PROCESSENTRY32W,
) -> BOOL {
    0
}

#[no_mangle]
pub unsafe extern "system" fn Process32NextW(
    _hsnapshot: HANDLE,
    _lppe: *mut PROCESSENTRY32W,
) -> BOOL {
    0
}

#[no_mangle]
pub unsafe extern "system" fn SelectObject(_hdc: HDC, _h: HGDIOBJ) -> HGDIOBJ {
    core::ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "system" fn SendInput(
    _cinputs: u32,
    _pinputs: *const INPUT,
    _cbsize: i32,
) -> u32 {
    0
}

#[no_mangle]
pub unsafe extern "system" fn DrawIconEx(
    _hdc: HDC,
    _xleft: i32,
    _ytop: i32,
    _hicon: HICON,
    _cxwidth: i32,
    _cywidth: i32,
    _istepifanicur: u32,
    _hbrflickerfreedraw: HBRUSH,
    _diflags: DI_FLAGS,
) -> BOOL {
    0
}

#[no_mangle]
pub unsafe extern "system" fn GetClassNameW(
    _hwnd: HWND,
    lpclassname: PWSTR,
    nmaxcount: i32,
) -> i32 {
    if !lpclassname.is_null() && nmaxcount > 0 {
        unsafe { core::ptr::write_bytes(lpclassname, 0, nmaxcount as usize) };
    }
    0
}

#[no_mangle]
pub unsafe extern "system" fn GetForegroundWindow() -> HWND {
    core::ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "system" fn GetWindowTextW(_hwnd: HWND, lpstring: PWSTR, nmaxcount: i32) -> i32 {
    if !lpstring.is_null() && nmaxcount > 0 {
        unsafe { core::ptr::write_bytes(lpstring, 0, nmaxcount as usize) };
    }
    0
}

#[no_mangle]
pub unsafe extern "system" fn MessageBoxW(
    _hwnd: HWND,
    _lptext: PCWSTR,
    _lpcaption: PCWSTR,
    _utype: MESSAGEBOX_STYLE,
) -> MESSAGEBOX_RESULT {
    // IDCANCEL.
    2
}

#[no_mangle]
pub unsafe extern "system" fn OpenProcess(
    _dwdesiredaccess: PROCESS_ACCESS_RIGHTS,
    _binherithandle: BOOL,
    _dwprocessid: u32,
) -> HANDLE {
    core::ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "system" fn QueryFullProcessImageNameW(
    _hprocess: HANDLE,
    _dwflags: PROCESS_NAME_FORMAT,
    lpexename: PWSTR,
    lpdwsize: *mut u32,
) -> BOOL {
    if !lpdwsize.is_null() {
        *lpdwsize = 0;
    }
    if !lpexename.is_null() {
        *lpexename = 0;
    }
    0
}
