#!/usr/bin/env python3
import subprocess
import shutil
import os
import sys
from pathlib import Path

# ===== Cargo-style Status Printer =====
def print_status(status: str, message: str):
    colors = {
        "Info": "\033[1;32m",     # green bold
        "Running": "\033[1;34m",  # blue bold
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
        raise SystemExit("Error: Failed to detect target triple")
    raise SystemExit("Error: Unable to determine host triple")

def extract_target_from_args(extra_args: list[str]) -> str | None:
    if "--target" in extra_args:
        idx = extra_args.index("--target")
        if idx + 1 < len(extra_args):
            return extra_args[idx + 1]
    return None

def ensure_tool_installed(tool: str, install_cmd: list[str] | None = None):
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

def get_release_dir(target: str | None) -> Path:
    base = Path("target")
    if target:
        return base / target / "release"
    return base / "release"

# ===== Tasks =====
def build_library(extra_args: list[str]):
    print_status("Info", "Building ninja-core")
    cmd = ["cargo", "build", "--package", "ninja-core"] + extra_args
    run_command(cmd, "Library build")

def build_commands(extra_args: list[str]):
    target = extract_target_from_args(extra_args)
    if target:
        print_status("Info", f"Using provided target: {target}")
    else:
        target = detect_target_triple()
        print_status("Info", f"Detected host target: {target}")

    release_dir = get_release_dir(target)
    host_release_dir = get_release_dir(None)
    binaries = [("shurikenctl", "ninja-cli")]

    for bin_name, pkg in binaries:
        print_status("Info", f"Building {pkg} ({bin_name})")
        cmd = ["cargo", "build", "--bin", bin_name, "--package", pkg, "--release"] + extra_args
        run_command(cmd, f"Build {bin_name}")

        ext = ".exe" if os.name == "nt" else ""
        built_bin = release_dir / f"{bin_name}{ext}"
        if not built_bin.exists():
            raise SystemExit(f"Error: {built_bin} not found (build may have failed).")

        # --- Move binary to target/release ---
        host_release_dir.mkdir(parents=True, exist_ok=True)
        dest_bin = host_release_dir / f"{bin_name}{ext}"

        if dest_bin.exists():
            dest_bin.unlink()
            print_status("Removed", f"Existing {dest_bin}")

        shutil.move(str(built_bin), str(dest_bin))
        print_status("Info", f"Moved {built_bin} → {dest_bin}")

        # --- Rename for target signature ---
        renamed = host_release_dir / (f"{bin_name}-{target}{ext}")
        shutil.copy2(dest_bin, renamed)
        print_status("Info", f"Copied {dest_bin.name} → {renamed.name}")

        # --- Copy to GUI binaries directory ---
        copy_dir = Path("GUI") / "src-tauri" / "binaries"
        copy_dir.mkdir(parents=True, exist_ok=True)
        copy_path = copy_dir / renamed.name
        shutil.copy2(renamed, copy_path)
        print_status("Info", f"Copied to {copy_path}")

    # --- Cleanup: remove crosscompiled release directory ---
    try:
        if release_dir.exists() and release_dir != host_release_dir:
            shutil.rmtree(release_dir)
            print_status("Removed", f"Cleaned {release_dir}")
    except Exception as e:
        print_status("Warning", f"Failed to clean {release_dir}: {e}")

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
    host_target = detect_target_triple()
    host_release_dir = get_release_dir(None)
    binaries = ["shurikenctl", "ninja-cli"]

    for bin_name in binaries:
        ext = ".exe" if os.name == "nt" else ""
        paths = [
            host_release_dir / f"{bin_name}{ext}",
            host_release_dir / f"{bin_name}-{host_target}{ext}",
        ]
        for p in paths:
            if p.exists():
                p.unlink()
                print_status("Removed", str(p))

    # Optionally clean GUI binaries directory
    gui_bin_dir = Path("GUI") / "src-tauri" / "binaries"
    if gui_bin_dir.exists():
        for p in gui_bin_dir.glob("*"):
            if p.is_file():
                p.unlink()
                print_status("Removed", f"GUI binary {p}")

# ===== CLI =====
def main():
    if len(sys.argv) < 2:
        print("Usage: python build.py [buildlibs|buildcli|buildninja|buildall|clean] [-- extra args]")
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
