// macOS input implementation using CGEventTap (capture) and CGEventPost (injection).
// Requires Accessibility permission — check with AXIsProcessTrusted() before starting.

use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use core_foundation::base::TCFType;
use core_foundation::boolean::CFBoolean;
use core_foundation::dictionary::CFDictionary;
use core_foundation::string::{CFString, CFStringRef};
use core_graphics::geometry::CGPoint;
use tokio::sync::mpsc;

use super::{InputCapture, InputError, InputEvent, InputInjector, PermissionStatus};
use crate::engine::screen_layout::Point;

// ---------------------------------------------------------------------------
// Opaque C types
// ---------------------------------------------------------------------------

enum OpaqueRunLoop {}
enum OpaqueMachPort {}
enum OpaqueRunLoopSource {}
enum OpaqueEvent {}

type CFRunLoopRef = *mut OpaqueRunLoop;
type CFMachPortRef = *mut OpaqueMachPort;
type CFRunLoopSourceRef = *mut OpaqueRunLoopSource;
type CGEventRef = *mut OpaqueEvent;
type CFIndex = isize;
type CFRunLoopModeRef = *const c_void;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const K_CG_SESSION_EVENT_TAP: u32 = 1;
const K_CG_HID_EVENT_TAP: u32 = 0;
const K_CG_HEAD_INSERT_EVENT_TAP: u32 = 0;
/// Default option: can suppress events by returning null from callback.
const K_CG_EVENT_TAP_OPTION_DEFAULT: u32 = 0;

const K_CG_EVENT_LEFT_MOUSE_DOWN: u32 = 1;
const K_CG_EVENT_LEFT_MOUSE_UP: u32 = 2;
const K_CG_EVENT_RIGHT_MOUSE_DOWN: u32 = 3;
const K_CG_EVENT_RIGHT_MOUSE_UP: u32 = 4;
const K_CG_EVENT_MOUSE_MOVED: u32 = 5;
const K_CG_EVENT_LEFT_MOUSE_DRAGGED: u32 = 6;
const K_CG_EVENT_RIGHT_MOUSE_DRAGGED: u32 = 7;
const K_CG_EVENT_SCROLL_WHEEL: u32 = 22;

const K_CG_MOUSE_EVENT_DELTA_X: i64 = 4;
const K_CG_MOUSE_EVENT_DELTA_Y: i64 = 5;

const K_CG_SCROLL_WHEEL_EVENT_DELTA_AXIS1: i64 = 11;
const K_CG_SCROLL_WHEEL_EVENT_DELTA_AXIS2: i64 = 12;
const K_CG_SCROLL_EVENT_UNIT_PIXEL: u32 = 0;

const K_CG_ANNOTATED_SESSION_EVENT_TAP: u32 = 2;
const K_CG_NULL_DIRECT_DISPLAY: u32 = 0;

/// Bitmask of CGEventType values the tap intercepts.
const TAP_EVENTS_MASK: u64 = (1 << K_CG_EVENT_MOUSE_MOVED)
    | (1 << K_CG_EVENT_LEFT_MOUSE_DOWN)
    | (1 << K_CG_EVENT_LEFT_MOUSE_UP)
    | (1 << K_CG_EVENT_RIGHT_MOUSE_DOWN)
    | (1 << K_CG_EVENT_RIGHT_MOUSE_UP)
    | (1 << K_CG_EVENT_LEFT_MOUSE_DRAGGED)
    | (1 << K_CG_EVENT_RIGHT_MOUSE_DRAGGED)
    | (1 << K_CG_EVENT_SCROLL_WHEEL);

// ---------------------------------------------------------------------------
// FFI — ApplicationServices
// ---------------------------------------------------------------------------

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    static kAXTrustedCheckOptionPrompt: CFStringRef;
    fn AXIsProcessTrusted() -> bool;
    fn AXIsProcessTrustedWithOptions(options: *const c_void) -> bool;
}

// ---------------------------------------------------------------------------
// FFI — CoreFoundation
// ---------------------------------------------------------------------------

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    static kCFRunLoopDefaultMode: CFRunLoopModeRef;
    fn CFMachPortCreateRunLoopSource(
        allocator: *mut c_void,
        port: CFMachPortRef,
        order: CFIndex,
    ) -> CFRunLoopSourceRef;
    fn CFRunLoopGetCurrent() -> CFRunLoopRef;
    fn CFRunLoopAddSource(rl: CFRunLoopRef, source: CFRunLoopSourceRef, mode: CFRunLoopModeRef);
    fn CFRunLoopRun();
    fn CFRunLoopStop(rl: CFRunLoopRef);
    fn CFRelease(p: *mut c_void);
}

