use anyhow::Result;
use either::Either;
use once_cell::sync::Lazy;
use serde::Serialize;
use std::{
    ffi::{CStr, CString},
    os::raw::{c_char, c_void},
    path::PathBuf,
    ptr,
    sync::Mutex,
};
use tokio::fs;

// ========================
// Global Tokio runtime
// ========================
static RUNTIME: Lazy<Runtime> = Lazy::new(|| {
    Builder::new_multi_thread()
        .enable_all()
        .worker_threads(4)
        .thread_name("ninja-ffi-runtime")
        .build()
        .expect("failed to build tokio runtime")
});

// ========================
// Opaque types
// ========================
#[repr(C)]
pub struct NinjaManagerOpaque {
    _private: [u8; 0],
}

struct ManagerBox(pub Box<ShurikenManager>);

// ========================
// Last error tracking
// ========================
static LAST_ERROR: Lazy<Mutex<Option<String>>> = Lazy::new(|| Mutex::new(None));

fn set_last_error(msg: String) {
    let mut lock = LAST_ERROR.lock().unwrap();
    *lock = Some(msg);
}

/// Retrieves the last error message recorded by the library.
///
/// # Safety
///
/// * The caller takes ownership of the returned string pointer and must eventually free it
///   using `ninja_string_free`.
/// * If no error has occurred, this returns a null pointer.
#[no_mangle]
pub extern "C" fn ninja_last_error() -> *mut c_char {
    let lock = LAST_ERROR.lock().unwrap();
    match &*lock {
        Some(s) => CString::new(s.as_str())
            .ok()
            .map_or(ptr::null_mut(), |c| c.into_raw()),
        None => ptr::null_mut(),
    }
}

/// Frees a string pointer previously allocated by this library.
///
/// # Safety
///
/// * `s` must be a pointer previously returned by a function in this library (like `ninja_last_error`
///   or `ninja_list_shurikens_sync`).
/// * `s` must not be used after this call.
/// * If `s` is null, this function does nothing.
#[no_mangle]
pub unsafe extern "C" fn ninja_string_free(s: *mut c_char) {
    if !s.is_null() {
        unsafe {
            let _ = CString::from_raw(s);
        }
    }
}

// ========================
// Helpers
// ========================
fn str_from_c(ptr: *const c_char) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    unsafe { CStr::from_ptr(ptr).to_str().ok().map(|s| s.to_string()) }
}

/// Converts an opaque C pointer back into a mutable Rust reference.
///
/// # Safety
///
/// * `mgr` must be a valid pointer obtained via `ninja_manager_new`.
/// * `mgr` must not have been freed yet.
unsafe fn mgr_from_ptr<'a>(mgr: *mut NinjaManagerOpaque) -> Option<&'a mut ShurikenManager> {
    if mgr.is_null() {
        return None;
    }
    let b = &mut *(mgr as *mut ManagerBox);
    Some(b.0.as_mut())
}

// ========================
// API Version
// ========================
#[no_mangle]
pub extern "C" fn ninja_api_version() -> u32 {
    1
}

// ========================
// Manager lifecycle
// ========================

/// Creates a new `ShurikenManager` instance.
///
/// # Safety
///
/// * The returned pointer is owned by the caller and must be freed using `ninja_manager_free`.
/// * `out_err` is an optional output parameter. If an error occurs (returning null),
///   `*out_err` will be set to a newly allocated error string which the caller must free.
/// * If `out_err` is not null, it must point to valid memory for writing a `*mut c_char`.
#[no_mangle]
pub unsafe extern "C" fn ninja_manager_new(out_err: *mut *mut c_char) -> *mut NinjaManagerOpaque {
    let res = RUNTIME.block_on(async { ShurikenManager::new().await });
    match res {
        Ok(manager) => {
            Box::into_raw(Box::new(ManagerBox(Box::new(manager)))) as *mut NinjaManagerOpaque
        }
        Err(e) => {
            let msg = format!("Failed to create manager: {}", e);
            set_last_error(msg.clone());
            if !out_err.is_null() {
                *out_err = CString::new(msg).unwrap().into_raw();
            }
            ptr::null_mut()
        }
    }
}

/// Frees a `ShurikenManager` instance.
///
/// # Safety
///
/// * `mgr` must be a valid pointer returned by `ninja_manager_new`.
/// * After calling this, `mgr` is invalid and must not be used again.
/// * If `mgr` is null, the function does nothing.
#[no_mangle]
pub unsafe extern "C" fn ninja_manager_free(mgr: *mut NinjaManagerOpaque) {
    if !mgr.is_null() {
        let _ = Box::from_raw(mgr as *mut ManagerBox);
    }
}

