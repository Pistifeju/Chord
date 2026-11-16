//! The Simulator view's native part: one NSView under the webview shows the shown worktree's simulator in its bezel (its framebuffer IOSurface as CALayer contents, no copies), follows it booting and shutting down, restarts a wedged boot once, and passes the view's input to the device.

use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::c_void;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

use block2::RcBlock;
use dispatch2::{DispatchQueue, DispatchTime};
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, Bool, Sel};
use objc2::{MainThreadMarker, sel};
use objc2_app_kit::{NSView, NSWindowOrderingMode};
use objc2_core_foundation::{CGPoint, CGRect, CGSize};
use objc2_foundation::{NSArray, NSUUID};
use objc2_quartz_core::{CALayer, CATransaction};
use serde::{Deserialize, Serialize};
use specta::Type;

use crate::hid::{Blocking, Button, Hid, Input};
use crate::screen_buttons::{self, ScreenButtons, srgb};
use crate::simulator;
use crate::simulator_kit::{
    self, VoidBlock, call1, class, get, guarded, is_booted, msg, responds, text, udid_of,
};

/// How often an attached device without a screen yet gets its frame callbacks again: SimulatorKit creates the framebuffer descriptors lazily.
const RETRY: Duration = Duration::from_secs(1);
/// How long after a Lock or Home press the lock state gets read again: SpringBoard reports it only after its animation, and not at all when the press changed nothing.
const LOCK_RECHECK: Duration = Duration::from_millis(1500);
/// How long after a Home on a locked device the lock state gets read: still locked means the press only woke the screen.
const WAKE_CHECK: Duration = Duration::from_millis(400);
/// Retries in a row without any framebuffer descriptor that make a wedged boot.
const WEDGED_AFTER: u32 = 5;

/// Where the screen goes, in the webview's CSS pixels from its top left; the bezel goes around it.
#[derive(Deserialize, Type, Clone, Copy, Debug, PartialEq, Default)]
pub struct ScreenRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// The phone's frame around the screen: the screen's corner radius and the window's colors as sRGB bytes. Drawn natively, so frame and screen move in the same frame.
#[derive(Deserialize, Type, Clone, Copy, Debug, PartialEq, Default)]
pub struct Bezel {
    pub radius: f64,
    /// The canvas under the frame's rounded corners.
    pub canvas: [u8; 3],
    pub frame: [u8; 3],
    /// The 1px line outside the frame.
    pub edge: [u8; 3],
}

/// The frame's width and its edge's, outside the screen.
const FRAME_WIDTH: f64 = 8.0;
const EDGE_WIDTH: f64 = 1.0;
/// Canvas around the bezel, under the web. The web's hole reaches 32px past the bezel and the buttons under it (`HOLE_MARGIN`): on a resize either side can run a frame ahead, and the hole then still opens onto this canvas, never the window's background, and never covers the bezel or the buttons.
const CANVAS_MARGIN: f64 = 160.0;

#[derive(Serialize, Type, Clone, Debug, PartialEq)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum ScreenState {
    /// Frames show; the screen's size in pixels and whether SpringBoard is locked.
    Live {
        width: u32,
        height: u32,
        locked: bool,
    },
    /// No booted simulator, or one with no screen yet; `message` says why when it is more than off.
    Off { message: Option<String> },
    /// The private API did not load; `app` names the simulator app to open instead.
    Unavailable { message: String, app: String },
}

#[link(name = "IOSurface", kind = "framework")]
unsafe extern "C" {
    fn IOSurfaceGetWidth(surface: *const c_void) -> usize;
    fn IOSurfaceGetHeight(surface: *const c_void) -> usize;
    fn IOSurfaceGetSeed(surface: *const c_void) -> u32;
}

