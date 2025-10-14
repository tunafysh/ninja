use clap::{Parser, Subcommand};
use owo_colors::OwoColorize;
use std::{fs, path::PathBuf, process::Command};

#[derive(Parser)]
#[command(name = "xtask")]
#[command(about = "Custom build commands for ninja", long_about = None)]
struct Cli {
    #[command(subcommand)]
    pub command: Commands,
    #[arg(long)]
    pub debug: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Build the static/dynamic libraries
    BuildLibs,
    /// Build the command line
    BuildCLI,
    /// Build only the ninja GUI
    BuildNinja,
    /// Build the whole ninja application
    BuildAll,
    /// Clean renamed binaries
    Clean,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::BuildLibs => build_library(cli.debug),
        Commands::BuildCLI => {
            build_commands();
        }
        Commands::BuildNinja => {
            build_library(cli.debug);
            build_gui();
        }
        Commands::BuildAll => {
            //  build_library(cli.debug); //for later when i finish the FFI
            build_commands();
            build_gui();
        }
        Commands::Clean => clean_binaries(),
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

fn build_library(debug: bool) {
    Command::new("cargo")
        .args([
            "build",
            if debug { "--debug" } else { "--release" },
            "--package",
            "ninja-core",
        ])
        .status()
        .expect("building the library failed");
}

fn build_commands() {
    let target = detect_target_triple();
    let release_dir = PathBuf::from("target/release");

    let binaries = vec![("shurikenctl", "ninja-cli")];

    for (bin, pkg) in binaries {
        let status = Command::new("cargo")
            .args(["build", "--release", "--bin", bin, "--package", pkg])
            .status()
            .expect("building the CLI failed");
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

        fs::rename(&orig, &renamed).expect("rename failed");

        let copy_dir = PathBuf::from("GUI").join("src-tauri").join("binaries");

        if !copy_dir.exists() {
            fs::create_dir_all(&copy_dir).expect("Failed to create dir");
        }

        let copy_dir = copy_dir.join(if cfg!(windows) {
            format!("{bin}-{target}.exe")
        } else {
            format!("{bin}-{target}")
        });

        println!(
            "{:>12} {} -> {}",
            "Renamed".green().bold(),
            orig.display(),
            renamed.display()
        );
        println!(
            "{:>12} {} -> {}",
            "Copying".green().bold(),
            orig.display(),
            &copy_dir.display()
        );
         
        fs::copy(renamed, copy_dir).expect("Failed to copy shurikenctl");
    }
}

fn build_gui() {

    if cfg!(target_os="windows") {

    Command::new("cargo")
        .args(["install", "tauri-cli"])
        .status()
        .expect("Failed to install tauri cli");

    Command::new("cargo")
        .args(["dlx", "@tauri-apps/cli", "build"])
        .status()
        .expect("Failed to build app");     
    }    
    else {

        let status = Command::new("pnpm")
        .args(["dlx", "@tauri-apps/cli", "build"])
        .status();

    match status {
        Ok(_) => {},
        Err(..) => {
            println!("{:>12} {}", "Info". green().bold(),"pnpm didn't work. switching to cargo");
            let res = Command::new("cargo")
            .args(["tauri", "build"])
            .status();
            match res {
                Ok(_) => {}
                Err(..) => {
                    println!("{:>12} {}", "Info". green().bold(),"cargo didn't work. switching to npm");
                    Command::new("npx")
                        .args(["@tauri-apps/cli", "build"])
                        .status()
                        .expect("Building the GUI failed");
                }
            }
        }
    }
    }
}

fn clean_binaries() {
    let target = detect_target_triple();
    let release_dir = PathBuf::from("target/release");

    let binaries = vec!["ninja_cli"];

    for bin in binaries {
        let renamed = release_dir.join(if cfg!(windows) {
            format!("{bin}-{target}.exe")
        } else {
            format!("{bin}-{target}")
        });

        if renamed.exists() {
            fs::remove_file(&renamed).expect("Failed to remove binary");
            println!("{:>12} {}", "Removed".green().bold(), renamed.display());
        }
    }
}
