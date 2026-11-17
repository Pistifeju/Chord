//! The DTUHID input channel from CoreSimulator 1155.4 (Xcode 27) on: one XPC connection to the guest's `dtuhidd` digitizer service, opened off the main thread, checked with a barrier, opened again when it breaks.
//! Connection sequence and wire format adapted from Meta idb (FBSimulatorControl/HID, FBSimulatorControl/XPC).
//!
//! MIT License
//!
//! Copyright (c) Meta Platforms, Inc. and affiliates.
//!
//! Permission is hereby granted, free of charge, to any person obtaining a copy
//! of this software and associated documentation files (the "Software"), to deal
//! in the Software without restriction, including without limitation the rights
//! to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
//! copies of the Software, and to permit persons to whom the Software is
//! furnished to do so, subject to the following conditions:
//!
//! The above copyright notice and this permission notice shall be included in all
//! copies or substantial portions of the Software.
//!
//! THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
//! IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
//! FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
//! AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
//! LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
//! OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
//! SOFTWARE.

use chord_core::lock;
use std::ffi::{CStr, c_char, c_void};
use std::sync::mpsc;
use std::sync::{Arc, Mutex, Weak};
use std::thread;
use std::time::Duration;

use block2::RcBlock;
use dispatch2::{DispatchQueue, DispatchRetained, DispatchTime};
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, Sel};
use objc2::sel;
use objc2_foundation::NSString;

use super::{Button, EDGE_BOTTOM, Input, Point, TouchPhase};
use crate::simulator_kit::{guarded, msg, responds, symbol};

const SERVICE: &CStr = c"com.apple.coredevice.feature.remote.hid.digitizer";
/// `dtuhidd` is demand-launched and can abort while the guest boots; launchd then throttles its respawn for seconds.
const ATTEMPTS: u32 = 5;
const RETRY_AFTER: Duration = Duration::from_secs(4);
const PROBE_TIMEOUT: Duration = Duration::from_secs(4);
/// The device opens after the probe's reply; events sent sooner get lost.
const PROBE_TAIL: Duration = Duration::from_millis(200);
/// How long a button stays pressed.
const PRESS: Duration = Duration::from_millis(80);
/// HID consumer page usages: Menu (Home) and Power (Lock).
const CONSUMER_PAGE: u64 = 0x0C;
const USAGE_HOME: u64 = 0x40;
const USAGE_LOCK: u64 = 0x30;
/// `HIDButtonState` is 1-based: 0 is rejected.
const DOWN: u64 = 1;
const UP: u64 = 2;

type Xpc = *mut c_void;

unsafe extern "C" {
    static _xpc_type_error: c_void;
    static _xpc_error_connection_invalid: c_void;
    fn xpc_dictionary_create(keys: *const *const c_char, values: *const Xpc, count: usize) -> Xpc;
    fn xpc_dictionary_set_string(dictionary: Xpc, key: *const c_char, value: *const c_char);
    fn xpc_dictionary_set_bool(dictionary: Xpc, key: *const c_char, value: bool);
    fn xpc_dictionary_set_uint64(dictionary: Xpc, key: *const c_char, value: u64);
    fn xpc_dictionary_set_double(dictionary: Xpc, key: *const c_char, value: f64);
    fn xpc_dictionary_set_value(dictionary: Xpc, key: *const c_char, value: Xpc);
    fn xpc_get_type(object: Xpc) -> *const c_void;
    fn xpc_release(object: Xpc);
    fn xpc_connection_set_target_queue(connection: Xpc, queue: *const DispatchQueue);
    fn xpc_connection_set_event_handler(connection: Xpc, handler: *const c_void);
    fn xpc_connection_resume(connection: Xpc);
    fn xpc_connection_cancel(connection: Xpc);
    fn xpc_connection_send_message(connection: Xpc, message: Xpc);
    fn xpc_connection_send_message_with_reply(
        connection: Xpc,
        message: Xpc,
        queue: *const DispatchQueue,
        handler: *const c_void,
    );
}

