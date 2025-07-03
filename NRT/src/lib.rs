use rquickjs::{context::EvalOptions, Context, Module, Runtime};

mod util;
use util::*;

mod api;
use api::js_ninja_api;

pub struct NinjaRuntime {
    ctx: Context,
}

impl NinjaRuntime {
    pub fn new() -> Self {
        let rt = Runtime::new().expect("Failed to create runtime");
        let ctx = Context::full(&rt).expect("Failed to create context");

        let mut options = EvalOptions::default();
        options.global = false;
        options.promise = true;

        ctx.with(|ctx| {
            Module::evaluate_def::<js_ninja_api, _>(ctx.clone(), "ninja").unwrap();
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
            options.global = false;
            options.promise = true;
            ctx.eval_with_options(script, options)
        })
    }

    pub fn execute_file(&self, path: &str) -> Result<(), rquickjs::Error> {
        self.ctx.with(|ctx| {

            let mut options = EvalOptions::default();
            options.global = false;
            options.promise = true;
            ctx.eval_file_with_options(path, options)
        })
    }
}
