"""Candidate-current agent-standards validation command."""

from __future__ import annotations

import hashlib
from pathlib import Path
import sys

from .contracts import (
    AuditRow,
    AuditStatus,
    EnforcementRow,
    EnforcementStatus,
    load_audit_rows,
    load_enforcement_rows,
    load_enforcement_tsv,
    regular_bytes,
)


REQUIRED_IDENTITIES = frozenset(
    {
        "authority-source-binding",
        "source-installed-cache-alignment",
        "reviewer-to-gate-conversion",
        "documentation-freshness",
        "runtime-tool-identity",
        "product-live-surface-receipts",
        "transcript-quality-reuse-gates",
        "privacy-raw-artifact-boundary",
        "coverage-proof-accountability",
        "coverage-scope-authority",
        "plugin-product-cohesion-authority",
        "std-product-fitness-001",
        "std-plans-001",
        "std-automation-001",
    }
)


def main() -> int:
    if len(sys.argv) > 2:
        print("usage: scripts/check-agent-standards [repo-root]", file=sys.stderr)
        return 2
    root = Path(sys.argv[1] if len(sys.argv) == 2 else ".").resolve()
    try:
        standards_root = select_standards_root(root)
        rows = load_enforcement_rows(standards_root / "enforcement.json")
        tsv = load_enforcement_tsv(standards_root / "enforcement.tsv")
        audits = load_audit_rows(standards_root / "enforcement-audit.tsv")
        validate_policy(rows, tsv)
        validate_audits(root, rows, audits)
    except (OSError, ValueError) as error:
        print(f"agent standards enforcement failed: {error}", file=sys.stderr)
        return 1
    print(f"agent standards enforcement ok: {len(rows)} rows")
    return 0


def select_standards_root(root: Path) -> Path:
    for candidate in (root / "agent-standards", root / "templates" / "agent-standards"):
        if candidate.joinpath("enforcement.json").is_file():
            return candidate
    raise ValueError("agent standards enforcement files missing")


def validate_policy(
    rows: list[EnforcementRow],
    tsv: dict[str, dict[str, str]],
) -> None:
    identities = {row.identity for row in rows}
    missing = sorted(REQUIRED_IDENTITIES - identities)
    if missing:
        raise ValueError(f"required rows missing: {', '.join(missing)}")
    if identities != set(tsv):
        raise ValueError("enforcement JSON and TSV identity sets differ")
    for row in rows:
        if row.enforcement_status is EnforcementStatus.MECHANIZED and not row.gate_or_fixture_path:
            raise ValueError(f"{row.identity}: mechanized row missing gate_or_fixture_path")
        if row.enforcement_status in {EnforcementStatus.BACKLOGGED, EnforcementStatus.BLOCKED}:
            if not row.blocker_or_repair_action:
                raise ValueError(f"{row.identity}: debt row missing repair action")
        if row.enforcement_status is EnforcementStatus.INFORMATIONAL:
            if row.claim_ids_affected != "none":
                raise ValueError(f"{row.identity}: informational row overclaims affected claims")
        projected = tsv[row.identity]
        if projected.get("enforcement_status") != row.enforcement_status.value:
            raise ValueError(f"{row.identity}: enforcement status projection drift")
        if projected.get("required_execplan_refs") != ";".join(row.required_execplan_refs):
            raise ValueError(f"{row.identity}: execplan reference projection drift")


def validate_audits(
    root: Path,
    rows: list[EnforcementRow],
    audits: list[AuditRow],
) -> None:
    audit_by_identity = {row.standard_id: row for row in audits}
    for row in rows:
        if row.enforcement_status is not EnforcementStatus.INFORMATIONAL:
            if row.identity not in audit_by_identity:
                raise ValueError(f"{row.identity}: enforcement audit missing")
    for audit in audits:
        if audit.audit_status is not AuditStatus.PASS:
            continue
        if not valid_digest(audit.evidence_digest):
            raise ValueError(f"{audit.standard_id}: invalid pass evidence digest")
        actual = "sha256:" + hashlib.sha256(evidence_bytes(root, audit.evidence_path)).hexdigest()
        if actual != audit.evidence_digest:
            raise ValueError(f"{audit.standard_id}: pass evidence digest mismatch")


def evidence_bytes(root: Path, relative: str) -> bytes:
    candidate = confined(root, relative)
    try:
        return regular_bytes(candidate)
    except FileNotFoundError:
        if relative.startswith("scripts/"):
            projected = confined(root, f"templates/{relative}")
            try:
                return regular_bytes(projected)
            except FileNotFoundError:
                pass
        raise ValueError(f"evidence path missing or not regular: {relative}") from None


def confined(root: Path, relative: str) -> Path:
    path = Path(relative)
    if path.is_absolute() or any(part in {"", ".", ".."} for part in path.parts):
        raise ValueError(f"unconfined evidence path: {relative}")
    return root.joinpath(path)


def valid_digest(value: str) -> bool:
    if not value.startswith("sha256:") or len(value) != 71:
        return False
    return value[7:] != "0" * 64 and all(character in "0123456789abcdef" for character in value[7:])
