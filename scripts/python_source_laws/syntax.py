"""Typed-boundary and effect-confinement checks for Python sources."""

from __future__ import annotations

import ast
from pathlib import Path

from .violation import SourceViolation


BOUNDARY_FILES = {"contracts.py", "boundary_values.py"}
WRITE_AUTHORITY = "scripts/repository_projection/atomic_file.py"
WRITE_METHODS = {"write_text", "write_bytes", "replace", "unlink", "chmod", "mkstemp"}


def failures(root: Path, path: Path) -> list[SourceViolation]:
    relative = str(path.relative_to(root))
    content = path.read_text(encoding="utf-8")
    rows = content.splitlines()
    found: list[SourceViolation] = []
    if len(rows) > 250:
        found.append(SourceViolation(relative, 0, "hand_authored_file_over_250_lines"))
    try:
        tree = ast.parse(content, filename=relative)
    except SyntaxError as error:
        return found + [SourceViolation(relative, error.lineno or 0, "syntax_invalid")]
    for node in ast.walk(tree):
        if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)):
            found.extend(function_annotations(relative, node))
        if isinstance(node, ast.Call):
            found.extend(call_boundary(relative, path.name, node))
    return found


def function_annotations(
    relative: str, node: ast.FunctionDef | ast.AsyncFunctionDef
) -> list[SourceViolation]:
    arguments = [*node.args.posonlyargs, *node.args.args, *node.args.kwonlyargs]
    if node.args.vararg is not None:
        arguments.append(node.args.vararg)
    if node.args.kwarg is not None:
        arguments.append(node.args.kwarg)
    missing = [
        argument.arg
        for argument in arguments
        if argument.annotation is None and argument.arg not in {"self", "cls"}
    ]
    failures = [
        SourceViolation(relative, node.lineno, f"function_argument_type_missing:{name}")
        for name in missing
    ]
    if node.returns is None:
        failures.append(SourceViolation(relative, node.lineno, "function_return_type_missing"))
    return failures


def call_boundary(relative: str, filename: str, node: ast.Call) -> list[SourceViolation]:
    name = call_name(node.func)
    failures: list[SourceViolation] = []
    if name in {"json.load", "json.loads"} and filename not in BOUNDARY_FILES:
        failures.append(SourceViolation(relative, node.lineno, "json_parse_outside_typed_boundary"))
    if name.rsplit(".", 1)[-1] in WRITE_METHODS and relative != WRITE_AUTHORITY:
        failures.append(SourceViolation(relative, node.lineno, "filesystem_write_outside_atomic_adapter"))
    if name == "open" and write_open_mode(node):
        failures.append(SourceViolation(relative, node.lineno, "filesystem_write_outside_atomic_adapter"))
    return failures


def call_name(node: ast.expr) -> str:
    if isinstance(node, ast.Name):
        return node.id
    if isinstance(node, ast.Attribute):
        prefix = call_name(node.value)
        return f"{prefix}.{node.attr}" if prefix else node.attr
    return ""


def write_open_mode(node: ast.Call) -> bool:
    modes = [keyword.value for keyword in node.keywords if keyword.arg == "mode"]
    if len(node.args) > 1:
        modes.append(node.args[1])
    return any(
        isinstance(mode, ast.Constant)
        and isinstance(mode.value, str)
        and any(flag in mode.value for flag in "wax+")
        for mode in modes
    )