/// The framebuffer to show among the descriptors' surface sizes: the one of the device's native size, never Xcode 27's extra 7680×4320 presentation surface. Without a native size, the largest live one.
fn pick_surface(sizes: &[(usize, usize)], native: Option<(usize, usize)>) -> Option<usize> {
    let live = || {
        sizes
            .iter()
            .enumerate()
            .filter(|(_, size)| size.0 > 0 && size.1 > 0)
    };
    let native_or_turned = |size: &(usize, usize)| {
        native.is_some_and(|native| *size == native || *size == (native.1, native.0))
    };
    live()
        .find(|(_, size)| native_or_turned(size))
        .or_else(|| live().max_by_key(|(_, size)| size.0 * size.1))
        .map(|(index, _)| index)
}

/// The device type's native screen size in pixels (`SimDeviceType.mainScreenSize`).
// shortcut: skips capabilities.plist, which only foldables need; read it when foldables come in.
fn native_size(device: *mut AnyObject) -> Option<(usize, usize)> {
    let kind = get(device, sel!(deviceType));
    if !responds(kind, sel!(mainScreenSize)) {
        return None;
    }
    // SAFETY: `mainScreenSize` returns a CGSize.
    let size = unsafe {
        msg::<unsafe extern "C" fn(*mut AnyObject, Sel) -> CGSize>()(kind, sel!(mainScreenSize))
    };
    (size.width.is_finite() && size.height.is_finite() && size.width >= 1.0 && size.height >= 1.0)
        .then(|| (size.width.round() as usize, size.height.round() as usize))
}

fn register_selector() -> Sel {
    sel!(registerScreenCallbacksWithUUID:callbackQueue:frameCallback:surfacesChangedCallback:propertiesChangedCallback:)
}

/// The device's framebuffer display descriptors (main screen and overlay planes).
fn descriptors(device: *mut AnyObject) -> Vec<Retained<AnyObject>> {
    let io = get(device, sel!(io));
    get(io, sel!(updateIOPorts));
    let ports = get(io, sel!(deviceIOPorts));
    if ports.is_null() {
        return vec![];
    }
    // SAFETY: `deviceIOPorts` returns an NSArray.
    let ports = unsafe { &*(ports as *const NSArray<AnyObject>) };
    ports
        .iter()
        .filter_map(|port| {
            let port = Retained::as_ptr(&port) as *mut AnyObject;
            if text(get(port, sel!(portIdentifier))).as_deref()
                != Some("com.apple.framebuffer.display")
            {
                return None;
            }
            let descriptor = get(port, sel!(descriptor));
            (responds(descriptor, sel!(framebufferSurface))
                && responds(descriptor, register_selector()))
            // SAFETY: an object pointer.
            .then(|| unsafe { Retained::retain(descriptor) })
            .flatten()
        })
        .collect()
}

/// Registers `on_frame` for every frame and surface change of the descriptors, on `queue`. SimulatorKit only fills `framebufferSurface` once a client listens.
fn register(
    descriptors: &[Retained<AnyObject>],
    uuid: &NSUUID,
    queue: &DispatchQueue,
    on_frame: &RcBlock<dyn Fn()>,
) {
    let nothing = RcBlock::with_encoding::<_, _, _, VoidBlock>(|| {});
    for descriptor in descriptors {
        // SAFETY: checked in `descriptors`; SimulatorKit copies the blocks.
        unsafe {
            msg::<
                unsafe extern "C" fn(
                    *mut AnyObject,
                    Sel,
                    *const NSUUID,
                    *const DispatchQueue,
                    *const c_void,
                    *const c_void,
                    *const c_void,
                ),
            >()(
                Retained::as_ptr(descriptor) as *mut AnyObject,
                register_selector(),
                uuid,
                queue,
                &**on_frame as *const _ as *const c_void,
                &**on_frame as *const _ as *const c_void,
                &*nothing as *const _ as *const c_void,
            )
        };
    }
}

fn unregister(descriptors: &[Retained<AnyObject>], uuid: &NSUUID) {
    for descriptor in descriptors {
        call1(
            Retained::as_ptr(descriptor) as *mut AnyObject,
            sel!(unregisterScreenCallbacksWithUUID:),
            uuid as *const NSUUID as *mut AnyObject,
        );
    }
}

