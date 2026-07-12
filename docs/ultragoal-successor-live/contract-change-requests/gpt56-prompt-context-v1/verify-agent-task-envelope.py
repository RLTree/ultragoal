#!/usr/bin/env python3
"""Deterministic proposal-only AgentTaskEnvelope-v1 red-fixture verifier."""

from __future__ import annotations

import copy
import hashlib
import json
import os
import stat
import subprocess
import sys
import tomllib
from pathlib import Path
from typing import Any

import jsonschema


HERE = Path(__file__).resolve().parent
SCHEMA_PATH = HERE / "AGENT-TASK-ENVELOPE.schema.json"
FIXTURE_PATH = HERE / "AGENT-TASK-ENVELOPE-RED-FIXTURES.json"
ID_KEYS = {
    "nodes": "node_id",
    "requirements": "requirement_id",
    "surfaces": "surface_id",
    "tools": "tool_id",
    "claims": "claim_id",
    "migration_routes": "route_id",
    "decisions": "decision_id",
}


class EnvelopeError(ValueError):
    pass


def canonical(value: Any) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode()


def digest_bytes(value: bytes) -> str:
    return "sha256:" + hashlib.sha256(value).hexdigest()


def regular_file(root: Path, relative: str) -> Path:
    path = root
    for part in Path(relative).parts:
        path = path / part
        metadata = os.lstat(path)
        if stat.S_ISLNK(metadata.st_mode):
            raise EnvelopeError(f"symlink path rejected: {relative}")
    metadata = os.lstat(path)
    if not stat.S_ISREG(metadata.st_mode):
        raise EnvelopeError(f"non-regular file rejected: {relative}")
    return path


def file_digest(root: Path, relative: str) -> str:
    return digest_bytes(regular_file(root, relative).read_bytes())


def verify_bound_file(root: Path, relative: str, expected: str) -> Path:
    path = regular_file(root, relative)
    actual = digest_bytes(path.read_bytes())
    if actual != expected:
        raise EnvelopeError(f"digest mismatch: {relative}")
    return path


def git(root: Path, *args: str) -> str:
    result = subprocess.run(
        ["git", *args],
        cwd=root,
        check=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        env={**os.environ, "GIT_OPTIONAL_LOCKS": "0"},
    )
    return result.stdout.strip()


def collect_rows(value: Any, key: str, rows: list[dict[str, Any]]) -> None:
    if isinstance(value, dict):
        if isinstance(value.get(key), str):
            rows.append(value)
        for child in value.values():
            collect_rows(child, key, rows)
    elif isinstance(value, list):
        for child in value:
            collect_rows(child, key, rows)


def artifact_set(root: Path, rows: list[dict[str, Any]]) -> tuple[str, int, int]:
    ordered = sorted(rows, key=lambda row: row["path"].encode())
    aggregate = bytearray()
    total_bytes = 0
    total_lines = 0
    for row in ordered:
        path = regular_file(root, row["path"])
        data = path.read_bytes()
        actual = digest_bytes(data)
        lines = data.count(b"\n")
        if actual != row["sha256"] or len(data) != row["bytes"] or lines != row["lines"]:
            raise EnvelopeError(f"artifact row mismatch: {row['path']}")
        aggregate.extend(row["path"].encode())
        aggregate.extend(b"\t")
        aggregate.extend(actual.removeprefix("sha256:").encode())
        aggregate.extend(b"\n")
        total_bytes += len(data)
        total_lines += lines
    return digest_bytes(bytes(aggregate)), total_bytes, total_lines


def candidate_core(candidate: dict[str, Any]) -> dict[str, Any]:
    return {
        "repository": candidate["repository"],
        "branch": candidate["branch"],
        "head_commit": candidate["head_commit"],
        "head_tree": candidate["head_tree"],
        "dirty": candidate["dirty"],
        "freeze": candidate["freeze"],
    }


def scope_core(scope: dict[str, Any]) -> dict[str, Any]:
    return {key: value for key, value in scope.items() if key != "path_resolution"}


def verify_scope_path(root: Path, relative: str, may_be_absent: bool) -> None:
    current = root
    parts = Path(relative).parts
    for index, part in enumerate(parts):
        current = current / part
        try:
            metadata = os.lstat(current)
        except FileNotFoundError:
            if may_be_absent:
                return
            raise EnvelopeError(f"required scope path missing: {relative}")
        if stat.S_ISLNK(metadata.st_mode):
            raise EnvelopeError(f"scope symlink rejected: {relative}")
        if index < len(parts) - 1 and not stat.S_ISDIR(metadata.st_mode):
            raise EnvelopeError(f"scope ancestor is not a directory: {relative}")
        if index == len(parts) - 1 and not (stat.S_ISREG(metadata.st_mode) or stat.S_ISDIR(metadata.st_mode)):
            raise EnvelopeError(f"scope special file rejected: {relative}")


