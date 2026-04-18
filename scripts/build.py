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
def target_dir(target: str | None, warn_missing: bool = True) -> Path:
    base = Path("target")
    tdir = base / target / "release" if target else base / "release"
    if not tdir.exists():
        if warn_missing:
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
    root = Path(__file__).resolve().parent.parent
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
    run([cargo, "build", "--release", "--package", "ninja-core"] + args, "Core build")


def build_ffi(args): 
    cargo = find_cargo()
    print_status("Info", "Building ninja-ffi")
    run([cargo, "build", "--release", "--package", "ninja-ffi"] + args, "Core build")

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
    src = Path("target")/ "release" / src_name
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

    release = target_dir(target, warn_missing=False)
    host_release = target_dir(None, warn_missing=False)
    bins = [("shurikenctl", "ninja-cli")]
    ext = ".exe" if os.name == "nt" else ""

    for bin_name, pkg in bins:
        print_status("Info", f"Building {pkg}")
        run(
            [cargo, "build", "--bin", bin_name, "--package", pkg, "--release"] + args,
            "Build",
        )

        built_candidates = [
            release / f"{bin_name}{ext}",
            Path("target") / target / "release" / f"{bin_name}{ext}",
            host_release / f"{bin_name}{ext}",
        ]
        built = next((p for p in built_candidates if p.exists()), None)
        if built is None:
            root = Path(__file__).resolve().parent.parent
            discovered = sorted(
                root.glob(f"target/**/release/{bin_name}{ext}"),
                key=lambda p: p.stat().st_mtime,
                reverse=True,
            )
            if discovered:
                built = discovered[0]
                print_status("Info", f"Discovered built binary at {built}")
            else:
                print_status("Warn", "Built binary not found in expected target paths; scanning target/**/release...")
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

def run_ps(command: str):
    shell = shutil.which("pwsh") or "powershell"
    return subprocess.call([
        shell,
        "-NoProfile",
        "-ExecutionPolicy", "Bypass",
        "-Command",
        command
    ])

def build_gui(args):
    import platform

    print_status("Info", "Building GUI")
    if platform.system() == "Windows":
        # On windows use powershell + pnpm, as cargo tauri is very slow on windows for some reason
        ensure_tool("pnpm", ["npm", "install", "-g", "pnpm"])
        tauri_cmd = "pnpm dlx @tauri-apps/cli build " + " ".join(args)
        if run_ps(tauri_cmd) != 0:
            print_status("Warn", "pnpm failed, trying cargo tauri as fallback")
            ensure_tool("cargo", ["cargo", "install", "tauri-cli"])
            cargo = find_cargo()
            fallback_cmd = f'cargo tauri build -- ' + " ".join(args)
            run_ps(fallback_cmd)
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
    root = Path(__file__).resolve().parent.parent
    dist_dir = root / "dist"

    # Clean dist/
    if dist_dir.exists():
        shutil.rmtree(dist_dir)
    dist_dir.mkdir()

    # -----------------------
    # 1. Collect Tauri bundles
    # -----------------------
    bundle_roots = [
        root / "target" / "release" / "bundle",
        root / "target" / "debug" / "bundle",
        root / "GUI" / "src-tauri" / "target" / "release" / "bundle",
        root / "GUI" / "src-tauri" / "target" / "debug" / "bundle",
    ]
    bundle_roots.extend(root.glob("target/**/release/bundle"))
    bundle_roots.extend((root / "GUI" / "src-tauri").glob("target/**/release/bundle"))
    existing_bundle_roots = [p for p in bundle_roots if p.exists()]

    if not existing_bundle_roots:
        print_status("Warn", "Tauri bundle directory not found; exporting CLI binaries only")

    allowed_exts = {
        ".msi",
        ".exe",
        ".dmg",
        ".app",
        ".AppImage",
        ".deb",
        ".rpm",
        ".zip",
        ".gz",
        ".sig",
        ".json",
    }

    blacklist = {
        "data.tar.gz",
        "control.tar.gz",
    }

    for bundle_root in existing_bundle_roots:
        for path in bundle_root.rglob("*"):
            # Skip directories unless it's a macOS .app bundle
            if path.is_dir() and path.suffix != ".app" and path.name not in blacklist:
                continue

            # Allow .app bundles
            if path.is_dir() and path.suffix == ".app":
                dest = dist_dir / path.name
                if dest.exists():
                    shutil.rmtree(dest)
                shutil.copytree(path, dest)
                print_status("Info", f"Copied {path} → dist/{path.name}")
                continue

            # Allow files by extension
            if path.is_file() and path.suffix in allowed_exts:
                dest = dist_dir / path.name
                shutil.copy2(path, dest)
                print_status("Info", f"Copied {path} → dist/{path.name}")

    # ----------------------------------------
    # 2. Add shurikenctl Linux binary only
    # ----------------------------------------
    shuriken_candidates = []
    shuriken_candidates.extend((root / "target" / "release").glob("shurikenctl*"))
    shuriken_candidates.extend(
        (root / "GUI" / "src-tauri" / "binaries").glob("shurikenctl*")
    )

    copied_cli = 0
    for binary in shuriken_candidates:
        if binary.is_file():
            dest = dist_dir / binary.name
            shutil.copy2(binary, dest)
            copied_cli += 1
            print_status("Info", f"Included {binary.name} → dist/{binary.name}")

    if copied_cli == 0:
        print_status("Warn", "No shurikenctl binary found to include in dist")

    print_status("Info", "Dist export completed.")

def install():
    """
    Install the CLI binary to /usr/local/bin or equivalent (POSIX only).
    """
    host_target = detect_target()
    src = Path("target") / "release" / f"shurikenctl-{host_target}"
    if not src.exists():
        print_status("Err", f"CLI binary not found at {src}. Build it first.")
        return
    if os.name == "posix":
        dest = Path("/usr/local/bin/shurikenctl")
    else:
        print_status("Err", "Installation is only supported on POSIX systems.")
        return

    shutil.copy2(src, dest)

    src.chmod(0o755)
    dest.chmod(0o755)

    print_status("Info", f"Installed {src.name} → {dest}")

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

    parser.add_argument(
        "--install", action="store_true", help="Install the built CLI binary to /usr/local/bin or equivalent."
    )

    args, passthrough = parser.parse_known_args()
    if args.clean:
        clean()
        return

    if args.install:
        install()
        return

    # please place args manually lol
    if args.libs_only:
        build_lib(args=passthrough)
        return

    if args.cli_only:
        build_cli(args=passthrough)
        export_dist()
        return
    
    if args.ffi_only:
        build_ffi(args=passthrough)
        return

    if args.gui_only:
        build_gui(args=passthrough)
        export_dist()
        return

    # Default: build the cli and gui. 
    build_cli(args=passthrough)
    build_gui(args=passthrough)
    export_dist()


if __name__ == "__main__":
    main()