fn surface_size(surface: *mut AnyObject) -> (usize, usize) {
    if surface.is_null() {
        return (0, 0);
    }
    // SAFETY: an IOSurface.
    unsafe {
        (
            IOSurfaceGetWidth(surface as _),
            IOSurfaceGetHeight(surface as _),
        )
    }
}

/// The window's backing scale, for layers that draw their own contents.
fn backing_scale(view: &NSView) -> f64 {
    let window = get(view as *const NSView as *mut AnyObject, sel!(window));
    if !responds(window, sel!(backingScaleFactor)) {
        return 2.0;
    }
    // SAFETY: `backingScaleFactor` returns a CGFloat.
    unsafe {
        msg::<unsafe extern "C" fn(*mut AnyObject, Sel) -> f64>()(window, sel!(backingScaleFactor))
    }
}

/// The webview paints no background while the screen shows through it.
fn set_draws_background(webview: &NSView, draws: bool) {
    let Some(number) = class(c"NSNumber") else {
        return;
    };
    // SAFETY: `numberWithBool:` returns an autoreleased NSNumber; WKWebView takes `drawsBackground` by key-value coding.
    unsafe {
        let value = msg::<unsafe extern "C" fn(*mut AnyObject, Sel, Bool) -> *mut AnyObject>()(
            number,
            sel!(numberWithBool:),
            Bool::new(draws),
        );
        let key = objc2_foundation::NSString::from_str("drawsBackground");
        msg::<
            unsafe extern "C" fn(
                *const NSView,
                Sel,
                *mut AnyObject,
                *const objc2_foundation::NSString,
            ),
        >()(webview, sel!(setValue:forKey:), value, &*key);
    }
}

/// The device the view shows, with its frame callbacks.
struct Attached {
    udid: String,
    device: Retained<AnyObject>,
    descriptors: Vec<Retained<AnyObject>>,
    uuid: Retained<NSUUID>,
    native: Option<(usize, usize)>,
    /// Retries in a row that found no framebuffer descriptor.
    missing: u32,
    /// Shown surface and its seed: an unchanged seed means no new pixels.
    shown: (usize, u32),
    size: (usize, usize),
    locked: bool,
    /// The lock state watch's process group.
    lock_watch: Option<u32>,
}

struct Shown {
    worktree: PathBuf,
    /// The worktree's simulator name.
    name: String,
    on_state: Box<dyn Fn(ScreenState)>,
}

/// Main-thread state of the one screen view in the window.
struct Screen {
    webview: Retained<NSView>,
    view: Retained<NSView>,
    /// The bezel, inside the view's canvas-colored layer.
    frame: Retained<CALayer>,
    /// The screen, inside the frame.
    layer: Retained<CALayer>,
    /// Home, Lock and Shake under the frame, once the web has laid them out.
    capsule: Retained<CALayer>,
    buttons: Option<ScreenButtons>,
    /// The device set's notification handler, for the app's life.
    _watch: RcBlock<dyn Fn(*mut AnyObject)>,
    shown: Option<Shown>,
    rect: ScreenRect,
    bezel: Bezel,
    attached: Option<Attached>,
    /// Bumped on every attach and detach: callbacks of an older attach do nothing.
    generation: u64,
    sent: Option<ScreenState>,
    /// One input link per booted device, until it shuts down: a second Indigo client in one process gets its touches dropped.
    links: HashMap<String, Hid>,
    /// Device restarted once for a wedged boot; a second wedge shows the error instead.
    restarted: Option<String>,
    restarting: bool,
    /// Device still wedged after its restart: shown as off with the error until it shuts down.
    wedged: Option<String>,
}

thread_local! {
    static SCREEN: RefCell<Option<Screen>> = const { RefCell::new(None) };
}

