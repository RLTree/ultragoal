"""Closed contracts parsed at the standards-policy boundary."""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum
import json
from pathlib import Path


class ProjectionContractError(ValueError):
    pass


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
class PolicyRow:
    identity: str
    source_law: str
    required_behavior: str
    enforcement_status: EnforcementStatus
    gate_or_fixture_path: str
    owner_lane: str
    claim_ids_affected: str
    current_status: str
    blocker_or_repair_action: str
    required_execplan_refs: tuple[str, ...]

    def fields(self) -> dict[str, object]:
        return {
            "id": self.identity,
            "source_law": self.source_law,
            "required_behavior": self.required_behavior,
            "enforcement_status": self.enforcement_status.value,
            "gate_or_fixture_path": self.gate_or_fixture_path,
            "owner_lane": self.owner_lane,
            "claim_ids_affected": self.claim_ids_affected,
            "current_status": self.current_status,
            "blocker_or_repair_action": self.blocker_or_repair_action,
            "required_execplan_refs": list(self.required_execplan_refs),
        }


@dataclass(frozen=True)
class AuditRow:
    standard_id: str
    audit_status: AuditStatus
    evidence_path: str
    evidence_digest: str
    audited_at: str
    claim_ceiling_impact: str

    def fields(self) -> dict[str, str]:
        return {
            "standard_id": self.standard_id,
            "audit_status": self.audit_status.value,
            "evidence_path": self.evidence_path,
            "evidence_digest": self.evidence_digest,
            "audited_at": self.audited_at,
            "claim_ceiling_impact": self.claim_ceiling_impact,
        }


def strict_json(content: bytes, path: Path) -> object:
    def unique_object(pairs: list[tuple[str, object]]) -> dict[str, object]:
        result: dict[str, object] = {}
        for key, value in pairs:
            if key in result:
                raise ProjectionContractError(f"duplicate JSON key in {path}: {key}")
            result[key] = value
        return result

    try:
        return json.loads(content, object_pairs_hook=unique_object)
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        raise ProjectionContractError(f"invalid JSON in {path}: {error}") from error


def shard_rows(content: bytes, path: Path, schema: str) -> tuple[object, ...]:
    payload = strict_json(content, path)
    if not isinstance(payload, dict) or set(payload) != {"schema", "rows"}:
        raise ProjectionContractError(f"invalid shard envelope: {path}")
    if payload["schema"] != schema or not isinstance(payload["rows"], list):
        raise ProjectionContractError(f"invalid shard schema or rows: {path}")
    return tuple(payload["rows"])


def text(row: dict[str, object], field: str, path: Path) -> str:
    value = row.get(field)
    if not isinstance(value, str) or not value or value.strip() != value:
        raise ProjectionContractError(f"invalid {field} in {path}")
    return value


def policy_row(value: object, path: Path) -> PolicyRow:
    fields = set(PolicyRow("", "", "", EnforcementStatus.BLOCKED, "", "", "", "", "", ()).fields())
    if not isinstance(value, dict) or set(value) != fields:
        raise ProjectionContractError(f"invalid policy row shape: {path}")
    refs = value["required_execplan_refs"]
    if not isinstance(refs, list) or not refs:
        raise ProjectionContractError(f"invalid required_execplan_refs in {path}")
    typed_refs = tuple(text({"value": item}, "value", path) for item in refs)
    try:
        status = EnforcementStatus(text(value, "enforcement_status", path))
    except ValueError as error:
        raise ProjectionContractError(f"invalid enforcement_status in {path}") from error
    return PolicyRow(
        text(value, "id", path),
        text(value, "source_law", path),
        text(value, "required_behavior", path),
        status,
        text(value, "gate_or_fixture_path", path),
        text(value, "owner_lane", path),
        text(value, "claim_ids_affected", path),
        text(value, "current_status", path),
        text(value, "blocker_or_repair_action", path),
        typed_refs,
    )


def audit_row(value: object, path: Path) -> AuditRow:
    fields = set(AuditRow("", AuditStatus.BLOCKED, "", "", "", "").fields())
    if not isinstance(value, dict) or set(value) != fields:
        raise ProjectionContractError(f"invalid audit row shape: {path}")
    try:
        status = AuditStatus(text(value, "audit_status", path))
    except ValueError as error:
        raise ProjectionContractError(f"invalid audit_status in {path}") from error
    return AuditRow(
        text(value, "standard_id", path),
        status,
        text(value, "evidence_path", path),
        text(value, "evidence_digest", path),
        text(value, "audited_at", path),
        text(value, "claim_ceiling_impact", path),
    )
