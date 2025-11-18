use once_cell::sync::Lazy;
use serde::Serialize;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_void};
use std::ptr;
use std::sync::Mutex;
use tokio::runtime::{Builder, Runtime};
use either::Either;

// Adapt this import to match your core crate
use ninja::{manager::ShurikenManager, types::ShurikenState};

// ========================
// Global Tokio runtime
// ========================1
static RUNTIME: Lazy<Runtime> = Lazy::new(|| {
    Builder::new_multi_thread()
        .enable_all()
        .worker_threads(4)
        .thread_name("ninja-ffi-runtime")
        .build()
        .expect("failed to build tokio runtime")
});

// ========================
// Opaque types for C
// ========================
#[repr(C)]
pub struct NinjaManagerOpaque {
    _private: [u8; 0],
}

// Internal boxed wrapper
struct ManagerBox(pub Box<ShurikenManager>);

// ========================
// Global last error
// ========================
static LAST_ERROR: Lazy<Mutex<Option<CString>>> = Lazy::new(|| Mutex::new(None));

fn set_last_error(msg: String) {
    let mut lock = LAST_ERROR.lock().unwrap();
    *lock = Some(CString::new(msg).unwrap_or_else(|_| CString::new("error").unwrap()));
}

#[no_mangle]
pub extern "C" fn ninja_last_error() -> *mut c_char {
    let lock = LAST_ERROR.lock().unwrap();
    match &*lock {
        Some(s) => CString::new(s.as_bytes()).unwrap().into_raw(),
        None => ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "C" fn ninja_string_free(s: *mut c_char) {
    if s.is_null() { return; }
    unsafe { let _ = CString::from_raw(s); }
}

// ========================
// Helper: C string -> Rust String
// ========================
fn str_from_c(ptr: *const c_char) -> Option<String> {
    if ptr.is_null() { return None; }
    unsafe { CStr::from_ptr(ptr).to_str().ok().map(|s| s.to_string()) }
}

// ========================
// API Version
// ========================
#[no_mangle]
pub extern "C" fn ninja_api_version() -> u32 { 1 }

// ========================
// Create / free manager
// ========================
#[no_mangle]
pub extern "C" fn ninja_manager_new(out_err: *mut *mut c_char) -> *mut NinjaManagerOpaque {
    let res = RUNTIME.block_on(async { ShurikenManager::new().await });

    match res {
        Ok(manager) => {
            let boxed = Box::new(ManagerBox(Box::new(manager)));
            Box::into_raw(boxed) as *mut NinjaManagerOpaque
        }
        Err(e) => {
            let msg = format!("Failed to create manager: {}", e);
            set_last_error(msg.clone());
            if !out_err.is_null() {
                unsafe { *out_err = CString::new(msg).unwrap().into_raw(); }
            }
            ptr::null_mut()
        }
    }
}

#[no_mangle]
pub extern "C" fn ninja_manager_free(mgr: *mut NinjaManagerOpaque) {
    if mgr.is_null() { return; }
    unsafe { let _ = Box::from_raw(mgr as *mut ManagerBox); }
}

// ========================
// Helper: convert opaque ptr to &mut manager
// ========================
fn mgr_from_ptr<'a>(mgr: *mut NinjaManagerOpaque) -> Option<&'a mut ShurikenManager> {
    if mgr.is_null() { return None; }
    let b = unsafe { &mut *(mgr as *mut ManagerBox) };
    Some(b.0.as_mut())
}

// ========================
// Synchronous Shuriken control
// ========================
#[no_mangle]
pub extern "C" fn ninja_start_shuriken_sync(
    mgr: *mut NinjaManagerOpaque,
    name: *const c_char,
    out_err: *mut *mut c_char
) -> i32 {
    let name = match str_from_c(name) {
        Some(s) => s,
        None => { unsafe { if !out_err.is_null() { *out_err = CString::new("null name").unwrap().into_raw(); } } return -1; }
    };
    let manager = match mgr_from_ptr(mgr) {
        Some(m) => m,
        None => { unsafe { if !out_err.is_null() { *out_err = CString::new("null manager").unwrap().into_raw(); } } return -1; }
    };

    let res = RUNTIME.block_on(async { manager.start(&name).await });

    match res {
        Ok(()) => 0,
        Err(e) => {
            let msg = format!("{}", e);
            set_last_error(msg.clone());
            unsafe { if !out_err.is_null() { *out_err = CString::new(msg).unwrap().into_raw(); } }
            -1
        }
    }
}

#[no_mangle]
pub extern "C" fn ninja_stop_shuriken_sync(
    mgr: *mut NinjaManagerOpaque,
    name: *const c_char,
    out_err: *mut *mut c_char
) -> i32 {
    let name = match str_from_c(name) {
        Some(s) => s,
        None => { unsafe { if !out_err.is_null() { *out_err = CString::new("null name").unwrap().into_raw(); } } return -1; }
    };
    let manager = match mgr_from_ptr(mgr) {
        Some(m) => m,
        None => { unsafe { if !out_err.is_null() { *out_err = CString::new("null manager").unwrap().into_raw(); } } return -1; }
    };

    let res = RUNTIME.block_on(async { manager.stop(&name).await });

    match res {
        Ok(()) => 0,
        Err(e) => {
            let msg = format!("{}", e);
            set_last_error(msg.clone());
            unsafe { if !out_err.is_null() { *out_err = CString::new(msg).unwrap().into_raw(); } }
            -1
        }
    }
}

// ========================
// Synchronous list shurikens (serialize Either properly)
// ========================
#[derive(Serialize)]
struct ShurikenPair {
    name: String,
    state: ShurikenState,
}

#[no_mangle]
pub extern "C" fn ninja_list_shurikens_sync(mgr: *mut NinjaManagerOpaque, out_err: *mut *mut c_char) -> *mut c_char {
    let manager = match mgr_from_ptr(mgr) {
        Some(m) => m,
        None => { unsafe { if !out_err.is_null() { *out_err = CString::new("null manager").unwrap().into_raw(); } } return ptr::null_mut(); }
    };

    let res = RUNTIME.block_on(async { manager.list(false).await });

    match res {
        Ok(list) => {
            let serializable: Vec<ShurikenPair> = match list {
                Either::Left(vec) => vec.into_iter().map(|(name, state)| ShurikenPair { name, state }).collect(),
                Either::Right(vec) => vec.into_iter().map(|name| ShurikenPair { name, state: ShurikenState::Idle }).collect(),
            };
            let json = serde_json::to_string(&serializable)
                .unwrap_or_else(|e| format!("{{\"error\":\"serde error: {}\"}}", e));
            CString::new(json).unwrap().into_raw()
        }
        Err(e) => {
            let msg = format!("{}", e);
            set_last_error(msg.clone());
            unsafe { if !out_err.is_null() { *out_err = CString::new(msg).unwrap().into_raw(); } }
            ptr::null_mut()
        }
    }
}

// ========================
// Async callbacks (Send-safe)
// ========================
#[repr(C)]
pub struct NinjaCallback(pub extern "C" fn(*mut c_void, *const c_char));

fn call_callback(cb: Option<extern "C" fn(*mut c_void, *const c_char)>, userdata: *mut c_void, json: String) {
    if let Some(cb_fn) = cb {
        let cstr = CString::new(json).unwrap();
        let ptr = cstr.into_raw();
        cb_fn(userdata, ptr as *const c_char);
        unsafe { let _ =  CString::from_raw(ptr); }
    }
}

#[no_mangle]
pub extern "C" fn ninja_start_shuriken_async(
    mgr: *mut NinjaManagerOpaque,
    name: *const c_char,
    cb: Option<extern "C" fn(*mut c_void, *const c_char)>,
    userdata: *mut c_void
) {
    let name = match str_from_c(name) { Some(s) => s, None => return };
    let manager = match mgr_from_ptr(mgr) { Some(m) => m, None => return };

    let name_clone = name.clone();
    let userdata_usize = userdata as usize;

    let _handle = RUNTIME.spawn(async move {
        let r = manager.start(&name_clone).await;
        let json = match r {
            Ok(()) => "{\"ok\":true}".to_string(),
            Err(e) => format!("{{\"error\":\"{}\"}}", e),
        };
        let userdata_ptr = userdata_usize as *mut c_void;
        call_callback(cb, userdata_ptr, json);
    });
}

#[no_mangle]
pub extern "C" fn ninja_stop_shuriken_async(
    mgr: *mut NinjaManagerOpaque,
    name: *const c_char,
    cb: Option<extern "C" fn(*mut c_void, *const c_char)>,
    userdata: *mut c_void
) {
    let name = match str_from_c(name) { Some(s) => s, None => return };
    let manager = match mgr_from_ptr(mgr) { Some(m) => m, None => return };

    let name_clone = name.clone();
    let userdata_usize = userdata as usize;

    let _handle = RUNTIME.spawn(async move {
        let r = manager.stop(&name_clone).await;
        let json = match r {
            Ok(()) => "{\"ok\":true}".to_string(),
            Err(e) => format!("{{\"error\":\"{}\"}}", e),
        };
        let userdata_ptr = userdata_usize as *mut c_void;
        call_callback(cb, userdata_ptr, json);
    });
}