/// Runs `f` on the screen state; skips when it is borrowed or not created yet.
fn with(f: impl FnOnce(&mut Screen)) {
    SCREEN.with(|screen| {
        if let Ok(mut screen) = screen.try_borrow_mut()
            && let Some(screen) = screen.as_mut()
        {
            guarded(|| f(screen));
        }
    });
}

fn later(generation: u64, f: fn(&mut Screen)) {
    if let Ok(when) = DispatchTime::try_from(RETRY) {
        let _ = DispatchQueue::main().after(when, move || {
            with(|screen| {
                if screen.generation == generation {
                    f(screen);
                }
            })
        });
    }
}

const WEDGED: &str = "The simulator booted without a screen. Shut it down and boot it again.";

impl Screen {
    fn create(mtm: MainThreadMarker, webview: Retained<NSView>) -> Option<Screen> {
        let kit = simulator_kit::load().ok()?;
        // SAFETY: AppKit's own view hierarchy.
        let parent = unsafe { webview.superview() }?;
        let view = NSView::new(mtm);
        view.setWantsLayer(true);
        let frame = CALayer::new();
        let layer = CALayer::new();
        layer.setMasksToBounds(true);
        frame.addSublayer(&layer);
        let capsule = CALayer::new();
        capsule.setHidden(true);
        let root = view.layer()?;
        root.addSublayer(&frame);
        root.addSublayer(&capsule);
        view.setHidden(true);
        parent.addSubview_positioned_relativeTo(&view, NSWindowOrderingMode::Below, Some(&webview));
        let watch = kit.watch(|| with(Screen::sync));
        Some(Screen {
            webview,
            view,
            frame,
            layer,
            capsule,
            buttons: None,
            _watch: watch,
            shown: None,
            rect: ScreenRect::default(),
            bezel: Bezel::default(),
            attached: None,
            generation: 0,
            sent: None,
            links: HashMap::new(),
            restarted: None,
            restarting: false,
            wedged: None,
        })
    }

    fn send_state(&mut self, state: ScreenState) {
        let Some(shown) = &self.shown else { return };
        if self.sent.as_ref() != Some(&state) {
            (shown.on_state)(state.clone());
            self.sent = Some(state);
        }
    }

    fn off(&mut self, message: Option<&str>) {
        self.send_state(ScreenState::Off {
            message: message.map(String::from),
        });
    }

    fn place(&mut self) {
        let Some(parent) = (unsafe { self.webview.superview() }) else {
            return;
        };
        let web = self.webview.frame();
        let bezel_width = FRAME_WIDTH + EDGE_WIDTH;
        let outset = bezel_width + CANVAS_MARGIN;
        let rect = self.rect;
        let screen = CGSize::new(rect.width.max(0.0), rect.height.max(0.0));
        let framed = CGSize::new(
            screen.width + 2.0 * bezel_width,
            screen.height + 2.0 * bezel_width,
        );
        let size = CGSize::new(screen.width + 2.0 * outset, screen.height + 2.0 * outset);
        let y = if parent.isFlipped() {
            web.origin.y + rect.y - outset
        } else {
            web.origin.y + web.size.height - rect.y - rect.height - outset
        };
        let bezel = self.bezel;
        CATransaction::begin();
        CATransaction::setDisableActions(true);
        self.view.setFrame(CGRect::new(
            CGPoint::new(web.origin.x + rect.x - outset, y),
            size,
        ));
        if let Some(root) = self.view.layer() {
            root.setBackgroundColor(Some(&srgb(bezel.canvas)));
        }
        self.frame.setFrame(CGRect::new(
            CGPoint::new(CANVAS_MARGIN, CANVAS_MARGIN),
            framed,
        ));
        self.frame.setCornerRadius(bezel.radius + bezel_width);
        self.frame.setBorderWidth(EDGE_WIDTH);
        self.frame.setBackgroundColor(Some(&srgb(bezel.frame)));
        self.frame.setBorderColor(Some(&srgb(bezel.edge)));
        self.layer
            .setFrame(CGRect::new(CGPoint::new(bezel_width, bezel_width), screen));
        self.layer.setCornerRadius(bezel.radius);
        self.capsule.setHidden(self.buttons.is_none());
        if let Some(buttons) = &self.buttons {
            self.capsule.setFrame(screen_buttons::capsule_frame(
                buttons,
                size.height,
                CGPoint::new(outset, outset),
                screen,
            ));
        }
        CATransaction::commit();
    }

