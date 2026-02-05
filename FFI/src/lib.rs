use anyhow::Result;
use ninja::manager::{ArmoryMetadata, ShurikenManager};
use once_cell::sync::Lazy;
use serde::Serialize;
use std::{
    ffi::{CStr, CString},
    os::raw::{c_char, c_int, c_void},
    path::PathBuf,
    ptr,
    sync::Mutex,
};
use tokio::{
    fs,
    runtime::{Builder, Runtime},
};

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

// ========================
// Error helpers
// ========================

/// Clears the last error message.
///
/// # Safety
/// Safe to call at any time. Does not dereference any pointers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ninja_clear_last_error() {
    let mut lock = LAST_ERROR.lock().unwrap();
    *lock = None;
}

/// Returns 1 if there is an error, 0 otherwise.
///
/// # Safety
/// Safe to call at any time. Does not dereference any pointers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ninja_has_error() -> c_int {
    if LAST_ERROR.lock().unwrap().is_some() {
        1
    } else {
        0
    }
}

/// Writes the last error into a buffer.
///
/// Returns:
/// - Number of bytes written (excluding null terminator)
/// - -1 if buffer is null or too small
/// - 0 if no error
///
/// # Safety
/// `buffer` must be valid for `buffer_size` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ninja_get_last_error_buf(
    buffer: *mut c_char,
    buffer_size: usize,
) -> c_int {
    if buffer.is_null() || buffer_size == 0 {
        return -1;
    }
    match &*LAST_ERROR.lock().unwrap() {
        Some(s) => {
            let bytes = s.as_bytes();
            if bytes.len() + 1 > buffer_size {
                return -1;
            }
            unsafe{
                ptr::copy_nonoverlapping(bytes.as_ptr(), buffer as *mut u8, bytes.len());
                *buffer.add(bytes.len()) = 0;
            }
            bytes.len() as c_int
        }
        None => 0,
    }
}

/// Returns the last error string (caller must free via `ninja_string_free`).
///
/// # Safety
/// The returned string must be freed with `ninja_string_free`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ninja_last_error() -> *mut c_char {
    match &*LAST_ERROR.lock().unwrap() {
        Some(s) => CString::new(s.as_str())
            .ok()
            .map_or(ptr::null_mut(), |c| c.into_raw()),
        None => ptr::null_mut(),
    }
}

/// Frees a string returned by the library.
///
/// # Safety
/// `s` must be a pointer returned by this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ninja_string_free(s: *mut c_char) {
    if !s.is_null() {
        let _ = unsafe { CString::from_raw(s) };
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

unsafe fn mgr_from_ptr<'a>(mgr: *mut NinjaManagerOpaque) -> Option<&'a mut ShurikenManager> {
    if mgr.is_null() {
        return None;
    }
    let b = unsafe { &mut *(mgr as *mut ManagerBox) };
    Some(b.0.as_mut())
}

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
                    unsafe { *out_err = CString::new(msg).unwrap().into_raw() };
                }
                ptr::null_mut()
            }
        },
        Err(e) => {
            let msg = format!("{}", e);
            set_last_error(msg.clone());
            if !out_err.is_null() {
                unsafe {  *out_err = CString::new(msg).unwrap().into_raw() };
            }
            ptr::null_mut()
        }
    }
}

// ========================
// Macros for FFI
// ========================

macro_rules! ffi_sync {
    ($fn_name:ident, $action:expr) => {
        #[unsafe(no_mangle)]
        /// Synchronous FFI function.
        ///
        /// # Safety
        /// `mgr` must be a valid pointer to a NinjaManagerOpaque.
        /// `name` must be valid C string.
        /// `out_err` can be null or a valid pointer.
        pub unsafe extern "C" fn $fn_name(
            mgr: *mut NinjaManagerOpaque,
            name: *const c_char,
            out_err: *mut *mut c_char,
        ) -> i32 {
            let manager = if let Some(mgr) = unsafe { mgr_from_ptr(mgr) } {
                mgr
            } else {
                if !out_err.is_null() {
                    unsafe { *out_err = CString::new("Manager pointer was null").unwrap().into_raw() };
                }
                return -1;
            };

            let name = if let Some(s) = str_from_c(name) {
                s
            } else {
                if !out_err.is_null() {
                    unsafe { *out_err = CString::new("Name pointer was null").unwrap().into_raw() };
                }
                return -1;
            };

            match $action(manager, &name) {
                Ok(_) => 0,
                Err(e) => {
                    let msg = format!(
                        "Operation '{}' failed for '{}': {}",
                        stringify!($fn_name),
                        name,
                        e
                    );
                    set_last_error(msg.clone());
                    if !out_err.is_null() {
                        unsafe { *out_err = CString::new(msg).unwrap().into_raw() };
                    }
                    -1
                }
            }
        }
    };
}

