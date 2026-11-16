//! Xcode's private CoreSimulator and SimulatorKit, loaded once from the selected Xcode: the default device set and Chord's preview device set, a device by name, the sets' change notifications, and the checks every private call goes through.

use std::ffi::{CStr, CString, c_void};
use std::sync::OnceLock;

use block2::{ManualBlockEncoding, RcBlock};
use dispatch2::DispatchQueue;
use objc2::rc::{Retained, autoreleasepool};
use objc2::runtime::{AnyClass, AnyObject, Bool, Sel};
use objc2::sel;
use objc2_foundation::{NSArray, NSString};

use crate::preview;
use crate::simulator::{self, DeviceSet};

/// `SimDevice.state` of a booted device.
const BOOTED: u64 = 3;

pub(crate) struct Kit {
    /// The default `SimDeviceSet`, retained for the app's life. A pointer, so `Kit` can sit in a static.
    devices: usize,
    /// The preview device set the same way; 0 when this Xcode cannot open a set by its path.
    previews: usize,
    /// CoreSimulator's `CFBundleVersion`, such as `[1174, 9, 2]`.
    pub version: Vec<u32>,
}

/// Loads the frameworks once per app run. Err is the text the Simulator view shows.
pub(crate) fn load() -> Result<&'static Kit, String> {
    static KIT: OnceLock<Result<Kit, String>> = OnceLock::new();
    KIT.get_or_init(|| {
        autoreleasepool(|_| guarded(init)).unwrap_or_else(|| {
            Err("Xcode's simulator frameworks changed in a way Chord does not know.".into())
        })
    })
    .as_ref()
    .map_err(String::clone)
}

fn init() -> Result<Kit, String> {
    let developer = simulator::developer_dir()
        .ok_or("Chord could not find the selected Xcode.")?
        .to_string_lossy()
        .into_owned();
    for path in [
        "/Library/Developer/PrivateFrameworks/CoreSimulator.framework/CoreSimulator".to_string(),
        format!("{developer}/../SharedFrameworks/SimulatorKit.framework/SimulatorKit"), // Xcode 27+
        format!("{developer}/Library/PrivateFrameworks/SimulatorKit.framework/SimulatorKit"),
    ] {
        if let Ok(path) = CString::new(path) {
            // SAFETY: a C string path; a missing file returns null.
            unsafe { libc::dlopen(path.as_ptr(), libc::RTLD_NOW) };
        }
    }
    let unknown = || "Xcode's simulator frameworks did not load.".to_string();
    let bundle = call1(
        class(c"NSBundle").ok_or_else(unknown)?,
        sel!(bundleWithIdentifier:),
        &*NSString::from_str("com.apple.CoreSimulator") as *const NSString as *mut AnyObject,
    );
    let version = call1(
        bundle,
        sel!(objectForInfoDictionaryKey:),
        &*NSString::from_str("CFBundleVersion") as *const NSString as *mut AnyObject,
    );
    let version = simulator::dotted_version(&text(version).ok_or_else(unknown)?);
    let context = class(c"SimServiceContext").ok_or_else(unknown)?;
    let shared = sel!(sharedServiceContextForDeveloperDir:error:);
    if !responds(context, shared) {
        return Err(unknown());
    }
    let developer = NSString::from_str(&developer);
    // SAFETY: the selector exists; it takes an NSString and an optional error out-pointer.
    let context = unsafe {
        msg::<
            unsafe extern "C" fn(
                *mut AnyObject,
                Sel,
                *const NSString,
                *mut c_void,
            ) -> *mut AnyObject,
        >()(context, shared, &*developer, std::ptr::null_mut())
    };
    let set = call1(
        context,
        sel!(defaultDeviceSetWithError:),
        std::ptr::null_mut(),
    );
    // SAFETY: an object pointer or null.
    let set = unsafe { Retained::retain(set) }.ok_or_else(unknown)?;
    let devices = Retained::as_ptr(&set) as *mut AnyObject;
    if !responds(devices, sel!(devices))
        || !responds(devices, sel!(registerNotificationHandlerOnQueue:handler:))
    {
        return Err(unknown());
    }
    Ok(Kit {
        devices: Retained::into_raw(set) as usize,
        previews: preview_set(context),
        version,
    })
}

