//! Global keyboard and mouse input.
//! - Keyboard uses a low-level hook (`WH_KEYBOARD_LL`) so presses work across normal and elevated
//!   windows alike without taking focus.
//! - Mouse uses Raw Input (`RIDEV_INPUTSINK`) on a message-only window instead of `WH_MOUSE_LL`,
//!   so 1000–8000 Hz gaming mice never suffer synchronous cross-process hook overhead on move.
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::SyncSender;
use std::sync::Arc;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Button {
    Left,
    Right,
    Other,
}

pub enum Input {
    /// `id` names the physical key (scan code, plus 0x100 for E0 keys) so autorepeat can be
    /// told apart from a new press; `vk` is used to reconcile stuck keys after Win+L / UAC;
    /// `code` is `None` for keys the scene has no name for.
    Key { id: u32, vk: u32, code: Option<&'static str>, down: bool },
    Button { button: Button, down: bool },
    /// Wheel notches, positive away from the user.
    Wheel(f64),
    /// Edge-triggered wakeup when the mouse moves, so the pump can sleep at 0 Hz while idle.
    PointerWake,
}

#[cfg(windows)]
mod imp {
    use std::ffi::c_void;
    use std::mem;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::mpsc::{self, SyncSender};
    use std::sync::{Arc, OnceLock};
    use std::thread;

    use windows::core::{w, PCWSTR};
    use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::System::Threading::GetCurrentThreadId;
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        GetAsyncKeyState, MapVirtualKeyW, MAPVK_VK_TO_VSC_EX,
    };
    use windows::Win32::UI::Input::{
        GetRawInputData, RegisterRawInputDevices, HRAWINPUT, RAWINPUT, RAWINPUTDEVICE,
        RAWINPUTHEADER, RIDEV_INPUTSINK, RID_INPUT, RIM_TYPEMOUSE,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW,
        GetMessageW, PostThreadMessageW, RegisterClassW, SetWindowsHookExW, UnhookWindowsHookEx,
        HC_ACTION, HWND_MESSAGE, KBDLLHOOKSTRUCT, LLKHF_EXTENDED, MSG, WH_KEYBOARD_LL,
        WINDOW_EX_STYLE, WINDOW_STYLE, WM_INPUT, WM_KEYDOWN, WM_QUIT, WM_SYSKEYDOWN, WNDCLASSW,
    };

    use super::{Button, Input};
    use crate::keymap;

    // Standard Win32 RI_MOUSE_* button transition flags (from winuser.h).
    const RI_LEFT_DOWN: u16 = 0x0001;
    const RI_LEFT_UP: u16 = 0x0002;
    const RI_RIGHT_DOWN: u16 = 0x0004;
    const RI_RIGHT_UP: u16 = 0x0008;
    const RI_MIDDLE_DOWN: u16 = 0x0010;
    const RI_MIDDLE_UP: u16 = 0x0020;
    const RI_BTN4_DOWN: u16 = 0x0040;
    const RI_BTN4_UP: u16 = 0x0080;
    const RI_BTN5_DOWN: u16 = 0x0100;
    const RI_BTN5_UP: u16 = 0x0200;
    const RI_WHEEL: u16 = 0x0400;

    static SINK: OnceLock<SyncSender<Input>> = OnceLock::new();
    static POINTER_DIRTY: OnceLock<Arc<AtomicBool>> = OnceLock::new();

    fn send(input: Input) {
        if let Some(sink) = SINK.get() {
            // Non-blocking: never stall the Windows input hook chain if the UI thread is busy.
            let _ = sink.try_send(input);
        }
    }

    fn wake_pointer() {
        if let Some(dirty) = POINTER_DIRTY.get() {
            if !dirty.swap(true, Ordering::AcqRel) {
                send(Input::PointerWake);
            }
        }
    }

