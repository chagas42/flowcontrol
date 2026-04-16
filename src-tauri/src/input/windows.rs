// Windows input implementation using SetWindowsHookEx (WH_MOUSE_LL) for capture
// and SendInput for injection. No special permissions required.

use std::cell::RefCell;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;
use std::thread::JoinHandle;

use tokio::sync::mpsc;

use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_MOUSE, MOUSEEVENTF_ABSOLUTE, MOUSEEVENTF_LEFTDOWN,
    MOUSEEVENTF_LEFTUP, MOUSEEVENTF_MOVE, MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP,
    MOUSEEVENTF_WHEEL, MOUSEINPUT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, GetMessageW, GetSystemMetrics, PostThreadMessageW, SetWindowsHookExW,
    ShowCursor, UnhookWindowsHookEx, HHOOK, MSLLHOOKSTRUCT, MSG, SM_CXSCREEN, SM_CYSCREEN,
    WH_MOUSE_LL, WHEEL_DELTA, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE, WM_MOUSEWHEEL,
    WM_QUIT, WM_RBUTTONDOWN, WM_RBUTTONUP,
};

use super::{InputCapture, InputError, InputEvent, InputInjector, PermissionStatus};
use crate::engine::screen_layout::Point;

thread_local! {
    static LOCAL_TX: RefCell<Option<mpsc::Sender<InputEvent>>> = RefCell::new(None);
}

static HOOK_THREAD_ID: AtomicU32 = AtomicU32::new(0);

unsafe extern "system" fn mouse_hook_callback(
    code: i32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if code >= 0 {
        let ms = &*(lparam.0 as *const MSLLHOOKSTRUCT);
        let event = match wparam.0 as u32 {
            WM_MOUSEMOVE => Some(InputEvent::MouseMove(Point {
                x: ms.pt.x as f64,
                y: ms.pt.y as f64,
            })),
            WM_LBUTTONDOWN => Some(InputEvent::MouseButton {
                button: 0,
                pressed: true,
            }),
            WM_LBUTTONUP => Some(InputEvent::MouseButton {
                button: 0,
                pressed: false,
            }),
            WM_RBUTTONDOWN => Some(InputEvent::MouseButton {
                button: 1,
                pressed: true,
            }),
            WM_RBUTTONUP => Some(InputEvent::MouseButton {
                button: 1,
                pressed: false,
            }),
            WM_MOUSEWHEEL => {
                let delta = ((ms.mouseData >> 16) as i16) as f32 / WHEEL_DELTA as f32;
                Some(InputEvent::Scroll { dx: 0.0, dy: delta })
            }
            _ => None,
        };
        if let Some(ev) = event {
            LOCAL_TX.with(|tx| {
                if let Some(sender) = tx.borrow().as_ref() {
                    let _ = sender.try_send(ev);
                }
            });
        }
    }
    CallNextHookEx(HHOOK::default(), code, wparam, lparam)
}

pub struct WindowsCapture {
    thread: Option<JoinHandle<()>>,
}

impl WindowsCapture {
    pub fn new() -> Self {
        Self { thread: None }
    }
}

impl InputCapture for WindowsCapture {
    fn start(&mut self, tx: mpsc::Sender<InputEvent>) -> Result<(), InputError> {
        let thread = std::thread::spawn(move || {
            LOCAL_TX.with(|cell| *cell.borrow_mut() = Some(tx));
            HOOK_THREAD_ID.store(unsafe { GetCurrentThreadId() }, Ordering::SeqCst);

            let hook = unsafe {
                SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_hook_callback), None, 0)
                    .expect("SetWindowsHookExW failed")
            };

            let mut msg = MSG::default();
            // GetMessageW returns 0 on WM_QUIT, -1 on error
            loop {
                let ret = unsafe { GetMessageW(&mut msg, None, 0, 0) };
                if ret.0 <= 0 {
                    break;
                }
            }

            unsafe {
                let _ = UnhookWindowsHookEx(hook);
            }
            LOCAL_TX.with(|cell| *cell.borrow_mut() = None);
        });
        self.thread = Some(thread);
        Ok(())
    }

    fn stop(&mut self) {
        let tid = HOOK_THREAD_ID.swap(0, Ordering::SeqCst);
        if tid != 0 {
            unsafe {
                let _ = PostThreadMessageW(tid, WM_QUIT, WPARAM(0), LPARAM(0));
            }
        }
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }

    fn permission_status(&self) -> PermissionStatus {
        PermissionStatus::Granted
    }

    fn request_permission(&self) {}
}

pub struct WindowsInjector {
    last_pos: Mutex<(i32, i32)>,
}

impl WindowsInjector {
    pub fn new() -> Self {
        Self {
            last_pos: Mutex::new((0, 0)),
        }
    }
}

impl InputInjector for WindowsInjector {
    fn inject_move(&self, pos: Point, _button: Option<u8>) {
        let sw = unsafe { GetSystemMetrics(SM_CXSCREEN) } as f64;
        let sh = unsafe { GetSystemMetrics(SM_CYSCREEN) } as f64;
        let x = ((pos.x / sw) * 65535.0) as i32;
        let y = ((pos.y / sh) * 65535.0) as i32;
        *self.last_pos.lock().unwrap() = (x, y);
        let input = INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: x,
                    dy: y,
                    mouseData: 0,
                    dwFlags: MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };
        unsafe {
            SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
        }
    }

    fn inject_button(&self, button: u8, pressed: bool) {
        let flags = match (button, pressed) {
            (0, true) => MOUSEEVENTF_LEFTDOWN,
            (0, false) => MOUSEEVENTF_LEFTUP,
            (1, true) => MOUSEEVENTF_RIGHTDOWN,
            (1, false) => MOUSEEVENTF_RIGHTUP,
            _ => return,
        };
        let (dx, dy) = *self.last_pos.lock().unwrap();
        let input = INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx,
                    dy,
                    mouseData: 0,
                    dwFlags: flags,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };
        unsafe {
            SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
        }
    }

    fn inject_scroll(&self, _dx: f32, dy: f32) {
        let wheel_data = (dy * WHEEL_DELTA as f32) as u32;
        let input = INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: 0,
                    dy: 0,
                    mouseData: wheel_data,
                    dwFlags: MOUSEEVENTF_WHEEL,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };
        unsafe {
            SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
        }
    }

    fn hide_cursor(&self) {
        unsafe {
            ShowCursor(false);
        }
    }

    fn show_cursor(&self) {
        unsafe {
            ShowCursor(true);
        }
    }
}