    /// The view shows only while a frame is on it; until then the web shows the state under the hole.
    fn update_hidden(&self) {
        let live = self
            .attached
            .as_ref()
            .is_some_and(|attached| attached.shown.0 != 0);
        self.view.setHidden(self.shown.is_none() || !live);
    }

    /// Follows the shown worktree's simulator: attaches when it is booted, detaches when it is not. Runs on show and on every device set notification.
    fn sync(&mut self) {
        let Ok(kit) = simulator_kit::load() else {
            return;
        };
        self.links.retain(|udid, _| {
            kit.device_with_udid(udid)
                .is_some_and(|device| is_booted(Retained::as_ptr(&device) as *mut AnyObject))
        });
        let Some(shown) = &self.shown else { return };
        if self.restarting {
            return;
        }
        let booted = kit
            .device_named(&shown.name)
            .filter(|device| is_booted(Retained::as_ptr(device) as *mut AnyObject));
        let udid = booted
            .as_ref()
            .and_then(|device| udid_of(Retained::as_ptr(device) as *mut AnyObject));
        match (booted, udid) {
            (Some(_), Some(udid)) if self.wedged.as_deref() == Some(&udid) => {}
            (Some(_), Some(udid)) if self.attached.as_ref().is_some_and(|a| a.udid == udid) => {}
            (Some(device), Some(udid)) => {
                self.detach();
                self.attach(kit, udid, device);
            }
            _ => {
                self.wedged = None;
                self.detach();
                self.off(None);
            }
        }
        self.update_hidden();
    }

    fn attach(&mut self, kit: &simulator_kit::Kit, udid: String, device: Retained<AnyObject>) {
        self.generation += 1;
        if !self.links.contains_key(&udid)
            && let Some(link) = Hid::new(&kit.version, udid.clone(), &device)
        {
            self.links.insert(udid.clone(), link);
        }
        let native = native_size(Retained::as_ptr(&device) as *mut AnyObject);
        let generation = self.generation;
        let lock_watch = simulator::watch_lock(&udid, move |locked| {
            DispatchQueue::main().exec_async(move || {
                with(|screen| {
                    if screen.generation == generation {
                        screen.set_locked(locked);
                    }
                })
            });
        });
        self.attached = Some(Attached {
            udid,
            device,
            descriptors: vec![],
            uuid: NSUUID::new(),
            native,
            missing: 0,
            shown: (0, 0),
            size: (0, 0),
            locked: false,
            lock_watch,
        });
        self.off(Some("Waiting for the simulator's screen"));
        self.wire();
        later(self.generation, Screen::check_screen);
    }

    fn wire(&mut self) {
        let generation = self.generation;
        let Some(attached) = self.attached.as_mut() else {
            return;
        };
        unregister(&attached.descriptors, &attached.uuid);
        attached.descriptors = descriptors(Retained::as_ptr(&attached.device) as *mut AnyObject);
        let on_frame = RcBlock::with_encoding::<_, _, _, VoidBlock>(move || {
            with(|screen| {
                if screen.generation == generation {
                    screen.refresh();
                }
            })
        });
        register(
            &attached.descriptors,
            &attached.uuid,
            DispatchQueue::main(),
            &on_frame,
        );
        self.refresh();
    }

    fn detach(&mut self) {
        if let Some(attached) = self.attached.take() {
            unregister(&attached.descriptors, &attached.uuid);
            if let Some(group) = attached.lock_watch {
                simulator::stop_lock_watch(group);
            }
        }
        self.generation += 1;
        // SAFETY: clearing the contents.
        unsafe { self.layer.setContents(None) };
        self.update_hidden();
    }

