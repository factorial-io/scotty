#!/usr/bin/env python3
"""Fail when an accepted security exception has passed its expiry date.

Every exception in the files below must carry a marker of the form
`expires: YYYY-MM-DD` (in the `reason` string in deny.toml, in a comment
on the line above the entry in .trivyignore, or on the same line as an
inline `# zizmor: ignore[...]` suppression in a workflow). Exceptions without a marker are reported as errors too,
so none can stay forever by accident.

Usage: check-security-exceptions.py [--today YYYY-MM-DD] [FILE ...]
"""

import argparse
import datetime
import pathlib
import re
import sys

EXPIRES = re.compile(r"expires:\s*(\d{4}-\d{2}-\d{2})")
DEFAULT_FILES = ["deny.toml", ".trivyignore"] + sorted(
    str(p) for p in pathlib.Path(".github/workflows").glob("*.yml")
)


def deny_entries(text):
    """Yield (line_no, line) for each entry of an `ignore = [...]` list.

    Entries are expected one per line, e.g.
    `{ id = "RUSTSEC-2024-0001", reason = "... expires: 2026-12-31" },`
    """
    in_ignore = False
    for no, line in enumerate(text.splitlines(), 1):
        stripped = line.strip()
        if re.match(r"^ignore\s*=\s*\[\s*$", stripped):
            in_ignore = True
        elif in_ignore and stripped.startswith("]"):
            in_ignore = False
        elif in_ignore and stripped and not stripped.startswith("#"):
            yield no, line


def trivyignore_entries(text):
    """Yield (line_no, entry plus preceding comment) for .trivyignore."""
    lines = text.splitlines()
    for no, line in enumerate(lines, 1):
        stripped = line.strip()
        if stripped and not stripped.startswith("#"):
            prev = lines[no - 2] if no >= 2 else ""
            yield no, prev + " " + line


def zizmor_entries(text):
    """Yield (line_no, line) for each inline `zizmor: ignore[...]` suppression."""
    for no, line in enumerate(text.splitlines(), 1):
        if "zizmor: ignore[" in line:
            yield no, line


def check(path, today):
    text = path.read_text()
    if path.name == "deny.toml":
        entries = deny_entries(text)
    elif path.suffix in (".yml", ".yaml"):
        entries = zizmor_entries(text)
    else:
        entries = trivyignore_entries(text)
    errors = []
    for no, content in entries:
        match = EXPIRES.search(content)
        if not match:
            errors.append(f"{path}:{no}: exception has no 'expires: YYYY-MM-DD' marker")
            continue
        expiry = datetime.date.fromisoformat(match.group(1))
        if expiry < today:
            errors.append(f"{path}:{no}: exception expired on {expiry}")
    return errors


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--today", type=datetime.date.fromisoformat, default=datetime.date.today())
    parser.add_argument("files", nargs="*", default=DEFAULT_FILES)
    args = parser.parse_args()

    errors = []
    for name in args.files:
        path = pathlib.Path(name)
        if path.exists():
            errors.extend(check(path, args.today))
    for error in errors:
        print(f"::error::{error}")
    if errors:
        return 1
    print("All security exceptions are within their expiry date.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
