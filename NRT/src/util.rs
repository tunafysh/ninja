use rquickjs::{function::IntoJsFunc, Ctx, Function};
use log::{error, info, warn};

pub fn add_global_function<'js, P, F>(ctx: &Ctx<'js>, key: &str, f: F)
where F: IntoJsFunc<'js, P> + 'js {
    let global = ctx.globals();
    global.set(key,
        Function::new(ctx.clone(), f).unwrap().with_name(key).unwrap(),
    ).unwrap();
}

pub fn print(s: String) {
    println!("{s}");
}

pub fn info(s: String) {
    info!("{s}");
}

pub fn error(s: String) {
    error!("{s}");
}

pub fn warn(s: String) {
    warn!("{s}");
}
pub fn sleep(milliseconds: i32) {
    std::thread::sleep(std::time::Duration::from_millis(milliseconds as u64));
}