use clap::{Parser, Subcommand};
use owo_colors::OwoColorize;
use std::{fs, path::PathBuf, process::Command};

#[derive(Parser)]
#[command(name = "xtask")]
#[command(about = "Custom build commands for ninja", long_about = None)]
struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Build the static/dynamic libraries
    BuildLibs {
        /// Extra args passed after `--`, e.g. `--target aarch64-apple-darwin`
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        extra_args: Option<Vec<String>>,
    },
    /// Build the command line
    BuildCLI{
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        extra_args: Option<Vec<String>>,
    },
    /// Build only the ninja GUI
    BuildNinja {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        extra_args: Option<Vec<String>>,
    },
    /// Build the whole ninja application
    BuildAll {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        extra_args: Option<Vec<String>>,
    },
    /// Clean renamed binaries
    Clean,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::BuildLibs { extra_args } => build_library(extra_args),
        Commands::BuildCLI { extra_args } => build_commands(extra_args),
        Commands::BuildNinja { extra_args } => {
            build_library(extra_args.clone());
            build_gui(extra_args);
        }
        Commands::BuildAll { extra_args } => {
            build_library(extra_args.clone());
            build_commands(extra_args.clone());
            build_gui(extra_args);
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

fn build_library(extra_args: Option<Vec<String>>) {
    let mut args: Vec<String> = vec!["build".to_string(), "--package".to_string(), "ninja-core".to_string()];
    if let Some(extra_args) = extra_args{
        for arg in extra_args {
            args.push(arg);
        }
    }

    let status = Command::new("cargo")
        .args(&args)
        .status()
        .expect("building the library failed");

    assert!(status.success(), "Library build failed");
}

fn build_commands(extra_args: Option<Vec<String>>) {
    let target = detect_target_triple();
    let release_dir = PathBuf::from("target/release");
    let binaries = vec![("shurikenctl", "ninja-cli")];

    for (bin, pkg) in binaries {
        // Base cargo args
        let mut args: Vec<String> = vec![
            "build".to_string(),
            "--bin".to_string(),
            bin.to_string(),
            "--package".to_string(),
            pkg.to_string(),
            "--release".to_string(),
        ];

        // Append extra args, if any
        if let Some(extra_args) = extra_args.clone() {
            args.extend(extra_args);
        }

        // Run the build
        let status = Command::new("cargo")
            .args(&args)
            .status()
            .expect("building the CLI failed");
        assert!(status.success(), "Build failed for {bin}");

        // Rename
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

        // Copy into GUI binaries directory
        let copy_dir = PathBuf::from("GUI").join("src-tauri").join("binaries");
        if !copy_dir.exists() {
            fs::create_dir_all(&copy_dir).expect("Failed to create dir");
        }

        let copy_path = copy_dir.join(if cfg!(windows) {
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
            renamed.display(),
            copy_path.display()
        );

        fs::copy(renamed, copy_path).expect("Failed to copy shurikenctl");
    }
}

fn build_gui(extra_args: Option<Vec<String>>) {
    println!("{:>12} {}", "Info".green().bold(), "Building GUI...");

    if cfg!(target_os = "windows") {
        Command::new("cargo")
            .args(["install", "tauri-cli"])
            .status()
            .expect("Failed to install tauri cli");

        let mut args: Vec<String> = vec!["tauri".to_string(), "build".to_string(),"--".to_string()];
        if let Some(extra_args) = extra_args{
        for arg in extra_args {
            args.push(arg);
        }
    }

        Command::new("cargo")
            .args(&args)
            .status()
            .expect("Failed to build app");
    } else {
        let mut args: Vec<String> = vec!["@tauri-apps/cli".to_string(), "build".to_string(), "--".to_string()];
        if let Some(extra_args) = extra_args.clone() {
            for arg in extra_args {
                args.push(arg);
            }
        }

        let status = Command::new("pnpm")
            .args(["dlx"].iter().map(|s| s.to_string()).chain(args.into_iter()))
            .status();


        match status {
            Ok(_) => {}
            Err(..) => {
                println!("{:>12} {}", "Info".green().bold(), "pnpm didn't work. switching to cargo");
                let mut cargo_args: Vec<String> = vec!["tauri".to_string(), "build".to_string(), "--".to_string()];
                if let Some(extra_args) = extra_args.clone() {
                    for arg in extra_args {
                        cargo_args.push(arg);
                    }
                }

                let res = Command::new("cargo").args(&cargo_args).status();
                match res {
                    Ok(_) => {}
                    Err(..) => {
                        println!(
                            "{:>12} {}",
                            "Info".green().bold(),
                            "cargo didn't work. switching to npm"
                        );
                        let mut npm_args: Vec<String>  = vec!["@tauri-apps/cli".to_string(), "build".to_string(), "--".to_string()];
                        if let Some(extra_args) = extra_args{
                            for arg in extra_args {
                                npm_args.push(arg);
                            }
                        }
                        Command::new("npx")
                            .args(&npm_args)
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