macro_rules! ffi_async {
    ($fn_name:ident, $action:expr) => {
        #[unsafe(no_mangle)]
        /// Asynchronous FFI function.
        ///
        /// # Safety
        /// `mgr` must be valid.
        /// `name` must be valid C string.
        /// `cb` can be null.
        /// `userdata` is passed to callback as-is.
        pub unsafe extern "C" fn $fn_name(
            mgr: *mut NinjaManagerOpaque,
            name: *const c_char,
            cb: Option<extern "C" fn(*mut c_void, *const c_char)>,
            userdata: *mut c_void,
        ) {
            let manager = match unsafe { mgr_from_ptr(mgr) } {
                Some(m) => m.clone(),
                None => return,
            };
            let name = match str_from_c(name) {
                Some(s) => s,
                None => return,
            };

            let userdata_ptr = userdata as usize;
            RUNTIME.spawn(async move {
                let res = $action(&mut manager.clone(), &name);
                let json = match res {
                    Ok(_) => "{\"ok\":true}".to_string(),
                    Err(e) => format!(
                        "{{\"error\":\"Operation '{}' failed for '{}': {}\"}}",
                        stringify!($fn_name),
                        name,
                        e
                    ),
                };
                if let Some(cb_fn) = cb {
                    let userdata_ptr = userdata_ptr as *mut c_void;
                    cb_fn(userdata_ptr, CString::new(json).unwrap().into_raw());
                }
            });
        }
    };
}

// ========================
// Manager lifecycle
// ========================

/// Creates a new Ninja manager.
///
/// Returns a pointer to the manager, or null on failure.
/// `out_err` receives descriptive error message on failure.
///
/// # Safety
/// Caller must free the manager with `ninja_manager_free`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ninja_manager_new(out_err: *mut *mut c_char) -> *mut NinjaManagerOpaque {
    let res = RUNTIME.block_on(async { ShurikenManager::new().await });
    match res {
        Ok(manager) => Box::into_raw(Box::new(ManagerBox(Box::new(manager)))) as *mut NinjaManagerOpaque,
        Err(e) => {
            let msg = format!("Failed to create manager: {}", e);
            set_last_error(msg.clone());
            if !out_err.is_null() {
                unsafe { *out_err = CString::new(msg).unwrap().into_raw()};
            }
            ptr::null_mut()
        }
    }
}

/// Frees a Ninja manager pointer.
///
/// # Safety
/// `mgr` must be a valid pointer returned by `ninja_manager_new`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ninja_manager_free(mgr: *mut NinjaManagerOpaque) {
    if !mgr.is_null() {
        let _ = unsafe { Box::from_raw(mgr as *mut ManagerBox) };
    }
}

// ========================
// Sync Shuriken operations
// ========================
ffi_sync!(ninja_start_shuriken_sync, |m: &mut ShurikenManager, n| {
    RUNTIME.block_on(async { m.start(n).await })
});
ffi_sync!(ninja_stop_shuriken_sync, |m: &mut ShurikenManager, n| {
    RUNTIME.block_on(async { m.stop(n).await })
});
ffi_sync!(ninja_refresh_shuriken_sync, |m: &mut ShurikenManager, _| {
    RUNTIME.block_on(async { m.refresh().await })
});
ffi_sync!(ninja_remove_shuriken_sync, |m: &mut ShurikenManager, n| {
    RUNTIME.block_on(async { m.remove(n).await })
});

// ========================
// Async Shuriken operations
// ========================
ffi_async!(ninja_start_shuriken_async, |m: &mut ShurikenManager, n| {
    RUNTIME.block_on(async { m.start(n).await })
});
ffi_async!(ninja_stop_shuriken_async, |m: &mut ShurikenManager, n| {
    RUNTIME.block_on(async { m.stop(n).await })
});
ffi_async!(
    ninja_refresh_shuriken_async,
    |m: &mut ShurikenManager, _| RUNTIME.block_on(async { m.refresh().await })
);
ffi_async!(ninja_remove_shuriken_async, |m: &mut ShurikenManager, n| {
    RUNTIME.block_on(async { m.remove(n).await })
});

// ========================
// Forge / Install / Write options
// ========================