    /// No screen yet: listen again. A booted device with no framebuffer descriptor at all for `WEDGED_AFTER` retries is a wedged boot: it gets restarted once; a second wedge shows the error.
    fn check_screen(&mut self) {
        let Some(attached) = self.attached.as_mut() else {
            return;
        };
        if attached.shown.0 != 0 {
            return;
        }
        attached.missing = if attached.descriptors.is_empty() {
            attached.missing + 1
        } else {
            0
        };
        if attached.missing < WEDGED_AFTER {
            self.wire();
            return later(self.generation, Screen::check_screen);
        }
        let udid = attached.udid.clone();
        self.detach();
        if self.restarted.as_deref() == Some(&udid) {
            self.wedged = Some(udid);
            return self.off(Some(WEDGED));
        }
        self.restarted = Some(udid.clone());
        self.restarting = true;
        self.off(Some("Restarting the simulator"));
        thread::spawn(move || {
            simulator::shut_down(&udid);
            let _ = simulator::boot(&udid);
            DispatchQueue::main().exec_async(|| {
                with(|screen| {
                    screen.restarting = false;
                    screen.sync();
                })
            });
        });
    }

    /// Points the layer at the current framebuffer surface. Runs on every frame callback.
    fn refresh(&mut self) {
        let Some(attached) = self.attached.as_mut() else {
            return;
        };
        let surfaces: Vec<*mut AnyObject> = attached
            .descriptors
            .iter()
            .map(|descriptor| {
                get(
                    Retained::as_ptr(descriptor) as *mut AnyObject,
                    sel!(framebufferSurface),
                )
            })
            .collect();
        let sizes: Vec<_> = surfaces
            .iter()
            .map(|surface| surface_size(*surface))
            .collect();
        let Some(index) = pick_surface(&sizes, attached.native) else {
            return;
        };
        let surface = surfaces[index];
        // SAFETY: an IOSurface.
        let seed = unsafe { IOSurfaceGetSeed(surface as _) };
        if attached.shown == (surface as usize, seed) {
            return;
        }
        let same = attached.shown.0 == surface as usize;
        attached.shown = (surface as usize, seed);
        CATransaction::begin();
        CATransaction::setDisableActions(true);
        let layer = Retained::as_ptr(&self.layer) as *mut AnyObject;
        // Core Animation does not redraw when the same IOSurface changes underneath: tell it.
        if same && responds(layer, sel!(setContentsChanged)) {
            get(layer, sel!(setContentsChanged));
        } else {
            // SAFETY: an IOSurface is valid layer contents.
            unsafe {
                self.layer.setContents(None);
                self.layer.setContents(Some(&*surface));
            }
        }
        CATransaction::commit();
        if attached.size != sizes[index] {
            // A screen came up: a later wedge of this device gets its restart again.
            if attached.size == (0, 0) {
                self.restarted = None;
            }
            attached.size = sizes[index];
            self.send_live();
            self.update_hidden();
        }
    }

    fn set_locked(&mut self, locked: bool) {
        if let Some(attached) = self.attached.as_mut() {
            attached.locked = locked;
            self.send_live();
        }
    }

    /// Reads the lock state again `after` a press, which corrects a wrong guess. A Home that only woke the lock screen (`unlocking` and still locked) gets a second Home, which unlocks.
    fn recheck_lock(&self, after: Duration, unlocking: bool) {
        let Some(attached) = &self.attached else {
            return;
        };
        let (udid, generation) = (attached.udid.clone(), self.generation);
        thread::spawn(move || {
            thread::sleep(after);
            let Some(locked) = simulator::lock_state(&udid) else {
                return;
            };
            DispatchQueue::main().exec_async(move || {
                with(|screen| {
                    if screen.generation != generation {
                        return;
                    }
                    if !(unlocking && locked) {
                        return screen.set_locked(locked);
                    }
                    let home = Input::Button {
                        button: Button::Home,
                    };
                    if let Some(blocking) =
                        screen.links.get_mut(&udid).and_then(|link| link.send(home))
                    {
                        thread::spawn(move || blocking.run());
                    }
                    screen.recheck_lock(LOCK_RECHECK, false);
                })
            });
        });
    }

