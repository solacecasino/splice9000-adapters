#!/usr/bin/env python3
"""Turns a "List my adapter" issue into a catalog entry.

Reads the issue form from the ISSUE_BODY environment variable, writes
catalog/<name>.toml, and prints the file's name. The issue text is never
run or passed to a shell; the repo name must look like owner/name.
"""

import os
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CATEGORIES = {"Editing", "Export", "Audio", "Look", "Tools", "Other"}


def field(body: str, label: str) -> str:
    m = re.search(rf"^### {re.escape(label)}\s*\n+(.+?)\s*(?:\n### |\Z)", body, re.M | re.S)
    return m.group(1).strip() if m else ""


def main():
    body = os.environ.get("ISSUE_BODY", "")
    repo = field(body, "GitHub repo").removeprefix("https://github.com/").strip("/ ")
    if not re.fullmatch(r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+", repo):
        sys.exit(f"That doesn't look like a GitHub repo (owner/name): {repo!r}")
    category = field(body, "Category")
    if category not in CATEGORIES:
        category = "Other"

    name = re.sub(r"[^a-z0-9-]+", "-", repo.split("/")[1].lower()).strip("-")
    path = ROOT / "catalog" / f"{name}.toml"
    if path.exists():
        sys.exit(f"catalog/{path.name} already exists")
    path.write_text(f'repo = "{repo}"\ncategory = "{category}"\n')
    print(path.name)


if __name__ == "__main__":
    main()
