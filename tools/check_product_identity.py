#!/usr/bin/env python3
"""Check product names and bytes for the retired experimental identity.

Scan source roots normally; pass built first-party binaries and exports as
explicit roots. Dependency caches and Git object history are not product input.
The prohibited prefix is encoded numerically to keep this checker identity-clean.
"""

from __future__ import annotations

import argparse
import os
from pathlib import Path
import re
import sys


PREFIX = "".join(map(chr, (108, 105, 98, 114, 105, 115, 99, 97, 112, 101, 115)))
PATTERNS = tuple(
    PREFIX.encode(encoding)
    for encoding in ("ascii", "utf-16-le", "utf-16-be", "utf-32-le", "utf-32-be")
)
REDACT = re.compile(re.escape(PREFIX), re.IGNORECASE)
SKIP_DIRS = frozenset(
    {
        ".git",
        "node_modules",
        "target",
        ".venv",
        "venv",
        "__pycache__",
        ".pytest_cache",
        ".mypy_cache",
        ".ruff_cache",
    }
)
CHUNK_BYTES = 1024 * 1024
OVERLAP = max(map(len, PATTERNS)) - 1


def safe_label(value: str) -> str:
    return REDACT.sub("<retired-prefix>", value)


def name_matches(value: str) -> bool:
    return PREFIX in value.casefold()


def content_matches(path: Path) -> bool:
    tail = b""
    with path.open("rb") as stream:
        while chunk := stream.read(CHUNK_BYTES):
            data = tail + chunk
            folded = data.lower()
            if any(pattern in folded for pattern in PATTERNS):
                return True
            tail = data[-OVERLAP:]
    return False


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("roots", nargs="+", type=Path)
    parser.add_argument(
        "--include-builds",
        action="store_true",
        help="Also traverse dependency/build caches; Git object history stays excluded.",
    )
    args = parser.parse_args(argv)
    issues: set[tuple[str, str]] = set()
    checked: set[Path] = set()
    scanned = 0

    def issue(label: str, reason: str) -> None:
        issues.add((safe_label(label), reason))

    def scan_file(path: Path, label: str) -> None:
        nonlocal scanned
        if path in checked:
            return
        checked.add(path)
        if name_matches(path.name):
            issue(label, "prohibited name")
        if path.is_symlink():
            issue(label, "symlink requires a product-local regular copy")
            return
        if not path.is_file():
            issue(label, "missing or non-regular file")
            return
        try:
            scanned += 1
            if content_matches(path):
                issue(label, "prohibited identity in file bytes")
        except OSError:
            issue(label, "file could not be read")

    for index, root in enumerate(args.roots, 1):
        label = f"root-{index}"
        if name_matches(root.name):
            issue(label, "prohibited root name")
        if root.is_symlink():
            issue(label, "symlink root requires a product-local regular copy")
            continue
        if root.is_file():
            scan_file(root, label)
            continue
        if not root.is_dir():
            issue(label, "root is missing or not a directory/file")
            continue

        def walk_error(_error: OSError) -> None:
            issue(label, "directory traversal failed")

        for current, directories, files in os.walk(root, onerror=walk_error):
            base = Path(current)
            kept: list[str] = []
            for directory in sorted(directories):
                path = base / directory
                entry_label = f"{label}/{path.relative_to(root)}"
                if name_matches(directory):
                    issue(entry_label, "prohibited directory name")
                if directory == ".git":
                    if path.is_symlink():
                        issue(entry_label, "Git metadata is an external symlink")
                    else:
                        config = path / "config"
                        if config.exists() or config.is_symlink():
                            scan_file(config, f"{entry_label}/config")
                    continue
                if not args.include_builds and directory in SKIP_DIRS:
                    continue
                if path.is_symlink():
                    issue(entry_label, "symlink requires a product-local regular copy")
                    continue
                kept.append(directory)
            directories[:] = kept
            for filename in sorted(files):
                path = base / filename
                scan_file(path, f"{label}/{path.relative_to(root)}")

    if issues:
        for label, reason in sorted(issues):
            print(f"FAIL {label}: {reason}", file=sys.stderr)
        print(f"Identity check failed: {len(issues)} issue(s), {scanned} file(s) scanned.")
        return 1
    print(f"Identity check passed: {scanned} file(s) scanned.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