/// Chord's preview device set from the service context, retained for the app's life; 0 when it does not open.
fn preview_set(context: *mut AnyObject) -> usize {
    let with_path = sel!(deviceSetWithPath:error:);
    let Some(folder) = preview::devices_folder().filter(|_| responds(context, with_path)) else {
        return 0;
    };
    let folder = NSString::from_str(&folder.to_string_lossy());
    // SAFETY: the selector exists; it takes an NSString and an optional error out-pointer.
    let set = unsafe {
        msg::<
            unsafe extern "C" fn(
                *mut AnyObject,
                Sel,
                *const NSString,
                *mut c_void,
            ) -> *mut AnyObject,
        >()(context, with_path, &*folder, std::ptr::null_mut())
    };
    // SAFETY: an object pointer or null.
    unsafe { Retained::retain(set) }
        .filter(|set| {
            let set = Retained::as_ptr(set) as *mut AnyObject;
            responds(set, sel!(devices))
                && responds(set, sel!(registerNotificationHandlerOnQueue:handler:))
        })
        .map_or(0, |set| Retained::into_raw(set) as usize)
}

impl Kit {
    fn set(&self, set: DeviceSet) -> *mut AnyObject {
        match set {
            DeviceSet::Run => self.devices as *mut AnyObject,
            DeviceSet::Preview => self.previews as *mut AnyObject,
        }
    }

    /// The device of the set named `name`, read from CoreSimulator's live list (no simctl call).
    pub fn device_named(&self, set: DeviceSet, name: &str) -> Option<Retained<AnyObject>> {
        self.device_where(set, |device| {
            text(get(device, sel!(name))).as_deref() == Some(name)
        })
    }

    fn device_where(
        &self,
        set: DeviceSet,
        matches: impl Fn(*mut AnyObject) -> bool,
    ) -> Option<Retained<AnyObject>> {
        let list = get(self.set(set), sel!(devices));
        if list.is_null() {
            return None;
        }
        // SAFETY: `devices` returns an NSArray of SimDevice.
        let list = unsafe { &*(list as *const NSArray<AnyObject>) };
        list.iter()
            .find(|device| matches(Retained::as_ptr(device) as *mut AnyObject))
    }

    /// The device of either set with the UDID.
    pub fn device_with_udid(&self, udid: &str) -> Option<Retained<AnyObject>> {
        [DeviceSet::Run, DeviceSet::Preview]
            .into_iter()
            .find_map(|set| {
                self.device_where(set, |device| udid_of(device).as_deref() == Some(udid))
            })
    }

    /// Calls `on_change` on the main queue for every notification of both sets: a device added, removed, or changing state or boot status. Keep the returned block for as long as it should fire.
    pub fn watch(&self, on_change: impl Fn() + 'static) -> RcBlock<dyn Fn(*mut AnyObject)> {
        let block =
            RcBlock::with_encoding::<_, _, _, ObjectBlock>(move |_: *mut AnyObject| on_change());
        for set in [DeviceSet::Run, DeviceSet::Preview] {
            if self.set(set).is_null() {
                continue;
            }
            // SAFETY: the selector exists (checked in `init`); it takes a queue and a `void (^)(NSDictionary *)`.
            unsafe {
                msg::<
                    unsafe extern "C" fn(
                        *mut AnyObject,
                        Sel,
                        *const DispatchQueue,
                        *const c_void,
                    ) -> u64,
                >()(
                    self.set(set),
                    sel!(registerNotificationHandlerOnQueue:handler:),
                    DispatchQueue::main(),
                    &*block as *const _ as *const c_void,
                )
            };
        }
        block
    }
}

/// The device's UDID.
pub(crate) fn udid_of(device: *mut AnyObject) -> Option<String> {
    text(get(get(device, sel!(UDID)), sel!(UUIDString)))
}

pub(crate) fn is_booted(device: *mut AnyObject) -> bool {
    let state = sel!(state);
    // SAFETY: `state` returns an unsigned long.
    responds(device, state)
        && unsafe { msg::<unsafe extern "C" fn(*mut AnyObject, Sel) -> u64>()(device, state) }
            == BOOTED
}

