"""Closed generated-authority definition contracts."""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path

from .boundary_values import (
    AuthorityContractError,
    exact,
    required_text,
    required_token,
    safe_path,
    strict_json,
    string_tuple,
)


CONTRACT_ID = "harness-ultragoal-successor-contract-v2"
SHARD_SCHEMA = "GeneratedSurfaceAuthorityShard-v1"


@dataclass(frozen=True)
class CanonicalProjection:
    output: str
    generator: str
    recipe: str
    inputs: tuple[str, ...]

    def aggregate(self, _: Path) -> dict[str, object]:
        return {
            "disposition": "canonical_projection",
            "output": self.output,
            "generator": self.generator,
            "recipe": self.recipe,
            "inputs": list(self.inputs),
        }


@dataclass(frozen=True)
class RetainedContext:
    output: str
    sha256: str
    reason: str
    replacement_targets: tuple[str, ...]
    preserve: bool
    physical_deletion_authorized: bool

    def aggregate(self, _: Path) -> dict[str, object]:
        return {
            "disposition": "retained_context",
            "output": self.output,
            "sha256": self.sha256,
            "reason": self.reason,
            "replacement_targets": list(self.replacement_targets),
            "preserve": self.preserve,
            "physical_deletion_authorized": self.physical_deletion_authorized,
        }


@dataclass(frozen=True)
class SourceProjection:
    output: str
    generator: str
    canonical_sources: tuple[str, ...]
    regeneration_command: str

    def aggregate(self, root: Path) -> dict[str, object]:
        import hashlib

        output = root / self.output
        if output.is_symlink() or not output.is_file():
            raise AuthorityContractError(f"source projection output missing: {self.output}")
        return {
            "disposition": "source_projection",
            "output": self.output,
            "generator": self.generator,
            "canonical_sources": list(self.canonical_sources),
            "regeneration_command": self.regeneration_command,
            "output_sha256": hashlib.sha256(output.read_bytes()).hexdigest(),
        }


@dataclass(frozen=True)
class ToolProjection:
    output: str
    tool: str
    tool_version: str
    canonical_sources: tuple[str, ...]
    regeneration_command: str

    def aggregate(self, root: Path) -> dict[str, object]:
        import hashlib

        output = root / self.output
        if output.is_symlink() or not output.is_file():
            raise AuthorityContractError(f"tool projection output missing: {self.output}")
        return {
            "disposition": "tool_projection",
            "output": self.output,
            "tool": self.tool,
            "tool_version": self.tool_version,
            "canonical_sources": list(self.canonical_sources),
            "regeneration_command": self.regeneration_command,
            "output_sha256": hashlib.sha256(output.read_bytes()).hexdigest(),
        }


Definition = CanonicalProjection | RetainedContext | SourceProjection | ToolProjection


def parse_definition(value: object, path: Path) -> Definition:
    if not isinstance(value, dict) or not isinstance(value.get("disposition"), str):
        raise AuthorityContractError(f"authority disposition missing in {path}")
    disposition = value["disposition"]
    if disposition == "canonical_projection":
        row = exact(value, {"disposition", "output", "generator", "recipe", "inputs"}, path)
        return CanonicalProjection(
            safe_path(row["output"], "output", path),
            safe_path(row["generator"], "generator", path),
            required_text(row["recipe"], "recipe", path),
            string_tuple(row["inputs"], "inputs", path, paths=True),
        )
    if disposition == "source_projection":
        row = exact(
            value,
            {"disposition", "output", "generator", "canonical_sources", "regeneration_command"},
            path,
        )
        generator = safe_path(row["generator"], "generator", path)
        command = required_text(row["regeneration_command"], "regeneration_command", path)
        if command.split()[0] != generator:
            raise AuthorityContractError(f"generator command mismatch in {path}")
        return SourceProjection(
            safe_path(row["output"], "output", path),
            generator,
            string_tuple(row["canonical_sources"], "canonical_sources", path, paths=True),
            command,
        )
    if disposition == "tool_projection":
        row = exact(
            value,
            {
                "disposition", "output", "tool", "tool_version",
                "canonical_sources", "regeneration_command",
            },
            path,
        )
        tool = required_token(row["tool"], "tool", path)
        command = required_text(row["regeneration_command"], "regeneration_command", path)
        if command.split()[0] != tool:
            raise AuthorityContractError(f"tool command mismatch in {path}")
        return ToolProjection(
            safe_path(row["output"], "output", path),
            tool,
            required_token(row["tool_version"], "tool_version", path),
            string_tuple(row["canonical_sources"], "canonical_sources", path, paths=True),
            command,
        )
    return retained_context(value, path)


def retained_context(value: object, path: Path) -> RetainedContext:
    fields = {
        "disposition", "output", "sha256", "reason", "replacement_targets",
        "preserve", "physical_deletion_authorized",
    }
    row = exact(value, fields, path)
    digest = required_text(row["sha256"], "sha256", path)
    if len(digest) != 64 or any(char not in "0123456789abcdef" for char in digest):
        raise AuthorityContractError(f"invalid sha256 in {path}")
    if row["preserve"] is not True or row["physical_deletion_authorized"] is not False:
        raise AuthorityContractError(f"invalid retained context controls in {path}")
    return RetainedContext(
        safe_path(row["output"], "output", path), digest,
        required_text(row["reason"], "reason", path),
        string_tuple(row["replacement_targets"], "replacement_targets", path),
        True, False,
    )


def parse_shard(content: bytes, path: Path) -> tuple[Definition, ...]:
    payload = strict_json(content, path)
    if not isinstance(payload, dict) or set(payload) != {"schema_version", "contract_id", "surfaces"}:
        raise AuthorityContractError(f"invalid shard envelope: {path}")
    if payload["schema_version"] != SHARD_SCHEMA or payload["contract_id"] != CONTRACT_ID:
        raise AuthorityContractError(f"invalid shard identity: {path}")
    surfaces = payload["surfaces"]
    if not isinstance(surfaces, list) or not surfaces:
        raise AuthorityContractError(f"empty shard: {path}")
    return tuple(parse_definition(row, path) for row in surfaces)