// ---------------------------------------------------------------------------
// FFI — CoreGraphics
// ---------------------------------------------------------------------------

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGEventTapCreate(
        tap: u32,
        place: u32,
        options: u32,
        events_of_interest: u64,
        callback: unsafe extern "C" fn(*mut c_void, u32, CGEventRef, *mut c_void) -> CGEventRef,
        user_info: *mut c_void,
    ) -> CFMachPortRef;
    fn CGEventTapEnable(tap: CFMachPortRef, enable: bool);
    fn CGEventGetLocation(event: CGEventRef) -> CGPoint;
    fn CGEventGetIntegerValueField(event: CGEventRef, field: i64) -> i64;
    fn CGWarpMouseCursorPosition(pos: CGPoint) -> i32;
    fn CGEventCreateMouseEvent(
        source: *mut c_void,
        mouse_type: u32,
        mouse_cursor_position: CGPoint,
        mouse_button: u32,
    ) -> CGEventRef;
    fn CGEventCreateScrollWheelEvent(
        source: *mut c_void,
        units: u32,
        wheel_count: u32,
        wheel1: i32,
        wheel2: i32,
    ) -> CGEventRef;
    fn CGEventPost(tap: u32, event: CGEventRef);
    fn CGDisplayHideCursor(display: u32) -> i32;
    fn CGDisplayShowCursor(display: u32) -> i32;
    fn CGAssociateMouseAndMouseCursorPosition(connected: bool) -> i32;
}

// ---------------------------------------------------------------------------
// Tap context — passed to the C callback via user_info pointer
// ---------------------------------------------------------------------------

struct TapContext {
    tx: mpsc::Sender<InputEvent>,
    /// True while the coordinator is forwarding events to the remote machine.
    /// When set, the callback suppresses events so the local cursor stays still.
    suppressing: Arc<AtomicBool>,
    grace_frames: Arc<AtomicU8>,
}

/// CGEventTap callback. Must be a free `extern "C" fn` — closures cannot be
/// used as C function pointers.
///
/// SAFETY: `user_info` is a `*mut TapContext` valid until `stop()` disables
/// the tap and joins the tap thread before dropping it.
unsafe extern "C" fn tap_callback(
    _proxy: *mut c_void,
    event_type: u32,
    event: CGEventRef,
    user_info: *mut c_void,
) -> CGEventRef {
    let ctx = &*(user_info as *const TapContext);

    // Grace period: suppress first N events after returning from Remote state.
    // Prevents cursor flash caused by stale absolute events firing before
    // the injected entry-point position takes effect.
    if ctx.grace_frames.load(Ordering::Relaxed) > 0 {
        ctx.grace_frames.fetch_sub(1, Ordering::Relaxed);
        return std::ptr::null_mut();
    }

    match event_type {
        K_CG_EVENT_MOUSE_MOVED | K_CG_EVENT_LEFT_MOUSE_DRAGGED | K_CG_EVENT_RIGHT_MOUSE_DRAGGED => {
            if ctx.suppressing.load(Ordering::Relaxed) {
                // In Remote/suppressing mode: CGEventGetLocation returns the frozen
                // cursor position (never moves because events are suppressed). Use
                // hardware deltas which are always valid.
                let dx = CGEventGetIntegerValueField(event, K_CG_MOUSE_EVENT_DELTA_X) as f64;
                let dy = CGEventGetIntegerValueField(event, K_CG_MOUSE_EVENT_DELTA_Y) as f64;
                // Preserve drag type so the client can post the correct event.
                let button = match event_type {
                    K_CG_EVENT_LEFT_MOUSE_DRAGGED => Some(0u8),
                    K_CG_EVENT_RIGHT_MOUSE_DRAGGED => Some(1u8),
                    _ => None,
                };
                let _ = ctx.tx.try_send(InputEvent::MouseDelta { dx, dy, button });
            } else {
                let pt = CGEventGetLocation(event);
                let _ = ctx
                    .tx
                    .try_send(InputEvent::MouseMove(Point { x: pt.x, y: pt.y }));
            }
        }
        K_CG_EVENT_LEFT_MOUSE_DOWN => {
            let _ = ctx.tx.try_send(InputEvent::MouseButton {
                button: 0,
                pressed: true,
            });
        }
        K_CG_EVENT_LEFT_MOUSE_UP => {
            let _ = ctx.tx.try_send(InputEvent::MouseButton {
                button: 0,
                pressed: false,
            });
        }
        K_CG_EVENT_RIGHT_MOUSE_DOWN => {
            let _ = ctx.tx.try_send(InputEvent::MouseButton {
                button: 1,
                pressed: true,
            });
        }
        K_CG_EVENT_RIGHT_MOUSE_UP => {
            let _ = ctx.tx.try_send(InputEvent::MouseButton {
                button: 1,
                pressed: false,
            });
        }
        K_CG_EVENT_SCROLL_WHEEL => {
            let dy = CGEventGetIntegerValueField(event, K_CG_SCROLL_WHEEL_EVENT_DELTA_AXIS1) as f32;
            let dx = CGEventGetIntegerValueField(event, K_CG_SCROLL_WHEEL_EVENT_DELTA_AXIS2) as f32;
            let _ = ctx.tx.try_send(InputEvent::Scroll { dx, dy });
        }
        _ => {}
    }

    if ctx.suppressing.load(Ordering::Relaxed) {
        std::ptr::null_mut() // consume the event — local cursor stays still
    } else {
        event // pass through
    }
}

