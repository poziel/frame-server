"""Validate release versions and stage unique main-build crate versions for CI."""

import os
from pathlib import Path
import re
import tomllib

root = Path(__file__).resolve().parents[1]
workspace_path = root / "Cargo.toml"
workspace = tomllib.loads(workspace_path.read_text())
base = workspace["workspace"]["package"]["version"]
paths = [root / member / "Cargo.toml" for member in workspace["workspace"]["members"]]
for path in paths:
    version = tomllib.loads(path.read_text())["package"]["version"]
    if version not in (base, {"workspace": True}):
        raise SystemExit(f"Version mismatch: {path}")

tag = os.environ.get("RELEASE_TAG", "")
prerelease = os.environ.get("PACKAGE_PRERELEASE", "")
if tag and (tag != "v" + base or prerelease):
    raise SystemExit("Release tag must match the workspace version without a development suffix")
if prerelease and not re.fullmatch(r"dev\.\d+\.\d+", prerelease):
    raise SystemExit("PACKAGE_PRERELEASE must have the form dev.RUN.ATTEMPT")

if prerelease:
    version = base + "-" + prerelease
    for path in [workspace_path, *paths]:
        text = path.read_text()
        text = text.replace(f'version = "{base}"', f'version = "{version}"', 1)
        path.write_text(text, newline="\n")
    print(f"Prepared development crates: {version}")
else:
    print(f"Validated release crates: {base}")
