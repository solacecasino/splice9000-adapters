#!/usr/bin/env python3
"""Builds the adapters and packs one zip per adapter into dist/.

Each zip holds a single folder with adapter.toml, ready to drop on
splice9000's window. Compiled (Rust) adapters are built for
this computer and named for it, e.g. pdf-export-0.2.0-windows-x64.zip;
script adapters work everywhere and have no platform in the name.

Usage: python3 scripts/build.py [--compiled-only] [--root DIR] [--name ID]

--root builds a different folder: one adapter (a folder with adapter.toml),
or a folder of adapters. --name sets the zip name for a single adapter
(default: its folder's name). Other repos use this through the reusable
workflow .github/workflows/build-adapter.yml.
"""

from __future__ import annotations

import platform
import re
import shutil
import subprocess
import sys
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DIST = ROOT / "dist"
# Only what splice9000 needs to run an adapter: no sources or build output.
SKIP = {"target", "src", "tests", "Cargo.toml", "Cargo.lock", ".gitignore", "__pycache__", ".DS_Store"}


def platform_name():
    system = {"Darwin": "macos", "Windows": "windows", "Linux": "linux"}[platform.system()]
    machine = platform.machine().lower()
    arch = {"amd64": "x64", "x86_64": "x64", "aarch64": "arm64", "arm64": "arm64"}.get(machine, machine)
    return f"{system}-{arch}"


def arg(name: str) -> str | None:
    return sys.argv[sys.argv.index(name) + 1] if name in sys.argv else None


def program_name(folder: Path) -> str | None:
    """The compiled program's name: the crate's package name."""
    m = re.search(r'^\[package\][^\[]*?^name\s*=\s*"(.+?)"', (folder / "Cargo.toml").read_text(), re.M | re.S)
    return m.group(1) if m else None


def zip_adapter(folder: Path, out: Path, top: str, binary: Path | None):
    """Zips the adapter folder as one top-level folder. The compiled
    program, if any, is marked executable for macOS and Linux."""
    name = binary.stem if binary else folder.name
    programs = {folder / name, folder / f"{name}.exe"}
    with zipfile.ZipFile(out, "w", zipfile.ZIP_DEFLATED) as z:
        for path in sorted(folder.rglob("*")):
            rel = path.relative_to(folder)
            if path.is_dir() or rel.parts[0] in SKIP or path.name in SKIP:
                continue
            if path in programs and path != binary:
                continue  # a leftover build for another platform
            if rel.parts[0].startswith(".") or rel.parts[0] == "dist":
                continue  # .git, .github, earlier builds
            info = zipfile.ZipInfo.from_file(path, f"{top}/{rel.as_posix()}")
            info.compress_type = zipfile.ZIP_DEFLATED
            if path == binary:
                info.external_attr = 0o100755 << 16
            z.writestr(info, path.read_bytes())


def main():
    compiled_only = "--compiled-only" in sys.argv
    root = Path(arg("--root") or ROOT).resolve()
    single = (root / "adapter.toml").is_file()
    folders = [root] if single else sorted(p for p in root.iterdir() if (p / "adapter.toml").is_file())
    dist = (root if single else ROOT) / "dist"
    shutil.rmtree(dist, ignore_errors=True)
    dist.mkdir()
    for folder in folders:
        top = (arg("--name") if single else None) or folder.name
        version = re.search(r'^version = "(.+)"', (folder / "adapter.toml").read_text(), re.M).group(1)
        binary = None
        if (folder / "Cargo.toml").is_file():
            subprocess.run(["cargo", "build", "--release"], cwd=folder, check=True)
            exe = ".exe" if platform.system() == "Windows" else ""
            program = program_name(folder) or folder.name
            binary = folder / f"{program}{exe}"
            shutil.copy2(folder / "target/release" / binary.name, binary)
            name = f"{top}-{version}-{platform_name()}.zip"
        elif compiled_only:
            continue
        else:
            name = f"{top}-{version}.zip"
        zip_adapter(folder, dist / name, top, binary)
        print(f"Built {dist / name}", flush=True)


if __name__ == "__main__":
    main()
