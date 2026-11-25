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

const BOOTED: u64 = 3;

pub(crate) struct Kit {
    devices: usize,
    previews: usize,
    pub version: Vec<u32>,
}

static KIT: OnceLock<Result<Kit, String>> = OnceLock::new();

pub(crate) fn loaded() -> Option<Result<&'static Kit, String>> {
    KIT.get().map(|kit| kit.as_ref().map_err(String::clone))
}

pub(crate) fn load() -> Result<&'static Kit, String> {
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
        format!("{developer}/../SharedFrameworks/SimulatorKit.framework/SimulatorKit"),
        format!("{developer}/Library/PrivateFrameworks/SimulatorKit.framework/SimulatorKit"),
    ] {
        if let Ok(path) = CString::new(path) {
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

fn preview_set(context: *mut AnyObject) -> usize {
    let with_path = sel!(deviceSetWithPath:error:);
    let Some(folder) = preview::devices_folder().filter(|_| responds(context, with_path)) else {
        return 0;
    };
    let folder = NSString::from_str(&folder.to_string_lossy());
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

    pub fn booted_device_named(&self, set: DeviceSet, name: &str) -> Option<Retained<AnyObject>> {
        self.device_where(set, |device| {
            is_booted(device) && text(get(device, sel!(name))).as_deref() == Some(name)
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
        let list = unsafe { &*(list as *const NSArray<AnyObject>) };
        list.iter()
            .find(|device| matches(Retained::as_ptr(device) as *mut AnyObject))
    }

    pub fn device_with_udid(&self, udid: &str) -> Option<Retained<AnyObject>> {
        [DeviceSet::Run, DeviceSet::Preview]
            .into_iter()
            .find_map(|set| {
                self.device_where(set, |device| udid_of(device).as_deref() == Some(udid))
            })
    }

    pub fn watch(&self, on_change: impl Fn() + 'static) -> RcBlock<dyn Fn(*mut AnyObject)> {
        let block =
            RcBlock::with_encoding::<_, _, _, ObjectBlock>(move |_: *mut AnyObject| on_change());
        for set in [DeviceSet::Run, DeviceSet::Preview] {
            if self.set(set).is_null() {
                continue;
            }
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

pub(crate) fn udid_of(device: *mut AnyObject) -> Option<String> {
    text(get(get(device, sel!(UDID)), sel!(UUIDString)))
}

pub(crate) fn is_booted(device: *mut AnyObject) -> bool {
    let state = sel!(state);
    responds(device, state)
        && unsafe { msg::<unsafe extern "C" fn(*mut AnyObject, Sel) -> u64>()(device, state) }
            == BOOTED
}

pub(crate) struct ObjectBlock;
unsafe impl ManualBlockEncoding for ObjectBlock {
    type Arguments = (*mut AnyObject,);
    type Return = ();
    const ENCODING_CSTR: &'static CStr = c"v16@?0@8";
}

pub(crate) struct VoidBlock;
unsafe impl ManualBlockEncoding for VoidBlock {
    type Arguments = ();
    type Return = ();
    const ENCODING_CSTR: &'static CStr = c"v8@?0";
}

pub(crate) unsafe fn msg<F: Copy>() -> F {
    unsafe { std::mem::transmute_copy(&(objc2::ffi::objc_msgSend as *const ())) }
}

pub(crate) fn guarded<R>(f: impl FnOnce() -> R) -> Option<R> {
    objc2::exception::catch(std::panic::AssertUnwindSafe(f)).ok()
}

pub(crate) fn class(name: &CStr) -> Option<*mut AnyObject> {
    AnyClass::get(name).map(|class| class as *const AnyClass as *mut AnyObject)
}

pub(crate) fn responds(object: *mut AnyObject, selector: Sel) -> bool {
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

pub(crate) fn get(object: *mut AnyObject, selector: Sel) -> *mut AnyObject {
    if !responds(object, selector) {
        return std::ptr::null_mut();
    }
    unsafe {
        msg::<unsafe extern "C" fn(*mut AnyObject, Sel) -> *mut AnyObject>()(object, selector)
    }
}

pub(crate) fn call1(
    object: *mut AnyObject,
    selector: Sel,
    argument: *mut AnyObject,
) -> *mut AnyObject {
    if !responds(object, selector) {
        return std::ptr::null_mut();
    }
    unsafe {
        msg::<unsafe extern "C" fn(*mut AnyObject, Sel, *mut AnyObject) -> *mut AnyObject>()(
            object, selector, argument,
        )
    }
}

pub(crate) fn text(object: *mut AnyObject) -> Option<String> {
    unsafe { (get(object, sel!(description)) as *const NSString).as_ref() }
        .map(|string| string.to_string())
}

pub(crate) unsafe fn symbol<F: Copy>(name: &CStr) -> Option<F> {
    let pointer = unsafe { libc::dlsym(libc::RTLD_DEFAULT, name.as_ptr()) };
    (!pointer.is_null()).then(|| unsafe { std::mem::transmute_copy(&pointer) })
}
