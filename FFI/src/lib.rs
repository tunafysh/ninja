use anyhow;
use either::Either;
use once_cell::sync::Lazy;
use serde::Serialize;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_void};
use std::path::PathBuf;
use std::ptr;
use std::sync::Mutex;
use tokio::fs;
use tokio::runtime::{Builder, Runtime};

// Adapt this import to match your core crate
use ninja::{
    manager::{ArmoryMetadata, ShurikenManager},
    types::ShurikenState,
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
// store a Rust String here. When C asks for last error, we create a fresh CString and hand it over.
// C is responsible for freeing that CString with ninja_string_free.
static LAST_ERROR: Lazy<Mutex<Option<String>>> = Lazy::new(|| Mutex::new(None));

fn set_last_error(msg: String) {
    let mut lock = LAST_ERROR.lock().unwrap();
    *lock = Some(msg);
}

#[no_mangle]
pub extern "C" fn ninja_last_error() -> *mut c_char {
    let lock = LAST_ERROR.lock().unwrap();
    match &*lock {
        Some(s) => {
            // s is a Rust String (no interior nulls). Convert to CString for the caller.
            match CString::new(s.as_str()) {
                Ok(c) => c.into_raw(),
                Err(_) => ptr::null_mut(),
            }
        }
        None => ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "C" fn ninja_string_free(s: *mut c_char) {
    if s.is_null() {
        return;
    }
    // Take ownership and drop - this uses the C allocator agreement.
    unsafe {
        let _ = CString::from_raw(s);
    }
}

// ========================
// Helper: C string -> Rust String
// ========================
fn str_from_c(ptr: *const c_char) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    unsafe { CStr::from_ptr(ptr).to_str().ok().map(|s| s.to_string()) }
}

// ========================
// API Version
// ========================
#[no_mangle]
pub extern "C" fn ninja_api_version() -> u32 {
    1
}

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
                unsafe {
                    *out_err = CString::new(msg).unwrap().into_raw();
                }
            }
            ptr::null_mut()
        }
    }
}

#[no_mangle]
pub extern "C" fn ninja_manager_free(mgr: *mut NinjaManagerOpaque) {
    if mgr.is_null() {
        return;
    }
    unsafe {
        let _ = Box::from_raw(mgr as *mut ManagerBox);
    }
}

// ========================
// Helper: convert opaque ptr to &mut manager
// ========================
fn mgr_from_ptr<'a>(mgr: *mut NinjaManagerOpaque) -> Option<&'a mut ShurikenManager> {
    if mgr.is_null() {
        return None;
    }
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
    out_err: *mut *mut c_char,
) -> i32 {
    let name = match str_from_c(name) {
        Some(s) => s,
        None => {
            unsafe {
                if !out_err.is_null() {
                    *out_err = CString::new("null name").unwrap().into_raw();
                }
            }
            return -1;
        }
    };
    let manager = match mgr_from_ptr(mgr) {
        Some(m) => m,
        None => {
            unsafe {
                if !out_err.is_null() {
                    *out_err = CString::new("null manager").unwrap().into_raw();
                }
            }
            return -1;
        }
    };

    let res = RUNTIME.block_on(async { manager.start(&name).await });

    match res {
        Ok(()) => 0,
        Err(e) => {
            let msg = format!("{}", e);
            set_last_error(msg.clone());
            unsafe {
                if !out_err.is_null() {
                    *out_err = CString::new(msg).unwrap().into_raw();
                }
            }
            -1
        }
    }
}