def semantic_validate(root: Path, envelope: dict[str, Any]) -> None:
    role = envelope["role_binding"]
    role_path = verify_bound_file(root, role["source_ref"], role["source_sha256"])
    if role["source_kind"] == "project_agent_manifest":
        parsed = tomllib.loads(role_path.read_text())
        if parsed.get("name") != role["role_id"]:
            raise EnvelopeError("role ID does not resolve to the bound manifest")
        if parsed.get("sandbox_mode") != role["sandbox_mode"]:
            raise EnvelopeError("role sandbox does not resolve to the bound manifest")

    candidate = envelope["candidate_identity"]
    canonical_root = root.resolve()
    if candidate["repository"]["root_uri"] != f"repo://{canonical_root.name}":
        raise EnvelopeError("repository URI mismatch")
    if candidate["repository"]["canonical_root_sha256"] != digest_bytes(str(canonical_root).encode()):
        raise EnvelopeError("repository root digest mismatch")
    if candidate["branch"] != git(root, "branch", "--show-current"):
        raise EnvelopeError("branch mismatch")
    if candidate["head_commit"] != git(root, "rev-parse", "HEAD"):
        raise EnvelopeError("HEAD mismatch")
    if candidate["head_tree"] != git(root, "rev-parse", "HEAD^{tree}"):
        raise EnvelopeError("tree mismatch")
    freeze = candidate["freeze"]
    if freeze["kind"] != "artifact_set":
        raise EnvelopeError("proposal red-fixture verifier requires artifact_set freeze")
    set_digest, total_bytes, total_lines = artifact_set(root, freeze["artifacts"])
    if set_digest != freeze["artifact_set_sha256"]:
        raise EnvelopeError("artifact-set digest mismatch")
    if len(freeze["artifacts"]) != freeze["artifact_count"]:
        raise EnvelopeError("artifact-set count mismatch")
    if total_bytes != freeze["bytes"] or total_lines != freeze["lines"]:
        raise EnvelopeError("artifact-set aggregate mismatch")
    expected_candidate_id = digest_bytes(canonical(candidate_core(candidate)))
    if candidate["candidate_id"] != expected_candidate_id:
        raise EnvelopeError("candidate ID mismatch")

    authority = envelope["authority_binding"]
    loaded_sources: dict[str, Any] = {}
    for source in authority["sources"]:
        source_path = verify_bound_file(root, source["path"], source["sha256"])
        loaded_sources[source["path"]] = json.loads(source_path.read_bytes())
    expected_rows: dict[str, tuple[str, str]] = {}
    for category, key in ID_KEYS.items():
        for stable_id in authority["ids"][category]:
            matches: list[tuple[str, dict[str, Any]]] = []
            for source_path, document in loaded_sources.items():
                rows: list[dict[str, Any]] = []
                collect_rows(document, key, rows)
                matches.extend((source_path, row) for row in rows if row.get(key) == stable_id)
            if len(matches) != 1:
                raise EnvelopeError(f"stable ID did not resolve exactly once: {stable_id}")
            source_path, row = matches[0]
            expected_rows[stable_id] = (source_path, digest_bytes(canonical(row)))
    supplied_rows = {
        row["stable_id"]: (row["source_path"], row["row_sha256"])
        for row in authority["resolved_rows"]
    }
    if len(supplied_rows) != len(authority["resolved_rows"]) or supplied_rows != expected_rows:
        raise EnvelopeError("resolved authority rows mismatch")
    ordered_resolved = sorted(authority["resolved_rows"], key=lambda row: row["stable_id"].encode())
    resolved_digest = digest_bytes(canonical(ordered_resolved))
    if authority["resolution"]["resolved_set_sha256"] != resolved_digest:
        raise EnvelopeError("resolved-set digest mismatch")
    authority_session = digest_bytes(canonical({"sources": authority["sources"], "resolved_rows": ordered_resolved}))
    if authority["session_id"] != authority_session:
        raise EnvelopeError("authority session ID mismatch")

    scope = envelope["scope"]
    for grant in scope["artifact_paths"] + scope["fixtures"]:
        verify_scope_path(root, grant["path"], grant["access"] == "write")
    for relative in scope["generated_outputs"]:
        verify_scope_path(root, relative, True)
    for relative in scope["forbidden_paths"]:
        verify_scope_path(root, relative, True)
    resolved_scope = digest_bytes(canonical(scope_core(scope)))
    if scope["path_resolution"]["resolved_scope_sha256"] != resolved_scope:
        raise EnvelopeError("resolved scope digest mismatch")
    scope_session = digest_bytes(canonical({"candidate_id": candidate["candidate_id"], "scope": scope_core(scope)}))
    if scope["path_resolution"]["session_id"] != scope_session:
        raise EnvelopeError("scope session ID mismatch")

    for reference in envelope["evidence"]["primary_references"]:
        verify_bound_file(root, reference["path"], reference["sha256"])

    output = envelope["output_contract"]
    output_schema = verify_bound_file(root, output["schema_ref"], output["schema_sha256"])
    parsed_output_schema = json.loads(output_schema.read_bytes())
    if parsed_output_schema.get("title") != output["schema_id"]:
        raise EnvelopeError("output schema ID does not resolve to the bound schema")

    expected_context_id = digest_bytes(
        canonical(
            {
                "task_id": envelope["task_id"],
                "candidate_id": candidate["candidate_id"],
                "role_source_sha256": role["source_sha256"],
                "authority_session_id": authority["session_id"],
                "scope_session_id": scope["path_resolution"]["session_id"],
                "output_schema_sha256": output["schema_sha256"],
            }
        )
    )
    if candidate["context_id"] != expected_context_id:
        raise EnvelopeError("context ID mismatch")

    artifact_set(root, freeze["artifacts"])
    for source in authority["sources"]:
        verify_bound_file(root, source["path"], source["sha256"])
    verify_bound_file(root, role["source_ref"], role["source_sha256"])
    verify_bound_file(root, output["schema_ref"], output["schema_sha256"])


