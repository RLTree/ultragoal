"""Deterministic GeneratedSurfaceAuthority-v3 rendering."""

from __future__ import annotations

import json
from pathlib import Path

from .contracts import CONTRACT_ID, AuthorityContractError, Definition, parse_shard
from repository_projection.atomic_file import regular_bytes


GENERATOR = "scripts/project-generated-authority"
COMMAND = "scripts/project-generated-authority write"


def definitions(root: Path) -> tuple[list[str], list[Definition]]:
    directory = root / "migration" / "generated-surface-authority"
    paths = sorted(directory.glob("*.json"))
    if not paths:
        raise AuthorityContractError("generated authority shards missing")
    relative_paths = [str(path.relative_to(root)) for path in paths]
    rows: list[Definition] = []
    for path in paths:
        rows.extend(parse_shard(regular_bytes(path), path))
    outputs = [row.output for row in rows]
    if len(outputs) != len(set(outputs)):
        raise AuthorityContractError("duplicate generated surface output")
    return relative_paths, sorted(rows, key=lambda row: row.output)


def expected_bytes(root: Path) -> bytes:
    sources, rows = definitions(root)
    payload = {
        "schema_version": "GeneratedSurfaceAuthority-v3",
        "contract_id": CONTRACT_ID,
        "registry_projection": {
            "generator": GENERATOR,
            "canonical_sources": sources,
            "regeneration_command": COMMAND,
        },
        "surfaces": [row.aggregate(root) for row in rows],
    }
    return (json.dumps(payload, indent=2, ensure_ascii=False) + "\n").encode()
