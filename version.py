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

def read_sig_for_asset(asset_name: str, search_root: Path = Path(".")) -> str:
    """
    Look up **/{asset_name}.sig starting from search_root.
    Returns empty string if not found.
    """
    sig_name = f"{asset_name}.sig"
    matches = list(search_root.glob(f"**/{sig_name}"))
    if not matches:
        return ""
    return matches[0].read_text(encoding="utf-8")

def get_changelog_for_version(version: str, changelog_file="CHANGELOG.md") -> str:
    with open(changelog_file, "r", encoding="utf-8") as f:
        content = f.read()
    pattern = rf"##\s*\[?{re.escape(version)}\]?(?:\s*-\s*\d{{4}}-\d{{2}}-\d{{2}})?\s*\n(.*?)(?=\n##\s|\Z)"
    match = re.search(pattern, content, re.DOTALL)
    return match.group(1).strip() if match else ""

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
            "signature": read_sig_for_asset(f"Ninja_{version}_amd64.AppImage", Path("artifacts"))
        },
        "linux-aarch64": {
            "url": make_asset_url(f"Ninja_{version}_aarch64.AppImage.tar.gz", version),
            "signature": read_sig_for_asset(f"Ninja_{version}_aarch64.AppImage.sig", Path("artifacts"))
        },
        "windows-x86_64": {
            "url": make_asset_url(f"Ninja_{version}_x64-setup.exe", version),
            "signature": read_sig_for_asset(f"Ninja_{version}_x64-setup.exe.sig", Path("artifacts"))
        },
        "darwin-aarch64": {
            "url": make_asset_url(f"Ninja_{version}_aarch64.dmg", version),
            "signature": read_sig_for_asset(f"Ninja_{version}_aarch64.dmg.sig", Path("artifacts"))
        }
    }
}

print(json.dumps(root, indent=4))
with open("latest.json", "w", encoding="utf-8") as f:
    json.dump(root, f, indent=4)