// ========================
// Shuriken control (sync)
// ========================

/// Internal helper for synchronous actions.
///
/// # Safety
///
/// * `mgr` must be a valid manager pointer.
/// * `name` must be a valid, null-terminated UTF-8 C string.
unsafe fn shuriken_action_sync(
    mgr: *mut NinjaManagerOpaque,
    name: *const c_char,
    out_err: *mut *mut c_char,
    action: fn(&mut ShurikenManager, &str) -> Result<()>,
) -> i32 {
    let manager = match mgr_from_ptr(mgr) {
        Some(m) => m,
        None => {
            if !out_err.is_null() {
                *out_err = CString::new("null manager").unwrap().into_raw();
            }
            return -1;
        }
    };
    let name = match str_from_c(name) {
        Some(s) => s,
        None => {
            if !out_err.is_null() {
                *out_err = CString::new("null name").unwrap().into_raw();
            }
            return -1;
        }
    };
    match action(manager, &name) {
        Ok(_) => 0,
        Err(e) => {
            let msg = format!("{}", e);
            set_last_error(msg.clone());
            if !out_err.is_null() {
                *out_err = CString::new(msg).unwrap().into_raw();
            }
            -1
        }
    }
}

/// Starts a shuriken synchronously.
///
/// # Safety
///
/// * `mgr` must be a valid `NinjaManagerOpaque` pointer.
/// * `name` must be a valid, null-terminated UTF-8 C string representing the shuriken name.
/// * `out_err` must be null or a valid pointer to a `*mut c_char` to receive error messages.
#[no_mangle]
pub unsafe extern "C" fn ninja_start_shuriken_sync(
    mgr: *mut NinjaManagerOpaque,
    name: *const c_char,
    out_err: *mut *mut c_char,
) -> i32 {
    shuriken_action_sync(mgr, name, out_err, |m, n| {
        RUNTIME.block_on(async { m.start(n).await })
    })
}

/// Stops a shuriken synchronously.
///
/// # Safety
///
/// * `mgr` must be a valid `NinjaManagerOpaque` pointer.
/// * `name` must be a valid, null-terminated UTF-8 C string.
/// * `out_err` must be null or a valid pointer to a `*mut c_char`.
#[no_mangle]
pub unsafe extern "C" fn ninja_stop_shuriken_sync(
    mgr: *mut NinjaManagerOpaque,
    name: *const c_char,
    out_err: *mut *mut c_char,
) -> i32 {
    shuriken_action_sync(mgr, name, out_err, |m, n| {
        RUNTIME.block_on(async { m.stop(n).await })
    })
}

// ========================
// Shuriken control (async with callback)
// ========================
#[repr(C)]
pub struct NinjaCallback(pub extern "C" fn(*mut c_void, *const c_char));

use tokio::runtime::{Builder, Runtime};

// ========================
// Ninja core imports
// ========================
use ninja::{
    manager::{ArmoryMetadata, ShurikenManager},
    types::ShurikenState,
};

fn path_from_c(ptr: *const c_char) -> Option<PathBuf> {
    str_from_c(ptr).map(PathBuf::from)
}

#[allow(dead_code)]
unsafe fn json_result_or_error<T: Serialize>(
    res: Result<T>,
    out_err: *mut *mut c_char,
) -> *mut c_char {
    match res {
        Ok(v) => match serde_json::to_string(&v) {
            Ok(s) => CString::new(s).unwrap().into_raw(),
            Err(e) => {
                let msg = format!("serde_json error: {}", e);
                set_last_error(msg.clone());
                if !out_err.is_null() {
                    *out_err = CString::new(msg).unwrap().into_raw();
                }
                ptr::null_mut()
            }
        },
        Err(e) => {
            let msg = format!("{}", e);
            set_last_error(msg.clone());
            if !out_err.is_null() {
                *out_err = CString::new(msg).unwrap().into_raw();
            }
            ptr::null_mut()
        }
    }
}

