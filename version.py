import tomllib
import re
import json
from datetime import datetime, timezone
from pathlib import Path

def get_version():
    with open("Cargo.toml", "rb") as f:
        config = tomllib.load(f)
    return config['workspace']["package"]["version"]

def get_changelog_for_version(version: str, changelog_file="CHANGELOG.md") -> str:
    with open(changelog_file, "r", encoding="utf-8") as f:
        content = f.read()
    
    # Match headings like ## [1.2.3] - 2025-11-30 or ## 1.2.3
    pattern = rf"##\s*\[?{re.escape(version)}\]?(?:\s*-\s*\d{{4}}-\d{{2}}-\d{{2}})?\s*\n(.*?)(?=\n##\s|\Z)"
    match = re.search(pattern, content, re.DOTALL)
    if match:
        return match.group(1).strip()
    return ""

def get_signature(name: str) -> str:
    folder = Path("artifacts") / name
    
    # Determine pattern based on platform keyword in name
    if "macos" in name.lower():
        pattern = "*.app.tar.gz.sig"
    elif "windows" in name.lower():
        pattern = "*setup.exe.sig"
    elif "linux" in name.lower():
        pattern = "*.AppImage.sig"
    else:
        pattern = "*.sig"  # fallback
    
    # Find the first matching file
    sig_file = next(folder.glob(pattern), None)
    
    if sig_file and sig_file.is_file():
        # Read and return its contents
        return sig_file.read_text(encoding="utf-8")
    
    return ""  # return empty string if no file found

root = {
    "version": get_version(),
    "notes": get_changelog_for_version(get_version()),
    "pub_date": datetime.now(timezone.utc).isoformat(),
    "platforms": {
        "linux-x86_64": {
            "signature": get_signature("ubuntu-22.04-build"),
            "url": ""
        },
        "linux-aarch64": {
            "signature": get_signature("ubuntu-22.04-arm-build"),
            "url": ""
        },
        "windows-x86_64": {
            "signature": get_signature("windows-latest-build"),
            "url": ""
        },
        "darwin-aarch64": {
            "signature": get_signature("macos-latest-build"),
            "url": ""
        }
    }
}

print(f"Writing to latest.json: {root}")

with open("latest.json", "w") as f:
    json.dump(root, f, indent=4)
