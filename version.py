import tomllib
import re
import json
import os
from datetime import datetime, timezone
from pathlib import Path


ARTIFACTS_DIR = Path("artifacts")


def get_version() -> str:
    with open("Cargo.toml", "rb") as f:
        config = tomllib.load(f)
    return config["workspace"]["package"]["version"]


def read_sig_for_asset(asset_name: str, search_root: Path) -> str:
    """
    Look up **/{asset_name}.sig starting from search_root.
    Fails if not found (updater MUST have a signature).
    """
    sig_name = f"{asset_name}.sig"
    matches = list(search_root.glob(f"**/{sig_name}"))

    if not matches:
        raise FileNotFoundError(f"Signature not found: {sig_name}")

    return matches[0].read_text(encoding="utf-8").strip()


def get_changelog_for_version(version: str, changelog_file="CHANGELOG.md") -> str:
    with open(changelog_file, "r", encoding="utf-8") as f:
        content = f.read()

    pattern = rf"""
        ##\s*\[?{re.escape(version)}\]?
        (?:\s*-\s*\d{{4}}-\d{{2}}-\d{{2}})?
        \s*\n
        (.*?)
        (?=\n##\s|\Z)
    """
    match = re.search(pattern, content, re.DOTALL | re.VERBOSE)
    return match.group(1).strip() if match else ""


def make_asset_url(name: str, version: str) -> str:
    repo = os.getenv("GITHUB_REPOSITORY")
    if not repo:
        raise RuntimeError("GITHUB_REPOSITORY must be set")

    tag = f"v{version}"
    return f"https://github.com/{repo}/releases/download/{tag}/{name}"


version = get_version()
notes = get_changelog_for_version(version)

root = {
    "version": version,
    "notes": notes,
    "pub_date": datetime.now(timezone.utc).isoformat(),
    "platforms": {
        "linux-x86_64": {
            "url": make_asset_url(
                f"Ninja_{version}_amd64.AppImage.tar.gz", version
            ),
            "signature": read_sig_for_asset(
                f"Ninja_{version}_amd64.AppImage.tar.gz", ARTIFACTS_DIR
            ),
        },
        "linux-aarch64": {
            "url": make_asset_url(
                f"Ninja_{version}_aarch64.AppImage.tar.gz", version
            ),
            "signature": read_sig_for_asset(
                f"Ninja_{version}_aarch64.AppImage", ARTIFACTS_DIR
            ),
        },
        "windows-x86_64": {
            "url": make_asset_url(
                f"Ninja_{version}_x64-setup.exe", version
            ),
            "signature": read_sig_for_asset(
                f"Ninja_{version}_x64-setup.exe", ARTIFS_DIR
            ),
        },
        "darwin-aarch64": {
            "url": make_asset_url(
                f"Ninja_{version}_aarch64.dmg", version
            ),
            "signature": read_sig_for_asset(
                f"Ninja_{version}_aarch64.dmg", ARTIFACTS_DIR
            ),
        },
    },
}

print(json.dumps(root, indent=4))

with open("latest.json", "w", encoding="utf-8") as f:
    json.dump(root, f, indent=4)