/// `void (^)(id)`. CoreSimulator hands handlers to a remote proxy, which needs the block's signature.
pub(crate) struct ObjectBlock;
// SAFETY: the encoding matches `(id) -> void`.
unsafe impl ManualBlockEncoding for ObjectBlock {
    type Arguments = (*mut AnyObject,);
    type Return = ();
    const ENCODING_CSTR: &'static CStr = c"v16@?0@8";
}

/// `void (^)(void)`.
pub(crate) struct VoidBlock;
// SAFETY: the encoding matches `() -> void`.
unsafe impl ManualBlockEncoding for VoidBlock {
    type Arguments = ();
    type Return = ();
    const ENCODING_CSTR: &'static CStr = c"v8@?0";
}

/// `objc_msgSend` typed as `F`. Private classes declare odd type encodings (Swift pointers), so these calls skip objc2's checks.
///
/// # Safety
/// `F` must match the selector's real signature.
pub(crate) unsafe fn msg<F: Copy>() -> F {
    // SAFETY: a function pointer transmuted to another function pointer type of the same size.
    unsafe { std::mem::transmute_copy(&(objc2::ffi::objc_msgSend as *const ())) }
}

/// Runs a private-API call, turning an Objective-C exception into `None`: a changed SimulatorKit must never crash Chord.
pub(crate) fn guarded<R>(f: impl FnOnce() -> R) -> Option<R> {
    objc2::exception::catch(std::panic::AssertUnwindSafe(f)).ok()
}

pub(crate) fn class(name: &CStr) -> Option<*mut AnyObject> {
    AnyClass::get(name).map(|class| class as *const AnyClass as *mut AnyObject)
}

pub(crate) fn responds(object: *mut AnyObject, selector: Sel) -> bool {
    // SAFETY: every object answers `respondsToSelector:`.
    !object.is_null()
        && unsafe {
            msg::<unsafe extern "C" fn(*mut AnyObject, Sel, Sel) -> Bool>()(
                object,
                sel!(respondsToSelector:),
                selector,
            )
        }
        .as_bool()
}

/// `[object selector]` for a getter returning an object or pointer; null when `object` is null or does not know `selector`.
pub(crate) fn get(object: *mut AnyObject, selector: Sel) -> *mut AnyObject {
    if !responds(object, selector) {
        return std::ptr::null_mut();
    }
    // SAFETY: the object knows the selector; callers only pass getters that return an object or pointer.
    unsafe {
        msg::<unsafe extern "C" fn(*mut AnyObject, Sel) -> *mut AnyObject>()(object, selector)
    }
}

/// `[object selector argument]` for a method taking and returning an object or pointer.
pub(crate) fn call1(
    object: *mut AnyObject,
    selector: Sel,
    argument: *mut AnyObject,
) -> *mut AnyObject {
    if !responds(object, selector) {
        return std::ptr::null_mut();
    }
    // SAFETY: as `get`, with one object argument.
    unsafe {
        msg::<unsafe extern "C" fn(*mut AnyObject, Sel, *mut AnyObject) -> *mut AnyObject>()(
            object, selector, argument,
        )
    }
}

/// The text of an NSString, or of any object's `description`.
pub(crate) fn text(object: *mut AnyObject) -> Option<String> {
    let string = get(object, sel!(description));
    // SAFETY: `description` returns an NSString.
    (!string.is_null()).then(|| unsafe { &*(string as *const NSString) }.to_string())
}

/// A C symbol of the loaded frameworks, typed as `F`.
///
/// # Safety
/// `F` must match the symbol's real signature.
pub(crate) unsafe fn symbol<F: Copy>(name: &CStr) -> Option<F> {
    // SAFETY: a C string name; a missing symbol returns null.
    let pointer = unsafe { libc::dlsym(libc::RTLD_DEFAULT, name.as_ptr()) };
    // SAFETY: the caller names the type.
    (!pointer.is_null()).then(|| unsafe { std::mem::transmute_copy(&pointer) })
}
