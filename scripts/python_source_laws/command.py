"""Python source-law command."""

from __future__ import annotations

from pathlib import Path
import sys

from .scope import discover
from .syntax import failures


def main() -> int:
    if len(sys.argv) > 2:
        print("usage: scripts/check-python-source-laws [repo-root]", file=sys.stderr)
        return 2
    root = Path(sys.argv[1] if len(sys.argv) == 2 else ".").resolve()
    sources, found = discover(root)
    for path in sources:
        found.extend(failures(root, path))
    if found:
        for failure in sorted(set(found)):
            print(failure.diagnostic(), file=sys.stderr)
        return 1
    print(f"python source laws pass: {len(sources)} typed sources")
    return 0