    unsafe extern "system" fn keyboard_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        if code == HC_ACTION as i32 {
            let info = unsafe { &*(lparam.0 as *const KBDLLHOOKSTRUCT) };
            let down = matches!(wparam.0 as u32, WM_KEYDOWN | WM_SYSKEYDOWN);
            let mut scan = info.scanCode;
            let mut extended = info.flags.0 & LLKHF_EXTENDED.0 != 0;
            if scan == 0 {
                // Injected input (remote desktop, macro tools) may carry only the virtual key.
                let mapped = unsafe { MapVirtualKeyW(info.vkCode, MAPVK_VK_TO_VSC_EX) };
                scan = mapped & 0xff;
                extended = mapped & 0xff00 != 0;
            }
            send(Input::Key {
                id: scan | if extended { 0x100 } else { 0 },
                vk: info.vkCode,
                code: keymap::code(scan, extended, info.vkCode),
                down,
            });
        }
        unsafe { CallNextHookEx(None, code, wparam, lparam) }
    }

    unsafe extern "system" fn raw_wnd_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        if msg == WM_INPUT {
            let mut raw = RAWINPUT::default();
            let mut size = mem::size_of::<RAWINPUT>() as u32;
            let read = unsafe {
                GetRawInputData(
                    HRAWINPUT(lparam.0 as *mut c_void),
                    RID_INPUT,
                    Some((&mut raw as *mut RAWINPUT).cast()),
                    &mut size,
                    mem::size_of::<RAWINPUTHEADER>() as u32,
                )
            };
            if read != u32::MAX && raw.header.dwType == RIM_TYPEMOUSE.0 {
                let mouse = unsafe { raw.data.mouse };
                let flags = unsafe { mouse.Anonymous.Anonymous.usButtonFlags };
                let data = unsafe { mouse.Anonymous.Anonymous.usButtonData };

                if flags & RI_LEFT_DOWN != 0 {
                    send(Input::Button { button: Button::Left, down: true });
                }
                if flags & RI_LEFT_UP != 0 {
                    send(Input::Button { button: Button::Left, down: false });
                }
                if flags & RI_RIGHT_DOWN != 0 {
                    send(Input::Button { button: Button::Right, down: true });
                }
                if flags & RI_RIGHT_UP != 0 {
                    send(Input::Button { button: Button::Right, down: false });
                }
                if flags & (RI_MIDDLE_DOWN | RI_BTN4_DOWN | RI_BTN5_DOWN) != 0 {
                    send(Input::Button { button: Button::Other, down: true });
                }
                if flags & (RI_MIDDLE_UP | RI_BTN4_UP | RI_BTN5_UP) != 0 {
                    send(Input::Button { button: Button::Other, down: false });
                }
                if flags & RI_WHEEL != 0 {
                    let delta = data as i16;
                    send(Input::Wheel(f64::from(delta) / 120.0));
                }
                // Any non-zero delta or pure motion packet wakes the pointer follower.
                if mouse.lLastX != 0 || mouse.lLastY != 0 || flags == 0 {
                    wake_pointer();
                }
            }
        }
        unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
    }

    pub struct HookGuard {
        thread_id: u32,
        join: Option<thread::JoinHandle<()>>,
    }

    impl Drop for HookGuard {
        fn drop(&mut self) {
            unsafe {
                let _ = PostThreadMessageW(self.thread_id, WM_QUIT, WPARAM(0), LPARAM(0));
            }
            if let Some(handle) = self.join.take() {
                let _ = handle.join();
            }
        }
    }

    /// Queries the live OS physical key state to recover from missed WM_KEYUP (e.g. Win+L or UAC).
    pub fn is_vk_down(vk: u32) -> bool {
        if vk == 0 || vk >= 256 {
            return false;
        }
        unsafe { (GetAsyncKeyState(vk as i32) as u16 & 0x8000) != 0 }
    }

    pub fn start(sink: SyncSender<Input>, pointer_dirty: Arc<AtomicBool>) -> Result<HookGuard, String> {
        SINK.set(sink).map_err(|_| "input hook already started".to_string())?;
        let _ = POINTER_DIRTY.set(pointer_dirty);
        let (ready, started) = mpsc::channel::<Result<u32, String>>();
        let join = thread::Builder::new()
            .name("input-hook".into())
            .spawn(move || unsafe {
                let tid = GetCurrentThreadId();
                let module = GetModuleHandleW(None).ok().map(|m| HINSTANCE(m.0));
                let keyboard = match SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_proc), module, 0) {
                    Ok(h) => h,
                    Err(error) => {
                        let _ = ready.send(Err(error.to_string()));
                        return;
                    }
                };
                let class_name = w!("KeyCatRawInputSink");
                let wc = WNDCLASSW {
                    lpfnWndProc: Some(raw_wnd_proc),
                    hInstance: module.unwrap_or_default(),
                    lpszClassName: class_name,
                    ..Default::default()
                };
                let _ = RegisterClassW(&wc);
                let hwnd = match CreateWindowExW(
                    WINDOW_EX_STYLE(0),
                    class_name,
                    PCWSTR::null(),
                    WINDOW_STYLE(0),
                    0,
                    0,
                    0,
                    0,
                    Some(HWND_MESSAGE),
                    None,
                    module,
                    None,
                ) {
                    Ok(hwnd) => hwnd,
                    Err(error) => {
                        let _ = UnhookWindowsHookEx(keyboard);
                        let _ = ready.send(Err(error.to_string()));
                        return;
                    }
                };
                let rid = [RAWINPUTDEVICE {
                    usUsagePage: 0x01,
                    usUsage: 0x02, // Generic Desktop Controls: Mouse
                    dwFlags: RIDEV_INPUTSINK,
                    hwndTarget: hwnd,
                }];
                if let Err(error) =
                    RegisterRawInputDevices(&rid, mem::size_of::<RAWINPUTDEVICE>() as u32)
                {
                    let _ = DestroyWindow(hwnd);
                    let _ = UnhookWindowsHookEx(keyboard);
                    let _ = ready.send(Err(error.to_string()));
                    return;
                }
                let _ = ready.send(Ok(tid));
                // Both WH_KEYBOARD_LL and WM_INPUT are dispatched through this thread's message loop.
                let mut msg = MSG::default();
                while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                    DispatchMessageW(&msg);
                }
                let _ = DestroyWindow(hwnd);
                let _ = UnhookWindowsHookEx(keyboard);
            })
            .map_err(|error| error.to_string())?;
        let thread_id = started.recv().map_err(|error| error.to_string())??;
        Ok(HookGuard { thread_id, join: Some(join) })
    }
}

#[cfg(not(windows))]
mod imp {
    use std::sync::atomic::AtomicBool;
    use std::sync::mpsc::SyncSender;
    use std::sync::Arc;

    use super::Input;

    pub struct HookGuard;

    pub fn is_vk_down(_vk: u32) -> bool {
        false
    }

    pub fn start(_sink: SyncSender<Input>, _pointer_dirty: Arc<AtomicBool>) -> Result<HookGuard, String> {
        Err("global input is only implemented for Windows".into())
    }
}

pub use imp::{is_vk_down, start, HookGuard};
