"""Zero-write check and explicit-write standards projection commands."""

from __future__ import annotations

import stat
from pathlib import Path
import sys

from .contracts import ProjectionContractError
from .rendered_outputs import expected_outputs
from repository_projection.atomic_file import ProjectionFilesystemError, atomic_write


def run(root: Path, mode: str) -> int:
    stale: list[str] = []
    for path, output in expected_outputs(root).items():
        current = path.read_bytes() if path.is_file() and not path.is_symlink() else None
        current_mode = stat.S_IMODE(path.stat().st_mode) if current is not None else None
        if current == output.content and current_mode == output.mode.value:
            continue
        stale.append(str(path.relative_to(root)))
        if mode == "write":
            atomic_write(path, output.content, output.mode)
    if stale and mode == "check":
        print("agent standards projection stale:", file=sys.stderr)
        print("\n".join(f"  {path}" for path in stale), file=sys.stderr)
        return 1
    action = "updated" if stale else "current"
    print(f"agent standards projection {action}: {len(stale)} changed outputs")
    return 0


def main() -> int:
    if len(sys.argv) not in {2, 3} or sys.argv[1] not in {"check", "write"}:
        print("usage: scripts/project-agent-standards <check|write> [repo-root]", file=sys.stderr)
        return 2
    root = Path(sys.argv[2] if len(sys.argv) == 3 else ".").resolve()
    try:
        return run(root, sys.argv[1])
    except (OSError, ValueError, ProjectionContractError, ProjectionFilesystemError) as error:
        print(f"agent standards projection failed: {error}", file=sys.stderr)
        return 1