    /// Sends Live once the attached device has a screen.
    fn send_live(&mut self) {
        let Some(attached) = &self.attached else {
            return;
        };
        let ((width, height), locked) = (attached.size, attached.locked);
        if width > 0 {
            self.send_state(ScreenState::Live {
                width: width as u32,
                height: height as u32,
                locked,
            });
        }
    }
}

fn unavailable(message: String) -> ScreenState {
    let app = simulator::simulator_app()
        .and_then(|app| Some(app.file_stem()?.to_string_lossy().into_owned()))
        .unwrap_or_else(|| "Simulator".into());
    ScreenState::Unavailable { message, app }
}

/// Loads the private frameworks once, which finds the selected Xcode with a tool run: call it off the main thread before `show_screen`. The Unavailable state when they do not load.
pub fn prepare_screen() -> Option<ScreenState> {
    simulator_kit::load().err().map(unavailable)
}

/// Main thread. Shows the worktree's simulator under `webview` (a WKWebView) at `rect` in `bezel`; its state goes to `on_state`.
///
/// # Safety
/// `webview` must point to the window's live WKWebView.
pub unsafe fn show_screen(
    webview: *mut c_void,
    worktree: &Path,
    rect: ScreenRect,
    bezel: Bezel,
    on_state: impl Fn(ScreenState) + 'static,
) {
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    if let Some(state) = prepare_screen() {
        return on_state(state);
    }
    // SAFETY: the caller passes the live webview.
    let Some(webview) = (unsafe { Retained::retain(webview as *mut NSView) }) else {
        return;
    };
    SCREEN.with(|screen| {
        let Ok(mut screen) = screen.try_borrow_mut() else {
            return;
        };
        if screen.is_none() {
            *screen = guarded(|| Screen::create(mtm, webview.clone())).flatten();
        }
        let Some(screen) = screen.as_mut() else {
            return on_state(unavailable(
                "The Simulator view could not attach to the window.".into(),
            ));
        };
        guarded(|| {
            let name = simulator::name_of(worktree);
            if screen.shown.as_ref().is_none_or(|shown| shown.name != name) {
                screen.detach();
                screen.wedged = None;
            }
            screen.shown = Some(Shown {
                worktree: worktree.to_path_buf(),
                name,
                on_state: Box::new(on_state),
            });
            screen.sent = None;
            screen.rect = rect;
            screen.bezel = bezel;
            set_draws_background(&screen.webview, false);
            screen.place();
            screen.sync();
        });
    });
}

/// Main thread. Moves the shown screen to `rect` and redraws its bezel.
pub fn move_screen(rect: ScreenRect, bezel: Bezel) {
    with(|screen| {
        screen.rect = rect;
        screen.bezel = bezel;
        screen.place();
    });
}

/// Main thread. Draws the buttons under the shown screen as the web laid them out.
pub fn set_buttons(buttons: ScreenButtons) {
    with(|screen| {
        CATransaction::begin();
        CATransaction::setDisableActions(true);
        screen_buttons::draw(&screen.capsule, &buttons, backing_scale(&screen.view));
        CATransaction::commit();
        screen.buttons = Some(buttons);
        screen.place();
    });
}

/// Main thread. Hides the screen and drops its frame callbacks; the webview paints its background again.
pub fn hide_screen() {
    with(|screen| {
        screen.detach();
        screen.shown = None;
        screen.sent = None;
        set_draws_background(&screen.webview, true);
        screen.update_hidden();
    });
}