/// Internal helper for async actions.
///
/// # Safety
///
/// * `mgr` must be a valid manager pointer.
/// * `name` must be a valid C string.
/// * `cb` must be a valid function pointer if provided.
/// * `userdata` is passed blindly to the callback; the caller is responsible for its validity and thread safety.
unsafe fn shuriken_action_async(
    mgr: *mut NinjaManagerOpaque,
    name: *const c_char,
    cb: Option<extern "C" fn(*mut c_void, *const c_char)>,
    userdata: *mut c_void,
    action: fn(&mut ShurikenManager, &str) -> Result<()>,
) {
    let manager = match mgr_from_ptr(mgr) {
        Some(m) => m.clone(),
        None => return,
    };
    let name = match str_from_c(name) {
        Some(s) => s,
        None => return,
    };

    let userdata_ptr: *mut c_void = userdata;
    let userdata_ptr = userdata_ptr as usize; // cast to integer for Send
    RUNTIME.spawn(async move {
        let r = action(&mut manager.clone(), &name);
        let json = match r {
            Ok(_) => "{\"ok\":true}".to_string(),
            Err(e) => format!("{{\"error\":\"{}\"}}", e),
        };
        if let Some(cb_fn) = cb {
            let userdata_ptr = userdata_ptr as *mut c_void;
            cb_fn(userdata_ptr, CString::new(json).unwrap().into_raw());
        }
    });
}

/// Starts a shuriken asynchronously.
///
/// # Safety
///
/// * `mgr` must be a valid `NinjaManagerOpaque` pointer.
/// * `name` must be a valid C string.
/// * `cb` is a function pointer invoked upon completion. It receives `userdata` and a JSON result string.
/// * `userdata` is an opaque pointer passed to the callback. The library does not dereference it.
#[no_mangle]
pub unsafe extern "C" fn ninja_start_shuriken_async(
    mgr: *mut NinjaManagerOpaque,
    name: *const c_char,
    cb: Option<extern "C" fn(*mut c_void, *const c_char)>,
    userdata: *mut c_void,
) {
    shuriken_action_async(mgr, name, cb, userdata, |m, n| {
        RUNTIME.block_on(async { m.start(n).await })
    })
}

/// Stops a shuriken asynchronously.
///
/// # Safety
///
/// * `mgr` must be a valid `NinjaManagerOpaque` pointer.
/// * `name` must be a valid C string.
/// * `cb` is a function pointer invoked upon completion.
/// * `userdata` is passed through to the callback.
#[no_mangle]
pub unsafe extern "C" fn ninja_stop_shuriken_async(
    mgr: *mut NinjaManagerOpaque,
    name: *const c_char,
    cb: Option<extern "C" fn(*mut c_void, *const c_char)>,
    userdata: *mut c_void,
) {
    shuriken_action_async(mgr, name, cb, userdata, |m, n| {
        RUNTIME.block_on(async { m.stop(n).await })
    })
}

// ========================
// Shuriken list
// ========================
#[derive(Serialize)]
struct ShurikenPair {
    name: String,
    state: ShurikenState,
}

/// Lists all shurikens managed by the system.
///
/// # Safety
///
/// * `mgr` must be a valid `NinjaManagerOpaque` pointer.
/// * Returns a JSON string pointer that must be freed via `ninja_string_free`.
/// * On error, returns NULL and writes to `out_err` if provided.
#[no_mangle]
pub unsafe extern "C" fn ninja_list_shurikens_sync(
    mgr: *mut NinjaManagerOpaque,
    out_err: *mut *mut c_char,
) -> *mut c_char {
    let manager = match mgr_from_ptr(mgr) {
        Some(m) => m,
        None => {
            if !out_err.is_null() {
                *out_err = CString::new("null manager").unwrap().into_raw();
            }
            return ptr::null_mut();
        }
    };
    let res = RUNTIME.block_on(async { manager.list(false).await });
    match res {
        Ok(list) => {
            let serializable: Vec<ShurikenPair> = match list {
                Either::Left(vec) => vec
                    .into_iter()
                    .map(|(n, s)| ShurikenPair { name: n, state: s })
                    .collect(),
                Either::Right(vec) => vec
                    .into_iter()
                    .map(|n| ShurikenPair {
                        name: n,
                        state: ShurikenState::Idle,
                    })
                    .collect(),
            };
            CString::new(serde_json::to_string(&serializable).unwrap())
                .unwrap()
                .into_raw()
        }
        Err(e) => {
            let msg = format!("{}", e);
            set_last_error(msg.clone());
            if !out_err.is_null() {
                *out_err = CString::new(msg).unwrap().into_raw();
            }
            ptr::null_mut()
        }
    }
}

// ========================
// Write options TOML
// ========================

