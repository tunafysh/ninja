
#[rquickjs::module]
#[allow(non_upper_case_globals)]
pub mod ninja_api {
    use std::env::consts;

    pub const platform: &str = consts::OS;

    #[rquickjs::function]
    pub fn add(a: i32, b: i32) -> i32 {
        a + b
    }
}