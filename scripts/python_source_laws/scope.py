"""Exact active script-source discovery."""

from __future__ import annotations

import re
import shlex
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


def stdin_program(line: str) -> bool:
    try:
        lexer = shlex.shlex(line, posix=True, punctuation_chars=";&|()<>")
        lexer.whitespace_split = True
        tokens = list(lexer)
    except ValueError:
        return False
    expecting_command = True
    for index, token in enumerate(tokens):
        if token in {";", "&", "&&", "|", "||", "(", ")", "{"}:
            expecting_command = True
        elif expecting_command:
            if re.fullmatch(r"[A-Za-z_][A-Za-z_0-9]*=.*", token):
                continue
            if token in {"if", "then", "elif", "else", "do", "!"}:
                continue
            if Path(token).name in {"env", "command", "exec", "time", "nohup"}:
                continue
            if re.fullmatch(r"python(?:[0-9]+(?:\.[0-9]+)?)?", Path(token).name):
                if index + 1 < len(tokens) and tokens[index + 1] == "-":
                    return True
            expecting_command = False
    return False


def embedded_python(relative: str, content: bytes) -> list[SourceViolation]:
    text = content.decode("utf-8", errors="replace")
    heredoc_markers = ("<<'PY'", '<<"PY"', "<<PY")
    return [
        SourceViolation(relative, line_number, "embedded_python_authority_rejected")
        for line_number, line in enumerate(text.splitlines(), start=1)
        if any(marker in line for marker in heredoc_markers) or stdin_program(line)
    ]
