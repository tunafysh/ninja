use owo_colors::OwoColorize;
use std::{fs, env};
use glob::glob;

fn main() -> std::io::Result<()> {
    println!("   {} the binaries","Injecting".bold().green());
    
    let target = env::var("TARGET").expect("Failed to get TARGET environment variable");

    let parent = if path::Path::new("target/").exists() {
        "target/"
    } else {
        ""
    };

    if glob(format!("{parent}kurokage*").as_str()).is_err() && glob(format!("{parent}shurikenctl*").as_str()).is_err() {
        println!("{}", "Kurokage or shurikenctl were not compiled".bold().red());
        return Ok(());
    }

    #[cfg(target_os = "linux")]
    fs::rename(format!("{parent}kurokage"), format!("{parent}kurokage-{}", target))?;

    #[cfg(target_os = "windows")]
    fs::rename(format!("{parent}kurokage.exe"), format!("{parent}kurokage-{}.exe", target))?;


    

    #[cfg(target_os = "linux")]
    fs::rename("shurikenctl", format!("shurikenctl-{}", target))?;

    #[cfg(target_os = "windows")]
    fs::rename("shurikenctl.exe", format!("shurikenctl-{}.exe", target))?;

    println!("    {} binaries successfully", "Injected".bold().green());

    Ok(())
}
