#!/usr/bin/env python3
import subprocess
import shutil
import os
import sys
import glob
from pathlib import Path


# ===== Pretty printing =====
def print_status(status: str, msg: str):
    colors = {
        "Info": "\033[1;32m",
        "Run": "\033[1;34m",
        "Warn": "\033[1;33m",
        "Err": "\033[1;31m",
        "Rm": "\033[1;33m",
    }
    reset = "\033[0m"
    color = colors.get(status, "\033[1;37m")
    arrow = "->" if os.name == "nt" else "→"
    msg = msg.replace("→", arrow)

    print(f"{color}{status:>12}{reset} {msg}")


# ===== Utilities =====
def find_cargo() -> str:
    """Find absolute path to cargo executable."""
    cargo_path = shutil.which("cargo")
    if not cargo_path:
        sys.exit("Error: Cargo not found. Make sure Rust is installed and in PATH.")
    return os.path.abspath(cargo_path)


def run(cmd: list[str], desc: str):
    print_status("Run", " ".join(cmd))
    if subprocess.call(cmd) != 0:
        sys.exit(f"{desc} failed")


# ===== Build logic =====
def target_dir(target: str | None) -> Path:
    base = Path("target")
    tdir = base / target / "release" if target else base / "release"
    if not tdir.exists():
        print_status("Warn", f"{tdir} not found, using target/release")
        return base / "release"
    return tdir


def detect_target() -> str:
    try:
        out = subprocess.check_output(["rustc", "-vV"], text=True)
        return next(
            l.split(":")[1].strip() for l in out.splitlines() if l.startswith("host:")
        )
    except Exception:
        sys.exit("Failed to detect target triple")


def extract_target(args: list[str]) -> str | None:
    if "--target" in args:
        i = args.index("--target")
        return args[i + 1] if i + 1 < len(args) else None
    return None


# ===== Helper: Find and relocate binary =====
# ===== Helper: Find and relocate binary =====
def find_and_place_binary(extra_args=None):
    """
    Find any shurikenctl[.exe] in target/**/release and copy it to GUI/src-tauri/binaries.
    Renames the binary to include the target triple, using --target if specified.
    """
    root = Path(__file__).parent.resolve()
    binaries_dir = (root / "GUI" / "src-tauri" / "binaries").resolve()
    binaries_dir.mkdir(parents=True, exist_ok=True)

    patterns = [
        "target/**/release/shurikenctl",
        "target/**/release/shurikenctl.exe",
    ]
    found_files = []
    for pattern in patterns:
        found_files.extend(glob.glob(str(root / pattern), recursive=True))

    if not found_files:
        print_status("Err", "No shurikenctl binary found in any release folder.")
        return

    found_files.sort(key=lambda f: os.path.getmtime(f), reverse=True)
    latest = Path(found_files[0])

    # Determine target triple
    triple = None
    if extra_args:
        if "--target" in extra_args:
            i = extra_args.index("--target")
            if i + 1 < len(extra_args):
                triple = extra_args[i + 1]
    if not triple:
        # Fallback to rustc detection
        try:
            out = subprocess.check_output(["rustc", "-vV"], text=True)
            triple = next(
                l.split(":")[1].strip()
                for l in out.splitlines()
                if l.startswith("host:")
            )
        except Exception:
            sys.exit("Failed to detect target triple")

    # Rename binary with target triple
    dest_name = f"{latest.stem}-{triple}{latest.suffix}"
    dest = binaries_dir / dest_name

    shutil.copy2(latest, dest)
    print_status(
        "Info",
        f"Found and copied {latest.relative_to(root)} → {dest.relative_to(root)}",
    )


# ===== Build steps =====
def build_lib(args):
    cargo = find_cargo()
    print_status("Info", "Building ninja-core")
    run([cargo, "build", "--package", "ninja-core"] + args, "Core build")


def build_ffi(args): 
    cargo = find_cargo()
    print_status("Info", "Building ninja-ffi")
    run([cargo, "build", "--package", "ninja-ffi"] + args, "Core build")
    
    # Determine if release build
    release = "-r" in args or "--release" in args
    build_type = "release" if release else "debug"

    # Determine target library name based on OS
    if sys.platform.startswith("win"):
        src_name = "libninja_ffi.dll"
        out_name = "ninja.dll"
    elif sys.platform.startswith("darwin"):
        src_name = "libninja_ffi.dylib"
        out_name = "ninja.dylib"
    else:
        src_name = "libninja_ffi.so"
        out_name = "ninja.so"

    # Source path
    src = Path("target") / build_type / src_name
    if not src.exists():
        print(f"Error: {src_name} not found at {src}")
        return

    # Destination path
    sdk_dir = Path("./sdk")
    include_dir = sdk_dir / "include"
    sdk_dir.mkdir(exist_ok=True)
    include_dir.mkdir(exist_ok=True)

    dest = sdk_dir / out_name
    shutil.copy(src, dest)
    print_status("Info", f"Copied {src_name} → {dest}")


def build_cli(args):
    cargo = find_cargo()
    target = extract_target(args) or detect_target()
    print_status("Info", f"Target: {target}")

    release = target_dir(target)
    host_release = target_dir(None)
    bins = [("shurikenctl", "ninja-cli")]
    ext = ".exe" if os.name == "nt" else ""

    for bin_name, pkg in bins:
        print_status("Info", f"Building {pkg}")
        run(
            [cargo, "build", "--bin", bin_name, "--package", pkg, "--release"] + args,
            "Build",
        )

        built = release / f"{bin_name}{ext}"
        if not built.exists():
            print_status("Warn", f"{built} not found, scanning target/**/release...")
            find_and_place_binary(args)
            return

        host_release.mkdir(parents=True, exist_ok=True)
        dest = host_release / f"{bin_name}{ext}"
        shutil.move(built, dest)
        print_status("Info", f"Moved {built.name} → {dest.name}")

        renamed = host_release / f"{bin_name}-{target}{ext}"
        dest.rename(renamed)
        print_status("Info", f"Renamed to {renamed.name}")

        copy_dir = Path("GUI/src-tauri")
        copy_dir.mkdir(parents=True, exist_ok=True)
        shutil.copy2(renamed, copy_dir / renamed.name)
        print_status("Info", "Copied to GUI")

    if release.exists() and release != host_release:
        shutil.rmtree(release, ignore_errors=True)
        print_status("Rm", f"Cleaned {release}")


