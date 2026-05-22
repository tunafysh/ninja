#!/usr/bin/env python3
# Install logic for the desktop app.
import os
import subprocess
import sys
import platform
import shutil
from pathlib import Path

def get_target_triple():
    """Get the Rust target triple for the current platform."""
    system = platform.system()
    machine = platform.machine()
    
    # Map Python machine names to Rust target triples
    machine_map = {
        "x86_64": "x86_64",
        "amd64": "x86_64",
        "AMD64": "x86_64",
        "arm64": "aarch64",
        "aarch64": "aarch64",
        "armv7l": "armv7",
        "i386": "i686",
        "i686": "i686",
    }
    
    rust_machine = machine_map.get(machine, machine.lower())
    
    if system == "Linux":
        return f"{rust_machine}-unknown-linux-gnu"
    elif system == "Windows":
        return f"{rust_machine}-pc-windows-msvc"
    elif system == "Darwin":
        return f"{rust_machine}-apple-darwin"
    else:
        return None

def get_installation_directory():
    """Get platform-specific installation directory."""
    if platform.system() == "Windows":
        return os.path.join(os.environ["ProgramFiles"], "Ninja")
    elif platform.system() == "Darwin":
        return "/Applications"
    else:  # Linux
        return os.path.expanduser("~/.local/share/ninja")

def get_binary_path():
    """Locate the compiled binary."""
    script_dir = Path(__file__).parent.parent
    possible_paths = [
        script_dir / "target" / "release" / "ninja-desktop",
        script_dir / "target" / "debug" / "ninja-desktop",        
        script_dir / "target" / get_target_triple() / "release" / "ninja-desktop",
        script_dir / "target" / get_target_triple() / "debug" / "ninja-desktop",
    ]
    
    for path in possible_paths:
        if path.exists():
            return path
    
    raise FileNotFoundError(
        "Could not find ninja-desktop binary. Please build it first with: cargo build --release"
    )

def get_icon_path(icon_type="png"):
    """Locate the icon file."""
    script_dir = Path(__file__).parent.parent
    icon_dir = script_dir / "GUI" / "src-tauri" / "icons"
    
    icon_map = {
        "png": icon_dir / "icon.png",
        "ico": icon_dir / "icon.ico",
        "icns": icon_dir / "icon.icns",
    }
    
    path = icon_map.get(icon_type)
    if path and path.exists():
        return path
    
    return None

def copy_files(install_dir):
    """Copy application files to installation directory."""
    binary_path = get_binary_path()
    binary_name = "ninja-desktop"
    
    os.makedirs(install_dir, exist_ok=True)
    dest_binary = os.path.join(install_dir, binary_name)
    
    print(f"Copying {binary_path} to {dest_binary}...")
    shutil.copy2(binary_path, dest_binary)
    
    # Make it executable on Unix systems
    if platform.system() != "Windows":
        os.chmod(dest_binary, 0o755)
    
    print(f"✓ Binary installed to {dest_binary}")

def create_linux_desktop_entry(install_dir):
    """Create .desktop entry for Linux."""
    desktop_dir = os.path.expanduser("~/.local/share/applications")
    os.makedirs(desktop_dir, exist_ok=True)
    
    binary_path = os.path.join(install_dir, "ninja-desktop")
    
    # Copy icon
    icon_src = get_icon_path("png")
    if icon_src:
        icon_dest = os.path.join(install_dir, "icon.png")
        shutil.copy2(icon_src, icon_dest)
        icon_ref = "ninja-desktop"
        print(f"✓ Icon copied to {icon_dest}")
    else:
        icon_ref = ""
        print("⚠ Icon not found, desktop entry will use default icon")
    
    desktop_content = f"""[Desktop Entry]
Type=Application
Name=Ninja Desktop
Comment=A control panel similar to XAMPP
Exec={binary_path}
Icon={icon_ref}
Categories=Development;
Terminal=false
StartupNotify=true
"""
    
    desktop_file = os.path.join(desktop_dir, "ninja-desktop.desktop")
    with open(desktop_file, "w") as f:
        f.write(desktop_content)
    
    os.chmod(desktop_file, 0o644)
    print(f"✓ Desktop entry created at {desktop_file}")
    
    # Update desktop database
    try:
        subprocess.run(["update-desktop-database", desktop_dir], check=False)
        print("✓ Desktop database updated")
    except FileNotFoundError:
        print("  (update-desktop-database not found, skipping)")

