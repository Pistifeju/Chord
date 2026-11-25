use std::ffi::c_void;
use std::sync::OnceLock;

use objc2::rc::Retained;
use objc2::runtime::{AnyObject, Bool, Sel};
use objc2::sel;
use objc2_core_foundation::CGPoint;

use super::{Button, EDGE_BOTTOM, Input, TouchPhase};
use crate::simulator_kit::{class, get, msg, responds, symbol};

const TARGET_TOUCH: u32 = 0x32;
const TARGET_BUTTON: i32 = 0x33;
const MOUSE_DOWN: i32 = 1;
const MOUSE_UP: i32 = 2;
const BUTTON_LOCK: i32 = 0x1;

type MouseFn =
    unsafe extern "C" fn(*const CGPoint, *const CGPoint, u32, i32, f64, f64, u32) -> *mut c_void;
type ButtonFn = unsafe extern "C" fn(i32, i32, i32) -> *mut c_void;
type KeyFn = unsafe extern "C" fn(u32, u32) -> *mut c_void;

struct Builders {
    mouse: MouseFn,
    button: ButtonFn,
    keyboard: Option<KeyFn>,
}

fn builders() -> Option<&'static Builders> {
    static BUILDERS: OnceLock<Option<Builders>> = OnceLock::new();
    BUILDERS
        .get_or_init(|| unsafe {
            Some(Builders {
                mouse: symbol(c"IndigoHIDMessageForMouseNSEvent")?,
                button: symbol(c"IndigoHIDMessageForButton")?,
                keyboard: symbol(c"IndigoHIDMessageForKeyboardArbitrary"),
            })
        })
        .as_ref()
}

pub(super) struct Client(Retained<AnyObject>);

impl Client {
    pub fn new(device: &Retained<AnyObject>) -> Option<Client> {
        builders()?;
        let class = class(c"_TtC12SimulatorKit24SimDeviceLegacyHIDClient")?;
        let alloc = get(class, sel!(alloc));
        let init = sel!(initWithDevice:error:);
        if !responds(alloc, init) {
            return None;
        }
        let client = unsafe {
            msg::<
                unsafe extern "C" fn(
                    *mut AnyObject,
                    Sel,
                    *const AnyObject,
                    *mut c_void,
                ) -> *mut AnyObject,
            >()(alloc, init, Retained::as_ptr(device), std::ptr::null_mut())
        };
        let client = unsafe { Retained::from_raw(client) }?;
        responds(
            Retained::as_ptr(&client) as *mut AnyObject,
            sel!(sendWithMessage:freeWhenDone:completionQueue:completion:),
        )
        .then_some(Client(client))
    }

    pub fn send(&self, input: Input) {
        let Some(build) = builders() else { return };
        match input {
            Input::Touch {
                phase,
                first,
                second,
                from_bottom,
                ..
            } => {
                let kind = if phase == TouchPhase::Up {
                    MOUSE_UP
                } else {
                    MOUSE_DOWN
                };
                let first = CGPoint::new(first.x, first.y);
                let second = second.map(|point| CGPoint::new(point.x, point.y));
                let second = second
                    .as_ref()
                    .map_or(std::ptr::null(), |point| point as *const CGPoint);
                let edge = if from_bottom { EDGE_BOTTOM } else { 0 };
                self.deliver(unsafe {
                    (build.mouse)(&first, second, TARGET_TOUCH, kind, 1.0, 1.0, edge)
                });
            }
            Input::Key { usage, down } => {
                if let Some(keyboard) = build.keyboard {
                    self.deliver(unsafe { keyboard(usage, if down { 1 } else { 2 }) });
                }
            }
            Input::Button {
                button: Button::Lock,
            } => {
                for direction in [1, 2] {
                    self.deliver(unsafe { (build.button)(BUTTON_LOCK, direction, TARGET_BUTTON) });
                }
            }
            Input::Button { .. } => {}
        }
    }

    fn deliver(&self, message: *mut c_void) {
        if message.is_null() {
            return;
        }
        unsafe {
            msg::<
                unsafe extern "C" fn(
                    *mut AnyObject,
                    Sel,
                    *mut c_void,
                    Bool,
                    *mut AnyObject,
                    *mut c_void,
                ),
            >()(
                Retained::as_ptr(&self.0) as *mut AnyObject,
                sel!(sendWithMessage:freeWhenDone:completionQueue:completion:),
                message,
                Bool::YES,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };
    }
}
