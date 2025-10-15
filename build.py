#!/usr/bin/env python3
import subprocess
import shutil
import os
import sys
from pathlib import Path

# ===== Cargo-style Status Printer =====
def print_status(status: str, message: str):
    """
    status: "Info", "Running", "Removed", "Warning", "Error", etc.
    message: descriptive text
    """
    colors = {
        "Info": "\033[1;32m",     # green bold
        "Running": "\033[1;32m",  # blue bold
        "Removed": "\033[1;33m",  # yellow bold
        "Warning": "\033[1;33m",  # yellow bold
        "Error": "\033[1;31m",    # red bold
    }
    reset = "\033[0m"
    color = colors.get(status, "\033[1;37m")  # default white
    print(f"{color}{status:>12}{reset} {message}")

# ===== Helpers =====
def run_command(cmd: list[str], desc: str):
    print_status("Running", " ".join(cmd))
    result = subprocess.run(cmd)
    if result.returncode != 0:
        raise SystemExit(f"{desc} failed with code {result.returncode}")

def detect_target_triple() -> str:
    try:
        out = subprocess.check_output(["rustc", "-vV"], text=True)
        for line in out.splitlines():
            if line.startswith("host:"):
                return line.split(":")[1].strip()
    except Exception:
        raise SystemExit(f"{red_bold('Error')}: Failed to detect target triple")
    raise SystemExit(f"{red_bold('Error')}: Unable to determine host triple")

def ensure_tool_installed(tool: str, install_cmd: list[str] | None = None):
    """Check if a tool exists in PATH, optionally install it if missing."""
    if shutil.which(tool):
        return True
    print_status("Warning", f"{tool} not found.")
    if install_cmd:
        print_status("Info", f"Installing {tool}...")
        result = subprocess.run(install_cmd)
        if result.returncode != 0:
            raise SystemExit(f"Failed to install {tool}")
    else:
        raise SystemExit(f"Missing required tool {tool}")
    return True

# ===== Tasks =====
def build_library(extra_args: list[str]):
    print_status("Info", "Building ninja-core")
    cmd = ["cargo", "build", "--package", "ninja-core"] + extra_args
    run_command(cmd, "Library build")

def build_commands(extra_args: list[str]):
    target = detect_target_triple()
    release_dir = Path("target/release")
    binaries = [("shurikenctl", "ninja-cli")]

    for bin_name, pkg in binaries:
        print_status("Info", f"Building {pkg} ({bin_name})")
        cmd = ["cargo", "build", "--bin", bin_name, "--package", pkg, "--release"] + extra_args
        run_command(cmd, f"Build {bin_name}")

        orig = release_dir / (bin_name + (".exe" if os.name == "nt" else ""))
        renamed = release_dir / (f"{bin_name}-{target}" + (".exe" if os.name == "nt" else ""))

        shutil.move(orig, renamed)

        copy_dir = Path("GUI") / "src-tauri" / "binaries"
        copy_dir.mkdir(parents=True, exist_ok=True)
        copy_path = copy_dir / renamed.name

        print_status("Removed", str(orig))
        print_status("Info", f"Renamed {renamed.name}")
        print_status("Info", f"Copying to {copy_path}")
        shutil.copy2(renamed, copy_path)

def build_gui(extra_args: list[str]):
    print_status("Info", "Building GUI...")
    if os.name == "nt":
        ensure_tool_installed("cargo")
        ensure_tool_installed("tauri", ["cargo", "install", "tauri-cli"])
        cmd = ["cargo", "tauri", "build", "--"] + extra_args
        run_command(cmd, "GUI build (cargo tauri)")
    else:
        ensure_tool_installed("pnpm", ["npm", "install", "-g", "pnpm"])
        cmd = ["pnpm", "dlx", "@tauri-apps/cli", "build", "--"] + extra_args
        result = subprocess.run(cmd)
        if result.returncode != 0:
            print_status("Warning", "pnpm failed, trying cargo...")
            ensure_tool_installed("cargo")
            cmd = ["cargo", "tauri", "build", "--"] + extra_args
            result = subprocess.run(cmd)
            if result.returncode != 0:
                print_status("Warning", "cargo failed, trying npx...")
                ensure_tool_installed("npx", ["npm", "install", "-g", "npm"])
                cmd = ["npx", "@tauri-apps/cli", "build", "--"] + extra_args
                run_command(cmd, "GUI build (npx)")

def clean_binaries():
    target = detect_target_triple()
    release_dir = Path("target/release")
    binaries = ["ninja_cli"]

    for bin_name in binaries:
        renamed = release_dir / (f"{bin_name}-{target}" + (".exe" if os.name == "nt" else ""))
        if renamed.exists():
            renamed.unlink()
            print_status("Removed", str(renamed))

# ===== CLI =====
def main():
    if len(sys.argv) < 2:
        print("Usage: python xtask.py [buildlibs|buildcli|buildninja|buildall|clean] [-- extra args]")
        raise SystemExit(1)

    command = sys.argv[1].lower()
    if "--" in sys.argv:
        idx = sys.argv.index("--")
        extra_args = sys.argv[idx + 1 :]
    else:
        extra_args = []

    match command:
        case "buildlibs":
            build_library(extra_args)
        case "buildcli":
            build_commands(extra_args)
        case "buildninja":
            build_library(extra_args)
            build_gui(extra_args)
        case "buildall":
            build_library(extra_args)
            build_commands(extra_args)
            build_gui(extra_args)
        case "clean":
            clean_binaries()
        case _:
            print_status("Error", f"Unknown command '{command}'")
            raise SystemExit(1)

if __name__ == "__main__":
    main()