#[no_mangle]
pub extern "C" fn ninja_stop_shuriken_sync(
    mgr: *mut NinjaManagerOpaque,
    name: *const c_char,
    out_err: *mut *mut c_char,
) -> i32 {
    let name = match str_from_c(name) {
        Some(s) => s,
        None => {
            unsafe {
                if !out_err.is_null() {
                    *out_err = CString::new("null name").unwrap().into_raw();
                }
            }
            return -1;
        }
    };
    let manager = match mgr_from_ptr(mgr) {
        Some(m) => m,
        None => {
            unsafe {
                if !out_err.is_null() {
                    *out_err = CString::new("null manager").unwrap().into_raw();
                }
            }
            return -1;
        }
    };

    let res = RUNTIME.block_on(async { manager.stop(&name).await });

    match res {
        Ok(()) => 0,
        Err(e) => {
            let msg = format!("{}", e);
            set_last_error(msg.clone());
            unsafe {
                if !out_err.is_null() {
                    *out_err = CString::new(msg).unwrap().into_raw();
                }
            }
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
pub extern "C" fn ninja_list_shurikens_sync(
    mgr: *mut NinjaManagerOpaque,
    out_err: *mut *mut c_char,
) -> *mut c_char {
    let manager = match mgr_from_ptr(mgr) {
        Some(m) => m,
        None => {
            unsafe {
                if !out_err.is_null() {
                    *out_err = CString::new("null manager").unwrap().into_raw();
                }
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
                    .map(|(name, state)| ShurikenPair { name, state })
                    .collect(),
                Either::Right(vec) => vec
                    .into_iter()
                    .map(|name| ShurikenPair {
                        name,
                        state: ShurikenState::Idle,
                    })
                    .collect(),
            };
            let json = serde_json::to_string(&serializable)
                .unwrap_or_else(|e| format!("{{\"error\":\"serde error: {}\"}}", e));
            // Return ownership to C. C must call ninja_string_free().
            CString::new(json).unwrap().into_raw()
        }
        Err(e) => {
            let msg = format!("{}", e);
            set_last_error(msg.clone());
            unsafe {
                if !out_err.is_null() {
                    *out_err = CString::new(msg).unwrap().into_raw();
                }
            }
            ptr::null_mut()
        }
    }
}

// ========================
// Async callbacks (Send-safe)
// ========================
#[repr(C)]
pub struct NinjaCallback(pub extern "C" fn(*mut c_void, *const c_char));

fn call_callback(
    cb: Option<extern "C" fn(*mut c_void, *const c_char)>,
    userdata: *mut c_void,
    json: String,
) {
    if let Some(cb_fn) = cb {
        // Create a CString and hand ownership to C via into_raw().
        // IMPORTANT: Do NOT reconstruct or free it on the Rust side here.
        // The C side MUST call ninja_string_free(ptr) when done with it.
        let cstr = CString::new(json).unwrap();
        let ptr = cstr.into_raw();
        // call the C callback (C receives ownership of 'ptr' and must free it)
        cb_fn(userdata, ptr as *const c_char);
        // DO NOT call CString::from_raw(ptr) here — caller must free.
    }
}

#[no_mangle]
pub extern "C" fn ninja_start_shuriken_async(
    mgr: *mut NinjaManagerOpaque,
    name: *const c_char,
    cb: Option<extern "C" fn(*mut c_void, *const c_char)>,
    userdata: *mut c_void,
) {
    let name = match str_from_c(name) {
        Some(s) => s,
        None => return,
    };
    let manager = match mgr_from_ptr(mgr) {
        Some(m) => m,
        None => return,
    };

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
    userdata: *mut c_void,
) {
    let name = match str_from_c(name) {
        Some(s) => s,
        None => return,
    };
    let manager = match mgr_from_ptr(mgr) {
        Some(m) => m,
        None => return,
    };

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

// -----------------------------
// Additional FFI helpers
// -----------------------------
fn path_from_c(ptr: *const c_char) -> Option<PathBuf> {
    str_from_c(ptr).map(PathBuf::from)
}

fn json_result_or_error<T: Serialize>(
    res: Result<T, anyhow::Error>,
    out_err: *mut *mut c_char,
) -> *mut c_char {
    match res {
        Ok(v) => match serde_json::to_string(&v) {
            Ok(s) => CString::new(s).unwrap().into_raw(),
            Err(e) => {
                let msg = format!("serde_json error: {}", e);
                set_last_error(msg.clone());
                unsafe {
                    if !out_err.is_null() {
                        *out_err = CString::new(msg).unwrap().into_raw();
                    }
                }
                ptr::null_mut()
            }
        },
        Err(e) => {
            let msg = format!("{}", e);
            set_last_error(msg.clone());
            unsafe {
                if !out_err.is_null() {
                    *out_err = CString::new(msg).unwrap().into_raw();
                }
            }
            ptr::null_mut()
        }
    }
}

// -----------------------------
// ninja_refresh_sync
// -----------------------------
#[no_mangle]
pub extern "C" fn ninja_refresh_sync(
    mgr: *mut NinjaManagerOpaque,
    out_err: *mut *mut c_char,
) -> i32 {
    let manager = match mgr_from_ptr(mgr) {
        Some(m) => m,
        None => {
            unsafe {
                if !out_err.is_null() {
                    *out_err = CString::new("null manager").unwrap().into_raw();
                }
            }
            return -1;
        }
    };

    let res = RUNTIME.block_on(async { manager.refresh().await });

    match res {
        Ok(()) => 0,
        Err(e) => {
            let msg = format!("{}", e);
            set_last_error(msg.clone());
            unsafe {
                if !out_err.is_null() {
                    *out_err = CString::new(msg).unwrap().into_raw();
                }
            }
            -1
        }
    }
}

// -----------------------------
// ninja_configure_sync
// (calls manager.configure(name))
// -----------------------------
#[no_mangle]
pub extern "C" fn ninja_configure_sync(
    mgr: *mut NinjaManagerOpaque,
    name: *const c_char,
    out_err: *mut *mut c_char,
) -> i32 {
    let name = match str_from_c(name) {
        Some(s) => s,
        None => {
            unsafe {
                if !out_err.is_null() {
                    *out_err = CString::new("null name").unwrap().into_raw();
                }
            }
            return -1;
        }
    };
    let manager = match mgr_from_ptr(mgr) {
        Some(m) => m,
        None => {
            unsafe {
                if !out_err.is_null() {
                    *out_err = CString::new("null manager").unwrap().into_raw();
                }
            }
            return -1;
        }
    };

    let res = RUNTIME.block_on(async { manager.configure(&name).await });

    match res {
        Ok(()) => 0,
        Err(e) => {
            let msg = format!("{}", e);
            set_last_error(msg.clone());
            unsafe {
                if !out_err.is_null() {
                    *out_err = CString::new(msg).unwrap().into_raw();
                }
            }
            -1
        }
    }
}

// -----------------------------
// ninja_write_options_toml_sync
// (writes raw TOML string into <root>/shurikens/<name>/.ninja/options.toml)
// This is a simple alternative to marshalling FieldValue across FFI.
// -----------------------------
#[no_mangle]
pub extern "C" fn ninja_write_options_toml_sync(
    mgr: *mut NinjaManagerOpaque,
    name: *const c_char,
    toml_str: *const c_char,
    out_err: *mut *mut c_char,
) -> i32 {
    let name = match str_from_c(name) {
        Some(s) => s,
        None => {
            unsafe {
                if !out_err.is_null() {
                    *out_err = CString::new("null name").unwrap().into_raw();
                }
            }
            return -1;
        }
    };
    let toml = match str_from_c(toml_str) {
        Some(s) => s,
        None => {
            unsafe {
                if !out_err.is_null() {
                    *out_err = CString::new("null toml string").unwrap().into_raw();
                }
            }
            return -1;
        }
    };

    let manager = match mgr_from_ptr(mgr) {
        Some(m) => m,
        None => {
            unsafe {
                if !out_err.is_null() {
                    *out_err = CString::new("null manager").unwrap().into_raw();
                }
            }
            return -1;
        }
    };

    // Determine path and write
    let options_path = manager
        .root_path
        .join("shurikens")
        .join(&name)
        .join(".ninja")
        .join("options.toml");

    let res = RUNTIME.block_on(async {
        if let Some(parent) = options_path.parent() {
            fs::create_dir_all(parent).await?;
        }
        fs::write(&options_path, toml).await?;
        Ok::<(), anyhow::Error>(())
    });

    match res {
        Ok(()) => 0,
        Err(e) => {
            let msg = format!("{}", e);
            set_last_error(msg.clone());
            unsafe {
                if !out_err.is_null() {
                    *out_err = CString::new(msg).unwrap().into_raw();
                }
            }
            -1
        }
    }
}

// -----------------------------
// ninja_remove_sync
// -----------------------------
#[no_mangle]
pub extern "C" fn ninja_remove_sync(
    mgr: *mut NinjaManagerOpaque,
    name: *const c_char,
    out_err: *mut *mut c_char,
) -> i32 {
    let name = match str_from_c(name) {
        Some(s) => s,
        None => {
            unsafe {
                if !out_err.is_null() {
                    *out_err = CString::new("null name").unwrap().into_raw();
                }
            }
            return -1;
        }
    };
    let manager = match mgr_from_ptr(mgr) {
        Some(m) => m,
        None => {
            unsafe {
                if !out_err.is_null() {
                    *out_err = CString::new("null manager").unwrap().into_raw();
                }
            }
            return -1;
        }
    };

    let res = RUNTIME.block_on(async { manager.remove(&name).await });

    match res {
        Ok(()) => 0,
        Err(e) => {
            let msg = format!("{}", e);
            set_last_error(msg.clone());
            unsafe {
                if !out_err.is_null() {
                    *out_err = CString::new(msg).unwrap().into_raw();
                }
            }
            -1
        }
    }
}

// -----------------------------
// ninja_install_sync
// (install from a shuriken package file path)
// -----------------------------
#[no_mangle]
pub extern "C" fn ninja_install_sync(
    mgr: *mut NinjaManagerOpaque,
    package_path: *const c_char,
    out_err: *mut *mut c_char,
) -> i32 {
    let path = match path_from_c(package_path) {
        Some(p) => p,
        None => {
            unsafe {
                if !out_err.is_null() {
                    *out_err = CString::new("null package path").unwrap().into_raw();
                }
            }
            return -1;
        }
    };

    let manager = match mgr_from_ptr(mgr) {
        Some(m) => m,
        None => {
            unsafe {
                if !out_err.is_null() {
                    *out_err = CString::new("null manager").unwrap().into_raw();
                }
            }
            return -1;
        }
    };

    let res = RUNTIME.block_on(async { manager.install(path).await });

    match res {
        Ok(()) => 0,
        Err(e) => {
            let msg = format!("{}", e);
            set_last_error(msg.clone());
            unsafe {
                if !out_err.is_null() {
                    *out_err = CString::new(msg).unwrap().into_raw();
                }
            }
            -1
        }
    }
}

// -----------------------------
// ninja_forge_sync
// (forges a shuriken package; meta is provided as JSON string; src path and output dir path are C strings)
// meta JSON should map to ArmoryMetadata shape
// -----------------------------
#[no_mangle]
pub extern "C" fn ninja_forge_sync(
    mgr: *mut NinjaManagerOpaque,
    meta_json: *const c_char,
    src_path: *const c_char,
    out_err: *mut *mut c_char,
) -> i32 {
    let meta_str = match str_from_c(meta_json) {
        Some(s) => s,
        None => {
            unsafe {
                if !out_err.is_null() {
                    *out_err = CString::new("null meta json").unwrap().into_raw();
                }
            }
            return -1;
        }
    };

    let meta: ArmoryMetadata = match serde_json::from_str(&meta_str) {
        Ok(m) => m,
        Err(e) => {
            let msg = format!("meta json parse error: {}", e);
            set_last_error(msg.clone());
            unsafe {
                if !out_err.is_null() {
                    *out_err = CString::new(msg).unwrap().into_raw();
                }
            }
            return -1;
        }
    };

    let src = match path_from_c(src_path) {
        Some(p) => p,
        None => {
            unsafe {
                if !out_err.is_null() {
                    *out_err = CString::new("null src path").unwrap().into_raw();
                }
            }
            return -1;
        }
    };

    let manager = match mgr_from_ptr(mgr) {
        Some(m) => m.clone(),
        None => {
            unsafe {
                if !out_err.is_null() {
                    *out_err = CString::new("null manager").unwrap().into_raw();
                }
            }
            return -1;
        }
    };

    let res = RUNTIME.block_on(async move { manager.forge(meta, src).await });

    match res {
        Ok(()) => 0,
        Err(e) => {
            let msg = format!("{}", e);
            set_last_error(msg.clone());
            unsafe {
                if !out_err.is_null() {
                    *out_err = CString::new(msg).unwrap().into_raw();
                }
            }
            -1
        }
    }
}

// -----------------------------
// ninja_get_projects_sync
// (returns JSON array of project names, caller must free string)
// -----------------------------
#[no_mangle]
pub extern "C" fn ninja_get_projects_sync(
    mgr: *mut NinjaManagerOpaque,
    out_err: *mut *mut c_char,
) -> *mut c_char {
    let manager = match mgr_from_ptr(mgr) {
        Some(m) => m,
        None => {
            unsafe {
                if !out_err.is_null() {
                    *out_err = CString::new("null manager").unwrap().into_raw();
                }
            }
            return ptr::null_mut();
        }
    };

    let res = RUNTIME.block_on(async { manager.get_projects().await });

    json_result_or_error(res.map(|v| v), out_err)
}

// -----------------------------
// ninja_get_shuriken_sync
// returns the Shuriken manifest (serialized to JSON) for a given name
// -----------------------------
#[no_mangle]
pub extern "C" fn ninja_get_shuriken_sync(
    mgr: *mut NinjaManagerOpaque,
    name: *const c_char,
    out_err: *mut *mut c_char,
) -> *mut c_char {
    let name = match str_from_c(name) {
        Some(s) => s,
        None => {
            unsafe {
                if !out_err.is_null() {
                    *out_err = CString::new("null name").unwrap().into_raw();
                }
            }
            return ptr::null_mut();
        }
    };

    let manager = match mgr_from_ptr(mgr) {
        Some(m) => m,
        None => {
            unsafe {
                if !out_err.is_null() {
                    *out_err = CString::new("null manager").unwrap().into_raw();
                }
            }
            return ptr::null_mut();
        }
    };

    let res = RUNTIME.block_on(async { manager.get(name).await });

    match res {
        Ok(shuriken) => match serde_json::to_string(&shuriken) {
            Ok(s) => CString::new(s).unwrap().into_raw(),
            Err(e) => {
                let msg = format!("serde_json error: {}", e);
                set_last_error(msg.clone());
                unsafe {
                    if !out_err.is_null() {
                        *out_err = CString::new(msg).unwrap().into_raw();
                    }
                }
                ptr::null_mut()
            }
        },
        Err(e) => {
            let msg = format!("{}", e);
            set_last_error(msg.clone());
            unsafe {
                if !out_err.is_null() {
                    *out_err = CString::new(msg).unwrap().into_raw();
                }
            }
            ptr::null_mut()
        }
    }
}

// -----------------------------
// Async versions using callbacks (pattern matches your start/stop async)
// Example: ninja_install_async, ninja_refresh_async, ninja_remove_async, ninja_forge_async
// Each will call the callback with a JSON string {"ok":true} or {"error":"..."}.
// -----------------------------
#[no_mangle]
pub extern "C" fn ninja_refresh_async(
    mgr: *mut NinjaManagerOpaque,
    cb: Option<extern "C" fn(*mut c_void, *const c_char)>,
    userdata: *mut c_void,
) {
    let manager = match mgr_from_ptr(mgr) {
        Some(m) => m.clone(),
        None => return,
    };

    let userdata_usize = userdata as usize;

    let _handle = RUNTIME.spawn(async move {
        let r = manager.refresh().await;
        let json = match r {
            Ok(()) => "{\"ok\":true}".to_string(),
            Err(e) => format!("{{\"error\":\"{}\"}}", e),
        };
        let userdata_ptr = userdata_usize as *mut c_void;
        call_callback(cb, userdata_ptr, json);
    });
}

#[no_mangle]
pub extern "C" fn ninja_remove_async(
    mgr: *mut NinjaManagerOpaque,
    name: *const c_char,
    cb: Option<extern "C" fn(*mut c_void, *const c_char)>,
    userdata: *mut c_void,
) {
    let name = match str_from_c(name) {
        Some(s) => s,
        None => return,
    };
    let manager = match mgr_from_ptr(mgr) {
        Some(m) => m.clone(),
        None => return,
    };
    let name_clone = name.clone();
    let userdata_usize = userdata as usize;

    let _handle = RUNTIME.spawn(async move {
        let r = manager.remove(&name_clone).await;
        let json = match r {
            Ok(()) => "{\"ok\":true}".to_string(),
            Err(e) => format!("{{\"error\":\"{}\"}}", e),
        };
        let userdata_ptr = userdata_usize as *mut c_void;
        call_callback(cb, userdata_ptr, json);
    });
}

#[no_mangle]
pub extern "C" fn ninja_install_async(
    mgr: *mut NinjaManagerOpaque,
    package_path: *const c_char,
    cb: Option<extern "C" fn(*mut c_void, *const c_char)>,
    userdata: *mut c_void,
) {
    let package = match path_from_c(package_path) {
        Some(p) => p,
        None => return,
    };
    let manager = match mgr_from_ptr(mgr) {
        Some(m) => m.clone(),
        None => return,
    };
    let userdata_usize = userdata as usize;

    let _handle = RUNTIME.spawn(async move {
        let r = manager.install(package).await;
        let json = match r {
            Ok(()) => "{\"ok\":true}".to_string(),
            Err(e) => format!("{{\"error\":\"{}\"}}", e),
        };
        let userdata_ptr = userdata_usize as *mut c_void;
        call_callback(cb, userdata_ptr, json);
    });
}

#[no_mangle]
pub extern "C" fn ninja_forge_async(
    mgr: *mut NinjaManagerOpaque,
    meta_json: *const c_char,
    src_path: *const c_char,
    cb: Option<extern "C" fn(*mut c_void, *const c_char)>,
    userdata: *mut c_void,
) {
    let meta_str = match str_from_c(meta_json) {
        Some(s) => s,
        None => return,
    };
    let meta: ArmoryMetadata = match serde_json::from_str(&meta_str) {
        Ok(m) => m,
        Err(_) => return,
    };
    let src = match path_from_c(src_path) {
        Some(p) => p,
        None => return,
    };
    let manager = match mgr_from_ptr(mgr) {
        Some(m) => m.clone(),
        None => return,
    };
    let userdata_usize = userdata as usize;

    let _handle = RUNTIME.spawn(async move {
        let r = manager.forge(meta, src).await;
        let json = match r {
            Ok(()) => "{\"ok\":true}".to_string(),
            Err(e) => format!("{{\"error\":\"{}\"}}", e),
        };
        let userdata_ptr = userdata_usize as *mut c_void;
        call_callback(cb, userdata_ptr, json);
    });
}