def set_pointer(document: dict[str, Any], pointer: str, value: Any) -> None:
    parts = [part.replace("~1", "/").replace("~0", "~") for part in pointer.split("/")[1:]]
    current: Any = document
    for part in parts[:-1]:
        current = current[int(part)] if isinstance(current, list) else current[part]
    final = parts[-1]
    if isinstance(current, list):
        current[int(final)] = value
    else:
        current[final] = value


def main() -> int:
    root = Path(git(HERE, "rev-parse", "--show-toplevel"))
    schema = json.loads(SCHEMA_PATH.read_bytes())
    fixtures = json.loads(FIXTURE_PATH.read_bytes())
    validator = jsonschema.Draft202012Validator(schema, format_checker=jsonschema.FormatChecker())
    baseline = fixtures["valid_baseline"]
    baseline_errors = list(validator.iter_errors(baseline))
    if baseline_errors:
        raise EnvelopeError("valid baseline failed schema: " + baseline_errors[0].message)
    semantic_validate(root, baseline)
    outcomes = []
    for case in fixtures["red_cases"]:
        candidate = copy.deepcopy(baseline)
        for mutation in case["mutations"]:
            set_pointer(candidate, mutation["pointer"], mutation["value"])
        schema_errors = list(validator.iter_errors(candidate))
        semantic_error = None
        if not schema_errors:
            try:
                semantic_validate(root, candidate)
            except EnvelopeError as error:
                semantic_error = str(error)
        rejected = bool(schema_errors or semantic_error)
        if not rejected:
            raise EnvelopeError(f"red fixture unexpectedly accepted: {case['case_id']}")
        outcomes.append(
            {
                "case_id": case["case_id"],
                "rejected_by": "schema" if schema_errors else "semantic_resolver",
            }
        )
    if len(outcomes) != 16:
        raise EnvelopeError("expected exactly 16 red fixtures")
    print(
        json.dumps(
            {
                "schema_version": "AgentTaskEnvelopeRedFixtureResult-v1",
                "baseline": "accepted",
                "red_cases": outcomes,
                "red_rejected": len(outcomes),
                "red_total": len(outcomes),
                "claim_effect": "none"
            },
            sort_keys=True,
            separators=(",", ":"),
        )
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (EnvelopeError, json.JSONDecodeError, OSError, subprocess.CalledProcessError) as error:
        print(f"agent-task-envelope verification failed: {error}", file=sys.stderr)
        raise SystemExit(1)
