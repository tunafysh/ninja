import tomllib
import re
import json
import os
from datetime import datetime, timezone
from pathlib import Path

def get_version():
    with open("Cargo.toml", "rb") as f:
        config = tomllib.load(f)
    return config["workspace"]["package"]["version"]

def get_changelog_for_version(version: str, changelog_file="CHANGELOG.md") -> str:
    with open(changelog_file, "r", encoding="utf-8") as f:
        content = f.read()
    pattern = rf"##\s*\[?{re.escape(version)}\]?(?:\s*-\s*\d{{4}}-\d{{2}}-\d{{2}})?\s*\n(.*?)(?=\n##\s|\Z)"
    match = re.search(pattern, content, re.DOTALL)
    return match.group(1).strip() if match else ""

def read_sig(file_path: Path) -> str:
    return file_path.read_text(encoding="utf-8") if file_path.exists() else ""

def make_asset_url(name: str, version: str) -> str:
    base = os.getenv("GITHUB_REPOSITORY")
    if not base:
        raise RuntimeError("GITHUB_REPOSITORY must be set")
    tag_name = f"v{version}"
    return f"https://github.com/{base}/releases/download/{tag_name}/{name}"

version = get_version()
notes = get_changelog_for_version(version)

root = {
    "version": version,
    "notes": notes,
    "pub_date": datetime.now(timezone.utc).isoformat(),
    "platforms": {
        "linux-x86_64": {
            "url": make_asset_url(f"Ninja_{version}_amd64.AppImage.tar.gz", version),
            "signature": read_sig(Path("artifacts") / f"Ninja_{version}_amd64.AppImage.sig")
        },
        "linux-aarch64": {
            "url": make_asset_url(f"Ninja_{version}_aarch64.AppImage.tar.gz", version),
            "signature": read_sig(Path("artifacts") / f"Ninja_{version}_aarch64.AppImage.sig")
        },
        "windows-x86_64": {
            "url": make_asset_url(f"Ninja_{version}_x64-setup.exe", version),
            "signature": read_sig(Path("artifacts") / f"Ninja_{version}_x64-setup.exe.sig")
        },
        "darwin-aarch64": {
            "url": make_asset_url(f"Ninja_{version}_aarch64.dmg", version),
            "signature": read_sig(Path("artifacts") / f"Ninja_{version}_aarch64.dmg.sig")
        }
    }
}

print(json.dumps(root, indent=4))
with open("latest.json", "w", encoding="utf-8") as f:
    json.dump(root, f, indent=4)
