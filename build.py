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

    sys.stdout.reconfigure(encoding="utf-8", errors="replace")
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

        copy_dir = Path("GUI/src-tauri/binaries")
        copy_dir.mkdir(parents=True, exist_ok=True)
        shutil.copy2(renamed, copy_dir / renamed.name)
        print_status("Info", f"Copied to GUI/binaries")

    if release.exists() and release != host_release:
        shutil.rmtree(release, ignore_errors=True)
        print_status("Rm", f"Cleaned {release}")


def build_gui(args):
    print_status("Info", "Building GUI")
    ensure_tool("pnpm", ["npm", "install", "-g", "pnpm"])
    cmd = ["pnpm", "dlx", "@tauri-apps/cli", "build", "--"] + args
    if subprocess.call(cmd) != 0:
        print_status("Warn", "pnpm failed, trying cargo tauri")
        ensure_tool("cargo")
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


# ===== CLI Entrypoint =====
def main():
    if len(sys.argv) < 2:
        sys.exit(
            "Usage: build.py [buildlibs|buildcli|buildninja|buildall|clean] [-- extra args]"
        )

    cmd = sys.argv[1].lower()
    extra = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []

    actions = {
        "buildlibs": lambda: build_lib(extra),
        "buildcli": lambda: build_cli(extra),
        "buildninja": lambda: (build_lib(extra), build_gui(extra)),
        "buildall": lambda: (build_lib(extra), build_cli(extra), build_gui(extra)),
        "clean": clean,
    }

    if cmd not in actions:
        sys.exit(f"Unknown command: {cmd}")
    actions[cmd]()


if __name__ == "__main__":
    main()