/// Writes a TOML configuration string to the specific shuriken's directory.
///
/// # Safety
///
/// * `mgr` must be a valid `NinjaManagerOpaque` pointer.
/// * `name` must be a valid, null-terminated UTF-8 C string.
/// * `toml_str` must be a valid, null-terminated UTF-8 C string containing the TOML data.
#[no_mangle]
pub unsafe extern "C" fn ninja_write_options_toml_sync(
    mgr: *mut NinjaManagerOpaque,
    name: *const c_char,
    toml_str: *const c_char,
    out_err: *mut *mut c_char,
) -> i32 {
    let manager = match mgr_from_ptr(mgr) {
        Some(m) => m,
        None => {
            if !out_err.is_null() {
                *out_err = CString::new("null manager").unwrap().into_raw();
            }
            return -1;
        }
    };
    let name = match str_from_c(name) {
        Some(s) => s,
        None => {
            if !out_err.is_null() {
                *out_err = CString::new("null name").unwrap().into_raw();
            }
            return -1;
        }
    };
    let toml_str = match str_from_c(toml_str) {
        Some(s) => s,
        None => {
            if !out_err.is_null() {
                *out_err = CString::new("null toml").unwrap().into_raw();
            }
            return -1;
        }
    };

    let path = manager
        .root_path
        .join("shurikens")
        .join(&name)
        .join(".ninja")
        .join("options.toml");
    let res = RUNTIME.block_on(async {
        if let Some(p) = path.parent() {
            fs::create_dir_all(p).await?;
        }
        fs::write(&path, toml_str).await?;
        Ok::<(), anyhow::Error>(())
    });

    match res {
        Ok(_) => 0,
        Err(e) => {
            let msg = format!("{}", e);
            set_last_error(msg.clone());
            if !out_err.is_null() {
                *out_err = CString::new(msg).unwrap().into_raw();
            }
            -1
        }
    }
}

// ========================
// Refresh shuriken
// ========================

/// Refreshes the state of a shuriken synchronously.
///
/// # Safety
///
/// * `mgr` must be a valid `NinjaManagerOpaque` pointer.
/// * `name` must be a valid, null-terminated UTF-8 C string.
#[no_mangle]
pub unsafe extern "C" fn ninja_refresh_shuriken_sync(
    mgr: *mut NinjaManagerOpaque,
    name: *const c_char,
    out_err: *mut *mut c_char,
) -> i32 {
    shuriken_action_sync(mgr, name, out_err, |m, _| {
        RUNTIME.block_on(async { m.refresh().await })
    })
}

// ========================
// Remove shuriken
// ========================

/// Removes a shuriken synchronously.
///
/// # Safety
///
/// * `mgr` must be a valid `NinjaManagerOpaque` pointer.
/// * `name` must be a valid, null-terminated UTF-8 C string.
#[no_mangle]
pub unsafe extern "C" fn ninja_remove_shuriken_sync(
    mgr: *mut NinjaManagerOpaque,
    name: *const c_char,
    out_err: *mut *mut c_char,
) -> i32 {
    shuriken_action_sync(mgr, name, out_err, |m, n| {
        RUNTIME.block_on(async { m.remove(n).await })
    })
}

// ========================
// Install shuriken from path
// ========================

/// Installs a shuriken from a file system path.
///
/// # Safety
///
/// * `mgr` must be a valid `NinjaManagerOpaque` pointer.
/// * `path_ptr` must be a valid, null-terminated UTF-8 C string representing the file path.
#[no_mangle]
pub unsafe extern "C" fn ninja_install_shuriken_sync(
    mgr: *mut NinjaManagerOpaque,
    path_ptr: *const c_char,
    out_err: *mut *mut c_char,
) -> i32 {
    let path = match path_from_c(path_ptr) {
        Some(p) => p,
        None => {
            if !out_err.is_null() {
                *out_err = CString::new("null path").unwrap().into_raw();
            }
            return -1;
        }
    };
    let manager = match mgr_from_ptr(mgr) {
        Some(m) => m,
        None => {
            if !out_err.is_null() {
                *out_err = CString::new("null manager").unwrap().into_raw();
            }
            return -1;
        }
    };
    match RUNTIME.block_on(manager.install(&path)) {
        Ok(_) => 0,
        Err(e) => {
            let msg = format!("{}", e);
            set_last_error(msg.clone());
            if !out_err.is_null() {
                *out_err = CString::new(msg).unwrap().into_raw();
            }
            -1
        }
    }
}

// ========================
// Forge shuriken (metadata + source path)
// ========================

