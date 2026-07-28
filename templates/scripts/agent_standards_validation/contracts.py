"""Closed file-format contracts for agent-standards enforcement."""

from __future__ import annotations

import csv
from dataclasses import dataclass
from enum import Enum
import io
import json
import os
from pathlib import Path
import stat


class EnforcementStatus(str, Enum):
    MECHANIZED = "mechanized"
    BACKLOGGED = "backlogged"
    BLOCKED = "blocked"
    INFORMATIONAL = "informational"


class AuditStatus(str, Enum):
    PASS = "pass"
    FAIL = "fail"
    PENDING = "pending"
    BLOCKED = "blocked"


@dataclass(frozen=True)
class EnforcementRow:
    identity: str
    enforcement_status: EnforcementStatus
    gate_or_fixture_path: str
    claim_ids_affected: str
    blocker_or_repair_action: str
    required_execplan_refs: tuple[str, ...]


@dataclass(frozen=True)
class AuditRow:
    standard_id: str
    audit_status: AuditStatus
    evidence_path: str
    evidence_digest: str


ENFORCEMENT_FIELDS = frozenset(
    {
        "id",
        "source_law",
        "required_behavior",
        "enforcement_status",
        "gate_or_fixture_path",
        "owner_lane",
        "claim_ids_affected",
        "current_status",
        "blocker_or_repair_action",
        "required_execplan_refs",
    }
)
TSV_FIELDS = tuple(ENFORCEMENT_FIELDS)
AUDIT_FIELDS = (
    "standard_id",
    "audit_status",
    "evidence_path",
    "evidence_digest",
    "audited_at",
    "claim_ceiling_impact",
)
MAX_REGULAR_BYTES = 4 * 1024 * 1024


def file_identity(metadata: os.stat_result) -> tuple[int, int, int, int, int, int]:
    return (
        metadata.st_dev, metadata.st_ino, metadata.st_nlink,
        metadata.st_size, metadata.st_mtime_ns, metadata.st_ctime_ns,
    )


def open_absolute_directory(path: Path) -> int:
    if not path.is_absolute():
        raise ValueError(f"directory path is not absolute: {path}")
    if not hasattr(os, "O_DIRECTORY") or not hasattr(os, "O_NOFOLLOW"):
        raise ValueError("descriptor-relative no-follow directory reads are unsupported")
    flags = os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW
    current = os.open(os.path.sep, flags)
    try:
        for part in path.parts[1:]:
            opened = os.open(part, flags, dir_fd=current)
            os.close(current)
            current = opened
            if not stat.S_ISDIR(os.fstat(current).st_mode):
                raise ValueError(f"required directory missing: {path}")
        return current
    except BaseException:
        os.close(current)
        raise


def regular_bytes(path: Path) -> bytes:
    absolute = path.absolute()
    parent = open_absolute_directory(absolute.parent)
    descriptor = -1
    try:
        if not hasattr(os, "O_NOFOLLOW") or not hasattr(os, "O_NONBLOCK"):
            raise ValueError("nonblocking no-follow regular-file reads are unsupported")
        flags = os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK
        descriptor = os.open(absolute.name, flags, dir_fd=parent)
        before = os.fstat(descriptor)
        if (
            not stat.S_ISREG(before.st_mode)
            or before.st_nlink != 1
            or before.st_size > MAX_REGULAR_BYTES
        ):
            raise ValueError(f"required bounded single-link regular file missing: {path}")
        chunks: list[bytes] = []
        remaining = MAX_REGULAR_BYTES + 1
        while remaining:
            chunk = os.read(descriptor, min(64 * 1024, remaining))
            if not chunk:
                break
            chunks.append(chunk)
            remaining -= len(chunk)
        content = b"".join(chunks)
        after = os.fstat(descriptor)
        if len(content) != before.st_size or file_identity(after) != file_identity(before):
            raise ValueError(f"required regular file changed while reading: {path}")
        return content
    finally:
        if descriptor >= 0:
            os.close(descriptor)
        os.close(parent)