def build_gui(args):
    import platform

    print_status("Info", "Building GUI")

    if platform.system() == "Windows":
        # Use cargo-installed Tauri CLI
        ensure_tool("cargo")
        cargo = find_cargo()
        # Install tauri-cli if not already installed
        try:
            subprocess.check_call([cargo, "tauri", "--version"])
        except subprocess.CalledProcessError:
            print_status("Info", "Installing tauri-cli via cargo")
            run([cargo, "install", "tauri-cli"], "Install tauri-cli")

        run([cargo, "tauri", "build", "--"] + args, "GUI build")
    else:
        # On Linux/macOS, use pnpm for speed
        ensure_tool("pnpm", ["npm", "install", "-g", "pnpm"])
        cmd = ["pnpm", "dlx", "@tauri-apps/cli", "build"] + args
        if subprocess.call(cmd) != 0:
            print_status("Warn", "pnpm failed, trying cargo tauri as fallback")
            ensure_tool("cargo", ["cargo", "install", "tauri-cli"])
            cargo = find_cargo()
            run([cargo, "tauri", "build", "--"] + args, "GUI build")



def ensure_tool(name, install_cmd=None):
    if shutil.which(name):
        return
    print_status("Warn", f"{name} not found")
    if install_cmd:
        run(install_cmd, f"Install {name}")
    else:
        sys.exit(f"{name} is required")


def clean():
    host_target = detect_target()
    host_release = target_dir(None)
    ext = ".exe" if os.name == "nt" else ""
    bins = ["shurikenctl", "ninja-cli"]

    for b in bins:
        for p in [host_release / f"{b}{ext}", host_release / f"{b}-{host_target}{ext}"]:
            if p.exists():
                p.unlink()
                print_status("Rm", str(p))

    gui_dir = Path("GUI/src-tauri/binaries")
    for p in gui_dir.glob("*"):
        if p.is_file():
            p.unlink()
            print_status("Rm", f"GUI binary {p}")

def export_dist():
    """
    Create a top-level dist/ folder and copy final Tauri bundle
    artifacts + shurikenctl Linux binary into it.
    """
    root = Path(__file__).parent.resolve()
    dist_dir = root / "dist"

    # Clean existing dist folder
    if dist_dir.exists():
        shutil.rmtree(dist_dir)
    dist_dir.mkdir(exist_ok=True)

    # -----------------------
    # 1. Collect Tauri bundles
    # -----------------------
    tauri_root = root / "target" / "release"

    patterns = [
        "**/*.msi",
        "**/*.exe",
        "**/*.dmg",
        "**/*.app",
        "**/*.AppImage*",
        "**/*.deb",
        "**/*.rpm*",
        "**/*.zip",
        "**/*.gz",
        "**/*.sig"
    ]

    found_artifacts = []
    for pattern in patterns:
        found_artifacts.extend(tauri_root.glob(pattern))

    # Copy Tauri outputs
    # Copy Tauri outputs
    for f in found_artifacts:
        # --- Skip unwanted Debian internals ---
        if f.name in ("control.tar.gz", "data.tar.gz"):
            continue

        dest = dist_dir / f.name
        if f.is_dir():
            shutil.copytree(f, dest)
        else:
            shutil.copy2(f, dest)
        print_status("Info", f"Copied {f} → dist/{f.name}")

    # ----------------------------------------
    # 2. Add shurikenctl Linux binary only
    # ----------------------------------------
    shuri = root / "target" / "release" / "shurikenctl"
    if shuri.exists():
        shutil.copy2(shuri, dist_dir / "shurikenctl")
        print_status("Info", "Included Linux shurikenctl → dist/shurikenctl")
    else:
        print_status("Warn", "Linux shurikenctl binary not found")

    print_status("Info", "Dist export completed.")


# ===== CLI Entrypoint =====
def main():
    import argparse

    parser = argparse.ArgumentParser(
        description="Ninja build script: builds libs, CLI, GUI, or cleans binaries."
    )

    parser.add_argument(
        "--clean", action="store_true", help="Clean all build artifacts and binaries."
    )
    parser.add_argument(
        "--libs-only", action="store_true", help="Build only the ninja-core library."
    )
    parser.add_argument(
        "--cli-only", action="store_true", help="Build only the CLI binaries."
    )
    parser.add_argument(
        "--ffi-only", action="store_true", help="Build only the FFI library."
    )
    parser.add_argument(
        "--gui-only", action="store_true", help="Build only the GUI."
    )

    args = parser.parse_args()
    if args.clean:
        clean()
        return

    # please place args manually lol
    if args.libs_only:
        build_lib(args=[])
        return

    if args.cli_only:
        build_cli(args=[])
        export_dist()
        return
    
    if args.ffi_only:
        build_ffi(args=[])
        return

    if args.gui_only:
        build_gui(args=[])
        export_dist()
        return

    # Default: build the cli and gui. 
    build_cli(args=[])
    build_gui(args=[])
    export_dist()


if __name__ == "__main__":
    main()
