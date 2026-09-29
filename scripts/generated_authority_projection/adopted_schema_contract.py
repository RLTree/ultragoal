"""Adoption-bound schema contract authority."""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path

from .boundary_values import exact, required_text, safe_path


@dataclass(frozen=True)
class AdoptedSchemaContract:
    output: str
    sha256: str
    schema: str
    schema_sha256: str
    source_contract: str
    source_contract_sha256: str
    amendment_log: str
    amendment_id: str
    amendment_hash: str
    claim_ceiling: str

    def aggregate(self, _: Path) -> dict[str, object]:
        return {
            "disposition": "adopted_schema_contract",
            "output": self.output,
            "sha256": self.sha256,
            "schema": self.schema,
            "schema_sha256": self.schema_sha256,
            "source_contract": self.source_contract,
            "source_contract_sha256": self.source_contract_sha256,
            "amendment_log": self.amendment_log,
            "amendment_id": self.amendment_id,
            "amendment_hash": self.amendment_hash,
            "claim_ceiling": self.claim_ceiling,
        }


def parse(value: object, path: Path) -> AdoptedSchemaContract:
    fields = {
        "disposition", "output", "sha256", "schema", "schema_sha256",
        "source_contract", "source_contract_sha256", "amendment_log", "amendment_id",
        "amendment_hash", "claim_ceiling",
    }
    row = exact(value, fields, path)
    return AdoptedSchemaContract(
        safe_path(row["output"], "output", path),
        digest(row["sha256"], "sha256", path),
        safe_path(row["schema"], "schema", path),
        digest(row["schema_sha256"], "schema_sha256", path),
        safe_path(row["source_contract"], "source_contract", path),
        digest(row["source_contract_sha256"], "source_contract_sha256", path),
        safe_path(row["amendment_log"], "amendment_log", path),
        amendment_id(row["amendment_id"], path),
        digest(row["amendment_hash"], "amendment_hash", path),
        claim_ceiling(row["claim_ceiling"], path),
    )


def digest(value: object, field: str, path: Path) -> str:
    text = required_text(value, field, path)
    if len(text) != 64 or any(char not in "0123456789abcdef" for char in text):
        raise ValueError(f"invalid {field} in {path}")
    return text


def amendment_id(value: object, path: Path) -> str:
    text = required_text(value, "amendment_id", path)
    if not text.startswith("AMEND-") or not text[6:].isdigit():
        raise ValueError(f"invalid amendment_id in {path}")
    return text


def claim_ceiling(value: object, path: Path) -> str:
    text = required_text(value, "claim_ceiling", path)
    if text != "contract_authority_only":
        raise ValueError(f"invalid claim_ceiling in {path}")
    return text