/// Main thread. Sends the view's input to the shown worktree's simulator; returns the tool run that carries it, if any.
pub fn send_input(worktree: &Path, input: Input) -> Option<Blocking> {
    let mut blocking = None;
    with(|screen| {
        let shows = screen
            .shown
            .as_ref()
            .is_some_and(|shown| shown.worktree == worktree);
        let link = screen
            .attached
            .as_ref()
            .filter(|_| shows)
            .and_then(|attached| screen.links.get_mut(&attached.udid));
        let Some(link) = link else { return };
        blocking = link.send(input);
        let Input::Button { button } = input else {
            return;
        };
        let unlocking = button == Button::Home
            && screen
                .attached
                .as_ref()
                .is_some_and(|attached| attached.locked);
        match button {
            Button::Lock => screen.set_locked(true),
            Button::Home => screen.set_locked(false),
            Button::Shake => return,
        }
        let after = if unlocking { WAKE_CHECK } else { LOCK_RECHECK };
        screen.recheck_lock(after, unlocking);
    });
    blocking
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn picks_the_native_surface() {
        let sizes = [(0, 0), (7680, 4320), (1320, 2868)];
        assert_eq!(pick_surface(&sizes, Some((1320, 2868))), Some(2));
        assert_eq!(pick_surface(&sizes, Some((2868, 1320))), Some(2));
        assert_eq!(pick_surface(&sizes, Some((100, 100))), Some(1));
        assert_eq!(pick_surface(&sizes, None), Some(1));
        assert_eq!(pick_surface(&[(0, 0)], None), None);
    }

    /// Needs a booted simulator: `CHORD_SIM_UDID=<udid> cargo test -p chord-ios frame_smoke -- --ignored --nocapture`.
    /// Waits for the first frame callback, then counts them while `simctl ui appearance` flips the screen; the appearance ends as it started.
    #[test]
    #[ignore]
    fn frame_smoke() {
        let udid = std::env::var("CHORD_SIM_UDID").unwrap();
        let device = simulator_kit::load()
            .unwrap()
            .device_with_udid(&udid)
            .unwrap();
        let descriptors = descriptors(Retained::as_ptr(&device) as *mut AnyObject);
        assert!(!descriptors.is_empty(), "no framebuffer descriptors");
        static FRAMES: AtomicUsize = AtomicUsize::new(0);
        let on_frame = RcBlock::with_encoding::<_, _, _, VoidBlock>(|| {
            FRAMES.fetch_add(1, Ordering::Relaxed);
        });
        let uuid = NSUUID::new();
        let queue = DispatchQueue::new("chord.frames.smoke", None);
        register(&descriptors, &uuid, &queue, &on_frame);
        let first = (0..50).any(|_| {
            thread::sleep(Duration::from_millis(100));
            FRAMES.load(Ordering::Relaxed) > 0
        });
        assert!(first, "no frame callback within 5 s");
        let appearance = |mode: Option<&str>| {
            std::process::Command::new("xcrun")
                .args(["simctl", "ui", &udid, "appearance"])
                .args(mode)
                .output()
                .unwrap()
        };
        let before = String::from_utf8_lossy(&appearance(None).stdout)
            .trim()
            .to_string();
        let idle = FRAMES.load(Ordering::Relaxed);
        appearance(Some(if before == "dark" { "light" } else { "dark" }));
        thread::sleep(Duration::from_millis(1500));
        appearance(Some(&before));
        thread::sleep(Duration::from_millis(500));
        let frames = FRAMES.load(Ordering::Relaxed);
        let sizes: Vec<_> = descriptors
            .iter()
            .map(|descriptor| {
                surface_size(get(
                    Retained::as_ptr(descriptor) as *mut AnyObject,
                    sel!(framebufferSurface),
                ))
            })
            .collect();
        unregister(&descriptors, &uuid);
        println!("surfaces {sizes:?}, frame callbacks {idle} before the flips, {frames} after");
        assert!(
            frames > idle,
            "no frame callbacks during the appearance flips"
        );
        let native = native_size(Retained::as_ptr(&device) as *mut AnyObject);
        assert!(pick_surface(&sizes, native).is_some(), "no live surface");
    }
}