/// Forges a new shuriken based on provided metadata JSON and a source path.
///
/// # Safety
///
/// * `mgr` must be a valid `NinjaManagerOpaque` pointer.
/// * `meta_json` must be a valid C string containing JSON data that matches `ArmoryMetadata`.
/// * `src_path` must be a valid C string representing the path.
#[no_mangle]
pub unsafe extern "C" fn ninja_forge_shuriken_sync(
    mgr: *mut NinjaManagerOpaque,
    meta_json: *const c_char,
    src_path: *const c_char,
    out_err: *mut *mut c_char,
) -> i32 {
    let meta_str = match str_from_c(meta_json) {
        Some(s) => s,
        None => {
            if !out_err.is_null() {
                *out_err = CString::new("null meta").unwrap().into_raw();
            }
            return -1;
        }
    };
    let meta: ArmoryMetadata = match serde_json::from_str(&meta_str) {
        Ok(m) => m,
        Err(e) => {
            if !out_err.is_null() {
                *out_err = CString::new(format!("invalid metadata: {}", e))
                    .unwrap()
                    .into_raw();
            }
            return -1;
        }
    };
    let src = match path_from_c(src_path) {
        Some(p) => p,
        None => {
            if !out_err.is_null() {
                *out_err = CString::new("null src path").unwrap().into_raw();
            }
            return -1;
        }
    };
    let manager = match mgr_from_ptr(mgr) {
        Some(m) => m,
        None => {
            if !out_err.is_null() {
                *out_err = CString::new("null manager").unwrap().into_raw();
            }
            return -1;
        }
    };

    match RUNTIME.block_on(manager.forge(meta, src)) {
        Ok(_) => 0,
        Err(e) => {
            let msg = format!("{}", e);
            set_last_error(msg.clone());
            if !out_err.is_null() {
                *out_err = CString::new(msg).unwrap().into_raw();
            }
            -1
        }
    }
}

// ========================
// Async versions with callback
// ========================

/// Internal async helper with callback support.
///
/// # Safety
///
/// * See `shuriken_action_async`.
unsafe fn shuriken_action_async_cb(
    mgr: *mut NinjaManagerOpaque,
    name: *const c_char,
    cb: Option<extern "C" fn(*mut c_void, *const c_char)>,
    userdata: *mut c_void,
    action: fn(&mut ShurikenManager, &str) -> Result<()>,
) {
    let manager = match mgr_from_ptr(mgr) {
        Some(m) => m.clone(),
        None => return,
    };
    let userdata = userdata as usize;
    let name = match str_from_c(name) {
        Some(s) => s,
        None => return,
    };
    RUNTIME.spawn(async move {
        let res = action(&mut manager.clone(), name.as_str());
        let json = match res {
            Ok(_) => "{\"ok\":true}".to_string(),
            Err(e) => format!("{{\"error\":\"{}\"}}", e),
        };
        if let Some(cb_fn) = cb {
            cb_fn(
                userdata as *mut c_void,
                CString::new(json).unwrap().into_raw(),
            );
        }
    });
}

// Examples:

/// Removes a shuriken asynchronously.
///
/// # Safety
///
/// * `mgr` must be a valid `NinjaManagerOpaque` pointer.
/// * `name` must be a valid C string.
/// * `cb` and `userdata`: see `ninja_start_shuriken_async`.
#[no_mangle]
pub unsafe extern "C" fn ninja_remove_shuriken_async(
    mgr: *mut NinjaManagerOpaque,
    name: *const c_char,
    cb: Option<extern "C" fn(*mut c_void, *const c_char)>,
    userdata: *mut c_void,
) {
    shuriken_action_async_cb(mgr, name, cb, userdata, |m, n| {
        RUNTIME.block_on(async { m.remove(n).await })
    })
}

/// Refreshes a shuriken asynchronously.
///
/// # Safety
///
/// * `mgr` must be a valid `NinjaManagerOpaque` pointer.
/// * `name` must be a valid C string.
/// * `cb` and `userdata`: see `ninja_start_shuriken_async`.
#[no_mangle]
pub unsafe extern "C" fn ninja_refresh_shuriken_async(
    mgr: *mut NinjaManagerOpaque,
    name: *const c_char,
    cb: Option<extern "C" fn(*mut c_void, *const c_char)>,
    userdata: *mut c_void,
) {
    shuriken_action_async_cb(mgr, name, cb, userdata, |m, _| {
        RUNTIME.block_on(async { m.refresh().await })
    })
}