fn is_error(object: Xpc) -> bool {
    // SAFETY: `xpc_get_type` takes any XPC object; the type is a static's address.
    unsafe { std::ptr::eq(xpc_get_type(object), &raw const _xpc_type_error) }
}

/// An owned, resumed XPC connection. XPC connections are thread-safe.
struct Connection(Xpc);
// SAFETY: see above.
unsafe impl Send for Connection {}
// SAFETY: see above.
unsafe impl Sync for Connection {}

impl Connection {
    fn send(&self, message: Message) {
        // SAFETY: a live connection and an owned message.
        unsafe { xpc_connection_send_message(self.0, message.0) };
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        // SAFETY: the connection is owned.
        unsafe {
            xpc_connection_cancel(self.0);
            xpc_release(self.0);
        }
    }
}

/// An owned XPC dictionary.
struct Message(Xpc);
// SAFETY: an XPC dictionary nobody else holds.
unsafe impl Send for Message {}

impl Message {
    fn new() -> Message {
        // SAFETY: an empty dictionary, returned +1.
        Message(unsafe { xpc_dictionary_create(std::ptr::null(), std::ptr::null(), 0) })
    }

    fn uint(self, key: &CStr, value: u64) -> Message {
        // SAFETY: an owned dictionary and a C string key.
        unsafe { xpc_dictionary_set_uint64(self.0, key.as_ptr(), value) };
        self
    }

    fn double(self, key: &CStr, value: f64) -> Message {
        // SAFETY: as above.
        unsafe { xpc_dictionary_set_double(self.0, key.as_ptr(), value) };
        self
    }

    fn value(self, key: &CStr, value: Message) -> Message {
        // SAFETY: the dictionary retains the value.
        unsafe { xpc_dictionary_set_value(self.0, key.as_ptr(), value.0) };
        self
    }

    /// The envelope `dtuhidd` decodes: `messageType`, `featureIdentifier`, `isBarrier` and the payload.
    fn envelope(kind: &CStr, payload: Message, barrier: bool) -> Message {
        let message = Message::new();
        // SAFETY: an owned dictionary and C strings.
        unsafe {
            xpc_dictionary_set_string(message.0, c"messageType".as_ptr(), kind.as_ptr());
            xpc_dictionary_set_string(message.0, c"featureIdentifier".as_ptr(), SERVICE.as_ptr());
            xpc_dictionary_set_bool(message.0, c"isBarrier".as_ptr(), barrier);
        }
        message.value(c"payload", payload)
    }

    fn key(usage: u64, state: u64, barrier: bool) -> Message {
        let payload = Message::new()
            .uint(c"usageCode", usage)
            .uint(c"state", state);
        Message::envelope(c"IndigoKeyboardButtonEvent", payload, barrier)
    }

    fn button(usage: u64, state: u64) -> Message {
        let payload = Message::new()
            .uint(c"usagePage", CONSUMER_PAGE)
            .uint(c"usageCode", usage)
            .uint(c"state", state);
        Message::envelope(c"IndigoButtonEvent", payload, false)
    }

    fn touch(phase: TouchPhase, first: Point, second: Option<Point>, from_bottom: bool) -> Message {
        let point = |point: Point| Message::new().double(c"x", point.x).double(c"y", point.y);
        let mut payload = Message::new().value(c"pointOne", point(first));
        if let Some(second) = second {
            payload = payload.value(c"pointTwo", point(second));
        }
        let phase = match phase {
            TouchPhase::Down => 0,
            TouchPhase::Move => 1,
            TouchPhase::Up => 2,
        };
        let payload = payload
            .uint(c"eventType", phase)
            .uint(
                c"edge",
                if from_bottom {
                    u64::from(EDGE_BOTTOM)
                } else {
                    0
                },
            )
            .uint(c"target", 0);
        Message::envelope(c"IndigoDigitizerEvent", payload, false)
    }
}

