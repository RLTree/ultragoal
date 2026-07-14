"""Zero-write check and explicit-write generated-authority commands."""

from __future__ import annotations

from pathlib import Path
import sys

from .contracts import AuthorityContractError
from .rendered_registry import expected_bytes
from repository_projection.atomic_file import (
    ProjectionFileMode,
    ProjectionFilesystemError,
    atomic_write,
)


def run(root: Path, mode: str) -> int:
    output = root / "migration" / "generated-surface-authority.json"
    expected = expected_bytes(root)
    current = output.read_bytes() if output.is_file() and not output.is_symlink() else None
    if current == expected:
        print("generated surface authority current")
        return 0
    if mode == "check":
        print("generated surface authority stale", file=sys.stderr)
        return 1
    atomic_write(output, expected, ProjectionFileMode.REGULAR)
    print("generated surface authority updated")
    return 0


def main() -> int:
    if len(sys.argv) not in {2, 3} or sys.argv[1] not in {"check", "write"}:
        print("usage: scripts/project-generated-authority <check|write> [repo-root]", file=sys.stderr)
        return 2
    root = Path(sys.argv[2] if len(sys.argv) == 3 else ".").resolve()
    try:
        return run(root, sys.argv[1])
    except (OSError, ValueError, AuthorityContractError, ProjectionFilesystemError) as error:
        print(f"generated authority projection failed: {error}", file=sys.stderr)
        return 1