def create_windows_shortcuts(install_dir):
    """Create Windows shortcuts."""
    try:
        import win32com.client
        import win32con
        
        desktop = os.path.expanduser("~/Desktop")
        start_menu = os.path.join(
            os.environ["APPDATA"],
            "Microsoft",
            "Windows",
            "Start Menu",
            "Programs"
        )
        
        binary_path = os.path.join(install_dir, "ninja-desktop.exe")
        
        # Copy icon
        icon_src = get_icon_path("ico")
        if icon_src:
            icon_dest = os.path.join(install_dir, "icon.ico")
            shutil.copy2(icon_src, icon_dest)
            print(f"✓ Icon copied to {icon_dest}")
        else:
            icon_dest = binary_path
            print("⚠ Icon not found, using binary as icon")
        
        for shortcut_dir in [desktop, start_menu]:
            os.makedirs(shortcut_dir, exist_ok=True)
            shortcut_path = os.path.join(shortcut_dir, "Ninja Desktop.lnk")
            
            shell = win32com.client.Dispatch("WScript.Shell")
            shortcut = shell.CreateShortCut(shortcut_path)
            shortcut.Targetpath = binary_path
            shortcut.WorkingDirectory = os.path.dirname(binary_path)
            shortcut.IconLocation = icon_dest
            shortcut.save()
            
            print(f"✓ Shortcut created at {shortcut_path}")
    except ImportError:
        print("⚠ pywin32 not installed, skipping Windows shortcuts")
        print("  Install with: pip install pywin32")

def create_macos_app_bundle(install_dir):
    """Create macOS app bundle."""
    app_dir = os.path.join(install_dir, "Ninja.app")
    contents_dir = os.path.join(app_dir, "Contents")
    macos_dir = os.path.join(contents_dir, "MacOS")
    
    os.makedirs(macos_dir, exist_ok=True)
    
    binary_path = get_binary_path()
    dest_binary = os.path.join(macos_dir, "ninja-desktop")
    shutil.copy2(binary_path, dest_binary)
    os.chmod(dest_binary, 0o755)
    
    # Create Info.plist
    plist_content = """<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key>
    <string>en</string>
    <key>CFBundleExecutable</key>
    <string>ninja-desktop</string>
    <key>CFBundleIdentifier</key>
    <string>com.tunafysh.ninja</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleName</key>
    <string>Ninja Desktop</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>1.13.0</string>
    <key>CFBundleVersion</key>
    <string>1</string>
</dict>
</plist>
"""
    
    plist_path = os.path.join(contents_dir, "Info.plist")
    with open(plist_path, "w") as f:
        f.write(plist_content)
    
    print(f"✓ macOS app bundle created at {app_dir}")

def create_shortcuts(install_dir):
    """Create platform-specific shortcuts/desktop entries."""
    system = platform.system()
    
    if system == "Linux":
        create_linux_desktop_entry(install_dir)
    elif system == "Windows":
        create_windows_shortcuts(install_dir)
    elif system == "Darwin":
        create_macos_app_bundle(install_dir)

def main():
    system = platform.system()
    install_dir = get_installation_directory()
    
    print(f"🚀 Installing Ninja Desktop on {system}")
    print(f"📁 Installation directory: {install_dir}\n")
    
    try:
        # Copy files
        copy_files(install_dir)
        print()
        
        # Create shortcuts
        create_shortcuts(install_dir)
        print()
        
        print(f"✅ Ninja Desktop has been successfully installed!")
        
        if system == "Linux":
            print(f"\n📝 You can now:")
            print(f"  • Launch from your applications menu")
            print(f"  • Run: {os.path.join(install_dir, 'ninja-desktop')}")
        elif system == "Windows":
            print(f"\n📝 Shortcuts have been created on your Desktop and Start Menu")
        elif system == "Darwin":
            print(f"\n📝 You can now open: {os.path.join(install_dir, 'Ninja.app')}")
    
    except Exception as e:
        print(f"❌ Installation failed: {e}", file=sys.stderr)
        sys.exit(1)

if __name__ == "__main__":
    main()