#[unsafe(no_mangle)]
/// Forge a shuriken from metadata JSON and source path.
///
/// # Safety
/// `mgr` must be valid, `meta_json` and `src_path` must be valid C strings.
/// `out_err` can be null.
pub unsafe extern "C" fn ninja_forge_shuriken_sync(
    mgr: *mut NinjaManagerOpaque,
    meta_json: *const c_char,
    src_path: *const c_char,
    out_err: *mut *mut c_char,
) -> i32 {
    let manager = match unsafe { mgr_from_ptr(mgr) } {
        Some(m) => m,
        None => {
            if !out_err.is_null() {
                unsafe { *out_err = CString::new("Manager was null").unwrap().into_raw() };
            }
            return -1;
        }
    };
    let meta_str = match str_from_c(meta_json) {
        Some(s) => s,
        None => {
            if !out_err.is_null() {
                unsafe { *out_err = CString::new("Metadata JSON was null").unwrap().into_raw() };
            }
            return -1;
        }
    };
    let src = match path_from_c(src_path) {
        Some(p) => p,
        None => {
            if !out_err.is_null() {
                unsafe { *out_err = CString::new("Source path was null").unwrap().into_raw() };
            }
            return -1;
        }
    };
    let meta: ArmoryMetadata = match serde_json::from_str(&meta_str) {
        Ok(m) => m,
        Err(e) => {
            if !out_err.is_null() {
                unsafe { *out_err = CString::new(format!("Invalid metadata JSON: {}", e))
                    .unwrap()
                    .into_raw();
                }
            }
            return -1;
        }
    };
    match RUNTIME.block_on(manager.forge(meta, src)) {
        Ok(_) => 0,
        Err(e) => {
            let msg = format!("Forge failed: {}", e);
            set_last_error(msg.clone());
            if !out_err.is_null() {
                unsafe { *out_err = CString::new(msg).unwrap().into_raw() };
            }
            -1
        }
    }
}

#[unsafe(no_mangle)]
/// Install a shuriken from a path.
///
/// # Safety
/// `mgr` must be valid. `path_ptr` must be a valid C string. `out_err` can be null.
pub unsafe extern "C" fn ninja_install_shuriken_sync(
    mgr: *mut NinjaManagerOpaque,
    path_ptr: *const c_char,
    out_err: *mut *mut c_char,
) -> i32 {
    let path = match path_from_c(path_ptr) {
        Some(p) => p,
        None => {
            if !out_err.is_null() {
                unsafe { *out_err = CString::new("Path was null").unwrap().into_raw() };
            }
            return -1;
        }
    };
    let manager = match unsafe { mgr_from_ptr(mgr) } {
        Some(m) => m,
        None => {
            if !out_err.is_null() {
                unsafe { *out_err = CString::new("Manager was null").unwrap().into_raw() };
            }
            return -1;
        }
    };
    match RUNTIME.block_on(manager.install(&path)) {
        Ok(_) => 0,
        Err(e) => {
            let msg = format!("Install failed: {}", e);
            set_last_error(msg.clone());
            if !out_err.is_null() {
                unsafe { *out_err = CString::new(msg).unwrap().into_raw() };
            }
            -1
        }
    }
}

#[unsafe(no_mangle)]
/// Write TOML options to a shuriken.
///
/// # Safety
/// `mgr` must be valid. `name` and `toml_str` must be valid C strings. `out_err` can be null.
pub unsafe extern "C" fn ninja_write_options_toml_sync(
    mgr: *mut NinjaManagerOpaque,
    name: *const c_char,
    toml_str: *const c_char,
    out_err: *mut *mut c_char,
) -> i32 {
    let manager = match unsafe { mgr_from_ptr(mgr) } {
        Some(m) => m,
        None => {
            if !out_err.is_null() {
                unsafe { *out_err = CString::new("Manager was null").unwrap().into_raw() };
            }
            return -1;
        }
    };
    let name = match str_from_c(name) {
        Some(s) => s,
        None => {
            if !out_err.is_null() {
                unsafe { *out_err = CString::new("Name was null").unwrap().into_raw() };
            }
            return -1;
        }
    };
    let toml_str = match str_from_c(toml_str) {
        Some(s) => s,
        None => {
            if !out_err.is_null() {
                unsafe { *out_err = CString::new("TOML string was null").unwrap().into_raw() };
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
            let msg = format!("Write TOML failed: {}", e);
            set_last_error(msg.clone());
            if !out_err.is_null() {
                unsafe { *out_err = CString::new(msg).unwrap().into_raw() };
            }
            -1
        }
    }
}
