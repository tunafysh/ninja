use std::{fs as filesystem, path::PathBuf};
use log::info;
use rquickjs::{context::EvalOptions, Context, Module, Runtime};

mod util;
use util::*;

mod api;
use api::js_ninja_api;

mod fs;
use fs::js_fs_api;

mod shell;
use shell::js_shell_api;

mod env;
use env::js_env_api;

mod net;
use net::js_net_api;

mod sys;
use sys::js_sys_api;

pub struct NinjaEngine {
    ctx: Context,
}

impl NinjaEngine {
    pub fn new() -> Self {
        let rt = Runtime::new().expect("Failed to create runtime");
        let ctx = Context::full(&rt).expect("Failed to create context");

        let mut options = EvalOptions::default();
        options.global = true; // Changed to true for module loading
        options.promise = true;

        ctx.with(|ctx| {
            //importing apis
            Module::evaluate_def::<js_ninja_api, _>(ctx.clone(), "ninja").unwrap();
            Module::evaluate_def::<js_fs_api, _>(ctx.clone(), "fs").unwrap();
            Module::evaluate_def::<js_shell_api, _>(ctx.clone(), "shell").unwrap();
            Module::evaluate_def::<js_env_api, _>(ctx.clone(), "env").unwrap();
            Module::evaluate_def::<js_net_api, _>(ctx.clone(), "net").unwrap();
            Module::evaluate_def::<js_sys_api, _>(ctx.clone(), "sys").unwrap();

            while ctx.execute_pending_job() {}
            add_global_function(&ctx, "print", print);
            add_global_function(&ctx, "__rust_info", info);
            add_global_function(&ctx, "__rust_warn", warn);
            add_global_function(&ctx, "__rust_error", error);
            add_global_function(&ctx, "sleep", sleep);

            ctx.eval::<(), _>(
            r#"
                globalThis.console = {
                    log: (...v) =>  globalThis.print(`${v.join(" ")}`),
                    info: (...v) =>  globalThis.__rust_info(`${v.join(" ")}`),
                    warn: (...v) =>  globalThis.__rust_warn(`${v.join(" ")}`),
                    error: (...v) =>  globalThis.__rust_error(`${v.join(" ")}`)
                }
            "#,).unwrap();
        });

        Self {
            ctx
        }
    }

    pub fn execute(&self, script: &str) -> Result<(), rquickjs::Error> {
        self.ctx.with(|ctx| {
            let mut options = EvalOptions::default();
            options.global = true; // Changed to true for proper context access
            options.promise = true;

            ctx.eval_with_options(script, options)
        })
    }

    pub fn execute_file(&self, path: &str) -> Result<(), rquickjs::Error> {
        self.ctx.with(|ctx| {
            let mut options = EvalOptions::default();
            options.global = true; // Changed to true for proper context access
            options.promise = true;

            ctx.eval_file_with_options(path, options)
        })
    }

    pub fn execute_function(
        &self,
        function: String,
        file: &PathBuf,
    ) -> Result<(), rquickjs::Error> {
        self.ctx.with(|ctx| {
            info!("Starting function execution.");
            info!("Loading file: {}", file.display());
            
            let mut options = EvalOptions::default();
            options.global = true; // Changed to true for proper context access
            options.promise = true;

            let script = format!("{}{}", filesystem::read_to_string(file)?, "\nglobalThis.start = start; globalThis.stop = stop"); 
            
            ctx.eval_with_options::<(), _>(script, options)?;
            info!("File loaded successfully: {}", file.display());

            let globals = ctx.globals();
            let func: rquickjs::Function = globals.get(function.as_str())?;

            // Call the function with no arguments and ignore returned value
            func.call::<(), ()>(())?;
            info!("Function executed successfully: {}", function);

            Ok(())
        })
    }
}