// ---------------------------------------------------------------------------
// MacOSCapture
// ---------------------------------------------------------------------------

pub struct MacOSCapture {
    tap: Option<CFMachPortRef>,
    /// CFRunLoopRef of the tap thread, stored as usize for Send compatibility.
    run_loop: Arc<AtomicUsize>,
    thread: Option<JoinHandle<()>>,
    /// Owned pointer to TapContext. Dropped in stop() after tap is disabled.
    ctx_ptr: Option<*mut TapContext>,
    /// The coordinator sets this to true on StartForwarding and false on
    /// StopForwarding. Read on every mouse event by the C callback.
    pub suppressing: Arc<AtomicBool>,
    pub grace_frames: Arc<AtomicU8>,
}

// SAFETY:
// - CFMachPortRef: CF retain/release is thread-safe. We hold one reference,
//   released in stop() after the thread has exited.
// - ctx_ptr: accessed only in start() (write) and stop() (drop), both
//   requiring &mut self. The callback reads it only while the tap is enabled;
//   stop() disables the tap before dropping.
// - suppressing, run_loop: Arc<Atomic*> — unconditionally Send.
unsafe impl Send for MacOSCapture {}

impl MacOSCapture {
    pub fn new() -> Self {
        Self {
            tap: None,
            run_loop: Arc::new(AtomicUsize::new(0)),
            thread: None,
            ctx_ptr: None,
            suppressing: Arc::new(AtomicBool::new(false)),
            grace_frames: Arc::new(AtomicU8::new(0)),
        }
    }
}

impl InputCapture for MacOSCapture {
    fn start(&mut self, tx: mpsc::Sender<InputEvent>) -> Result<(), InputError> {
        let ctx = Box::new(TapContext {
            tx,
            suppressing: self.suppressing.clone(),
            grace_frames: self.grace_frames.clone(),
        });
        let ctx_ptr = Box::into_raw(ctx);

        let tap = unsafe {
            CGEventTapCreate(
                K_CG_SESSION_EVENT_TAP,
                K_CG_HEAD_INSERT_EVENT_TAP,
                K_CG_EVENT_TAP_OPTION_DEFAULT,
                TAP_EVENTS_MASK,
                tap_callback,
                ctx_ptr as *mut c_void,
            )
        };

        if tap.is_null() {
            unsafe {
                drop(Box::from_raw(ctx_ptr));
            }
            return Err(InputError::EventTapFailed(
                "CGEventTapCreate returned null — grant Accessibility permission in System Settings"
                    .into(),
            ));
        }

        let source = unsafe { CFMachPortCreateRunLoopSource(std::ptr::null_mut(), tap, 0) };

        let run_loop = self.run_loop.clone();

        // Cast to usize before moving into the thread closure: raw pointers are
        // not Send, but usize is. SAFETY: both refs are valid until the thread exits.
        let tap_usize = tap as usize;
        let source_usize = source as usize;

        // Use std::thread::spawn — CFRunLoopRun blocks an OS thread and must
        // not run inside tokio's cooperative scheduler.
        let thread = std::thread::spawn(move || {
            let tap = tap_usize as CFMachPortRef;
            let source = source_usize as CFRunLoopSourceRef;
            unsafe {
                let rl = CFRunLoopGetCurrent();
                run_loop.store(rl as usize, Ordering::SeqCst);
                CFRunLoopAddSource(rl, source, kCFRunLoopDefaultMode);
                CGEventTapEnable(tap, true);
                CFRunLoopRun(); // returns when CFRunLoopStop is called
                CFRelease(source as *mut c_void);
            }
        });

        // Wait until the thread has stored the run loop reference.
        // Completes in a handful of iterations in practice.
        while self.run_loop.load(Ordering::SeqCst) == 0 {
            std::hint::spin_loop();
        }

        self.tap = Some(tap);
        self.thread = Some(thread);
        self.ctx_ptr = Some(ctx_ptr);
        Ok(())
    }