def load_enforcement_rows(path: Path) -> list[EnforcementRow]:
    payload = strict_json(regular_bytes(path), path)
    if not isinstance(payload, dict) or set(payload) != {"schema", "rows"}:
        raise ValueError("invalid enforcement envelope")
    if payload["schema"] != "harness-ultragoal.agent-standards-enforcement.v1":
        raise ValueError("unsupported enforcement schema")
    raw_rows = payload["rows"]
    if not isinstance(raw_rows, list) or not raw_rows:
        raise ValueError("standards enforcement rows missing")
    rows = [enforcement_row(value) for value in raw_rows]
    require_unique([row.identity for row in rows], "enforcement")
    return rows


def enforcement_row(value: object) -> EnforcementRow:
    if not isinstance(value, dict) or set(value) != ENFORCEMENT_FIELDS:
        raise ValueError("invalid enforcement row shape")
    references = value["required_execplan_refs"]
    if not isinstance(references, list) or not references:
        raise ValueError("required_execplan_refs missing")
    try:
        status = EnforcementStatus(text(value, "enforcement_status"))
    except ValueError as error:
        raise ValueError("invalid enforcement status") from error
    return EnforcementRow(
        text(value, "id"),
        status,
        text(value, "gate_or_fixture_path"),
        text(value, "claim_ids_affected"),
        text(value, "blocker_or_repair_action"),
        tuple(list_text(references)),
    )


def load_enforcement_tsv(path: Path) -> dict[str, dict[str, str]]:
    rows = parse_tsv(path, TSV_FIELDS)
    require_unique([row.get("id", "") for row in rows], "enforcement TSV")
    return {row["id"]: row for row in rows}


def load_audit_rows(path: Path) -> list[AuditRow]:
    rows = parse_tsv(path, AUDIT_FIELDS)
    typed: list[AuditRow] = []
    for row in rows:
        try:
            status = AuditStatus(row["audit_status"])
        except ValueError as error:
            raise ValueError("invalid audit status") from error
        typed.append(
            AuditRow(
                nonempty(row["standard_id"]),
                status,
                nonempty(row["evidence_path"]),
                nonempty(row["evidence_digest"]),
            )
        )
    require_unique([row.standard_id for row in typed], "audit")
    return typed


def parse_tsv(path: Path, fields: tuple[str, ...]) -> list[dict[str, str]]:
    content = regular_bytes(path).decode("utf-8")
    reader = csv.DictReader(io.StringIO(content), delimiter="\t")
    if reader.fieldnames is None or set(reader.fieldnames) != set(fields):
        raise ValueError(f"invalid TSV columns: {path}")
    rows = list(reader)
    if not rows or any(None in row or any(value is None for value in row.values()) for row in rows):
        raise ValueError(f"invalid TSV rows: {path}")
    return rows


def strict_json(content: bytes, path: Path) -> object:
    def unique_object(pairs: list[tuple[str, object]]) -> dict[str, object]:
        result: dict[str, object] = {}
        for key, value in pairs:
            if key in result:
                raise ValueError(f"duplicate JSON key in {path}: {key}")
            result[key] = value
        return result

    try:
        return json.loads(content, object_pairs_hook=unique_object)
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        raise ValueError(f"invalid JSON in {path}: {error}") from error


def text(row: dict[str, object], field: str) -> str:
    value = row.get(field)
    if not isinstance(value, str):
        raise ValueError(f"invalid {field}")
    return nonempty(value)


def list_text(values: list[object]) -> list[str]:
    return [nonempty(value) if isinstance(value, str) else invalid_list() for value in values]


def invalid_list() -> str:
    raise ValueError("list contains a non-string value")


def nonempty(value: str) -> str:
    if not value or value.strip() != value:
        raise ValueError("empty or untrimmed value")
    return value


def require_unique(values: list[str], label: str) -> None:
    if not values or len(values) != len(set(values)) or any(not value for value in values):
        raise ValueError(f"missing or duplicate {label} identity")
