# Linux Port Migration Plan (X11-first)

**Upstream:** [`Blur009/Blur-AutoClicker`](https://github.com/Blur009/Blur-AutoClicker) v3.9.6 — "accuracy and performance focused auto clicker for Windows"
**Fork:** `AramCZdev/Blur-AutoClicker-Linux`
**License:** GPL-3.0 (preserved, see [§11](#11-gpl-30-compliance))
**Target:** X11 / XLib + XTest, via Tauri 2 + WebKitGTK, for X11 desktops (GNOME, KDE Plasma, XFCE, MATE) under Xorg or XWayland.
**Non-goal:** native Wayland support (see [§10.1](#101-wayland)).

Status: **plan only — no code has been modified.**

---

## 0. Decision record: Linux-only

**This fork targets Linux exclusively. Windows support is not preserved and the Win32 code paths are
deleted outright — not `cfg`-gated.**

This was chosen over a cross-platform build and it materially simplifies the port:

| Consequence | Effect |
|---|---|
| No platform abstraction layer | `platform/mod.rs` as a `cfg` facade is pointless indirection. The X11 module is simply `x11/`, called directly. |
| ~36 `#[cfg(...)]` sites collapse | Each becomes its Linux body; the `#[cfg(not(windows))]` fallback twins are deleted outright. |
| ~1,400 lines of Windows code deleted outright | No dual-CI maintenance, no dual-test matrix, no "works on both" burden. |
| Windows-only subsystems removed entirely | `crash_handler.rs`, `window_lifecycle.rs`, RTSS detection, WebView2 bootstrapper — all have no Linux counterpart at all, not even a stub. |
| No `target.'cfg(windows)'` tables | Single flat `[dependencies]` + `x11rb`/`libc`. |
| CI is Linux-only | No build matrix. |
| `.ico` assets deleted | PNG only. |

**If cross-platform support is ever wanted**, it is recoverable from git history — the Windows
implementation is untouched upstream at `Blur009/Blur-AutoClicker`. That is the reason for deleting
rather than gating: this fork becomes a clean, readable Linux codebase, and upstream stays the
canonical Windows build.

**Attribution requirement.** Because the Windows original is the sole source of truth for the deleted
logic, the port must preserve credit per [§11](#11-gpl-30-compliance) and the README must state
plainly that this is a modified version of `Blur009/Blur-AutoClicker`.

---

## 1. Current state assessment

| Metric | Value |
|---|---|
| Rust backend | `src-tauri/src/**` — 9,726 lines across 28 files |
| Frontend | `src/**` — ~5,678 lines TS/TSX/CSS + `overlay.html` (17 KB) |
| Rust tests | 133 `#[test]` fns (icon 40, error 18, hotkeys 11, worker 10, click_point_picker 10, failsafe 9, app_events 9, custom_stop_zone_picker 7, diagnostics 7, cycle 4, portable 4, process 3, crash_handler 1). Deleting `crash_handler.rs` in P0 leaves **132** — the baseline to protect. `error.rs`'s `WindowsSystem(u32)` / `TimerPrecision` variants lose their own tests, offset by new X11-error variants |
| Frontend tests | 5 vitest files |
| Webview engine | WebView2 (Windows) → **WebKitGTK** (Linux) |
| Files with Windows-specific code | 16 Rust files, 1 build script, 3 PowerShell scripts, 4 CI workflows, 5 docs |
| Current build on Linux | **Fails immediately.** `cargo check` aborts in `gdk-pixbuf-sys` — WebKitGTK/GTK dev packages absent. Even once installed, `windows-sys`/`webview2-com` are *unconditional* deps and `windows_targets::link!` references `kernel32.dll`. |
| Current `npm run check` on Linux | **Always exits 1** — `scripts/check.mjs:113-123` exclusive-opens `src-tauri/resources/crashpad_handler.exe`; on Linux the file is absent, `ENOENT` is caught, and the script concludes the app "is running" and aborts. This also bricks the `lefthook` pre-commit hook. |

### What is *not* Windows-specific (keep as-is)

Roughly 55% of the codebase is platform-neutral and must not be touched:

- `engine/cycle.rs` — click cycle planner + hold/gap logic (188 lines, fully tested, pure logic)
- `engine/rng.rs` — `SmallRng`
- `engine/failsafe.rs` — zone/edge/corner geometry, takes rects as inputs (312 lines, 9 tests)
- `engine/stats.rs` — run statistics recording
- `engine/worker.rs` — the click loop, CPS pacing, duty cycle, click-point rotation, offsets, smoothing orchestration (1318 lines; only 4 Windows call sites)
- `settings/mod.rs`, `app_state.rs`, `app_events.rs`, `error.rs`, `diagnostics.rs`, `updates/*`
- `icon.rs` theme/accent decision logic (the 40 tests all cover decision-making, not GDI)
- The **entire frontend** except for the items listed in [§9](#9-frontend-changes)

---

## 2. Windows dependency inventory

### 2.1 Crate dependencies

**All Windows crates are deleted. There is no `cfg(windows)` table.**

| Dependency | `Cargo.toml` | Action |
|---|---|---|
| `windows-sys 0.61` (12 Win32 features) | `:21-22` | **Delete.** Every `use windows_sys::…` below has an `x11/` replacement |
| `webview2-com 0.38` | `:23` | **Delete.** No WebView2 on Linux |
| `windows-core 0.61` | `:24` | **Delete.** COM only |
| `windows-targets 0.53` | `:46` | **Delete.** `link!` of `kernel32.dll` |
| `winreg 0.56` | `:60-61` | **Delete.** Registry → XDG config files |
| `crashpad-rs 0.2` (prebuilt) | `:52,64-65` | **Delete, including the whole `[features]` section.** No prebuilt Linux crashpad handler exists; the `prebuilt` feature downloads a `.exe`. Crash handling is reduced to the panic hook — see P0.4 and §[10](#10-behavioural-differences-to-document) |
| `image` feat `ico` | `:47` | Drop the `ico` feature; keep `png` |
| `tauri` feat `image-ico` | `:20` | Drop; keep `protocol-asset`, `tray-icon` |
| **add** `x11rb` | — | `features = ["xtst", "xinput", "randr", "xkb", "record", "shape"]` |
| **add** `libc` | — | `prctl`, `abort`, `clock_gettime` |
| **add** `resvg` | — | P5b only — SVG rendering for process icons (MPL-2.0, GPL-3.0-compatible) |
| keep `dirs` | `:45` | XDG dirs for autostart/config |
| keep `image`, `walkdir`, `zip`, `open`, `chrono`, `base64`, `reqwest`, `tokio`, … | — | platform-neutral |

### 2.2 Win32 API call sites by subsystem

| Subsystem | File | Windows API | X11 replacement |
|---|---|---|---|
| **Mouse inject** | `engine/mouse.rs:7-11,162,176,204,214` | `SendInput`, `INPUT`, `MOUSEINPUT`, `MOUSEEVENTF_*` | `x11rb::xtst::fake_input` — `Motion`, `Button` |
| **Mouse read** | `engine/mouse.rs:73-78` | `GetCursorPos` | `xproto::QueryPointer` |
| **Virtual screen** | `engine/mouse.rs:86-89` | `GetSystemMetrics(SM_*VIRTUALSCREEN)` | `xproto::GetGeometry` on root |
| **Monitors** | `engine/mouse.rs:97-143` | `EnumDisplayMonitors` + `GetMonitorInfoW` | `x11rb::randr::get_monitors` (RandR ≥1.5) |
| **Keyboard inject** | `engine/keyboard.rs:4-8,17,42,108` | `SendInput`, `KEYBDINPUT`, `KEYEVENTF_SCANCODE`, `MapVirtualKeyW(MAPVK_VK_TO_VSC_EX)` | `xtst::fake_input` `Key`; **no scan codes needed** |
| **CapsLock** | `engine/keyboard.rs:50` | `GetKeyState(VK_CAPITAL)` | Track via XI2 raw events / `XQueryKeymap` on `Caps_Lock` |
| **Process enum** | `engine/process.rs:13-15,205-223,292-319,393-413` | `CreateToolhelp32Snapshot`, `Process32FirstW/NextW` | `/proc/[0-9]*/stat` + `/comm` + `/exe` readlink |
| **Proc exe path** | `engine/process.rs:31-37,55-67` | `OpenProcess` + `QueryFullProcessImageNameW` | `readlink("/proc/{pid}/exe")` |
| **Window titles** | `engine/process.rs:231-261` | `EnumWindows`, `GetWindowTextW`, `GetWindowThreadProcessId` | `_NET_CLIENT_LIST_STACKING` on root + `_NET_WM_PID` + `WM_NAME`/`_NET_WM_NAME` |
| **Foreground proc** | `engine/process.rs:270-281` | `GetForegroundWindow` | `_NET_ACTIVE_WINDOW` on root |
| **Proc icons** | `engine/process.rs:18,75-175` | `ExtractIconExW`, `GetIconInfo`, `CreateDIBSection`, `DrawIconEx` | freedesktop Icon Theme lookup from `/proc/pid/exe` |
| **Task switcher** | `engine/process.rs:364-389` | `GetClassNameW` + `GetAsyncKeyState(VK_MENU/VK_TAB)` | `XGetClassHint`/`_NET_WM_NAME` on `_NET_ACTIVE_WINDOW` + Alt/Tab keycode state |
| **`.exe` identity** | `engine/process.rs:190-197,302-304,402` | filename suffix | **Delete the concept** — see [§6.5](#65-process-enumeration) |
| **Global hotkey hook** | `hotkeys.rs:19-26,369-446,457-464,481-485,636-639` | `SetWindowsHookExW(WH_KEYBOARD_LL/WH_MOUSE_LL)`, `KBDLLHOOKSTRUCT`, `MSLLHOOKSTRUCT`, `PeekMessageW`, `WaitMessage`, `UnhookWindowsHookEx` | XI2 raw events **or** `XQueryKeymap` polling — see [§6.2](#62-global-hotkey-listening) |
| **Key-state poll** | `hotkeys.rs:944-946` | `GetAsyncKeyState` | `XQueryKeymap` |
| **"Cursor over own window"** | `hotkeys.rs:665-681` | `WindowFromPoint`, `GetAncestor(GA_ROOT)`, `GetWindowThreadProcessId`, `GetCurrentProcessId` | `QueryPointer`'s `child` + `XQueryTree` walk + own top-level XID |
| **Mouse hotkey detect** | `hotkeys.rs:654-663` | `VK_LBUTTON`…`VK_XBUTTON2` | X11 button numbers 1/2/3/8/9 |
| **Click-point picker** | `click_point_picker.rs:5-14,117-148,364-366` | `SetWindowsHookExW` + `GetMessageW` loop + `PostThreadMessageW(WM_QUIT)` + `VK_ESCAPE/VK_SHIFT/VK_CONTROL` | **Redesign** — route through overlay webview — see [§6.6](#66-pickers-click-point--stop-zone) |
| **Stop-zone picker** | `custom_stop_zone_picker.rs:5-11,150-240` | same as above | same redesign |
| **Overlay styles** | `overlay.rs:384-417` | `GetWindowLongW/SetWindowLongW(GWL_STYLE/GWL_EXSTYLE)`, `DwmSetWindowAttribute`, `SWP_FRAMECHANGED` | Tauri APIs + `_NET_WM_WINDOW_TYPE` / `XSetInputHints` |
| **Overlay bounds** | `overlay.rs:420-438` | `SetWindowPos` | `window.set_position` + `set_size` |
| **Overlay topmost** | `overlay.rs:450-458` | `SetWindowPos(HWND_TOPMOST)` | `set_always_on_top` (`_NET_WM_STATE_ABOVE`) |
| **Overlay hide** | `overlay.rs:363-365` | `ShowWindow(hwnd, 0)` | `window.hide()` (already the non-Windows branch) |
| **Overlay HWND** | `overlay.rs:372-381` | `RawWindowHandle::Win32` | `RawWindowHandle::Xlib`/`Xcb` → `.xid()` |
| **Click-through** | `overlay.rs:63` | Tauri's `set_ignore_cursor_events` → XShape | **Already works.** No change |
| **Taskbar/panel** | `overlay.rs:53` | `skip_taskbar(true)` | **Already works** (`_NET_WM_STATE_SKIP_TASKBAR`) |
| **Main window NOACTIVATE** | `lib.rs:76-97,409-416,485,520-524` | `WS_EX_NOACTIVATE` get/set | `XSetInputHints(input=False)` — [§6.4](#64-overlay--window-semantics) |
| **Browser accel keys** | `lib.rs:32-70` | `ICoreWebView2Settings3::SetAreBrowserAcceleratorKeysEnabled(false)` | **No equivalent.** Accept + document |
| **AppUserModelID** | `lib.rs:151-168` | `SetCurrentProcessExplicitAppUserModelID` | **Delete.** No EWMH analogue |
| **Taskbar icon big** | `icon.rs:333-369` | `WM_SETICON(ICON_BIG/SMALL)`, `CreateIconIndirect`, `DestroyIcon` | **Delete.** Tauri `set_icon` sets `_NET_WM_ICON` |
| **Tray re-add hack** | `icon.rs:317-330` | Explorer `NIM_MODIFY` cache | Keep remove/add (harmless) or drop |
| **Panic dialog** | `lib.rs:184-201` | `MessageBoxW` | `tauri-plugin-dialog` or `zenity`/`kdialog`, fallback stderr |
| **Fatal startup error** | `portable.rs:152-169` | `MessageBoxW` | same |
| **WebView2 runtime** | `portable.rs:8-11,90-148,171-208`, `lib.rs:494-517` | Registry probe + bootstrapper spawn + `WinVerifyTrust` | **Delete.** WebKitGTK is a system package |
| **Autostart** | `autostart.rs:1-40` | `HKCU\...\CurrentVersion\Run` | `$XDG_CONFIG_HOME/autostart/*.desktop` |
| **Text scale** | `ui_commands.rs:25-42` | Registry `TextScaleFactor` | XSettings / GSettings `text-scaling-factor` |
| **Portability marker** | `portable.rs:36-88` | none (pure fs) | **Keep verbatim** — platform-neutral |
| **Timer precision** | `engine/worker.rs:83-105`, `engine/mod.rs:108-115` | `NtSetTimerResolution(10000)` | `prctl(PR_SET_TIMERSLACK, 1)` |
| **CPU measurement** | `engine/worker.rs:41-81` | `QueryThreadCycleTime` + `calibrate_cycle_freq()` | `clock_gettime(CLOCK_THREAD_CPUTIME_ID)` — **removes the calibration hack** |
| **Double-click time** | `engine/worker.rs:290` | `GetDoubleClickTime()` | XSettings `double-click-interval` |
| **Key repeat delay/rate** | `engine/worker.rs:17-19,290+` | `SPI_GETKEYBOARDDELAY/SPEED` | `XkbGetAutoRepeatDelay/Interval` |
| **Webview RAM trim** | `window_lifecycle.rs:3-81` | ToolHelp + `SetProcessWorkingSetSize` on `msedgewebview2.exe` | **Delete** (already a no-op off-Windows) |
| **RTSS guard** | `lib.rs:99-101,469-472,526-532` | `is_process_running("RTSS.exe")` | `is_process_running("rtss")` — keep, suffix-agnostic ([§6.5](#65-process-enumeration)) |

### 2.3 Frontend (condensed — full detail in §9)

No OS detection exists anywhere in the frontend. The Windows assumptions are hardcoded literals:
`SHIFTED_SYMBOL_BASE_MAP` (US layout table, `hotkeys.ts:50-61`), `Digit1`–`Digit5` default keybinds
(`settingsSchema.ts:429-448`), `ctrl+y` presets, `.exe` process naming in types/CSS class names,
`"Taskbar Icon"` / `"Minimize to Tray"` / `"Stop on Alt+Tab"` UI copy, `Segoe UI` font
(`overlay.html:96`), `.ico` favicon (`index.html:5`), drive-letter regex
(`backgroundMedia.ts:245-248`), forward-slash path join (`store.ts:39`).

### 2.4 Build / CI / packaging

| File:line | Issue |
|---|---|
| `tauri.conf.json:11` | `beforeBundleCommand` = `powershell …` → **every `tauri build` fails on Linux** |
| `tauri.conf.json:37` | `targets: "nsis"` → no Linux artifact possible |
| `tauri.conf.json:48-50` | `bundle.icon` = `.ico` only; Linux needs `32x32.png`, `128x128.png`, `128x128@2x.png`, `icon.png` |
| `tauri.conf.json:30,32` | Updater endpoint **and** minisign pubkey belong to upstream → a fork build cannot verify or sign updates |
| `tauri.conf.json:5` | `identifier: "BlurAutoClicker"` is not reverse-DNS; Tauri v2 warns on AppImage/deb |
| `tauri.conf.json:51-59` | `bundle.windows` block (WebView2 bootstrapper, NSIS) — inert on Linux; **delete** |
| `build.rs:35-74` | Crashpad hash pin hardcodes `crashpad_handler.exe`; not `cfg`-gated |
| `scripts/check.mjs:113-123` | `.exe` lock probe aborts the gate on Linux |
| `.ps1` ×3, `tauri.windows.signing.conf.json` | Pure Windows |
| `ci.yml:63,70` | `rust` job = `windows-latest` / `x86_64-pc-windows-msvc` only |
| `release.yml` | Windows + winget only |
| `dependabot.yml:5,14,23`, `dependabot-auto-merge.yml:12` | `target-branch: "dev"` — **branch does not exist in this fork** |
| `token-expiry-watchdog.yml` | winget PAT watchdog only |
| `vite.config.ts:6-13` | `watch.ignored` is a Windows EBUSY workaround (still useful on Linux; re-comment) |
| `CONTRIBUTING.md:52` | References `npm run check:all`, which does not exist in `package.json` |

---

## 3. Target architecture

```
┌────────────────────────── Frontend (unchanged, ~95%) ──────────────────────────┐
│  React 19 + TypeScript   ·   WebKitGTK 2.44 (WebKitGTK 4.1 API)               │
│  DOM KeyboardEvent.code  ── the natural bridge to X11 keycodes ──              │
└────────────────────────────────────────────────────────────────────────────────┘
                                     │  invoke / emit
┌────────────────────────── Rust backend (tauri 2.11) ───────────────────────────┐
│  Platform-neutral core (unchanged):                                           │
│    engine/cycle · engine/failsafe · engine/rng · engine/stats · settings       │
│    engine/worker (click loop) · app_state · app_events · error · diagnostics   │
│                                                                               │
│  NEW X11 layer  src-tauri/src/x11/*          ← called directly, no cfg facade │
│    ├── mod.rs           — single shared connection + re-exports               │
│    ├── input.rs         — XTest button/key/motion synthesis                  │
│    ├── keyboard_map.rs  — DOM code name ⇄ X11 keycode ⇄ keysym              │
│    ├── listen.rs        — XI2 raw events (primary) + XQueryKeymap (fallback) │
│    ├── pointer.rs       — QueryPointer, own-window test                      │
│    ├── screen.rs        — root geometry, RandR monitors                      │
│    ├── windows.rs       — _NET_WM_* pokes, input hints, topmost              │
│    └── procs.rs         — /proc walk, _NET_ACTIVE_WINDOW, icon-theme lookup   │
│                                                                               │
│  Rewritten in place: engine/mouse · engine/keyboard · engine/process ·        │
│                       hotkeys · overlay · click_point_picker ·                │
│                       custom_stop_zone_picker · icon · autostart · portable    │
│                                                                               │
│  DELETED outright: crash_handler.rs · window_lifecycle.rs · all .ico assets  │
└────────────────────────────────────────────────────────────────────────────────┘
                                     │  pure-Rust X11 protocol
                    ┌────────────────┴─────────────────┐
                    │  x11rb (xtst, xinput, randr,     │
                    │        xkb, record, shape)       │
                    │  no C shims, no libX11 linkage    │
                    └──────────────────────────────────┘
```

`x11/mod.rs` holds one lazily-initialised shared `x11rb::rust_connection::RustConnection` in a
`OnceLock<Option<…>>`. Two reasons for the `Option`: (a) a headless CI box with no `DISPLAY` yields
`None` instead of panicking, and (b) it keeps the connection out of unit tests, so all 132 tests
run without an X server (see [§8.4](#84-ci) CI gotcha).

### 3.1 Key architectural decisions

**A. `x11rb` over `enigo`/`rdev`/`libxdo`.**

| Candidate | Verdict |
|---|---|
| `x11rb` (+`xtst`, `xinput`, `randr`, `xkb`, `shape`) | **Chosen.** Pure Rust, no `libX11` link-time dep, requests can be batched without awaiting each reply (best latency for 500+ CPS), exposes RandR/XInput2/XKB directly, actively maintained, already present in `Cargo.lock:6521` as a transitive dep |
| `enigo` | Too coarse. Hides `XI2`, RandR, and the batch-request optimisation. Adds its own layout indirection on top of what we need to control precisely |
| `rdev` | Its Linux backend *grabs* devices, which conflicts with needing to observe without blocking other apps. Also no RandR/`_NET_*` access |
| `libxdo` / `libxdo-dev` | C dependency, fine, but we need RandR + XInput2 + `_NET_WM_*` anyway, so we would link X11 regardless |

**B. Replace Virtual-Key codes with DOM-code-shaped key tokens.**

This is the single most important refactor, and the news is good: **the frontend already speaks `KeyboardEvent.code`.** Defaults are `Digit1`…`Digit5`, `KeyA`…`KeyZ`, `Numpad0`…`Numpad9`, `F1`…`F24` (`hotkeys.ts:24-74`, `settingsSchema.ts:429-448`). Those strings are *hardware positions*, which is exactly what X11 keycodes are. So:

```rust
// engine/keycodes.rs  (NEW — platform-neutral)
pub enum KeyToken {
    /// DOM KeyboardEvent.code value: "KeyA", "Digit1", "Numpad7", "F11", …
    Code(String),
    /// Named non-code keys that have no DOM code: "CapsLock", "NumLock", "ScrollLock"
    Named(String),
    /// Mouse button: 1..=3, 8, 9 (X11 numbering, see §6.4)
    Mouse(u8),
}
```

`HotkeyBinding.main_vks: Vec<i32>` becomes `main_keys: Vec<KeyToken>`; `is_vk_down(i32)` becomes `is_key_down(&KeyToken)`. Every existing `parse_*_token` function in `hotkeys.rs:952-1068` keeps its string table verbatim — only the returned numeric constant changes from a `VK_*` to a `KeyToken`. **All 11 hotkey tests keep passing with minimal edits.**

`keyboard_map.rs` then resolves tokens → X11 keycode via the DOM-code ⇄ evdev-code table (a static ~130-entry match, the standard X11 default keymap mapping) and, for `Named`, via `XKeysymToKeycode`. Runtime layout quirks (extra keycodes on unusual layouts) fall out of querying the server's actual `GetKeyboardMapping`, which is *more* correct than the Windows `MapVirtualKeyW` path it replaces.

Side-specific modifiers (`leftctrl`, `rightctrl`, `altgr`, … — `hotkeys.rs:972-983`) map to the well-known X11 keycodes `Control_L(0x25)`, `Control_R(0x5e)`, `Shift_L/Shift_R(0x5a/0x5b)`, `Alt_L/Alt_R/Meta_L/Meta_R(0x38/0x64/0xeb/0xec)`. All supported natively.

**C. Two-tier input listening, mirroring the existing design.**

The current code already has a two-mode structure: `HOOKS_ACTIVE` (low-level hook) with fallback to `GetAsyncKeyState`. Preserve it exactly:

| Tier | Mechanism | Analogue | Latency |
|---|---|---|---|
| Primary | `XIQueryVersion(2.0)` + `XISelectEvents(root, XI_RawKeyPress \| XI_RawButtonPress \| XI_RawMotion)` | `WH_KEYBOARD_LL` / `WH_MOUSE_LL` | Event-driven |
| Fallback | `XQueryKeymap` every 4 ms into the existing `PHYSICAL_KEY_STATE: [AtomicBool; 256]` array, indexed by keycode instead of VK | `GetAsyncKeyState` | ≤4 ms |

**D. Self-injection filtering without `dwExtraInfo`.**

XTest-injected events are indistinguishable from real ones under XI2 raw events. The codebase already has the mitigation scaffolding — `suppress_hotkey_until_ms` and `suppress_hotkey_until_release` (`hotkeys.rs:65-69`, `ui_commands.rs:146-164`). Extend it: maintain a short FIFO of expected synthetic event timestamps; the XI2 handler consumes from it instead of counting events. Exact, no false negatives for real keys.

**E. Pickers move from global hooks to the overlay webview.**

X11 raw events **cannot be swallowed** the way `WH_KEYBOARD_LL` can (return non-zero). So the current design (install a global hook, `RightClick` → emit `click-point-picked`, swallow so the click doesn't reach the app underneath) has no X11 equivalent.

Instead: during pick mode, call `window.set_ignore_cursor_events(false)` on the overlay (it already spans the entire virtual screen) and let `overlay.html` handle `contextmenu`/`mousedown` in the DOM. This is strictly *better*:
- No hook install/teardown, no thread-message-loop lifecycle, no `PostThreadMessageW(WM_QUIT)`
- `Shift`/`Ctrl` come free from `MouseEvent.shiftKey` / `ctrlKey` (replacing `GetAsyncKeyState(VK_SHIFT/VK_CONTROL)`)
- Escape/Enter are free from `KeyboardEvent.key`
- Real event swallowing (the overlay consumes it) instead of simulated

Net effect: `click_point_picker.rs` (452 lines) and `custom_stop_zone_picker.rs` (472 lines) shrink to state machines + event emit/collect (~120 lines each). **~680 lines deleted.** The 10 + 7 tests get rewritten against DOM-agnostic `MouseHookDecision`/`KeyboardHookDecision` enums that stay structurally identical.

**F. Overlay via Tauri APIs, not raw X11 property pokes.**

`apply_win32_styles` (`overlay.rs:384-417`) becomes mostly *deletions* — see the mapping table in [§6.4](#64-overlay--window-semantics).

**G. `CLOCK_THREAD_CPUTIME_ID` deletes the CPU-frequency calibration hack.**

`calibrate_cycle_freq()` (`worker.rs:63-81`) exists only because `QueryThreadCycleTime` counts raw cycles. `clock_gettime(CLOCK_THREAD_CPUTIME_ID)` returns nanoseconds directly. That removes `windows_targets::link!`, `QueryThreadCycleTime`, `GetCurrentThread`, the 3 GHz fallback constant, and the 5 ms spin-loop calibration. **~40 lines deleted, one less fragile.**

---

## 4. Phase plan

Each phase is independently mergeable and keeps `npm run check` green.

### P0 — Unblock the toolchain *(no behaviour change)*

**Goal:** `cargo check` succeeds on Linux. Zero feature changes — this phase is purely deletions plus
build config, so it is easy to review and impossible to get functionally wrong.

1. Install prerequisites (see [§12](#12-prerequisites)):
   ```bash
   sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev libsoup-3.0-dev \
                    libx11-dev libxext-dev libxrandr-dev libxtst-dev libxi-dev \
                    libayatana-appindicator3-dev librsvg2-dev build-essential curl wget file
   ```
2. `Cargo.toml`: **delete** `windows-sys`, `windows-core`, `webview2-com`, `windows-targets`, `winreg`,
   `crashpad-rs`, the whole `[target.'cfg(windows)'.dependencies]` table, the whole `[features]`
   section, the `image` `ico` feature, and the tauri `image-ico` feature. Add
   `x11rb = { version = "0.13", features = ["xtst", "xinput", "randr", "xkb", "record", "shape"] }`
   and `libc = "0.2"`.
3. `build.rs`: **delete** the entire crashpad hash-pin block (all 74 lines — its only job was
   `crashpad_handler.exe`), leaving just `tauri_build::build()`.
4. `lib.rs`: delete `initialize_crashpad` / `shutdown_crashpad` calls; **delete** `crash_handler.rs`
   and `window_lifecycle.rs` outright along with `mod` declarations and call sites.
   `ui_commands.rs:383-396` `debug_trigger_crash` → `libc::abort()`.
5. `tauri.conf.json`: **delete** `beforeBundleCommand`; `bundle.targets` → `["deb", "appimage"]`;
   `bundle.icon` → a generated PNG set; reverse-DNS `identifier`; **delete** the `bundle.windows`
   block.
6. Generate PNG icons from the four `.ico` files (they carry PNG-compressed frames for ≥256px) via a
   one-off `image`-crate script; commit `32x32.png`, `128x128.png`, `128x128@2x.png`, `icon.png`,
   and `{activated,deactivated}-{dark,light}.png` at 256px; **delete** the `.ico` originals.
   This also removes the mixed-case `.ico` filename hazard (`Icon-activated-light.ico` vs
   `icon-deactivated-dark.ico`) that a case-sensitive filesystem would break on.
7. `scripts/check.mjs`: delete the `.exe` lock probe (lines 100-133) and keep only the cross-platform
   `pgrep -f 'blur-autoclicker|crashpad_handler'` path.
8. **Verification:** `cargo check --locked`, `cargo test`, `cargo clippy`, `npm run frontend:build`,
   `npm test`, `npm run lint`, `npm run format:check`, `npm run check` — all green on Linux.

> After P0 the repo compiles on Linux but does **not** yet click: the X11 implementations land in
> P1–P3. That intermediate state is a valid, mergeable commit ("Linux build compiles; Windows
> implementation removed") and is the right place to stop if the port needs to be handed off.

### P1 — X11 layer skeleton + screen geometry

**Goal:** `VirtualScreenRect` and monitor enumeration work; engine still compiles against Windows types.

1. `src-tauri/src/x11/mod.rs` (shared lazily-initialised connection + re-exports), `screen.rs`, `pointer.rs`.
2. `screen.rs`: root `GetGeometry` → `VirtualScreenRect { left: 0, top: 0, width, height }`.
   **Behavioural note:** the X11 virtual desktop is always origin-`(0,0)` spanning the full root
   window, unlike Windows' negative-origin virtual screen. All negative-coordinate code paths stay
   exercised because `failsafe.rs` takes rects as parameters — its 9 tests are unaffected.
3. `screen.rs`: `randr::get_monitors` for `current_monitor_rects`, with fallback to a single root rect.
   Subscribe to `RRScreenChangeNotify` so the monitor list refreshes on hotplug.
4. `pointer.rs`: `QueryPointer` → `current_cursor_position()`; own-window test via `QueryPointer.child`
   + `XQueryTree` walk against our own top-level XID from `raw_window_handle::XlibWindowHandle::xid()`.
5. Replace the Windows bodies in `engine/mouse.rs:72-148` with `x11::screen` / `x11::pointer` calls.
   `current_monitor_rects`' `#[cfg(not(windows))]` fallback twin at `mouse.rs:145-148` is deleted —
   the X11 version supersedes it.

**Verification:** manual — failsafe edges/corners trigger on the correct physical monitor edges in a
multi-monitor layout including one monitor left of / above the primary.

### P2 — Input synthesis (mouse + keyboard)

**Goal:** the clicker works end-to-end on X11.

1. `x11/input.rs`: `xtst::fake_input` wrappers — `motion(x, y)` (absolute root coords, no
   0..65535 normalisation needed — *simpler* than the Windows path), `button(btn, press)`,
   `key(keycode, press)`.
2. **Button-numbering fix (critical).** `engine/mouse.rs:263-269` currently uses **VK** numbering:
   `2 => RIGHT, 3 => MIDDLE`. X11 numbering is `1 => left, 2 => middle, 3 => right, 8/9 => back/forward`.
   Port with an explicit translation table and a comment, or mouse-right/middle clicks silently swap.
3. `x11/keyboard_map.rs`: `KeyToken` ⇄ keycode tables (§3.1B).
4. Refactor `engine/keyboard.rs`: delete `vk_to_scan` / `MapVirtualKeyW` / `KEYEVENTF_SCANCODE`
   entirely — X11 needs no scan codes, so `make_keyboard_input` collapses to one XTest call.
   Keep `should_hold_shift_for_case` with `Caps_Lock` state from the listener.
5. `engine/mouse.rs:207-220` `send_batch`: one `SendInput` with N pairs becomes N queued requests +
   a single `send_and_flush`. Add a comment noting X11 has no batch API.
6. `worker.rs`: `prctl(PR_SET_TIMERSLACK, 1)` guard replacing `NtSetTimerResolution`; swap
   `QueryThreadCycleTime` + `calibrate_cycle_freq` for `clock_gettime(CLOCK_THREAD_CPUTIME_ID)`.

**Verification:** CPS accuracy measured against `30s_500cps_Speed_Test.png` methodology; key injection
verified across a non-US layout (e.g. German `ö` → `Equal`+`AltGr`); CapsLock on/off with
`keyboardUppercase` both ways.

### P3 — Global hotkeys + failsafes

1. `x11/listen.rs`: XI2 raw-event listener thread + `XQueryKeymap` polling fallback, writing
   into the existing `PHYSICAL_KEY_STATE` array (index by keycode, not VK).
2. `hotkeys.rs`: `is_vk_down` → `is_key_down`; `mouse_ll_proc` / `keyboard_ll_proc` → XI2 handlers;
   drop `PeekMessageW`/`WaitMessage`/`UnhookWindowsHookEx`; keep the 2 s install delay and the
   `HOOKS_ACTIVE` fallback branch verbatim.
3. Self-injection FIFO (§3.1D).
4. `GetDoubleClickTime()` → XSettings `org.gnome.desktop.peripherals.mouse double-click-interval`,
   desktop-agnostic read of `_XSETTINGS_S1` on the root window, fallback 400 ms.
   `SPI_GETKEYBOARDDELAY/SPEED` → `XkbGetAutoRepeatDelay` / `XkbGetAutoRepeatRate`.
5. `engine/process.rs:364-389` `is_task_switcher_active`: `_NET_ACTIVE_WINDOW` + `XGetClassHint`,
   matching a Linux class list (`GNOME-shell`, `org_gnome_Shell_Shell`, `kwin`, `KWin`,
   `_GTK_WINDOW_TYPE_POPUP_MENU`, `xfwm4`, `MATE`/`caja`) **plus** Alt+Tab keycode state.

**Verification:** all 11 `hotkeys.rs` tests pass unchanged except for `KeyToken` signature edits;
manual test of hold mode, toggle mode, master switch, chords, left/right-specific modifiers,
AltGr, mouse-button hotkeys, and hotkeys not firing while the clicker runs.

### P4 — Overlay, pickers, windows, tray

1. `x11/windows.rs`: `_NET_WM_WINDOW_TYPE_UTILITY`, `XSetInputHints(input=False)`,
   `_NET_WM_STATE_ABOVE` re-assert, `XShapeCombineRectangles` empty rect for click-through
   (or just keep Tauri's `set_ignore_cursor_events`).
2. Delete `overlay.rs:371-462` (`get_hwnd`, `apply_win32_styles`, `sync_overlay_bounds`,
   `show_overlay_window`'s `SetWindowPos`) and replace with Tauri `set_position`/`set_size`/
   `set_always_on_top`/`hide`.
3. Delete `lib.rs:76-97` `apply_ws_ex_noactivate` and `lib.rs:151-168` `set_app_aumid`.
4. Pickers: rewrite against the overlay DOM (§3.1E). Add `contextmenu` / `mousedown` /
   `keydown` handlers plus a crosshair cursor to `overlay.html` in pick mode.
   Toggle `set_ignore_cursor_events(false)` for the duration of pick mode.
5. `icon.rs`: delete `force_window_icon_big`, `rgba_to_hicon`, `premultiply_rgba_to_bgra`
   (`icon.rs:333-440`) and swap the `include_bytes!` `.ico` assets for PNG. Keep the theme/accent
   decision logic — all 40 `icon.rs` tests cover that logic and must continue to pass untouched.
6. `lib.rs:32-70` `disable_browser_accelerator_keys`: delete, document the loss.
7. `lib.rs:184-201` panic-hook `MessageBoxW` → `tauri-plugin-dialog`, falling back to stderr.
8. Crash handling is already settled by P0 (crashpad deleted). Nothing left here except confirming
   the panic hook still writes `diagnostics::write_panic_report` with no crashpad initialisation.

**Verification:** overlay spans the full virtual desktop on a mixed-DPI multi-monitor setup,
auto-hides after 3 s, is click-through except in pick mode, click points and stop zones can be
picked and dragged, tray icon reflects the activated/deactivated + accent theme.

> **Caveat to document prominently:** `transparent(true)` requires a compositing window manager.
> The overlay will not render on a bare X server without one.

### P5 — Process list, autostart, text scale, diagnostics

1. `x11/procs.rs`:
   - Enumerate `/proc/[0-9]*/stat` → `(pid, ppid, comm)`; `readlink("/proc/{pid}/exe")` for the
     canonical binary name (avoids the 15-char `comm` truncation problem).
   - Window titles: `_NET_CLIENT_LIST_STACKING` on root → per-window `_NET_WM_PID` + `_NET_WM_NAME`.
   - Foreground process: `_NET_ACTIVE_WINDOW` → `_NET_WM_PID`.
   - **`normalize_process_name` (`engine/process.rs:190-197`) becomes a pass-through.** Delete the
     `.exe` append *and* the `.exe`-only filter at `engine/process.rs:302-304`.
   - Process icons (P5b, separately): freedesktop Icon Theme lookup — resolve `/proc/pid/exe` →
     match `Exec=` in `$XDG_DATA_DIRS/applications/*.desktop` → `Icon=` → resolve in
     `$HOME/.icons`/`/usr/share/pixmaps`/`$XDG_DATA_DIRS/icons` per the Icon Theme Spec →
     decode PNG natively, SVG via `resvg`. Cache by exe path (the existing `icon_cache()` at
     `engine/process.rs:48-50` already does this). Fall back to `None`.
2. `autostart.rs`: write/remove/read `$XDG_CONFIG_HOME/autostart/blur-autoclicker.desktop`
   (`Type=Application`, `Exec="<exe>" --autostart`, `Terminal=false`, `X-GNOME-Autostart-enabled=true`).
   `dirs` is already a dependency (`Cargo.toml:45`). `was_autostart_launch` keeps working — the
   `--autostart` flag mechanism is platform-neutral.
3. `ui_commands.rs:25-42` `get_text_scale_factor`: read `org.gnome.desktop.desktop.interface
   text-scaling-factor` via XSettings (desktop-agnostic) with a `1.0` fallback. Semantics match
   Windows' `TextScaleFactor` (percent ÷ 100), so `App.tsx:710-712` (webview zoom) and
   `App.tsx:902-903` (root font size) transfer unchanged.
4. `portable.rs`: delete `ensure_webview2`, `webview2_installed`, `webview2_bootstrapper_started`,
   `bootstrapper_is_trusted`, `WEBVIEW2_CLIENT_KEYS`, `BOOTSTRAPPER_FILE`. Keep the marker
   mechanism and `portable.txt` layout **verbatim** — already platform-neutral.
5. `lib.rs:488-518`: delete the WebView2-bootstrapper branch of the window-creation failure path;
   WebKitGTK is a system package, so the failure is simply a hard error. The `debug_trigger_crash`
   null-deref (`ui_commands.rs:383-396`, now `libc::abort()` in P0) already covers crash simulation.

**Verification:** process list populates with real window titles on GNOME + KDE; whitelist/blacklist
gating works; autostart adds/removes the `.desktop` entry; diagnostics bundle exports cleanly.

### P6 — Frontend polish

See [§9](#9-frontend-changes). Small, mostly copy + asset changes.

### P7 — Packaging, CI, release, docs

See [§8](#8-build-ci--release) and [§11](#11-gpl-30-compliance).

---

## 5. Effort & risk

| Phase | Scope | Est. | Risk |
|---|---|---|---|
| P0 | Toolchain unblock, no behaviour change | 0.5–1 d | Low |
| P1 | Screen + pointer geometry | 1–2 d | Low |
| P2 | Input synthesis + keycode mapping | 3–4 d | **High** — layout edge cases, timing at high CPS |
| P3 | Hotkeys + failsafes | 3–4 d | **High** — XI2 quirks, Wayland/XWayland variance, WM-grab conflicts |
| P4 | Overlay + pickers + icons | 2–3 d | Medium — mixed-DPI, compositor requirements |
| P5 | Processes + autostart + scale | 2–3 d | Medium — XDG variability, icon-theme edge cases |
| P5b | Process icons via icon-theme lookup | 1–2 d | Medium — optional, deferrable |
| P6 | Frontend polish | 0.5–1 d | Low |
| P7 | Packaging + CI + docs | 1.5–2.5 d | Low |
| **Total** | | **~14–20 d** | |

Estimates assume the **Linux-only** decision ([§0](#0-decision-record-linux-only)) — no `cfg` split,
no dual-CI matrix, no cross-compilation. A cross-platform port would add roughly 20% to P1–P5.

### Highest-risk items, and the fallback for each

| Risk | Fallback |
|---|---|
| XI2 raw events unavailable / behave differently under some WMs | Already designed in: `XQueryKeymap` polling tier, ≤4 ms latency |
| Mixed-DPI multi-monitor: `overlay.html:336,407,470` assumes one uniform `devicePixelRatio`; `App.tsx:102` assumes one `monitor.scaleFactor` | Per-monitor `randr` geometry is already in physical px; convert per-monitor instead of globally. This is a **pre-existing** Windows bug too, not a port regression |
| No compositing WM ⇒ transparent overlay fails | Document; detect and warn at startup |
| High-CPS timing jitter | `PR_SET_TIMERSLACK` + busy-wait final ~200 µs before each click deadline |
| Hotkey conflicts with the WM or other apps | XI2 observes without grabbing, so no conflict; but a WM may consume the key first — document that WM-reserved combos (e.g. some Super combos) are unreliable |
| WebKitGTK vs WebView2 CSS/behaviour divergence | Small; mostly the `-webkit-app-region` removal (§9.4) |
| Updater signing | Needs a fresh minisign keypair for the fork (§8.4) |

---

## 6. Detailed subsystem designs

### 6.1 Virtual screen & monitors

```rust
// x11/screen.rs
pub fn current_virtual_screen_rect() -> Option<VirtualScreenRect> {
    let (_, root, ..) = conn().setup()?.roots().next()?;   // xproto::Setup roots
    let g = xproto::get_geometry(root)?.reply().ok()?;
    Some(VirtualScreenRect::new(0, 0, g.width(), g.height()))  // X11 root is always (0,0)
}

pub fn current_monitor_rects() -> Option<Vec<VirtualScreenRect>> {
    // randr::get_monitors → monitor_infos(), each with x/y/width/height in root coords
    // sort_by_key(|m| (m.top, m.left))  ← preserves the existing sort in mouse.rs:141
    // fallback: vec![current_virtual_screen_rect()?]
}
```

### 6.2 Global hotkey listening

```rust
// x11/listen.rs
pub enum ListenTier { Xi2Raw, Polling }

pub fn spawn_input_listener(
    on_key: impl Fn(KeyToken, bool) + Send + 'static,   // token, is_press
    on_button: impl Fn(u8, bool) + Send + 'static,     // X11 button number
) -> ListenTier {
    // 1. XIQueryVersion(2.0)
    // 2. XISelectEvents(root, XI_RawKeyPress|XI_RawButtonPress|XI_RawMotion)
    //    XI_RawEvent { detail: keycode, flags, mods }
    // 3. On failure → Polling: XQueryKeymap every POLL_INTERVAL (4 ms),
    //    diffing against the previous bitmap into the same callbacks.
}
```

Both tiers write into the existing `PHYSICAL_KEY_STATE: &'static [AtomicBool; 256]`
(`hotkeys.rs:354-367`) — the array is already sized 256 and X11 keycodes are ≤255, so it slots in
with **only** the index semantics changing from VK to keycode.

X11 keycode → `KeyToken` is the reverse of §3.1B's table.

### 6.3 Input synthesis

```rust
// x11/input.rs — batch requests, flush once (no per-request await)
pub fn send_batch(down: u32, up: u32, n: usize) {
    let c = conn();
    let mut reqs = Vec::with_capacity(n * 2);
    for _ in 0..n {
        reqs.push(xtst::fake_input(c, xtst::XTestFakeInputType::Button, button(down), 0, root, 0, 0));
        reqs.push(xtst::fake_input(c, xtst::XTestFakeInputType::Button, button(up),   0, root, 0, 0));
    }
    for r in reqs { c.send_request(r).unwrap(); }
    c.flush().unwrap();   // ← single round-trip; the key latency win over per-event calls
}
```

### 6.4 Overlay & window semantics

| Win32 (`overlay.rs:384-417`) | X11 equivalent |
|---|---|
| `WS_EX_LAYERED` | None needed — Tauri `transparent(true)` requests an ARGB visual |
| `WS_EX_TRANSPARENT` | **Already handled** by `set_ignore_cursor_events(true)` (tao → XShape empty input region) |
| `WS_EX_TOOLWINDOW`, clear `WS_EX_APPWINDOW` | **Already handled** by `skip_taskbar(true)` (`_NET_WM_STATE_SKIP_TASKBAR`) |
| `WS_EX_NOACTIVATE` | `XSetInputHints` `input = False` + `_NET_WM_WINDOW_TYPE_UTILITY`. **Weaker guarantee** — a WM may ignore the hint. Fallback: never map with input focus; Tauri already builds the overlay `focusable(false)` |
| `DwmSetWindowAttribute(DWMNCRP_DISABLED)` | **No analogue, and none needed** — with `decorations(false)` X11 draws no server-side border at all. This Windows call exists only to stop DWM repainting a border over CSS-drawn chrome |
| `SetWindowPos(...)` bounds sync | `window.set_position(PhysicalPosition)` + `window.set_size(PhysicalSize)` |
| `SetWindowPos(HWND_TOPMOST, …)` | `window.set_always_on_top(true)` (`_NET_WM_STATE_ABOVE`) |
| `ShowWindow(hwnd, 0)` | `window.hide()` — already the non-Windows branch at `overlay.rs:368` |

### 6.5 Process enumeration

```
Windows                          Linux
──────────────────────────────────────────────────────────────────────────────
CreateToolhelp32Snapshot         /proc/[0-9]+/stat   (pid, ppid, comm)
Process32FirstW/NextW            ── no equivalent needed ──
QueryFullProcessImageNameW       readlink /proc/{pid}/exe
ends_with(".exe") filter         ── DELETED ──
normalize_process_name           ── DELETED (pass-through) ──
EnumWindows + GetWindowTextW     _NET_CLIENT_LIST_STACKING → _NET_WM_PID + _NET_WM_NAME
GetForegroundWindow              _NET_ACTIVE_WINDOW (property on root)
GetClassNameW                    XGetClassHint / _NET_WM_NAME
ExtractIconExW + GDI             freedesktop Icon Theme lookup (P5b)
```

`is_process_running("RTSS.exe")` (`lib.rs:100`) is **deleted**, along with its three call sites
(`lib.rs:469-472`, `526-532`) and the `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS` block it guards.
RivaTuner Statistics Server is a Windows overlay; there is no Linux equivalent to detect, so keeping
the check would be a permanently-dead code path.

### 6.6 Pickers (click-point / stop-zone)

Current lifecycle: install 2 thread-local hooks → `GetMessageW` loop → `PostThreadMessageW(WM_QUIT)`
to tear down (`click_point_picker.rs:115-175`).

New lifecycle:
```
start_click_point_pick(app)
  ├─ set_ignore_cursor_events(false)      ← overlay becomes clickable
  ├─ emit "click-pick-mode" { active: true }
  └─ overlay.html: crosshair cursor; contextmenu/mousedown handler
       → emit "click-point-picked" { x, y, continuePicking: e.shiftKey }
       → Ctrl+RightClick → emit "click-point-delete-requested"
       → Escape → invoke cancel_click_point_pick
cancel_click_point_pick(app)
  ├─ set_ignore_cursor_events(true)
  └─ emit "click-pick-mode" { active: false }
```

Deletes: hook install/uninstall, `HHOOK`, `GetCurrentThreadId`, `GetLastError`, `mpsc` ready-channel,
`WM_QUIT` posting, `GetAsyncKeyState(VK_SHIFT/VK_CONTROL)`, `VK_ESCAPE` classification.

Preserves: `MouseHookDecision` / `KeyboardHookDecision` enums and all their tests (re-pointed at DOM
`MouseEvent.button` / `KeyboardEvent.key` values instead of `WM_*`/`VK_*`).

---

## 7. Files touched

### New (12)
```
src-tauri/src/x11/mod.rs
src-tauri/src/x11/screen.rs
src-tauri/src/x11/pointer.rs
src-tauri/src/x11/input.rs
src-tauri/src/x11/keyboard_map.rs
src-tauri/src/x11/listen.rs
src-tauri/src/x11/windows.rs
src-tauri/src/x11/procs.rs
src-tauri/src/engine/keycodes.rs        ← KeyToken, platform-neutral
src-tauri/icons/*.png                   ← 9 new assets (see P0.6)
BUILDING.linux.md
docs/linux-port-plan.md                 ← this file
```

Note there is **no `mod.rs` facade** and no `cfg` dispatch — `x11/` is the only backend, so
consumers import it directly.

### Heavily rewritten (7)
`src-tauri/src/engine/mouse.rs` · `engine/keyboard.rs` · `engine/process.rs` · `hotkeys.rs` ·
`overlay.rs` · `click_point_picker.rs` · `custom_stop_zone_picker.rs`

### Deleted (11)
```
scripts/package-portable.ps1
scripts/prepare-windows-redist.ps1
scripts/sign-windows.ps1
src-tauri/tauri.windows.signing.conf.json
docs/windows-release-trust.md
.github/wingetcreate.sha256
.github/workflows/token-expiry-watchdog.yml
.github/workflows/release.yml              ← Windows/winget pipeline; replaced by P7
src-tauri/src/crash_handler.rs            ← crashpad-only, no Linux handler exists
src-tauri/src/window_lifecycle.rs         ← WebView2 process trimming only
src-tauri/icons/*.ico                     ← after PNG extraction
```

### Modified (~18)
`Cargo.toml` · `build.rs` · `tauri.conf.json` · `capabilities/default.json` ·
`engine/mod.rs` · `engine/worker.rs` · `ui_commands.rs` · `lib.rs` · `icon.rs` · `autostart.rs` ·
`portable.rs` · `error.rs` · `overlay.html` · `index.html` ·
`scripts/check.mjs` · `package.json` · `vite.config.ts` · `lefthook.yml` · `.github/workflows/ci.yml` ·
all docs · `.github/dependabot.yml` · issue templates

### Deliberately unchanged
`engine/cycle.rs` · `engine/rng.rs` · `engine/failsafe.rs` · `engine/stats.rs` · `settings/mod.rs` ·
`app_state.rs` · `app_events.rs` · `diagnostics.rs` · `updates/*` · `LICENSE` · every `.css` except
the two `-webkit-app-region` declarations · ~90% of the frontend

---

## 8. Build, CI & release

### 8.1 `Cargo.toml`

Flat, no target tables, no feature table:

```toml
[dependencies]
# all six Windows crates deleted:
#   windows-sys, windows-core, webview2-com, windows-targets, winreg, crashpad-rs
x11rb = { version = "0.13", features = ["xtst","xinput","randr","xkb","record","shape"] }
libc  = "0.2"
# ... unchanged platform-neutral deps (dirs, image/png, walkdir, zip, open, chrono,
#     base64, reqwest, tokio, tauri, tauri-plugin-*)

# [features] section deleted entirely
```

Expected effect on `cargo tree`: the `windows-sys`/`windows-targets` subtree (≈300 transitive
`windows_*` crates) disappears, so `cargo build` gets noticeably faster.

### 8.2 `tauri.conf.json`

- **Remove** `build.beforeBundleCommand`
- `bundle.targets: ["deb", "appimage"]` (add `"rpm"` later if wanted)
- `bundle.icon`: PNG set + `.desktop`
- `identifier`: `io.github.aramczdev.blurautoclicker` (reverse-DNS, required for AppImage/deb)
- **Remove** the `bundle.windows` block
- `bundle.deb.depends`: `["libwebkit2gtk-4.1-0", "libgtk-3-0", "libayatana-appindicator3-1"]`
- `bundle.linux.deb.desktopTemplate`: hide from tray-menu noise
- `createUpdaterArtifacts: true` (AppImage supports it)
- `publisher`/`copyright`: `GPL-3.0` + original author credit (see §11)
- `homepage`: point at the fork, keep an upstream credit line in `longDescription`

### 8.3 `scripts/check.mjs`

Delete the `.exe` lock probe (lines 100-133) and replace the "is it running?" guard with the
already-cross-platform `pgrep -f 'blur-autoclicker|crashpad_handler'` path only. Also worth aligning
while in there: `check.mjs` omits `--locked` on `cargo test`/`clippy`, omits `cargo-audit` (CI has it),
and its ordering differs from `.agents/AGENTS.md:2-10` — which itself omits `npm test`.

### 8.4 CI

```yaml
# ci.yml — single Linux job; the Windows matrix entry is deleted
rust:
  runs-on: ubuntu-latest
  env:
    apt: >-
      libwebkit2gtk-4.1-dev libgtk-3-dev libsoup-3.0-dev libx11-dev libxext-dev
      libxrandr-dev libxtst-dev libxi-dev libayatana-appindicator3-dev librsvg2-dev
  steps:
    - uses: actions/checkout@<pinned-sha>
    - uses: dtolnay/rust-toolchain@<pinned-sha>
      with: { targets: "x86_64-unknown-linux-gnu" }
    - run: sudo apt-get update && sudo apt-get install -y ${{ env.apt }}
    - run: sudo apt-get install -y pkg-config libssl-dev
    - uses: Swatinem/rust-cache@<pinned-sha>
      with: { workspaces: "./src-tauri -> target" }
    - run: cargo fmt --check       # working-directory: src-tauri
    - run: cargo check --locked
    - run: cargo test  --locked    # needs a virtual X server on Linux
    - run: cargo clippy --locked
```

**CI gotcha:** `cargo test` on Linux must not require a real display. The 132 tests are all pure
logic, so this holds *as long as* no test touches the X connection. Enforce it: keep
`platform::*` constructors lazy (`OnceLock<Option<Connection>>`) so unit tests never open a socket.
Optionally add `xvfb-run --auto-servernum cargo test --locked` as a belt-and-braces measure.

### 8.5 Release

- **Delete** `.github/workflows/release.yml` (Windows/winget-only) and add
  `release-linux.yml`: tag `v*` → `ubuntu-latest` → AppImage + deb → `gh release create/upload`.
- **The updater needs a new keypair.** `tauri.conf.json:32` holds the *upstream* minisign public
  key; a fork has no matching private key, so updates cannot be signed or verified as-is.
  Generate `tauri signer generate -w ~/.tauri/blur-autoclicker.key` and ship a Linux-side overlay
  config (`tauri.linux.conf.json`) with the new pubkey and a Linux-specific `latest.json` endpoint,
  passed via `--config`.
- Set `dependabot.yml` + `dependabot-auto-merge.yml` `target-branch` / base-ref from `dev` → `main`
  (`dev` does not exist in this fork, so both are currently dead).
- Delete `token-expiry-watchdog.yml` (winget PAT only) — or repurpose it for the Tauri signing key.
- Installers: `deb` for Debian/Ubuntu/Mint, `AppImage` as the portable option. AppImage also makes the
  portable mode story easier — `portable.txt` next to the AppImage works the same way.
- Keep `docs/windows-release-trust.md` deleted; add `docs/signing-keys.md` describing the fork's own
  minisign keypair handling instead.

### 8.6 Docs

| File | Action |
|---|---|
| `README.md` | Update badges + install instructions to Linux; keep the upstream credit and the "this is a fork / modified version" notice (§11); keep `## License` → GPL-3.0 |
| `BUILDING.md` | Rewrite for Linux (deps, `cargo tauri build`, X11/Wayland note, packaging); archive the Windows version |
| `CONTRIBUTING.md` | Update the "Windows-first" line; fix the nonexistent `npm run check:all`; `dev` → `main` |
| `docs/windows-release-trust.md` | Delete (SmartScreen/Authenticode are irrelevant) |
| `docs/linux-port-plan.md` | This file |
| `.github/ISSUE_TEMPLATE/bug_report.yml:67` | "Windows version" → "Linux distro + desktop environment + WM" |
| `.github/ISSUE_TEMPLATE/config.yml:6-8` | Retarget the SmartScreen link |
| `CHANGELOG.md` | Add a `## Linux port` section; **do not rewrite history** |

---

## 9. Frontend changes

Nine items, none structural. The frontend has **zero** OS-detection code today, so the port needs
neither OS sniffing nor conditional UI trees — just literal fixes.

### 9.1 Process identity
- `ProcessListSection.tsx:8-13` — `name` no longer implies `.exe`; update the doc comment
- `.settings-proc-exe` (`SettingsPanel.css:1225`) and `.adv-proc-exe`
  (`zones/sections/ProcessListSection.tsx:216`) — rename the class (optional; purely cosmetic CSS)
- `settingsSchema.ts:990-1010` `sanitizeProcessListEntries` — on Linux, **drop** persisted
  entries ending in `.exe` so stale Windows lists don't silently never match
- Copy: no `.exe` appears in any user-facing string, so no copy changes needed

### 9.2 Hotkeys
- `hotkeys.ts:50-61` `SHIFTED_SYMBOL_BASE_MAP` — the hardcoded **US** shifted-symbol table. Keep it
  as the fallback, but the `navigator.keyboard.getLayoutMap()` path (`hotkeys.ts:661-675`) is
  **WebKitGTK-supported**, so non-US layouts are handled correctly on Linux once the primary path works.
- `hotkeys.ts:24-47` — `MetaLeft`/`OSLeft` → `super`. WebKitGTK emits `MetaLeft` for the Super key;
  works as-is. **No change needed.**
- `hotkeys.ts:63-74` numpad namespace — matches X11 numpad keycodes directly. No change.
- `settingsSchema.ts:429-448` (`Digit1`…`Digit5`) and `:736,:780,:824` (`ctrl+y`) — physically
  correct on X11 but assume US layout. Leave the values; they're user-editable. Consider changing
  the shipped default to something layout-neutral (`F1`…`F5`) as a follow-up — flagged, not blocking.

### 9.3 OS-specific commands
- `get_text_scale_factor` → now backed by XSettings/GSettings. No frontend change needed.
- `get_autostart_enabled` / `set_autostart_enabled` → now `.desktop` files. No frontend change.
- `get_portable_info` → returns the same shape. No frontend change.

### 9.4 Titlebar drag region — **real breakage**
`TitleBar.tsx:252-256` sets `WebkitAppRegion: "drag"` inline and `data-tauri-drag-region`;
`TitleBar.tsx:268,319,529` and `TitleBar.css:161,312` set `no-drag`. **`-webkit-app-region` is not
implemented in WebKitGTK**, so the CSS side is dead weight on Linux.

Fix: **keep** `data-tauri-drag-region` (it is handled by Tauri's injected script calling
`startDragging()`, which is platform-independent) and **remove** the `WebkitAppRegion` inline styles
and the two `no-drag` declarations. Verify dragging actually works — this is the single most likely
user-visible regression, so it goes in P6 with a manual check.

### 9.5 Paths
- `store.ts:39` — `` `${info.dataDir}/settings.json` `` → use `joinPath` from `@tauri-apps/api/path`
- `backgroundMedia.ts:245-248` — keep the drive-letter regex (harmless) and confirm POSIX paths work

### 9.6 Copy & assets
- `AppearanceSection.tsx:348-349` — `"Taskbar Icon"` → `"Panel & Tray Icon"`; description
  "Control how the taskbar and tray icon looks" → "Control how the panel and tray icon looks"
- `StartupSection.tsx:32-34`, `BehaviorSection.tsx:240-242` — `"Minimize to Tray"` /
  "instead of the taskbar" → "instead of the panel" / "dock"
- `BehaviorSection.tsx:142` `"Stop on Alt+Tab"` → `"Stop on window switch"` (Linux WMs bind
  Alt+Tab differently; GNOME uses Super+Tab by default)
- `StatusBar.tsx:35`, `TitleBar.tsx:75` — `"Blocked by Alt+Tab"` → `"Blocked by window switch"`
- `overlay.html:96` — add Linux fallbacks: `"RobotoFlex", "Cantarell", "Inter", system-ui, sans-serif`
- `index.css:9` — same font stack for the main window (currently no `Segoe UI`, so main and overlay
  already disagree; fix both)
- `index.html:5` — add a PNG/SVG favicon alongside the `.ico`
- `status` reason strings in `StatusBar.tsx:27-34` (8 edge/corner failsafes) — **keep unchanged**;
  `failsafe.rs` produces identical strings on Linux

### 9.7 `overlay.html` — new pick-mode handlers
Add `contextmenu` / `mousedown` / `keydown` handlers and a crosshair cursor for pick mode (§6.6).
Also note the pre-existing uniform-`devicePixelRatio` assumption at `overlay.html:336-340,407,470-473`.

### 9.8 Not changed
`settingsSchema.ts` field names (incl. `taskSwitcherStopEnabled` — internal), the whole
`invoke` command surface (29 commands, all preserved), all 5 vitest suites, `store.ts` shape,
`backgroundMedia.ts` / `accentTheme.ts` / `cadence.ts` / `numberInput.ts` / `changelog.ts` /
`diagnostics.ts` / `keyboardKeyCase.ts` — all platform-neutral.

---

## 10. Behavioural differences to document

### 10.1 Wayland

X11-first, as specified. Under a Wayland session the app runs on **XWayland**, which means:
- The global listener sees X11 clients only — **native Wayland windows are invisible** to it.
- `XTest` synthesis reaches XWayland clients only; native Wayland clients (including many games)
  will not receive synthetic input.
- Pure-Wayland compositors may not even expose an XWayland display.

Plan: at startup, detect `WAYLAND_DISPLAY` and surface a UI warning that global hotkeys, failsafes,
and input injection require an X11 session. Gate the check on `XDG_SESSION_TYPE`. Full Wayland
support is a separate project (`libinput`/`wl_virtual_pointer`/`xdg-foreign` + `evdev` `EVIOCGRAB`
for the hook tier) and is out of scope.

### 10.2 Other documented differences

| Behaviour | Windows | Linux/X11 |
|---|---|---|
| Virtual desktop origin | Can be negative (monitor left of primary) | Always `(0,0)` spanning the full root |
| Input injection ceiling | ~500 CPS (README FAQ cites a Windows limit) | Not yet measured — **re-measure and update the FAQ** |
| `WS_EX_NOACTIVATE` | Guaranteed by the OS | Best-effort `XSetInputHints`; WM-dependent |
| Browser accelerator keys (F6/Ctrl+F) | Disabled via `ICoreWebView2Settings3` | Not disabled — WebKitGTK has no equivalent |
| Transparent overlay | Works without a compositor | **Requires a compositing WM** |
| Crash reports | Crashpad minidumps via `crashpad_handler.exe` | **Removed** — no prebuilt Linux crashpad handler. Panic report file + log only, written by the existing `lib.rs` panic hook |
| Webview data dir | `EBWebView-*` | WebKitGTK `WebKitWebProcess` uses the same code path via Tauri |
| Case sensitivity | Case-insensitive FS | **Case-sensitive** — the mixed-case `.ico` names must be fixed |
| Text scale | Registry `TextScaleFactor` | XSettings / GSettings (same percent semantics) |

---

## 11. GPL-3.0 compliance

The licence is **GPL-3.0 and stays GPL-3.0.** Concretely:

1. **`LICENSE` is not modified.** It remains the verbatim GPL-3.0 text (674 lines, FSF header
   `LICENSE:1-2`, `Copyright (C) 2007 Free Software Foundation, Inc.` at `LICENSE:4`). The port
   adds no project copyright line to it — matching the upstream file exactly.
2. **`Cargo.toml:6`** stays `license = "GPL-3.0"` (SPSP-correct identifier; leave it — don't
   downgrade to the informal `GPLv3` used in `README.md:96` / `tauri.conf.json:43`).
3. **GPL-3.0 §5(b) — convey modified versions.** A distributed port *must* be marked as changed.
   Add to `README.md`:
   > This is a **modified version** of [Blur009/Blur-AutoClicker](https://github.com/Blur009/Blur-AutoClicker).
   > Original work © Blur009, licensed under GPL-3.0. Linux/X11 port modifications © 2026 the
   > Blur-AutoClicker-Linux contributors, also licensed under GPL-3.0. No affiliation or endorsement
   > is implied by Blur009.

   The repo already carries the first line at `README.md:14` ("THIS IS NOT THE ORIGINAL PROJECT THIS
   IS JUST A FORK THAT MAKES IT RUN ON LINUX") — upgrade it to the formal §5(b) notice.
4. **`tauri.conf.json`**:
   - `publisher`: `AramCZdev`
   - `copyright`: `"Copyright (c) Blur009 (original) and Blur-AutoClicker-Linux contributors (Linux port)"`
   - `longDescription`: keep the GPL-3.0 mention
   - `homepage`: the fork; add the upstream URL to `longDescription`
5. **Source headers.** GPL-3.0 does not require per-file headers (unlike Apache-2.0), so don't add
   them to the full test suite' worth of files. One `NOTICE` file at the repo root is sufficient and is
   the idiomatic choice.
6. **`CHANGELOG.md` history is preserved verbatim.** Add a new top section for the port; never rewrite
   upstream entries.
7. **Dependency licences.** The new deps must be GPL-3.0-compatible (all are, at runtime, as
   dynamically-linked libraries):
   `x11rb` (MIT/Apache-2.0), `libc` (MIT/Apache-2.0), `resvg` (MPL-2.0), plus the existing
   `image` (MIT/Apache-2.0), `rusb`/`rdev` if ever added (MIT). Runtime linking from a GPL-3.0
   program is fine. **Avoid adding any AGPL dependency** — it would impose stronger obligations on
   the whole app.
8. **Bundled WebKitGTK is a system library, not a distributed binary.** The AppImage will bundle
   WebKitGTK via linuxdeploy; GPL-3.0 §5(d) then requires providing the **complete corresponding
   source** for bundled GPL-licensed components, and §6 requires the AppImage to carry the GPL text.
   Practical mitigation: build the AppImage with `-u` (skip bundling WebKitGTK) so the app uses the
   distro package, or publish a `sources.txt` / `LICENSE.third-party` in the release listing every
   bundled component with its source URL. **Flagged as a release-blocking legal item for P7.**
9. **No trademark issue** in keeping "BlurAutoClicker" as the product name — but §11.3's disclaimer
   must be present, and the upstream author should be credited in spirit (§6 of the GPL asks
   contributors not to misrepresent origin).

---

## 12. Prerequisites

### 12.1 Debian / Ubuntu / Linux Mint (verified available on Mint 22.3 / Ubuntu 24.04)

```bash
sudo apt update
sudo apt install -y \
  build-essential curl wget file pkg-config libssl-dev \
  libwebkit2gtk-4.1-dev libgtk-3-dev libsoup-3.0-dev \
  libx11-dev libxext-dev libxrandr-dev libxtst-dev libxi-dev libxcursor-dev \
  libayatana-appindicator3-dev librsvg2-dev
```

### 12.2 Rust / Node

```bash
rustup default stable            # 1.97.1 satisfies rust-version = "1.97.1"
node --version                   # >= 20; CI uses 24
```

### 12.3 Runtime (for end users)

`libwebkit2gtk-4.1-0`, `libgtk-3-0`, `libayatana-appindicator3-1` — declared automatically in the
`.deb` `depends`. AppImage bundles them.

### 12.4 Compositing WM (for the overlay)

Required for `transparent(true)`: GNOME/Mutter, KDE Plasma/KWin, XFCE/xfwm4, MATE/Marco all qualify.

---

## 13. Immediate next steps

1. ~~Decide whether the Linux build is Linux-only or cross-platform.~~ **Decided: Linux-only**
   (see [§0](#0-decision-record-linux-only)). Windows code is deleted, not gated.
2. Get user sign-off on the remaining open items — [§3.1](#31-key-architectural-decisions) D & E
   (self-injection filtering; pickers moving to the webview) and [§10](#10-behavioural-differences-to-document)
   (Wayland scope, no crash dumps after the crashpad removal).
3. Install the §12.1 prerequisites.
4. Execute **P0** — it is mechanical, low-risk, and converts the repo from "does not compile on
   Linux" to "compiles and passes all 132 tests", giving a green `npm run check` baseline to protect
   throughout P1–P7.

---

## Appendix A — Windows call-site quick index

```
engine/mod.rs:108-115           #[link(name="ntdll")] NtSetTimerResolution
engine/mouse.rs:7-11,73-78,86-89,97-143,162,168-182,185-199,204,214,263-269   SendInput/GetCursorPos/GetSystemMetrics/EnumDisplayMonitors
engine/keyboard.rs:4-8,17,22-43,50,57-68,100-114   SendInput/MapVirtualKeyW/GetKeyState
engine/process.rs:8-19,31-37,55-67,75-160,190-197,205-225,231-261,270-281,292-319,364-389,393-413,402-403   ToolHelp/GDI/ExtractIconExW/EnumWindows
engine/worker.rs:16-19,41-46,83-105,290   GetDoubleClickTime/SPI_*/QueryThreadCycleTime/NtSetTimerResolution
hotkeys.rs:17-26,127-352,354-367,369-446,448-651,654-681,683-713,757-788,798-810,944-946,952-1068   hooks/VK table/GetAsyncKeyState/WindowFromPoint
click_point_picker.rs:5-14,73-91,93-175,359-451    hooks + message loop + VK_*
custom_stop_zone_picker.rs:5-11,150-240,388-389    hooks + message loop + VK_*
overlay.rs:18-25,66-70,94-101,178-184,203-207,250-254,360-462   Win32 styles/SetWindowPos/ShowWindow/Dwm
icon.rs:17-30,295-310,333-440    .ico assets/WM_SETICON/CreateIconIndirect/BGRA premultiply
lib.rs:32-70,76-97,151-168,184-201,256,284,409-416,485-486,494-517,520-524   WebView2 COM/WS_EX_NOACTIVATE/AUMID/MessageBoxW
portable.rs:8-11,31-32,90-208   registry/WebView2 bootstrapper/WinVerifyTrust/MessageBoxW
autostart.rs:1-40   winreg Run key
ui_commands.rs:25-42   winreg TextScaleFactor
window_lifecycle.rs:3-81   ToolHelp + SetProcessWorkingSetSize on msedgewebview2.exe
crash_handler.rs:56-83   crashpad_handler.exe paths
build.rs:35-74   crashpad_handler.exe hash pin
```