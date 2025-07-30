use std::{fs, path::PathBuf, process::Command};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "xtask")]
#[command(about = "Custom build commands for ninja", long_about = None)]
struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Build the ninja dynamic libraries
    BuildLibs,
    /// Build the command line tools
    BuildTools,
    /// Build the whole ninja application
    BuildNinja,
    /// Clean renamed binaries
    Clean,
    /// Generate manpage for shurikenctl.
    GenManpage,
}

fn main() {
    let cli = Cli::parse();
    
    match cli.command {
        Commands::BuildTools => build_commands(),
        Commands::BuildNinja => {
            build_commands();
            build_gui();
        },
        Commands::Clean => clean_binaries(),
        Commands::GenManpage => generate_manpage(),
    }
}

fn detect_target_triple() -> String {
    let out = Command::new("rustc")
        .arg("-vV")
        .output()
        .expect("rustc -vV failed");

    String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .find(|line| line.starts_with("host:"))
        .unwrap()
        .split_whitespace()
        .nth(1)
        .unwrap()
        .to_string()
}

fn build_commands() {
    let target = detect_target_triple();
    let release_dir = PathBuf::from("target/release");

    let binaries = vec![
        ("kurokage", "ninja-api"),
        ("shurikenctl", "ninja-cli"),
    ];

    for (bin, pkg) in binaries {
        let status = Command::new("cargo")
            .args(["build", "--release", "--bin", bin, "--package", pkg])
            .status()
            .expect("cargo build failed");
        assert!(status.success(), "Build failed for {bin}");

        let orig = release_dir.join(if cfg!(windows) {
            format!("{bin}.exe")
        } else {
            bin.to_string()
        });

        let renamed = release_dir.join(if cfg!(windows) {
            format!("{bin}-{target}.exe")
        } else {
            format!("{bin}-{target}")
        });

        let _ = fs::remove_file(&renamed); // clean existing
        fs::rename(&orig, &renamed).expect("rename failed");

        println!("✓ Renamed {} → {}", orig.display(), renamed.display());
    }
}

fn build_gui() {
    let status = Command::new("cargo")
        .args(["tauri", "build"])
        .current_dir("GUI")
        .status()
        .expect("Tauri build failed");

    assert!(status.success());
}

fn clean_binaries() {
    let target = detect_target_triple();
    let release_dir = PathBuf::from("target/release");

    let binaries = vec![
        "ninja_cli",
    ];

    for bin in binaries {
        let renamed = release_dir.join(if cfg!(windows) {
            format!("{bin}-{target}.exe")
        } else {
            format!("{bin}-{target}")
        });

        if renamed.exists() {
            fs::remove_file(&renamed).expect("Failed to remove binary");
            println!("✗ Removed {}", renamed.display());
        }
    }
}

fn generate_manpage() {
    // use clap_mangen::Man;
    // use std::fs::File;
    // use ninja_cli::cli; // must expose cli() fn from ninja_cli

    // let cmd = cli();
    // let man = Man::new(cmd);
    // let mut buffer = File::create("target/ninja_cli.1").expect("create failed");
    // man.render(&mut buffer).expect("render failed");

    // println!("✓ Generated manpage at target/ninja_cli.1");
}
