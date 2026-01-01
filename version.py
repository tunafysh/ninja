import tomllib
import re
import json
import os
from datetime import datetime, timezone
from pathlib import Path

# ---------------------------------------------------
# Helpers
# ---------------------------------------------------

def get_version():
    """Read the version from Cargo.toml workspace package."""
    with open("Cargo.toml", "rb") as f:
        config = tomllib.load(f)
    return config["workspace"]["package"]["version"]

def get_changelog_for_version(version: str, changelog_file="CHANGELOG.md") -> str:
    """Extract the changelog entry for this version from CHANGELOG.md."""
    with open(changelog_file, "r", encoding="utf-8") as f:
        content = f.read()

    # Matches headings like "## [1.2.3] - YYYY-MM-DD" or "## 1.2.3"
    pattern = rf"##\s*\[?{re.escape(version)}\]?(?:\s*-\s*\d{{4}}-\d{{2}}-\d{{2}})?\s*\n(.*?)(?=\n##\s|\Z)"
    match = re.search(pattern, content, re.DOTALL)
    return match.group(1).strip() if match else ""

def get_signature(name: str) -> str:
    """
    Find the .sig file for the given artifact folder and return its contents.
    """
    folder = Path("artifacts") / name

    # Pattern based on platform in the name
    if "macos" in name.lower():
        pattern = "*.app.tar.gz.sig"
    elif "windows" in name.lower():
        pattern = "*setup.exe.sig"
    elif "linux" in name.lower():
        pattern = "*.AppImage.sig"
    else:
        pattern = "*.sig"

    sig_file = next(folder.glob(pattern), None)
    return sig_file.read_text(encoding="utf-8") if sig_file and sig_file.is_file() else ""

def get_release_url(name: str, version: str) -> str:
    """
    Construct the GitHub release URL for the artifact.
    They will be hosted under GitHub releases for tag `v{version}`.
    The actual artifact file inside `artifacts/<name>` should match this naming.
    """
    base = os.getenv("GITHUB_REPOSITORY", "").strip()
    if not base:
        raise RuntimeError("GITHUB_REPOSITORY must be set in the environment")

    tag_name = f"v{version}"
    artifact = f"Ninja-{version}-{name}"  # adjust based on real artifact naming
    # Example: https://github.com/owner/repo/releases/download/v1.2.3/Ninja-1.2.3-macos-latest.zip
    return f"https://github.com/{base}/releases/download/{tag_name}/{artifact}"

# ---------------------------------------------------
# Main JSON build
# ---------------------------------------------------

version = get_version()
notes = get_changelog_for_version(version)

root = {
    "version": version,
    "notes": notes,
    "pub_date": datetime.now(timezone.utc).isoformat(),
    "platforms": {
        "linux-x86_64": {
            "url": get_release_url("ubuntu-22.04-build.tar.gz", version),
            "signature": get_signature("ubuntu-22.04-build")
        },
        "linux-aarch64": {
            "url": get_release_url("ubuntu-22.04-arm-build.tar.gz", version),
            "signature": get_signature("ubuntu-22.04-arm-build")
        },
        "windows-x86_64": {
            "url": get_release_url("windows-latest-build.exe", version),
            "signature": get_signature("windows-latest-build")
        },
        "darwin-aarch64": {
            "url": get_release_url("macos-latest-build.tar.gz", version),
            "signature": get_signature("macos-latest-build")
        }
    }
}

print(f"Writing latest.json for version {version}:\n{json.dumps(root, indent=4)}")

with open("latest.json", "w", encoding="utf-8") as f:
    json.dump(root, f, indent=4)