    fn stop(&mut self) {
        if let Some(tap) = self.tap.take() {
            unsafe {
                CGEventTapEnable(tap, false);
            }

            let rl = self.run_loop.load(Ordering::SeqCst) as CFRunLoopRef;
            if !rl.is_null() {
                unsafe {
                    CFRunLoopStop(rl);
                }
            }
            self.run_loop.store(0, Ordering::SeqCst);

            if let Some(thread) = self.thread.take() {
                let _ = thread.join();
            }

            // Release the mach port reference held by this struct.
            // The thread released the run loop source before exiting.
            unsafe {
                CFRelease(tap as *mut c_void);
            }
        }

        // Safe to drop now: tap disabled, thread exited, callback cannot fire.
        if let Some(ptr) = self.ctx_ptr.take() {
            unsafe {
                drop(Box::from_raw(ptr));
            }
        }
    }

    fn permission_status(&self) -> PermissionStatus {
        if unsafe { AXIsProcessTrusted() } {
            PermissionStatus::Granted
        } else {
            // AXIsProcessTrusted does not distinguish "denied" from "not asked yet".
            PermissionStatus::NotDetermined
        }
    }

    fn request_permission(&self) {
        // Passing kAXTrustedCheckOptionPrompt = true opens System Settings and
        // adds this process to the Accessibility list automatically.
        unsafe {
            let key = CFString::wrap_under_get_rule(kAXTrustedCheckOptionPrompt);
            let value = CFBoolean::true_value();
            let dict = CFDictionary::<CFString, CFBoolean>::from_CFType_pairs(&[(key, value)]);
            AXIsProcessTrustedWithOptions(dict.as_concrete_TypeRef() as *const c_void);
        }
    }
}

// ---------------------------------------------------------------------------
// MacOSInjector
// ---------------------------------------------------------------------------

pub struct MacOSInjector {
    /// Last injected position — used as the location for button and scroll events.
    last_pos: Mutex<CGPoint>,
}

impl MacOSInjector {
    pub fn new() -> Self {
        Self {
            last_pos: Mutex::new(CGPoint { x: 0.0, y: 0.0 }),
        }
    }
}

impl InputInjector for MacOSInjector {
    fn inject_move(&self, pos: Point, button: Option<u8>) {
        let cg_pos = CGPoint { x: pos.x, y: pos.y };
        *self.last_pos.lock().unwrap() = cg_pos;
        // Choose the correct event type so apps receive proper drag events.
        // CGWarpMouseCursorPosition is intentionally omitted — it has a ~250ms
        // internal delay and causes double-updates that make the cursor teleport.
        let (event_type, cg_button) = match button {
            Some(0) => (K_CG_EVENT_LEFT_MOUSE_DRAGGED, 0u32),
            Some(1) => (K_CG_EVENT_RIGHT_MOUSE_DRAGGED, 1u32),
            _ => (K_CG_EVENT_MOUSE_MOVED, 0u32),
        };
        unsafe {
            let event =
                CGEventCreateMouseEvent(std::ptr::null_mut(), event_type, cg_pos, cg_button);
            if !event.is_null() {
                CGEventPost(K_CG_HID_EVENT_TAP, event);
                CFRelease(event as *mut c_void);
            }
        }
    }

    fn inject_button(&self, button: u8, pressed: bool) {
        let pos = *self.last_pos.lock().unwrap();
        let (down_type, up_type, cg_button) = match button {
            0 => (K_CG_EVENT_LEFT_MOUSE_DOWN, K_CG_EVENT_LEFT_MOUSE_UP, 0u32),
            1 => (K_CG_EVENT_RIGHT_MOUSE_DOWN, K_CG_EVENT_RIGHT_MOUSE_UP, 1u32),
            _ => return,
        };
        let event_type = if pressed { down_type } else { up_type };
        unsafe {
            let event = CGEventCreateMouseEvent(std::ptr::null_mut(), event_type, pos, cg_button);
            if !event.is_null() {
                // Session level ensures click-to-focus and window dispatch work correctly.
                CGEventPost(K_CG_SESSION_EVENT_TAP, event);
                CFRelease(event as *mut c_void);
            }
        }
    }

    fn inject_scroll(&self, dx: f32, dy: f32) {
        unsafe {
            let event = CGEventCreateScrollWheelEvent(
                std::ptr::null_mut(),
                K_CG_SCROLL_EVENT_UNIT_PIXEL,
                2,
                dy as i32,
                dx as i32,
            );
            if !event.is_null() {
                CGEventPost(K_CG_HID_EVENT_TAP, event);
                CFRelease(event as *mut c_void);
            }
        }
    }

    fn hide_cursor(&self) {
        unsafe {
            CGAssociateMouseAndMouseCursorPosition(false);
            CGDisplayHideCursor(K_CG_NULL_DIRECT_DISPLAY);
        }
    }

    fn show_cursor(&self) {
        unsafe {
            CGDisplayShowCursor(K_CG_NULL_DIRECT_DISPLAY);
            CGAssociateMouseAndMouseCursorPosition(true);
        }
    }
}
