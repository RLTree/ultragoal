"""Exact active script-source discovery."""

from __future__ import annotations

from pathlib import Path

from .violation import SourceViolation


SOURCE_ROOTS = (
    "scripts",
    "templates/scripts",
    "templates/.codex",
    ".harness",
    ".codex-plugin",
    "hooks",
    "mcp",
    "connectors",
    "install",
)


def discover(root: Path) -> tuple[list[Path], list[SourceViolation]]:
    sources: list[Path] = []
    failures: list[SourceViolation] = []
    paths = sorted(
        path
        for relative in SOURCE_ROOTS
        if (source_root := root / relative).is_dir()
        for path in source_root.rglob("*")
    )
    for path in paths:
        relative = str(path.relative_to(root))
        if path.is_symlink():
            failures.append(SourceViolation(relative, 0, "script_symlink_rejected"))
            continue
        if not path.is_file():
            continue
        content = path.read_bytes()
        if path.suffix == ".py" or content.startswith(b"#!/usr/bin/env python3\n"):
            sources.append(path)
        elif content.startswith(b"#!/usr/bin/env bash\n"):
            failures.extend(embedded_python(relative, content))
    return sources, failures


def embedded_python(relative: str, content: bytes) -> list[SourceViolation]:
    text = content.decode("utf-8", errors="replace")
    markers = ("python3 -", "python -", "<<'PY'", '<<"PY"', "<<PY")
    return [
        SourceViolation(relative, line_number, "embedded_python_authority_rejected")
        for line_number, line in enumerate(text.splitlines(), start=1)
        if any(marker in line for marker in markers)
    ]