impl Drop for Message {
    fn drop(&mut self) {
        // SAFETY: owned.
        unsafe { xpc_release(self.0) };
    }
}

enum State {
    Connecting,
    Ready(Arc<Connection>),
    /// The connection is gone or never came up; the next input opens a new one.
    Broken,
}

/// A retained `SimDevice` handed to the connecting thread.
struct Device(*mut AnyObject);
// SAFETY: `SimDevice` is an XPC-backed proxy CoreSimulator calls from its own queues.
unsafe impl Send for Device {}

impl Drop for Device {
    fn drop(&mut self) {
        // SAFETY: retained in `Link::open`.
        drop(unsafe { Retained::from_raw(self.0) });
    }
}

pub(super) struct Link {
    device: Retained<AnyObject>,
    state: Arc<Mutex<State>>,
    queue: DispatchRetained<DispatchQueue>,
}

impl Link {
    /// Starts connecting on a thread of its own; input sent before it is up gets dropped.
    pub fn connect(device: &Retained<AnyObject>) -> Link {
        let link = Link {
            device: device.clone(),
            state: Arc::new(Mutex::new(State::Broken)),
            queue: DispatchQueue::new("chord.dtuhid", None),
        };
        link.open();
        link
    }

    fn open(&self) {
        *self.lock() = State::Connecting;
        let device = Device(Retained::into_raw(self.device.clone()));
        let state = Arc::downgrade(&self.state);
        let queue = self.queue.clone();
        thread::spawn(move || {
            let device = device;
            let connected = connect(device.0, &queue, &state);
            if let Some(state) = state.upgrade() {
                *lock(&state) = match connected {
                    Ok(connection) => State::Ready(Arc::new(connection)),
                    Err(_) => State::Broken,
                };
            }
        });
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, State> {
        lock(&self.state)
    }

    pub fn send(&self, input: Input) {
        let state = self.lock();
        let State::Ready(connection) = &*state else {
            let broken = matches!(*state, State::Broken);
            drop(state);
            if broken {
                self.open();
            }
            return;
        };
        match input {
            Input::Touch {
                phase,
                first,
                second,
                from_bottom,
                ..
            } => connection.send(Message::touch(phase, first, second, from_bottom)),
            Input::Key { usage, down } => connection.send(Message::key(
                u64::from(usage),
                if down { DOWN } else { UP },
                false,
            )),
            Input::Button { button } => {
                let usage = if button == Button::Home {
                    USAGE_HOME
                } else {
                    USAGE_LOCK
                };
                connection.send(Message::button(usage, DOWN));
                // A clone of the shared connection, not a second one: dropping it must not cancel the link.
                let held = Arc::clone(connection);
                let up = move || held.send(Message::button(usage, UP));
                if let Ok(when) = DispatchTime::try_from(PRESS) {
                    let _ = self.queue.after(when, up);
                }
            }
        }
    }
}

/// Looks the service up and opens the connection, retried while `dtuhidd` does not answer the barrier. Blocks.
fn connect(
    device: *mut AnyObject,
    queue: &DispatchQueue,
    state: &Weak<Mutex<State>>,
) -> Result<Connection, String> {
    let mut failure = String::new();
    for attempt in 1..=ATTEMPTS {
        if state.strong_count() == 0 {
            break;
        }
        match guarded(|| connect_once(device, queue, state.clone()))
            .unwrap_or_else(|| Err("CoreSimulator raised an exception.".into()))
        {
            Ok(connection) => return Ok(connection),
            Err(error) => failure = error,
        }
        if attempt < ATTEMPTS {
            thread::sleep(RETRY_AFTER);
        }
    }
    Err(failure)
}

fn connect_once(
    device: *mut AnyObject,
    queue: &DispatchQueue,
    state: Weak<Mutex<State>>,
) -> Result<Connection, String> {
    type FromPort = unsafe extern "C" fn(u32, u64, u64) -> Xpc;
    type FromEndpoint = unsafe extern "C" fn(Xpc) -> Xpc;
    type SimToHost = unsafe extern "C" fn(Xpc);
    // SAFETY: the types match libxpc's private declarations.
    let (from_port, from_endpoint, sim_to_host) = unsafe {
        (
            symbol::<FromPort>(c"xpc_endpoint_create_mach_port_4sim"),
            symbol::<FromEndpoint>(c"xpc_connection_create_from_endpoint"),
            symbol::<SimToHost>(c"xpc_connection_enable_sim2host_4sim"),
        )
    };
    let (Some(from_port), Some(from_endpoint), Some(sim_to_host)) =
        (from_port, from_endpoint, sim_to_host)
    else {
        return Err("This macOS has no simulator XPC endpoints.".into());
    };
    let lookup = sel!(lookup:error:);
    if !responds(device, lookup) {
        return Err("CoreSimulator cannot look up guest services.".into());
    }
    let service = NSString::from_str(SERVICE.to_str().unwrap_or_default());
    // SAFETY: `lookup:error:` takes the service name and an optional error out-pointer and returns a Mach port.
    let port = unsafe {
        msg::<unsafe extern "C" fn(*mut AnyObject, Sel, *const NSString, *mut c_void) -> u32>()(
            device,
            lookup,
            &*service,
            std::ptr::null_mut(),
        )
    };
    if port == 0 {
        return Err("The simulator has no digitizer service yet.".into());
    }
    // SAFETY: the endpoint takes the lookup's send right; both create functions return +1.
    let endpoint = unsafe { from_port(port, 0, 0) };
    if endpoint.is_null() {
        return Err("No endpoint for the digitizer service.".into());
    }
    let connection = unsafe { from_endpoint(endpoint) };
    unsafe { xpc_release(endpoint) };
    if connection.is_null() {
        return Err("No connection to the digitizer service.".into());
    }
    let connection = Connection(connection);
    let events = RcBlock::new(move |event: Xpc| {
        // SAFETY: the error constant is a static's address.
        let invalid = std::ptr::eq(
            event as *const c_void,
            &raw const _xpc_error_connection_invalid,
        );
        if let (true, Some(state)) = (invalid, state.upgrade()) {
            let mut state = lock(&state);
            if matches!(*state, State::Ready(_)) {
                *state = State::Broken;
            }
        }
    });
    // Without sim-to-host the service sees the peer but never a payload.
    // SAFETY: an unresumed connection; XPC copies the handler block.
    unsafe {
        sim_to_host(connection.0);
        xpc_connection_set_target_queue(connection.0, queue);
        xpc_connection_set_event_handler(connection.0, &*events as *const _ as *const c_void);
        xpc_connection_resume(connection.0);
    }
    // A port exists even when `dtuhidd` cannot run; only a reply proves a daemon behind it. Usage 0 is no key.
    let (replied, reply) = mpsc::channel();
    let on_reply = RcBlock::new(move |answer: Xpc| {
        let _ = replied.send(!is_error(answer));
    });
    let probe = Message::key(0, UP, true);
    // SAFETY: a live connection, an owned message, a block XPC copies.
    unsafe {
        xpc_connection_send_message_with_reply(
            connection.0,
            probe.0,
            queue,
            &*on_reply as *const _ as *const c_void,
        )
    };
    match reply.recv_timeout(PROBE_TIMEOUT) {
        Ok(true) => {
            thread::sleep(PROBE_TAIL);
            Ok(connection)
        }
        _ => Err("The simulator's digitizer service did not answer.".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::simulator_kit;

    fn locked(udid: &str) -> bool {
        let out = std::process::Command::new("xcrun")
            .args([
                "simctl",
                "spawn",
                udid,
                "notifyutil",
                "-g",
                "com.apple.springboard.lockstate",
            ])
            .output()
            .unwrap();
        String::from_utf8_lossy(&out.stdout).trim().ends_with(" 1")
    }

    fn swipe(connection: &Connection, from: Point, to: Point) {
        let at = |t: f64| Point {
            x: from.x + (to.x - from.x) * t,
            y: from.y + (to.y - from.y) * t,
        };
        connection.send(Message::touch(TouchPhase::Down, from, None, false));
        for step in 1..=10 {
            thread::sleep(Duration::from_millis(16));
            connection.send(Message::touch(
                TouchPhase::Move,
                at(f64::from(step) / 10.0),
                None,
                false,
            ));
        }
        connection.send(Message::touch(TouchPhase::Up, to, None, false));
    }

    fn press(connection: &Connection, usage: u64) {
        connection.send(Message::button(usage, DOWN));
        thread::sleep(PRESS);
        connection.send(Message::button(usage, UP));
    }

    /// Needs a booted simulator, which ends locked: `CHORD_SIM_UDID=<udid> cargo test -p chord-ios dtuhid_smoke -- --ignored --nocapture`.
    /// Home and Lock are checked through SpringBoard's lock state; watch the simulator for the rest: the home screen pages, Spotlight opens and gets "chord" typed.
    #[test]
    #[ignore]
    fn dtuhid_smoke() {
        let udid = std::env::var("CHORD_SIM_UDID").unwrap();
        let device = simulator_kit::load()
            .unwrap()
            .device_with_udid(&udid)
            .unwrap();
        let state = Arc::new(Mutex::new(State::Connecting));
        let queue = DispatchQueue::new("chord.dtuhid.smoke", None);
        let connection = connect(
            Retained::as_ptr(&device) as *mut AnyObject,
            &queue,
            &Arc::downgrade(&state),
        )
        .unwrap();
        if locked(&udid) {
            for _ in 0..2 {
                press(&connection, USAGE_HOME);
                thread::sleep(Duration::from_secs(1));
            }
            assert!(!locked(&udid), "Home did not unlock the device");
        }
        swipe(
            &connection,
            Point { x: 0.2, y: 0.5 },
            Point { x: 0.8, y: 0.5 },
        );
        thread::sleep(Duration::from_secs(1));
        swipe(
            &connection,
            Point { x: 0.5, y: 0.3 },
            Point { x: 0.5, y: 0.6 },
        );
        thread::sleep(Duration::from_millis(1500));
        // s h i p y a r d
        for usage in [0x16, 0x0B, 0x0C, 0x13, 0x1C, 0x04, 0x15, 0x07] {
            connection.send(Message::key(usage, DOWN, false));
            thread::sleep(Duration::from_millis(40));
            connection.send(Message::key(usage, UP, false));
            thread::sleep(Duration::from_millis(40));
        }
        thread::sleep(Duration::from_secs(2));
        press(&connection, USAGE_HOME);
        thread::sleep(Duration::from_secs(1));
        press(&connection, USAGE_LOCK);
        let locks = (0..6).any(|_| {
            thread::sleep(Duration::from_millis(500));
            locked(&udid)
        });
        assert!(locks, "Lock did not lock the device within 3 s");
    }

    /// Needs a booted simulator: `CHORD_SIM_UDID=<udid> cargo test -p chord-ios link_survives -- --ignored`. A Lock press must leave the link connected.
    #[test]
    #[ignore]
    fn link_survives_a_button() {
        let udid = std::env::var("CHORD_SIM_UDID").unwrap();
        let device = simulator_kit::load()
            .unwrap()
            .device_with_udid(&udid)
            .unwrap();
        let link = Link::connect(&device);
        let ready = |link: &Link| matches!(*link.lock(), State::Ready(_));
        assert!(
            (0..100).any(|_| {
                thread::sleep(Duration::from_millis(100));
                ready(&link)
            }),
            "the link did not connect within 10 s"
        );
        link.send(Input::Button {
            button: Button::Lock,
        });
        thread::sleep(PRESS * 4);
        assert!(ready(&link), "the press broke the link");
        link.send(Input::Button {
            button: Button::Home,
        });
        thread::sleep(PRESS * 4);
    }
}
