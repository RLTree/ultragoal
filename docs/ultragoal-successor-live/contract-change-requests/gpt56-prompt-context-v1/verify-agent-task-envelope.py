#!/usr/bin/env python3
"""Deterministic proposal-only AgentTaskEnvelope-v1 red-fixture verifier."""

from __future__ import annotations

import copy
import errno
import hashlib
import json
import os
import pickle
import re
import stat
import subprocess
import sys
import tempfile
import threading
import tomllib
import weakref
from datetime import datetime
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
RUNTIME_RECEIPT_ROOT = "docs/ultragoal-successor-live/runtime-capabilities"
EXTERNAL_EFFECT_AUTHORITY_ROOT = (
    "docs/ultragoal-successor-live/root-decisions/effect-authority"
)
EXTERNAL_EFFECT_AUTHORITY_SCHEMA = "ExternalEffectAuthority-v1"
EXTERNAL_EFFECT_SESSION_RECEIPT_ROOT = EXTERNAL_EFFECT_AUTHORITY_ROOT + "/session-receipts"
EXTERNAL_EFFECT_SESSION_RECEIPT_SCHEMA = "ExternalEffectSessionReceiptAuthority-v1"
EXTERNAL_EFFECT_MEDIATION_MAX_AGE_SECONDS = 300
EXTERNAL_EFFECT_AUTHORITY_MAX_BYTES = 1024 * 1024
IDENTIFIER_RE = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._:@+/-]{0,239}$")
SHA256_RE = re.compile(r"^sha256:[0-9a-f]{64}$")
EFFECT_ID_RE = re.compile(r"^EFFECT-[A-Z0-9][A-Z0-9-]{0,119}$")
APPROVAL_ID_RE = re.compile(r"^APPROVAL-[A-Z0-9][A-Z0-9-]{0,119}$")
EFFECT_SESSION_ID_RE = re.compile(r"^EFFECT-SESSION-[A-Z0-9][A-Z0-9-]{0,99}$")
RUNTIME_AGENT_TYPES = {
    "implementation-worker": "worker",
    "verification-executor": "verification-executor",
}
PROTECTED_WRITE_PREFIXES = {
    "git_metadata_write_forbidden": (
        ".git",
    ),
    "shared_root_authority_write_forbidden": (
        ".agents",
        ".codex",
        ".codex-worktree",
        ".codex-plugin",
        "AGENTS.md",
        "Cargo.lock",
        "Cargo.toml",
        "docs/ultragoal-contract-2026-07-successor-v2/FINAL-CONTRACT",
        "docs/ultragoal-successor-live/CRITICAL-PATH-BOARD.md",
        "docs/ultragoal-successor-live/acceptance",
        "docs/ultragoal-successor-live/generated-authority",
        "docs/ultragoal-successor-live/root-decisions",
        "docs/ultragoal-successor-live/runtime-capabilities",
        "docs/ultragoal-successor-live/work-packages",
        "migration/authority-routes.json",
        "plugin-manifest-draft.json",
        "validator/src/api_witness.rs",
        "validator/src/command_witness.rs",
        "validator/src/cli/successor/catalog.rs",
        "validator/src/cli/successor/clap_grammar.rs",
        "validator/src/cli/successor/parser.rs",
        "validator/src/cli/successor/runtime/dispatch.rs",
        "validator/src/lib.rs",
    ),
}
ROOT_OWNED_LEAF_NAMES = {"AGENTS.md", "Cargo.lock", "Cargo.toml"}
ROOT_OWNED_LEAF_NAMES_CASEFOLDED = {
    name.casefold() for name in ROOT_OWNED_LEAF_NAMES
}
SHARED_SCHEMA_ROOT = "schemas"


class EnvelopeError(ValueError):
    def __init__(self, code: str, detail: str | None = None) -> None:
        self.code = code
        super().__init__(detail or code)


def canonical(value: Any) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode()


def digest_bytes(value: bytes) -> str:
    return "sha256:" + hashlib.sha256(value).hexdigest()


def repository_root_object_identity(
    metadata: os.stat_result,
) -> dict[str, int]:
    return {
        "device": int(metadata.st_dev),
        "inode": int(metadata.st_ino),
        "mode": int(stat.S_IFMT(metadata.st_mode)),
    }


def open_canonical_repository(root: Path) -> dict[str, Any]:
    """Descriptor-open one unambiguous repository directory without following it."""
    if not root.is_absolute():
        raise EnvelopeError(
            "canonical_repository_alias_rejected",
            "repository root must be an absolute canonical path",
        )
    required_flags = ("O_DIRECTORY", "O_NOFOLLOW")
    if any(not hasattr(os, flag) for flag in required_flags):
        raise EnvelopeError(
            "canonical_repository_descriptor_open_unsupported",
            "repository root validation requires no-follow directory-open support",
        )
    flags = os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW
    if hasattr(os, "O_CLOEXEC"):
        flags |= os.O_CLOEXEC
    try:
        descriptor = os.open(root, flags)
    except OSError as error:
        try:
            metadata = os.lstat(root)
        except FileNotFoundError:
            raise EnvelopeError(
                "canonical_repository_missing",
                "repository root does not exist",
            ) from error
        if stat.S_ISLNK(metadata.st_mode):
            raise EnvelopeError(
                "canonical_repository_symlink_rejected",
                "repository root cannot be a symbolic-link alias",
            ) from error
        if stat.S_ISREG(metadata.st_mode):
            raise EnvelopeError(
                "canonical_repository_not_directory",
                "repository root must be a directory",
            ) from error
        if not stat.S_ISDIR(metadata.st_mode):
            raise EnvelopeError(
                "canonical_repository_special_file_rejected",
                "repository root cannot be a special file",
            ) from error
        raise EnvelopeError(
            "canonical_repository_descriptor_open_failed",
            "repository root could not be opened without following links",
        ) from error
    try:
        descriptor_metadata = os.fstat(descriptor)
        if not stat.S_ISDIR(descriptor_metadata.st_mode):
            raise EnvelopeError(
                "canonical_repository_not_directory",
                "opened repository root is not a directory",
            )
        path_metadata = os.lstat(root)
        if stat.S_ISLNK(path_metadata.st_mode):
            raise EnvelopeError(
                "canonical_repository_symlink_rejected",
                "repository root became a symbolic-link alias",
            )
        object_identity = repository_root_object_identity(descriptor_metadata)
        if repository_root_object_identity(path_metadata) != object_identity:
            raise EnvelopeError(
                "canonical_repository_object_mismatch",
                "repository path and opened directory name different objects",
            )
        try:
            canonical_root = root.resolve(strict=True)
        except (FileNotFoundError, RuntimeError) as error:
            raise EnvelopeError(
                "canonical_repository_missing",
                "repository root cannot be resolved without ambiguity",
            ) from error
        if root != canonical_root:
            raise EnvelopeError(
                "canonical_repository_alias_rejected",
                "repository root differs from its canonical physical path",
            )
        final_metadata = os.lstat(root)
        if repository_root_object_identity(final_metadata) != object_identity:
            raise EnvelopeError(
                "canonical_repository_object_mismatch",
                "repository root changed during descriptor-first resolution",
            )
        return {
            "descriptor": descriptor,
            "root": canonical_root,
            "repository_identity": {
                "root_uri": f"repo://{canonical_root.name}",
                "canonical_root_sha256": digest_bytes(
                    str(canonical_root).encode()
                ),
            },
            "object_identity": object_identity,
        }
    except BaseException:
        os.close(descriptor)
        raise


def close_canonical_repository(handle: dict[str, Any]) -> None:
    descriptor = handle.get("descriptor")
    if isinstance(descriptor, int) and descriptor >= 0:
        os.close(descriptor)
        handle["descriptor"] = -1


def revalidate_canonical_repository(handle: dict[str, Any]) -> None:
    descriptor = handle.get("descriptor")
    if not isinstance(descriptor, int) or descriptor < 0:
        raise EnvelopeError(
            "canonical_repository_object_mismatch",
            "retained repository root descriptor is unavailable",
        )
    try:
        descriptor_metadata = os.fstat(descriptor)
        path_metadata = os.lstat(handle["root"])
    except OSError as error:
        raise EnvelopeError(
            "canonical_repository_object_mismatch",
            "retained repository root no longer resolves at its original path",
        ) from error
    expected = handle["object_identity"]
    if (
        repository_root_object_identity(descriptor_metadata) != expected
        or repository_root_object_identity(path_metadata) != expected
        or stat.S_ISLNK(path_metadata.st_mode)
    ):
        raise EnvelopeError(
            "canonical_repository_object_mismatch",
            "repository root path no longer names the retained directory object",
        )
    try:
        canonical_root = handle["root"].resolve(strict=True)
    except (FileNotFoundError, RuntimeError) as error:
        raise EnvelopeError(
            "canonical_repository_object_mismatch",
            "repository root no longer has its original canonical path",
        ) from error
    if canonical_root != handle["root"]:
        raise EnvelopeError(
            "canonical_repository_object_mismatch",
            "repository root became an alias after initial validation",
        )


def resolve_canonical_repository(
    root: Path,
) -> tuple[Path, dict[str, str]]:
    handle = open_canonical_repository(root)
    try:
        return handle["root"], copy.deepcopy(handle["repository_identity"])
    finally:
        close_canonical_repository(handle)


def require_candidate_repository_identity(
    candidate: dict[str, Any], repository_identity: dict[str, str]
) -> None:
    if candidate.get("repository") != repository_identity:
        raise EnvelopeError(
            "canonical_repository_identity_mismatch",
            "candidate repository identity does not match the internally resolved root",
        )


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


def context_commitment(envelope: dict[str, Any]) -> dict[str, Any]:
    committed = copy.deepcopy(envelope)
    committed["candidate_identity"].pop("context_id", None)
    return committed


def scope_core(scope: dict[str, Any]) -> dict[str, Any]:
    return {key: value for key, value in scope.items() if key != "path_resolution"}


def path_parts(relative: str) -> tuple[str, ...]:
    return tuple(part.casefold() for part in relative.split("/"))


def paths_overlap(left: str, right: str) -> bool:
    left_parts = path_parts(left)
    right_parts = path_parts(right)
    common = min(len(left_parts), len(right_parts))
    return left_parts[:common] == right_parts[:common]


def path_is_within_exact(path: str, root: str) -> bool:
    if not root.endswith("/"):
        return path == root
    path_parts_exact = tuple(path.split("/"))
    root_parts_exact = tuple(root.rstrip("/").split("/"))
    return path_parts_exact[: len(root_parts_exact)] == root_parts_exact


def scope_proposed_write_paths(scope: dict[str, Any]) -> list[str]:
    proposed = [
        grant["path"]
        for grant in scope["artifact_paths"] + scope["fixtures"]
        if grant["access"] == "propose_write"
    ]
    proposed.extend(scope["generated_outputs"])
    return proposed


def identity_row(relative: str, metadata: os.stat_result) -> dict[str, Any]:
    if stat.S_ISREG(metadata.st_mode):
        kind = "regular"
    elif stat.S_ISDIR(metadata.st_mode):
        kind = "directory"
    elif stat.S_ISLNK(metadata.st_mode):
        kind = "symlink"
    else:
        kind = "special"
    row = {
        "path": relative,
        "kind": kind,
        "device": metadata.st_dev,
        "inode": metadata.st_ino,
        "mode": stat.S_IMODE(metadata.st_mode),
        "link_count": metadata.st_nlink,
    }
    if kind == "regular":
        row["size"] = metadata.st_size
    return row


def proposed_write_identity_rows(root: Path, scope: dict[str, Any]) -> list[dict[str, Any]]:
    proposed_paths = scope_proposed_write_paths(scope)
    if not proposed_paths:
        return []
    maximum = scope["path_resolution"]["max_existing_descendants"]
    root_metadata = os.lstat(root)
    root_device = root_metadata.st_dev
    rows: dict[str, dict[str, Any]] = {}
    inode_paths: dict[tuple[int, int], str] = {}

    def ensure_budget() -> None:
        if len(rows) > maximum:
            raise EnvelopeError(
                "writable_directory_scan_budget_exceeded",
                f"proposed write identity set exceeds {maximum} objects",
            )

    def record(relative: str, metadata: os.stat_result) -> None:
        if stat.S_ISLNK(metadata.st_mode):
            raise EnvelopeError(
                "writable_symlink_rejected",
                f"proposed write path contains a symlink: {relative}",
            )
        if not (stat.S_ISREG(metadata.st_mode) or stat.S_ISDIR(metadata.st_mode)):
            raise EnvelopeError(
                "writable_special_file_rejected",
                f"proposed write path contains a special file: {relative}",
            )
        if metadata.st_dev != root_device:
            raise EnvelopeError(
                "writable_cross_device_object_rejected",
                f"proposed write path crosses the repository device: {relative}",
            )
        if stat.S_ISREG(metadata.st_mode) and metadata.st_nlink != 1:
            raise EnvelopeError(
                "writable_regular_file_hardlink_rejected",
                f"proposed writable regular file has link count {metadata.st_nlink}: {relative}",
            )
        inode_key = (metadata.st_dev, metadata.st_ino)
        prior_path = inode_paths.get(inode_key)
        if prior_path is not None and prior_path != relative:
            raise EnvelopeError(
                "writable_filesystem_alias_rejected",
                f"two proposed writable paths resolve to one object: {prior_path} <> {relative}",
            )
        inode_paths[inode_key] = relative
        rows[relative] = identity_row(relative, metadata)
        ensure_budget()

    def walk_directory(directory: Path, relative: str) -> None:
        names: list[str] = []
        with os.scandir(directory) as entries:
            for entry in entries:
                names.append(entry.name)
                if len(rows) + len(names) > maximum:
                    raise EnvelopeError(
                        "writable_directory_scan_budget_exceeded",
                        f"proposed writable directory exceeds {maximum} existing objects: {relative}",
                    )
        for name in sorted(names, key=lambda value: value.encode()):
            child = directory / name
            child_relative = f"{relative}/{name}"
            metadata = os.lstat(child)
            record(child_relative, metadata)
            if stat.S_ISDIR(metadata.st_mode):
                walk_directory(child, child_relative)

    for relative in sorted(set(proposed_paths), key=lambda value: value.encode()):
        current = root
        current_parts: list[str] = []
        missing = False
        for part in relative.split("/"):
            current /= part
            current_parts.append(part)
            current_relative = "/".join(current_parts)
            try:
                metadata = os.lstat(current)
            except FileNotFoundError:
                parent_metadata = os.lstat(current.parent)
                rows[relative] = {
                    "path": relative,
                    "kind": "absent",
                    "first_missing_path": current_relative,
                    "parent_device": parent_metadata.st_dev,
                    "parent_inode": parent_metadata.st_ino,
                }
                ensure_budget()
                missing = True
                break
            record(current_relative, metadata)
        if missing:
            continue
        target_metadata = os.lstat(current)
        if stat.S_ISDIR(target_metadata.st_mode):
            walk_directory(current, relative)

    return [rows[path] for path in sorted(rows, key=lambda value: value.encode())]


def scope_resolution_core(scope: dict[str, Any]) -> dict[str, Any]:
    return {
        key: value
        for key, value in scope["path_resolution"].items()
        if key != "session_id"
    }


def resolve_write_scope(
    scope: dict[str, Any],
    loaded_sources: dict[str, Any],
    authority: dict[str, Any],
) -> dict[str, Any]:
    binding = scope["write_scope_binding"]
    matches: list[tuple[str, dict[str, Any], dict[str, Any]]] = []
    for source_path, document in loaded_sources.items():
        if not isinstance(document, dict):
            continue
        root_authority = document.get("root_only_authority")
        for row in document.get("write_scopes", []):
            if isinstance(row, dict) and row.get("scope_id") == binding["scope_id"]:
                matches.append((source_path, row, root_authority))
    if len(matches) != 1:
        raise EnvelopeError(
            "same_session_write_scope_resolution_required",
            f"write scope did not resolve exactly once: {binding['scope_id']}",
        )
    source_path, write_scope, root_authority = matches[0]
    if (
        source_path != binding["source_path"]
        or digest_bytes(canonical(write_scope)) != binding["write_scope_row_sha256"]
        or write_scope.get("owner") != binding["owner"]
        or not isinstance(root_authority, dict)
        or root_authority.get("owner") != "OWN-ULTRA-ROOT"
        or not isinstance(root_authority.get("paths_or_semantics"), list)
        or not root_authority["paths_or_semantics"]
        or digest_bytes(canonical(root_authority))
        != binding["root_only_authority_sha256"]
    ):
        raise EnvelopeError(
            "write_scope_binding_mismatch",
            "write scope or root-only authority binding mismatch",
        )
    node_owners: set[str] = set()
    for node_id in authority["ids"]["nodes"]:
        node_matches: list[dict[str, Any]] = []
        for document in loaded_sources.values():
            rows: list[dict[str, Any]] = []
            collect_rows(document, "node_id", rows)
            node_matches.extend(row for row in rows if row.get("node_id") == node_id)
        if len(node_matches) != 1 or not isinstance(node_matches[0].get("owner"), str):
            raise EnvelopeError(
                "same_session_write_scope_resolution_required",
                f"write scope node owner did not resolve exactly once: {node_id}",
            )
        node_owners.add(node_matches[0]["owner"])
    if node_owners != {binding["owner"]}:
        raise EnvelopeError(
            "write_scope_node_owner_mismatch",
            "canonical write-scope owner does not match the selected node owner",
        )
    return write_scope


def parse_timestamp(value: str, rejection: str) -> datetime:
    try:
        parsed = datetime.fromisoformat(value.replace("Z", "+00:00"))
    except (TypeError, ValueError) as error:
        raise EnvelopeError(rejection, f"invalid authority timestamp: {value!r}") from error
    if parsed.tzinfo is None:
        raise EnvelopeError(rejection, "authority timestamp must include an offset")
    return parsed


def external_effect_identity(row: dict[str, Any]) -> dict[str, Any]:
    return {
        "effect_id": row["effect_id"],
        "kind": row["kind"],
        "provider": row["provider"],
        "action": row["action"],
        "target_sha256": row["target_sha256"],
    }


def validate_external_effect_source_row(
    row: Any, required: set[str], source_path: str
) -> None:
    if not isinstance(row, dict) or set(row) != required:
        raise EnvelopeError(
            "external_effect_authority_source_malformed",
            f"malformed external-effect authority row in {source_path}",
        )
    if "effect_id" in row and not (
        isinstance(row["effect_id"], str) and EFFECT_ID_RE.fullmatch(row["effect_id"])
    ):
        raise EnvelopeError(
            "external_effect_authority_source_malformed",
            f"malformed effect ID in {source_path}",
        )
    if "approval_id" in row and not (
        isinstance(row["approval_id"], str)
        and APPROVAL_ID_RE.fullmatch(row["approval_id"])
    ):
        raise EnvelopeError(
            "external_effect_authority_source_malformed",
            f"malformed approval ID in {source_path}",
        )
    if "kind" in row and row["kind"] not in {"network", "external_write"}:
        raise EnvelopeError(
            "external_effect_authority_source_malformed",
            f"malformed effect kind in {source_path}",
        )
    for field in ("provider", "action"):
        if field in row and not (
            isinstance(row[field], str) and IDENTIFIER_RE.fullmatch(row[field])
        ):
            raise EnvelopeError(
                "external_effect_authority_source_malformed",
                f"malformed {field} in {source_path}",
            )
    if "target_sha256" in row and not (
        isinstance(row["target_sha256"], str)
        and SHA256_RE.fullmatch(row["target_sha256"])
    ):
        raise EnvelopeError(
            "external_effect_authority_source_malformed",
            f"malformed effect target digest in {source_path}",
        )
    if not isinstance(row.get("status"), str) or not row["status"]:
        raise EnvelopeError(
            "external_effect_authority_source_malformed",
            f"malformed authority status in {source_path}",
        )
    if "approval_id" in row:
        parse_timestamp(row["valid_from"], "external_effect_authority_source_malformed")
        parse_timestamp(row["expires_at"], "external_effect_authority_source_malformed")
        consumed = row["consumed_by_context_id"]
        if (
            not isinstance(row["single_use"], bool)
            or not isinstance(row["usage_count"], int)
            or isinstance(row["usage_count"], bool)
            or row["usage_count"] < 0
            or not (
                consumed is None
                or (isinstance(consumed, str) and SHA256_RE.fullmatch(consumed))
            )
        ):
            raise EnvelopeError(
                "external_effect_authority_source_malformed",
                f"malformed approval usage binding in {source_path}",
            )


def external_effect_resolution_digest(
    effect_rows: list[dict[str, Any]],
    approval_rows: list[dict[str, Any]],
    session_receipt: dict[str, Any] | None,
) -> str:
    return digest_bytes(
        canonical(
            {
                "effect_rows": sorted(
                    effect_rows, key=lambda row: row["effect_id"].encode()
                ),
                "approval_rows": sorted(
                    approval_rows, key=lambda row: row["approval_id"].encode()
                ),
                "session_receipt": session_receipt,
            }
        )
    )


def external_effect_session_digest(binding: dict[str, Any]) -> str:
    return digest_bytes(
        canonical(
            {
                "sources": sorted(binding["sources"], key=lambda row: row["path"].encode()),
                "effect_rows": sorted(
                    binding["effect_rows"], key=lambda row: row["effect_id"].encode()
                ),
                "approval_rows": sorted(
                    binding["approval_rows"], key=lambda row: row["approval_id"].encode()
                ),
                "session_receipt": binding["session_receipt"],
            }
        )
    )


def external_effect_source_identity(metadata: os.stat_result) -> dict[str, int]:
    return {
        "device": metadata.st_dev,
        "inode": metadata.st_ino,
        "mode": stat.S_IMODE(metadata.st_mode),
        "link_count": metadata.st_nlink,
        "size": metadata.st_size,
        "mtime_ns": metadata.st_mtime_ns,
        "ctime_ns": metadata.st_ctime_ns,
    }


def read_external_effect_authority_source(
    root: Path, relative: str
) -> tuple[bytes, dict[str, int]]:
    parts = tuple(Path(relative).parts)
    if not parts:
        raise EnvelopeError(
            "external_effect_authority_source_required",
            "external-effect authority source path is empty",
        )
    directory_flags = os.O_RDONLY | os.O_DIRECTORY | os.O_CLOEXEC
    leaf_flags = os.O_RDONLY | os.O_NOFOLLOW | os.O_CLOEXEC | os.O_NONBLOCK
    root_fd = os.open(root, directory_flags)
    current_fd = root_fd
    opened_fds = [root_fd]
    try:
        root_metadata = os.fstat(root_fd)
        root_device = root_metadata.st_dev
        for index, part in enumerate(parts):
            is_leaf = index == len(parts) - 1
            try:
                preflight = os.stat(part, dir_fd=current_fd, follow_symlinks=False)
            except FileNotFoundError as error:
                raise EnvelopeError(
                    "external_effect_authority_source_required",
                    f"external-effect authority source path is missing: {relative}",
                ) from error
            if stat.S_ISLNK(preflight.st_mode):
                raise EnvelopeError(
                    "external_effect_authority_symlink_rejected",
                    f"external-effect authority source contains a symlink: {relative}",
                )
            if preflight.st_dev != root_device:
                raise EnvelopeError(
                    "external_effect_authority_cross_device_rejected",
                    f"external-effect authority source crosses repository device: {relative}",
                )
            if is_leaf:
                if not stat.S_ISREG(preflight.st_mode):
                    raise EnvelopeError(
                        "external_effect_authority_special_file_rejected",
                        f"external-effect authority source is not a regular file: {relative}",
                    )
                flags = leaf_flags
            else:
                if not stat.S_ISDIR(preflight.st_mode):
                    raise EnvelopeError(
                        "external_effect_authority_special_file_rejected",
                        f"external-effect authority ancestor is not a directory: {relative}",
                    )
                flags = directory_flags | os.O_NOFOLLOW
            try:
                child_fd = os.open(part, flags, dir_fd=current_fd)
            except OSError as error:
                if error.errno in {errno.ELOOP, errno.EMLINK}:
                    code = "external_effect_authority_symlink_rejected"
                else:
                    code = "external_effect_authority_source_identity_mismatch"
                raise EnvelopeError(
                    code,
                    f"descriptor-safe authority open failed: {relative}: {error}",
                ) from error
            opened_fds.append(child_fd)
            current_fd = child_fd
            opened = os.fstat(child_fd)
            if opened.st_dev != root_device:
                raise EnvelopeError(
                    "external_effect_authority_cross_device_rejected",
                    f"opened authority object crosses repository device: {relative}",
                )
            if is_leaf and not stat.S_ISREG(opened.st_mode):
                raise EnvelopeError(
                    "external_effect_authority_special_file_rejected",
                    f"opened authority object is not a regular file: {relative}",
                )
            if not is_leaf and not stat.S_ISDIR(opened.st_mode):
                raise EnvelopeError(
                    "external_effect_authority_special_file_rejected",
                    f"opened authority ancestor is not a directory: {relative}",
                )
            if (
                opened.st_dev != preflight.st_dev
                or opened.st_ino != preflight.st_ino
                or stat.S_IFMT(opened.st_mode) != stat.S_IFMT(preflight.st_mode)
            ):
                raise EnvelopeError(
                    "external_effect_authority_source_identity_mismatch",
                    f"authority path changed during descriptor traversal: {relative}",
                )
        metadata = os.fstat(current_fd)
        if metadata.st_nlink != 1:
            raise EnvelopeError(
                "external_effect_authority_alias_rejected",
                f"authority source link count is {metadata.st_nlink}: {relative}",
            )
        if metadata.st_size > EXTERNAL_EFFECT_AUTHORITY_MAX_BYTES:
            raise EnvelopeError(
                "external_effect_authority_size_limit_exceeded",
                f"authority source exceeds {EXTERNAL_EFFECT_AUTHORITY_MAX_BYTES} bytes: {relative}",
            )
        chunks: list[bytes] = []
        remaining = EXTERNAL_EFFECT_AUTHORITY_MAX_BYTES + 1
        while remaining:
            chunk = os.read(current_fd, min(65536, remaining))
            if not chunk:
                break
            chunks.append(chunk)
            remaining -= len(chunk)
        data = b"".join(chunks)
        if len(data) > EXTERNAL_EFFECT_AUTHORITY_MAX_BYTES:
            raise EnvelopeError(
                "external_effect_authority_size_limit_exceeded",
                f"authority source read exceeds bound: {relative}",
            )
        final_metadata = os.fstat(current_fd)
        if external_effect_source_identity(final_metadata) != external_effect_source_identity(
            metadata
        ):
            raise EnvelopeError(
                "external_effect_authority_source_identity_mismatch",
                f"authority source changed while its descriptor was open: {relative}",
            )
        try:
            reopened_fd = os.open(parts[-1], leaf_flags, dir_fd=opened_fds[-2])
        except OSError as error:
            raise EnvelopeError(
                "external_effect_authority_source_identity_mismatch",
                f"authority path could not be reopened after read: {relative}",
            ) from error
        try:
            reopened_metadata = os.fstat(reopened_fd)
            if external_effect_source_identity(
                reopened_metadata
            ) != external_effect_source_identity(final_metadata):
                raise EnvelopeError(
                    "external_effect_authority_source_identity_mismatch",
                    f"authority path no longer names the opened object: {relative}",
                )
        finally:
            os.close(reopened_fd)
        return data, external_effect_source_identity(final_metadata)
    finally:
        for descriptor in reversed(opened_fds):
            try:
                os.close(descriptor)
            except OSError:
                pass


def load_external_effect_authority_sources(
    root: Path, binding: dict[str, Any]
) -> list[tuple[str, str, dict[str, int], dict[str, Any]]]:
    if binding["namespace"] != EXTERNAL_EFFECT_AUTHORITY_ROOT:
        raise EnvelopeError(
            "external_effect_authority_source_required",
            "external-effect authority namespace is not the protected root namespace",
        )
    source_paths = [source["path"] for source in binding["sources"]]
    if len(source_paths) != len(set(source_paths)):
        raise EnvelopeError(
            "external_effect_authority_duplicate_source",
            "external-effect authority repeats one source binding",
        )
    loaded: list[tuple[str, str, dict[str, int], dict[str, Any]]] = []
    required_keys = {
        "schema_version",
        "authority_owner",
        "source_kind",
        "captured_at",
        "effects",
        "approvals",
    }
    for source in binding["sources"]:
        source_path = source["path"]
        if not source_path.startswith(EXTERNAL_EFFECT_AUTHORITY_ROOT + "/"):
            raise EnvelopeError(
                "external_effect_authority_source_required",
                f"external-effect authority is outside the protected root namespace: {source_path}",
            )
        if source_path.startswith(EXTERNAL_EFFECT_SESSION_RECEIPT_ROOT + "/"):
            raise EnvelopeError(
                "external_effect_authority_source_required",
                f"effect rows cannot use the session-receipt namespace: {source_path}",
            )
        try:
            data, identity = read_external_effect_authority_source(root, source_path)
        except EnvelopeError as error:
            raise
        actual_sha256 = digest_bytes(data)
        if actual_sha256 != source["sha256"]:
            raise EnvelopeError(
                "external_effect_authority_source_digest_mismatch",
                f"external-effect authority source digest mismatch: {source_path}",
            )
        if identity != source["identity"]:
            raise EnvelopeError(
                "external_effect_authority_source_identity_mismatch",
                f"external-effect authority source object identity mismatch: {source_path}",
            )
        try:
            document = json.loads(data)
        except json.JSONDecodeError as error:
            raise EnvelopeError(
                "external_effect_authority_source_malformed",
                f"external-effect authority source is not JSON: {source_path}",
            ) from error
        if not isinstance(document, dict) or set(document) != required_keys:
            raise EnvelopeError(
                "external_effect_authority_source_malformed",
                f"external-effect authority source has missing or unknown fields: {source_path}",
            )
        if (
            document["schema_version"] != EXTERNAL_EFFECT_AUTHORITY_SCHEMA
            or document["authority_owner"] != "OWN-ULTRA-ROOT"
            or document["source_kind"] != "adopted-current-external-effect-authority"
            or not isinstance(document["effects"], list)
            or not isinstance(document["approvals"], list)
        ):
            raise EnvelopeError(
                "external_effect_authority_source_malformed",
                f"external-effect authority source lacks root-owned current authority: {source_path}",
            )
        parse_timestamp(
            document["captured_at"], "external_effect_authority_source_malformed"
        )
        loaded.append((source_path, source["sha256"], identity, document))
    return loaded


def effect_authority_source_set_digest(binding: dict[str, Any]) -> str:
    return digest_bytes(
        canonical(sorted(binding["sources"], key=lambda row: row["path"].encode()))
    )


def _external_effect_validation_binding(
    binding: dict[str, Any],
    authority: dict[str, Any],
    candidate: dict[str, Any],
    receipt: dict[str, Any],
    repository_identity: dict[str, str],
    repository_object_identity: dict[str, int],
) -> dict[str, Any]:
    return copy.deepcopy(
        {
        "repository_root_sha256": repository_identity[
            "canonical_root_sha256"
        ],
        "repository_root_object_identity": repository_object_identity,
        "candidate_id": candidate["candidate_id"],
        "context_id": candidate.get("context_id"),
        "authority_session_id": authority["session_id"],
        "effect_authority_sources_sha256": effect_authority_source_set_digest(
            binding
        ),
        "external_effect_authority_session_id": binding["session_id"],
        "receipt_id": receipt["receipt_id"],
        "receipt_session_id": receipt["session_id"],
        "receipt_source_sha256": receipt["source_sha256"],
        "receipt_row_sha256": receipt["row_sha256"],
        "external_effect_authority_resolution_sha256": binding["resolution"][
            "resolved_set_sha256"
        ],
        "effect_rows": binding["effect_rows"],
        "approval_rows": binding["approval_rows"],
        }
    )


def _external_effect_initial_result_digest(
    validation_binding: dict[str, Any], observed_at: str
) -> str:
    return digest_bytes(
        canonical(
            {
                "schema_version": "ExternalEffectInitialValidationResult-v1",
                "observed_at": observed_at,
                "validation_binding": validation_binding,
            }
        )
    )


def _make_external_effect_mediator_type() -> type:
    states = weakref.WeakKeyDictionary()
    state_seal = object()
    consumed_authorizations: set[str] = set()
    consumption_lock = threading.Lock()

    def consumption_keys(
        validation_binding: dict[str, Any], initial_result_sha256: str
    ) -> tuple[str, ...]:
        common = {
            "repository_root_sha256": validation_binding[
                "repository_root_sha256"
            ],
            "repository_root_object_identity": validation_binding[
                "repository_root_object_identity"
            ],
            "candidate_id": validation_binding["candidate_id"],
            "context_id": validation_binding["context_id"],
            "authority_session_id": validation_binding["authority_session_id"],
            "effect_authority_sources_sha256": validation_binding[
                "effect_authority_sources_sha256"
            ],
            "external_effect_authority_session_id": validation_binding[
                "external_effect_authority_session_id"
            ],
        }
        keys: set[str] = set()
        for approval in validation_binding["approval_rows"]:
            keys.add(
                digest_bytes(
                    canonical(
                        {
                            "kind": "external-effect-approval-id-tombstone-v1",
                            "repository_root_sha256": validation_binding[
                                "repository_root_sha256"
                            ],
                            "protected_namespace": EXTERNAL_EFFECT_AUTHORITY_ROOT,
                            "approval_id": approval["approval_id"],
                        }
                    )
                )
            )
            stable_approval = {
                "kind": "external-effect-approval-row-consumption-v1",
                "repository_root_sha256": validation_binding[
                    "repository_root_sha256"
                ],
                "approval_id": approval["approval_id"],
                "effect_id": approval["effect_id"],
                "source_path": approval["source_path"],
                "row_sha256": approval["row_sha256"],
            }
            keys.add(digest_bytes(canonical(stable_approval)))
            keys.add(
                digest_bytes(
                    canonical(
                        {
                            "kind": "external-effect-approval-bound-consumption-v1",
                            "common": common,
                            "source_sha256": approval["source_sha256"],
                            "approval": stable_approval,
                        }
                    )
                )
            )
        stable_receipt = {
            "kind": "external-effect-receipt-row-consumption-v1",
            "receipt_id": validation_binding["receipt_id"],
            "receipt_session_id": validation_binding["receipt_session_id"],
            "receipt_source_sha256": validation_binding["receipt_source_sha256"],
            "receipt_row_sha256": validation_binding["receipt_row_sha256"],
        }
        keys.add(
            digest_bytes(
                canonical(
                    {
                        "kind": "external-effect-receipt-id-tombstone-v1",
                        "repository_root_sha256": validation_binding[
                            "repository_root_sha256"
                        ],
                        "protected_namespace": EXTERNAL_EFFECT_SESSION_RECEIPT_ROOT,
                        "receipt_id": validation_binding["receipt_id"],
                    }
                )
            )
        )
        keys.add(digest_bytes(canonical(stable_receipt)))
        keys.add(
            digest_bytes(
                canonical(
                    {
                        "kind": "external-effect-authorization-set-consumption-v1",
                        "common": common,
                        "effect_rows": validation_binding["effect_rows"],
                        "approval_rows": validation_binding["approval_rows"],
                        "receipt": stable_receipt,
                        "initial_result_sha256": initial_result_sha256,
                    }
                )
            )
        )
        return tuple(sorted(keys))

    class ExternalEffectMediator:
        """Owns one non-serializable initial-to-final external-effect capability."""

        __slots__ = ("__weakref__",)

        def __init__(self) -> None:
            descriptor_holder: dict[str, int | None] = {"descriptor": None}

            def close_retained_descriptor() -> None:
                descriptor = descriptor_holder["descriptor"]
                if isinstance(descriptor, int) and descriptor >= 0:
                    try:
                        os.close(descriptor)
                    except OSError:
                        pass
                    descriptor_holder["descriptor"] = None

            states[self] = {
                "seal": state_seal,
                "pending_token": None,
                "pending_state": None,
                "consumed_tokens": [],
                "descriptor_holder": descriptor_holder,
                "descriptor_finalizer": weakref.finalize(
                    self, close_retained_descriptor
                ),
            }

        def initial(
            self,
            root: Path,
            scope: dict[str, Any],
            authority: dict[str, Any],
            *,
            candidate: dict[str, Any],
            trusted_now: datetime | None,
        ) -> object:
            state = states.get(self)
            if state is None or state.get("seal") is not state_seal:
                raise EnvelopeError(
                    "external_effect_mediator_state_injection_rejected",
                    "mediator does not own valid private state",
                )
            if state["pending_token"] is not None:
                raise EnvelopeError(
                    "external_effect_mediator_initial_pending",
                    "mediator already owns an unconsumed external-effect capability",
                )
            repository_handle = open_canonical_repository(root)
            transferred_handle = False
            try:
                validation_binding = _validate_external_effect_authority_core(
                    repository_handle["root"],
                    scope,
                    authority,
                    repository_handle=repository_handle,
                    candidate=candidate,
                    trusted_now=trusted_now,
                    final_revalidation=False,
                )
                revalidate_canonical_repository(repository_handle)
                if validation_binding is None or trusted_now is None:
                    raise EnvelopeError(
                        "external_effect_mediator_nonempty_scope_required",
                        "mediator capabilities require a successfully validated nonempty effect scope",
                    )
                observed_at = trusted_now.isoformat()
                mint_seal = object()

                class _OpaqueExternalEffectCapability:
                    __slots__ = ()

                    def __new__(cls, seal: object) -> object:
                        if seal is not mint_seal:
                            raise TypeError(
                                "external-effect capability construction rejected"
                            )
                        return super().__new__(cls)

                    def __copy__(self) -> object:
                        raise TypeError("external-effect capability copy rejected")

                    def __deepcopy__(self, memo: dict[int, Any]) -> object:
                        raise TypeError(
                            "external-effect capability deepcopy rejected"
                        )

                    def __reduce__(self) -> object:
                        raise TypeError(
                            "external-effect capability serialization rejected"
                        )

                    def __reduce_ex__(self, protocol: int) -> object:
                        raise TypeError(
                            "external-effect capability serialization rejected"
                        )

                    def __repr__(self) -> str:
                        return "<opaque-external-effect-capability>"

                token = _OpaqueExternalEffectCapability(mint_seal)
                state["pending_token"] = token
                state["pending_state"] = {
                    "validation_binding": validation_binding,
                    "observed_at": observed_at,
                    "initial_result_sha256": _external_effect_initial_result_digest(
                        validation_binding, observed_at
                    ),
                    "repository_handle": repository_handle,
                }
                state["descriptor_holder"]["descriptor"] = repository_handle[
                    "descriptor"
                ]
                transferred_handle = True
                return token
            finally:
                if not transferred_handle:
                    close_canonical_repository(repository_handle)

        def final(
            self,
            root: Path,
            scope: dict[str, Any],
            authority: dict[str, Any],
            token: object | None,
            *,
            candidate: dict[str, Any],
            trusted_now: datetime | None,
        ) -> None:
            state = states.get(self)
            if state is None or state.get("seal") is not state_seal:
                raise EnvelopeError(
                    "external_effect_mediator_state_injection_rejected",
                    "mediator does not own valid private state",
                )
            if token is None:
                raise EnvelopeError(
                    "external_effect_mediator_token_required",
                    "final mediation requires the exact capability returned by initial validation",
                )
            if any(token is consumed for consumed in state["consumed_tokens"]):
                raise EnvelopeError(
                    "external_effect_mediator_token_reuse_rejected",
                    "external-effect capability was already consumed",
                )
            if token is not state["pending_token"] or state["pending_state"] is None:
                raise EnvelopeError(
                    "external_effect_mediator_token_invalid",
                    "external-effect capability was not minted by this mediator's successful initial validation",
                )
            pending = state["pending_state"]
            retained_handle = pending["repository_handle"]
            revalidate_canonical_repository(retained_handle)
            current_handle = open_canonical_repository(root)
            try:
                require_candidate_repository_identity(
                    candidate, current_handle["repository_identity"]
                )
                if (
                    current_handle["object_identity"]
                    != retained_handle["object_identity"]
                ):
                    raise EnvelopeError(
                        "canonical_repository_object_mismatch",
                        "final repository root is not the object retained at initial validation",
                    )
                current_binding = _validate_external_effect_authority_core(
                    current_handle["root"],
                    scope,
                    authority,
                    repository_handle=current_handle,
                    candidate=candidate,
                    trusted_now=trusted_now,
                    final_revalidation=True,
                )
                revalidate_canonical_repository(retained_handle)
                revalidate_canonical_repository(current_handle)
                if current_binding is None or trusted_now is None:
                    raise EnvelopeError(
                        "external_effect_mediator_nonempty_scope_required",
                        "final mediation requires a validated nonempty external-effect scope",
                    )
                if current_binding != pending["validation_binding"]:
                    raise EnvelopeError(
                        "external_effect_mediator_token_binding_mismatch",
                        "current repository, candidate, authority, effect, or receipt binding differs from initial validation",
                    )
                if pending[
                    "initial_result_sha256"
                ] != _external_effect_initial_result_digest(
                    current_binding, pending["observed_at"]
                ):
                    raise EnvelopeError(
                        "external_effect_mediator_initial_result_mismatch",
                        "mediator capability no longer binds the exact successful initial validation result",
                    )
                observed_at = parse_timestamp(
                    pending["observed_at"],
                    "external_effect_mediator_initial_result_mismatch",
                )
                if trusted_now < observed_at:
                    raise EnvelopeError(
                        "external_effect_clock_rollback_rejected",
                        "trusted time moved backward between initial and final mediation",
                    )
                if (
                    trusted_now - observed_at
                ).total_seconds() > EXTERNAL_EFFECT_MEDIATION_MAX_AGE_SECONDS:
                    raise EnvelopeError(
                        "external_effect_mediator_token_stale",
                        "external-effect capability is too old for final mediation",
                    )
                keys = consumption_keys(
                    current_binding, pending["initial_result_sha256"]
                )
                with consumption_lock:
                    if any(key in consumed_authorizations for key in keys):
                        raise EnvelopeError(
                            "external_effect_authorization_reuse_rejected",
                            "approval or session-receipt identity was already consumed in this mediator process",
                        )
                    consumed_authorizations.update(keys)
                    state["consumed_tokens"].append(token)
                    state["pending_token"] = None
                    state["pending_state"] = None
                    close_canonical_repository(retained_handle)
                    state["descriptor_holder"]["descriptor"] = None
            finally:
                close_canonical_repository(current_handle)

    return ExternalEffectMediator


ExternalEffectMediator = _make_external_effect_mediator_type()
del _make_external_effect_mediator_type


def validate_external_effect_session_receipt(
    root: Path,
    binding: dict[str, Any],
    authority: dict[str, Any],
    candidate: dict[str, Any] | None,
    trusted_now: datetime | None,
    has_scoped_effects: bool,
) -> tuple[datetime | None, dict[str, Any] | None]:
    receipt_binding = binding["session_receipt"]
    if not has_scoped_effects:
        if receipt_binding is not None:
            raise EnvelopeError(
                "external_effect_session_receipt_unreferenced",
                "zero-effect scope must not carry a time/session receipt",
            )
        return trusted_now, None
    if receipt_binding is None:
        raise EnvelopeError(
            "external_effect_session_receipt_required",
            "nonempty external-effect scope requires a protected root session receipt",
        )
    if candidate is None:
        raise EnvelopeError(
            "external_effect_session_candidate_binding_required",
            "external-effect session receipt lacks candidate binding input",
        )
    if trusted_now is None:
        raise EnvelopeError(
            "external_effect_trusted_time_unavailable",
            "proposal validation cannot authorize an effect without mediator-supplied trusted time",
        )
    if trusted_now.tzinfo is None:
        raise EnvelopeError(
            "external_effect_trusted_time_invalid",
            "trusted time must include an offset",
        )
    if receipt_binding["namespace"] != EXTERNAL_EFFECT_SESSION_RECEIPT_ROOT:
        raise EnvelopeError(
            "external_effect_session_receipt_source_required",
            "session receipt namespace is not protected root authority",
        )
    source_paths = [source["path"] for source in receipt_binding["sources"]]
    if len(source_paths) != len(set(source_paths)):
        raise EnvelopeError(
            "external_effect_session_receipt_duplicate_source",
            "session receipt repeats one source binding",
        )
    required_document = {
        "schema_version",
        "authority_owner",
        "source_kind",
        "receipts",
    }
    required_row = {
        "receipt_id",
        "session_id",
        "nonce",
        "issuer",
        "issued_at",
        "expires_at",
        "permitted_skew_seconds",
        "repository_root_sha256",
        "candidate_id",
        "authority_session_id",
        "effect_authority_sources_sha256",
        "single_use",
        "usage_count",
        "consumed_by_context_id",
    }
    matches: list[tuple[str, str, dict[str, int], dict[str, Any]]] = []
    all_rows: list[tuple[str, dict[str, Any]]] = []
    for source in receipt_binding["sources"]:
        source_path = source["path"]
        if not source_path.startswith(EXTERNAL_EFFECT_SESSION_RECEIPT_ROOT + "/"):
            raise EnvelopeError(
                "external_effect_session_receipt_source_required",
                f"session receipt is outside protected root authority: {source_path}",
            )
        data, identity = read_external_effect_authority_source(root, source_path)
        if digest_bytes(data) != source["sha256"]:
            raise EnvelopeError(
                "external_effect_session_receipt_source_digest_mismatch",
                f"session receipt source digest mismatch: {source_path}",
            )
        if identity != source["identity"]:
            raise EnvelopeError(
                "external_effect_session_receipt_source_identity_mismatch",
                f"session receipt source object identity mismatch: {source_path}",
            )
        try:
            document = json.loads(data)
        except json.JSONDecodeError as error:
            raise EnvelopeError(
                "external_effect_session_receipt_source_malformed",
                f"session receipt source is not JSON: {source_path}",
            ) from error
        if (
            not isinstance(document, dict)
            or set(document) != required_document
            or document["schema_version"] != EXTERNAL_EFFECT_SESSION_RECEIPT_SCHEMA
            or document["authority_owner"] != "OWN-ULTRA-ROOT"
            or document["source_kind"] != "current-external-effect-session-receipt"
            or not isinstance(document["receipts"], list)
        ):
            raise EnvelopeError(
                "external_effect_session_receipt_source_malformed",
                f"session receipt source has invalid root authority shape: {source_path}",
            )
        for row in document["receipts"]:
            if not isinstance(row, dict) or set(row) != required_row:
                raise EnvelopeError(
                    "external_effect_session_receipt_source_malformed",
                    f"session receipt row is malformed: {source_path}",
                )
            if not (
                isinstance(row["receipt_id"], str)
                and EFFECT_SESSION_ID_RE.fullmatch(row["receipt_id"])
                and isinstance(row["session_id"], str)
                and SHA256_RE.fullmatch(row["session_id"])
                and isinstance(row["nonce"], str)
                and SHA256_RE.fullmatch(row["nonce"])
                and isinstance(row["permitted_skew_seconds"], int)
                and not isinstance(row["permitted_skew_seconds"], bool)
                and 0 <= row["permitted_skew_seconds"] <= 5
            ):
                raise EnvelopeError(
                    "external_effect_session_receipt_source_malformed",
                    f"session receipt identity or skew is malformed: {source_path}",
                )
            for field in (
                "repository_root_sha256",
                "candidate_id",
                "authority_session_id",
                "effect_authority_sources_sha256",
            ):
                if not (
                    isinstance(row[field], str) and SHA256_RE.fullmatch(row[field])
                ):
                    raise EnvelopeError(
                        "external_effect_session_receipt_source_malformed",
                        f"session receipt digest field is malformed: {field}",
                    )
            parse_timestamp(
                row["issued_at"], "external_effect_session_receipt_source_malformed"
            )
            parse_timestamp(
                row["expires_at"], "external_effect_session_receipt_source_malformed"
            )
            all_rows.append((source_path, row))
            if row["receipt_id"] == receipt_binding["receipt_id"]:
                matches.append((source_path, source["sha256"], identity, row))
    if len(matches) != 1:
        source_count = len({source_path for source_path, _, _, _ in matches})
        if not matches:
            code = "external_effect_session_receipt_required"
        elif source_count > 1:
            code = "external_effect_session_receipt_ambiguous"
        else:
            code = "external_effect_session_receipt_duplicate"
        raise EnvelopeError(code, "session receipt did not resolve exactly once")
    if len(all_rows) != 1:
        raise EnvelopeError(
            "external_effect_session_receipt_unreferenced",
            "session receipt authority contains unreferenced rows",
        )
    nonces = [row["nonce"] for _, row in all_rows]
    if len(nonces) != len(set(nonces)):
        raise EnvelopeError(
            "external_effect_session_receipt_reuse_rejected",
            "session receipt nonce is reused",
        )
    source_path, source_sha256, source_identity, row = matches[0]
    expected_session_id = digest_bytes(
        canonical({key: value for key, value in row.items() if key != "session_id"})
    )
    if row["session_id"] != expected_session_id:
        raise EnvelopeError(
            "external_effect_session_receipt_session_mismatch",
            "session receipt ID does not commit its immutable row",
        )
    expected_source_set = effect_authority_source_set_digest(binding)
    if (
        row["issuer"] != "OWN-ULTRA-ROOT"
        or row["repository_root_sha256"]
        != candidate["repository"]["canonical_root_sha256"]
        or row["candidate_id"] != candidate["candidate_id"]
        or row["authority_session_id"] != authority["session_id"]
        or row["effect_authority_sources_sha256"] != expected_source_set
    ):
        raise EnvelopeError(
            "external_effect_session_receipt_binding_mismatch",
            "session receipt does not match repository, candidate, authority session, or effect sources",
        )
    issued_at = parse_timestamp(
        row["issued_at"], "external_effect_session_receipt_source_malformed"
    )
    expires_at = parse_timestamp(
        row["expires_at"], "external_effect_session_receipt_source_malformed"
    )
    envelope_captured_at = parse_timestamp(
        authority["captured_at"], "external_effect_authority_session_invalid"
    )
    skew = row["permitted_skew_seconds"]
    if abs((envelope_captured_at - issued_at).total_seconds()) > skew:
        raise EnvelopeError(
            "external_effect_session_receipt_skew_violation",
            "envelope capture time is outside the root-issued receipt skew",
        )
    if trusted_now < issued_at:
        raise EnvelopeError(
            "external_effect_session_receipt_future",
            "root-issued session receipt is in the future",
        )
    if trusted_now >= expires_at:
        raise EnvelopeError(
            "external_effect_session_receipt_stale",
            "root-issued session receipt is expired at trusted current time",
        )
    if (
        row["single_use"] is not True
        or row["usage_count"] != 0
        or row["consumed_by_context_id"] is not None
    ):
        raise EnvelopeError(
            "external_effect_session_receipt_reuse_rejected",
            "root-issued session receipt is reusable or already consumed",
        )
    expected_resolved = {
        **row,
        "source_path": source_path,
        "source_sha256": source_sha256,
        "source_identity": source_identity,
        "row_sha256": digest_bytes(canonical(row)),
    }
    if receipt_binding["resolved_receipt"] != expected_resolved:
        raise EnvelopeError(
            "external_effect_session_receipt_binding_mismatch",
            "resolved session receipt row does not match protected authority",
        )
    if receipt_binding["resolution"]["resolved_set_sha256"] != digest_bytes(
        canonical(expected_resolved)
    ):
        raise EnvelopeError(
            "external_effect_session_receipt_binding_mismatch",
            "session receipt resolved-set digest mismatch",
        )
    return trusted_now, expected_resolved


def _validate_external_effect_authority_core(
    root: Path,
    scope: dict[str, Any],
    authority: dict[str, Any],
    *,
    repository_handle: dict[str, Any],
    candidate: dict[str, Any] | None = None,
    trusted_now: datetime | None = None,
    final_revalidation: bool = False,
) -> dict[str, Any] | None:
    revalidate_canonical_repository(repository_handle)
    if root != repository_handle["root"]:
        raise EnvelopeError(
            "canonical_repository_object_mismatch",
            "external-effect validation root differs from its opened repository handle",
        )
    repository_identity = repository_handle["repository_identity"]
    if candidate is not None:
        require_candidate_repository_identity(candidate, repository_identity)
    binding = authority["external_effect_authority"]
    scoped_effects = scope["external_effects"]
    effect_ids = [effect["effect_id"] for effect in scoped_effects]
    approval_ids = [effect["approval_id"] for effect in scoped_effects]
    if len(effect_ids) != len(set(effect_ids)):
        raise EnvelopeError(
            "external_effect_duplicate_id",
            "external-effect scope repeats one effect ID",
        )
    if len(approval_ids) != len(set(approval_ids)):
        raise EnvelopeError(
            "external_effect_approval_reuse_rejected",
            "one approval cannot authorize multiple scoped effects",
        )

    try:
        loaded = load_external_effect_authority_sources(root, binding)
    except EnvelopeError as error:
        if final_revalidation:
            raise EnvelopeError(
                "external_effect_authority_final_revalidation_failed",
                str(error),
            ) from error
        raise

    effect_matches: dict[
        str, list[tuple[str, str, dict[str, int], dict[str, Any]]]
    ] = {}
    approval_matches: dict[
        str, list[tuple[str, str, dict[str, int], dict[str, Any]]]
    ] = {}
    effect_required = {
        "effect_id",
        "kind",
        "provider",
        "action",
        "target_sha256",
        "status",
    }
    approval_required = {
        "approval_id",
        "effect_id",
        "status",
        "valid_from",
        "expires_at",
        "single_use",
        "usage_count",
        "consumed_by_context_id",
    }
    session_time = parse_timestamp(
        authority["captured_at"], "external_effect_authority_session_invalid"
    )
    for source_path, source_sha256, source_identity, document in loaded:
        if parse_timestamp(
            document["captured_at"], "external_effect_authority_source_malformed"
        ) > session_time:
            raise EnvelopeError(
                "external_effect_authority_not_current",
                f"authority source is newer than the bound session: {source_path}",
            )
        for row in document["effects"]:
            validate_external_effect_source_row(row, effect_required, source_path)
            effect_matches.setdefault(row.get("effect_id"), []).append(
                (source_path, source_sha256, source_identity, row)
            )
        for row in document["approvals"]:
            validate_external_effect_source_row(row, approval_required, source_path)
            approval_matches.setdefault(row.get("approval_id"), []).append(
                (source_path, source_sha256, source_identity, row)
            )

    for stable_id, matches in [*effect_matches.items(), *approval_matches.items()]:
        if len(matches) > 1:
            source_count = len({source_path for source_path, _, _, _ in matches})
            raise EnvelopeError(
                "external_effect_authority_ambiguous_id"
                if source_count > 1
                else "external_effect_authority_duplicate_id",
                f"external-effect authority ID did not resolve exactly once: {stable_id}",
            )

    scoped_effect_ids = set(effect_ids)
    scoped_approval_ids = set(approval_ids)
    authority_effect_ids = set(effect_matches)
    authority_approval_ids = set(approval_matches)
    missing_effects = scoped_effect_ids - authority_effect_ids
    missing_approvals = scoped_approval_ids - authority_approval_ids
    if missing_effects or missing_approvals:
        raise EnvelopeError(
            "external_effect_authority_resolution_required",
            "scoped external effect or approval does not resolve in current root authority: "
            + ",".join(sorted(missing_effects | missing_approvals)),
        )
    extra_effects = authority_effect_ids - scoped_effect_ids
    extra_approvals = authority_approval_ids - scoped_approval_ids
    if extra_effects or extra_approvals:
        raise EnvelopeError(
            "external_effect_authority_unreferenced_row",
            "authority source contains unreferenced external-effect rows: "
            + ",".join(sorted(extra_effects | extra_approvals)),
        )

    trusted_current, resolved_receipt = validate_external_effect_session_receipt(
        root,
        binding,
        authority,
        candidate,
        trusted_now,
        bool(scoped_effects),
    )

    expected_effect_rows: list[dict[str, Any]] = []
    expected_approval_rows: list[dict[str, Any]] = []
    for scoped in scoped_effects:
        (
            effect_source_path,
            effect_source_sha256,
            effect_source_identity,
            effect_row,
        ) = effect_matches[scoped["effect_id"]][0]
        if effect_row["status"] != "current":
            raise EnvelopeError(
                "external_effect_authority_not_current",
                f"external effect is stale or revoked: {scoped['effect_id']}",
            )
        if external_effect_identity(effect_row) != external_effect_identity(scoped):
            raise EnvelopeError(
                "external_effect_identity_mismatch",
                f"provider/action/target/effect identity mismatch: {scoped['effect_id']}",
            )
        (
            approval_source_path,
            approval_source_sha256,
            approval_source_identity,
            approval_row,
        ) = approval_matches[scoped["approval_id"]][0]
        if approval_row["effect_id"] != scoped["effect_id"]:
            raise EnvelopeError(
                "external_effect_approval_link_mismatch",
                f"approval does not authorize the scoped effect: {scoped['approval_id']}",
            )
        if approval_row["status"] != "approved":
            raise EnvelopeError(
                "external_effect_approval_not_current",
                f"approval is not approved: {scoped['approval_id']}",
            )
        valid_from = parse_timestamp(
            approval_row["valid_from"], "external_effect_approval_not_current"
        )
        expires_at = parse_timestamp(
            approval_row["expires_at"], "external_effect_approval_not_current"
        )
        if trusted_current is None or not valid_from <= trusted_current < expires_at:
            raise EnvelopeError(
                "external_effect_approval_not_current",
                f"approval is not current at trusted mediation time: {scoped['approval_id']}",
            )
        if (
            approval_row["single_use"] is not True
            or approval_row["usage_count"] != 0
            or approval_row["consumed_by_context_id"] is not None
        ):
            raise EnvelopeError(
                "external_effect_approval_reuse_rejected",
                f"approval is reusable or already consumed: {scoped['approval_id']}",
            )
        effect_identity = external_effect_identity(effect_row)
        expected_effect_rows.append(
            {
                **effect_identity,
                "effect_sha256": digest_bytes(canonical(effect_identity)),
                "source_path": effect_source_path,
                "source_sha256": effect_source_sha256,
                "source_identity": effect_source_identity,
                "row_sha256": digest_bytes(canonical(effect_row)),
            }
        )
        expected_approval_rows.append(
            {
                "approval_id": approval_row["approval_id"],
                "effect_id": approval_row["effect_id"],
                "status": approval_row["status"],
                "valid_from": approval_row["valid_from"],
                "expires_at": approval_row["expires_at"],
                "source_path": approval_source_path,
                "source_sha256": approval_source_sha256,
                "source_identity": approval_source_identity,
                "row_sha256": digest_bytes(canonical(approval_row)),
            }
        )

    expected_effect_rows.sort(key=lambda row: row["effect_id"].encode())
    expected_approval_rows.sort(key=lambda row: row["approval_id"].encode())
    if (
        binding["effect_rows"] != expected_effect_rows
        or binding["approval_rows"] != expected_approval_rows
    ):
        raise EnvelopeError(
            "external_effect_authority_binding_mismatch",
            "resolved external-effect or approval rows do not match current authority",
        )
    expected_set_digest = external_effect_resolution_digest(
        expected_effect_rows, expected_approval_rows, binding["session_receipt"]
    )
    if binding["resolution"]["resolved_set_sha256"] != expected_set_digest:
        raise EnvelopeError(
            "external_effect_authority_binding_mismatch",
            "external-effect resolved-set digest mismatch",
        )
    if binding["session_id"] != external_effect_session_digest(binding):
        raise EnvelopeError(
            "external_effect_authority_binding_mismatch",
            "external-effect authority session digest mismatch",
        )
    if not scoped_effects:
        return None
    if candidate is None or trusted_current is None or resolved_receipt is None:
        raise EnvelopeError(
            "external_effect_trusted_time_unavailable",
            "nonempty external-effect validation lacks trusted candidate/time/receipt state",
        )
    return _external_effect_validation_binding(
        binding,
        authority,
        candidate,
        resolved_receipt,
        repository_identity,
        repository_handle["object_identity"],
    )


def validate_external_effect_authority(
    root: Path,
    scope: dict[str, Any],
    authority: dict[str, Any],
    *,
    candidate: dict[str, Any] | None = None,
) -> None:
    if scope["external_effects"]:
        raise EnvelopeError(
            "external_effect_mediator_required",
            "nonempty external effects require mediator.initial followed by mediator.final",
        )
    repository_handle = open_canonical_repository(root)
    try:
        _validate_external_effect_authority_core(
            repository_handle["root"],
            scope,
            authority,
            repository_handle=repository_handle,
            candidate=candidate,
            trusted_now=None,
            final_revalidation=False,
        )
    finally:
        close_canonical_repository(repository_handle)


def validate_proposed_write_identity_binding(root: Path, scope: dict[str, Any]) -> None:
    identity_rows = proposed_write_identity_rows(root, scope)
    identity_digest = digest_bytes(canonical(identity_rows))
    resolution = scope["path_resolution"]
    if resolution["proposed_write_object_set_sha256"] != identity_digest:
        raise EnvelopeError(
            "proposed_write_object_identity_mismatch",
            "proposed write object identity set digest mismatch",
        )
    if resolution["proposed_write_object_count"] != len(identity_rows):
        raise EnvelopeError(
            "proposed_write_object_identity_mismatch",
            "proposed write object identity set count mismatch",
        )


def validate_scope_authority(
    root: Path, scope: dict[str, Any], write_scope: dict[str, Any]
) -> None:
    granted_paths = [
        grant["path"] for grant in scope["artifact_paths"] + scope["fixtures"]
    ]
    granted_paths.extend(scope["generated_outputs"])
    folded_grants = [path_parts(path) for path in granted_paths]
    if len(set(folded_grants)) != len(folded_grants):
        raise EnvelopeError(
            "duplicate_scope_path_rejected",
            "scope repeats one normalized candidate path",
        )

    for granted in granted_paths:
        for forbidden in scope["forbidden_paths"]:
            if paths_overlap(granted, forbidden):
                raise EnvelopeError(
                    "forbidden_scope_path_overlap",
                    f"scope path overlaps forbidden path: {granted} <> {forbidden}",
                )

    delegated_paths = write_scope.get("exclusive_paths")
    delegated_outputs = write_scope.get("generated_outputs")
    if (
        not isinstance(delegated_paths, list)
        or not delegated_paths
        or not all(isinstance(path, str) and path for path in delegated_paths)
        or not isinstance(delegated_outputs, list)
        or not all(isinstance(path, str) and path for path in delegated_outputs)
    ):
        raise EnvelopeError(
            "write_scope_binding_mismatch",
            "write scope lacks exact delegated path sets",
        )

    def require_delegated(write_path: str, delegated_roots: list[str]) -> None:
        delegated = any(
            path_is_within_exact(write_path, delegated_root)
            for delegated_root in delegated_roots
        )
        if paths_overlap(write_path, SHARED_SCHEMA_ROOT) and not delegated:
            raise EnvelopeError(
                "shared_root_authority_write_forbidden",
                f"shared schema authority was not delegated: {write_path}",
            )
        if write_path.split("/")[-1].casefold() in ROOT_OWNED_LEAF_NAMES_CASEFOLDED:
            raise EnvelopeError(
                "shared_root_authority_write_forbidden",
                f"root-owned leaf path cannot be delegated: {write_path}",
            )
        if not delegated:
            raise EnvelopeError(
                "write_scope_path_not_delegated",
                f"write path is outside the bound canonical write scope: {write_path}",
            )

    for write_path in scope_proposed_write_paths(scope):
        reject_protected_write_path(write_path)

    for grant in scope["artifact_paths"] + scope["fixtures"]:
        if grant["access"] == "propose_write":
            require_delegated(grant["path"], delegated_paths)
    for write_path in scope["generated_outputs"]:
        require_delegated(write_path, delegated_outputs)

    validate_proposed_write_identity_binding(root, scope)


def reject_protected_write_path(write_path: str) -> None:
    for code, protected_prefixes in PROTECTED_WRITE_PREFIXES.items():
        for protected in protected_prefixes:
            if paths_overlap(write_path, protected):
                raise EnvelopeError(
                    code,
                    f"write path overlaps protected authority: {write_path} <> {protected}",
                )


def validate_output_target(
    scope: dict[str, Any], effects: dict[str, Any], output: dict[str, Any]
) -> None:
    target = output["result_target"]
    if target == "message://final":
        return
    if target.startswith("scratch://"):
        if target not in scope["scratch_uris"] or effects["scratch"] != "bounded_read_write":
            raise EnvelopeError(
                "output_target_not_delegated",
                "scratch result target lacks an exact bounded scratch grant",
            )
        return
    reject_protected_write_path(target)
    if target not in scope["generated_outputs"]:
        raise EnvelopeError(
            "output_target_not_delegated",
            "repository result target is not an exact delegated generated output",
        )


def validate_runtime_role_receipt(role: dict[str, Any], role_path: Path) -> None:
    if not role["source_ref"].startswith(RUNTIME_RECEIPT_ROOT + "/"):
        raise EnvelopeError(
            "runtime_role_receipt_source_required",
            "runtime role binding must use the root-owned runtime-capability receipt namespace",
        )
    try:
        receipt = json.loads(role_path.read_bytes())
    except json.JSONDecodeError as error:
        raise EnvelopeError(
            "runtime_role_receipt_malformed",
            "runtime role receipt is not valid JSON",
        ) from error
    required = {
        "schema_version",
        "captured_at",
        "source",
        "role_id",
        "runtime_agent_type",
        "sandbox_mode",
        "may_exercise_root_authority",
        "model_metadata",
        "capability_ceiling",
    }
    if not isinstance(receipt, dict) or set(receipt) != required:
        raise EnvelopeError(
            "runtime_role_receipt_shape_mismatch",
            "runtime role receipt has missing or unknown fields",
        )
    if receipt["schema_version"] != "RuntimeAgentTypeReceipt-v1":
        raise EnvelopeError(
            "runtime_role_receipt_shape_mismatch",
            "runtime role receipt schema version is not supported",
        )
    expected_agent_type = RUNTIME_AGENT_TYPES.get(role["role_id"])
    if (
        receipt["role_id"] != role["role_id"]
        or receipt["sandbox_mode"] != role["sandbox_mode"]
        or receipt["runtime_agent_type"] != expected_agent_type
        or receipt["may_exercise_root_authority"] is not False
        or role["may_exercise_root_authority"] is not False
    ):
        raise EnvelopeError(
            "runtime_role_receipt_identity_mismatch",
            "runtime receipt does not prove the bound role, agent type, sandbox, and root-authority ceiling",
        )
    for key in ("captured_at", "source", "capability_ceiling"):
        if not isinstance(receipt[key], str) or not receipt[key].strip():
            raise EnvelopeError(
                "runtime_role_receipt_shape_mismatch",
                f"runtime role receipt field is empty: {key}",
            )


def validate_live_git_identity(root: Path, candidate: dict[str, Any]) -> None:
    if candidate["branch"] != git(root, "branch", "--show-current"):
        raise EnvelopeError("stale_candidate_identity", "branch mismatch")
    if candidate["head_commit"] != git(root, "rev-parse", "HEAD"):
        raise EnvelopeError("stale_candidate_identity", "HEAD mismatch")
    if candidate["head_tree"] != git(root, "rev-parse", "HEAD^{tree}"):
        raise EnvelopeError("stale_candidate_identity", "tree mismatch")
    actual_dirty = bool(git(root, "status", "--porcelain", "--untracked-files=all"))
    if candidate["dirty"] != actual_dirty:
        raise EnvelopeError("stale_candidate_identity", "dirty-state mismatch")


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
            raise EnvelopeError(
                "project_role_manifest_identity_mismatch",
                "role ID does not resolve to the bound manifest",
            )
        if parsed.get("sandbox_mode") != role["sandbox_mode"]:
            raise EnvelopeError(
                "project_role_manifest_identity_mismatch",
                "role sandbox does not resolve to the bound manifest",
            )
    else:
        validate_runtime_role_receipt(role, role_path)

    candidate = envelope["candidate_identity"]
    canonical_root, repository_identity = resolve_canonical_repository(root)
    require_candidate_repository_identity(candidate, repository_identity)
    root = canonical_root
    validate_live_git_identity(root, candidate)
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
                raise EnvelopeError(
                    "same_session_stable_id_resolution_required",
                    f"stable ID did not resolve exactly once: {stable_id}",
                )
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
    write_scope = resolve_write_scope(scope, loaded_sources, authority)
    validate_scope_authority(root, scope, write_scope)
    validate_external_effect_authority(
        root, scope, authority, candidate=candidate
    )
    for grant in scope["artifact_paths"] + scope["fixtures"]:
        verify_scope_path(root, grant["path"], grant["access"] == "propose_write")
    for relative in scope["generated_outputs"] + scope["forbidden_paths"]:
        verify_scope_path(root, relative, True)
    resolved_scope = digest_bytes(canonical(scope_core(scope)))
    if scope["path_resolution"]["resolved_scope_sha256"] != resolved_scope:
        raise EnvelopeError("resolved scope digest mismatch")
    scope_session = digest_bytes(
        canonical(
            {
                "candidate_id": candidate["candidate_id"],
                "scope": scope_core(scope),
                "path_resolution": scope_resolution_core(scope),
            }
        )
    )
    if scope["path_resolution"]["session_id"] != scope_session:
        raise EnvelopeError("scope session ID mismatch")

    for reference in envelope["evidence"]["primary_references"]:
        verify_bound_file(root, reference["path"], reference["sha256"])

    output = envelope["output_contract"]
    output_schema = verify_bound_file(root, output["schema_ref"], output["schema_sha256"])
    parsed_output_schema = json.loads(output_schema.read_bytes())
    if parsed_output_schema.get("title") != output["schema_id"]:
        raise EnvelopeError(
            "output_schema_must_resolve_to_bound_authority",
            "output schema ID does not resolve to the bound schema",
        )
    validate_output_target(scope, envelope["effects"], output)

    expected_context_id = digest_bytes(canonical(context_commitment(envelope)))
    if candidate["context_id"] != expected_context_id:
        raise EnvelopeError(
            "context_identity_mismatch",
            "context ID does not commit the complete envelope",
        )

    artifact_set(root, freeze["artifacts"])
    reloaded_sources: dict[str, Any] = {}
    for source in authority["sources"]:
        source_path = verify_bound_file(root, source["path"], source["sha256"])
        reloaded_sources[source["path"]] = json.loads(source_path.read_bytes())
    verify_bound_file(root, role["source_ref"], role["source_sha256"])
    verify_bound_file(root, output["schema_ref"], output["schema_sha256"])
    write_scope = resolve_write_scope(scope, reloaded_sources, authority)
    validate_scope_authority(root, scope, write_scope)
    validate_external_effect_authority(
        root,
        scope,
        authority,
        candidate=candidate,
    )
    for grant in scope["artifact_paths"] + scope["fixtures"]:
        verify_scope_path(root, grant["path"], grant["access"] == "propose_write")
    for relative in scope["generated_outputs"] + scope["forbidden_paths"]:
        verify_scope_path(root, relative, True)
    validate_live_git_identity(root, candidate)


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


def refresh_authority_binding(root: Path, envelope: dict[str, Any]) -> None:
    authority = envelope["authority_binding"]
    loaded_sources: dict[str, Any] = {}
    for source in authority["sources"]:
        source_path = verify_bound_file(root, source["path"], source["sha256"])
        loaded_sources[source["path"]] = json.loads(source_path.read_bytes())
    resolved_rows: list[dict[str, str]] = []
    for category, key in ID_KEYS.items():
        for stable_id in authority["ids"][category]:
            matches: list[tuple[str, dict[str, Any]]] = []
            for source_path, document in loaded_sources.items():
                rows: list[dict[str, Any]] = []
                collect_rows(document, key, rows)
                matches.extend((source_path, row) for row in rows if row.get(key) == stable_id)
            if len(matches) != 1:
                raise EnvelopeError(
                    "same_session_stable_id_resolution_required",
                    f"stable ID did not resolve exactly once: {stable_id}",
                )
            source_path, row = matches[0]
            resolved_rows.append(
                {
                    "stable_id": stable_id,
                    "source_path": source_path,
                    "row_sha256": digest_bytes(canonical(row)),
                }
            )
    resolved_rows.sort(key=lambda row: row["stable_id"].encode())
    authority["resolved_rows"] = resolved_rows
    authority["resolution"]["resolved_set_sha256"] = digest_bytes(
        canonical(resolved_rows)
    )
    authority["session_id"] = digest_bytes(
        canonical({"sources": authority["sources"], "resolved_rows": resolved_rows})
    )
    refresh_external_effect_authority_binding(root, envelope["scope"], authority)


def refresh_external_effect_authority_binding(
    root: Path, scope: dict[str, Any], authority: dict[str, Any]
) -> None:
    binding = authority["external_effect_authority"]
    loaded = load_external_effect_authority_sources(root, binding)
    effects: dict[str, tuple[str, str, dict[str, int], dict[str, Any]]] = {}
    approvals: dict[str, tuple[str, str, dict[str, int], dict[str, Any]]] = {}
    for source_path, source_sha256, source_identity, document in loaded:
        for row in document["effects"]:
            if isinstance(row, dict) and isinstance(row.get("effect_id"), str):
                effects.setdefault(
                    row["effect_id"],
                    (source_path, source_sha256, source_identity, row),
                )
        for row in document["approvals"]:
            if isinstance(row, dict) and isinstance(row.get("approval_id"), str):
                approvals.setdefault(
                    row["approval_id"],
                    (source_path, source_sha256, source_identity, row),
                )
    resolved_effects: list[dict[str, Any]] = []
    resolved_approvals: list[dict[str, Any]] = []
    for scoped in scope["external_effects"]:
        if scoped["effect_id"] not in effects or scoped["approval_id"] not in approvals:
            continue
        (
            effect_source_path,
            effect_source_sha256,
            effect_source_identity,
            effect_row,
        ) = effects[scoped["effect_id"]]
        (
            approval_source_path,
            approval_source_sha256,
            approval_source_identity,
            approval_row,
        ) = approvals[scoped["approval_id"]]
        identity = external_effect_identity(effect_row)
        resolved_effects.append(
            {
                **identity,
                "effect_sha256": digest_bytes(canonical(identity)),
                "source_path": effect_source_path,
                "source_sha256": effect_source_sha256,
                "source_identity": effect_source_identity,
                "row_sha256": digest_bytes(canonical(effect_row)),
            }
        )
        resolved_approvals.append(
            {
                "approval_id": approval_row["approval_id"],
                "effect_id": approval_row["effect_id"],
                "status": approval_row["status"],
                "valid_from": approval_row["valid_from"],
                "expires_at": approval_row["expires_at"],
                "source_path": approval_source_path,
                "source_sha256": approval_source_sha256,
                "source_identity": approval_source_identity,
                "row_sha256": digest_bytes(canonical(approval_row)),
            }
        )
    resolved_effects.sort(key=lambda row: row["effect_id"].encode())
    resolved_approvals.sort(key=lambda row: row["approval_id"].encode())
    binding["effect_rows"] = resolved_effects
    binding["approval_rows"] = resolved_approvals
    binding["resolution"]["resolved_set_sha256"] = external_effect_resolution_digest(
        resolved_effects, resolved_approvals, binding["session_receipt"]
    )
    binding["session_id"] = external_effect_session_digest(binding)


def refresh_derived_bindings(root: Path, envelope: dict[str, Any]) -> None:
    candidate = envelope["candidate_identity"]
    candidate["candidate_id"] = digest_bytes(canonical(candidate_core(candidate)))
    scope = envelope["scope"]
    scope["path_resolution"]["resolved_scope_sha256"] = digest_bytes(
        canonical(scope_core(scope))
    )
    identity_rows = proposed_write_identity_rows(root, scope)
    scope["path_resolution"]["proposed_write_object_set_sha256"] = digest_bytes(
        canonical(identity_rows)
    )
    scope["path_resolution"]["proposed_write_object_count"] = len(identity_rows)
    scope["path_resolution"]["session_id"] = digest_bytes(
        canonical(
            {
                "candidate_id": candidate["candidate_id"],
                "scope": scope_core(scope),
                "path_resolution": scope_resolution_core(scope),
            }
        )
    )
    candidate["context_id"] = digest_bytes(canonical(context_commitment(envelope)))


def synthetic_external_effect_scope() -> dict[str, Any]:
    return {
        "external_effects": [
            {
                "effect_id": "EFFECT-SYNTHETIC-ISSUE-COMMENT",
                "kind": "external_write",
                "provider": "github",
                "action": "create-issue-comment",
                "target_sha256": digest_bytes(b"synthetic-repository-issue-17"),
                "approval_id": "APPROVAL-SYNTHETIC-ISSUE-COMMENT",
            }
        ]
    }


def synthetic_external_effect_document() -> dict[str, Any]:
    scoped = synthetic_external_effect_scope()["external_effects"][0]
    return {
        "schema_version": EXTERNAL_EFFECT_AUTHORITY_SCHEMA,
        "authority_owner": "OWN-ULTRA-ROOT",
        "source_kind": "adopted-current-external-effect-authority",
        "captured_at": "2026-07-13T00:15:00Z",
        "effects": [
            {
                **external_effect_identity(scoped),
                "status": "current",
            }
        ],
        "approvals": [
            {
                "approval_id": scoped["approval_id"],
                "effect_id": scoped["effect_id"],
                "status": "approved",
                "valid_from": "2026-07-13T00:00:00Z",
                "expires_at": "2026-07-13T01:00:00Z",
                "single_use": True,
                "usage_count": 0,
                "consumed_by_context_id": None,
            }
        ],
    }


def write_synthetic_authority_source(
    root: Path, relative: str, document: dict[str, Any]
) -> str:
    path = root / relative
    path.parent.mkdir(parents=True, exist_ok=True)
    data = canonical(document) + b"\n"
    path.write_bytes(data)
    return digest_bytes(data)


def prepare_synthetic_external_effect_authority(
    root: Path, candidate: dict[str, Any], authority_session_id: str
) -> tuple[dict[str, Any], dict[str, Any], str, dict[str, Any]]:
    scope = synthetic_external_effect_scope()
    document = synthetic_external_effect_document()
    source_path = EXTERNAL_EFFECT_AUTHORITY_ROOT + "/CURRENT-SYNTHETIC.json"
    source_sha256 = write_synthetic_authority_source(root, source_path, document)
    _, source_identity = read_external_effect_authority_source(root, source_path)
    binding = {
        "namespace": EXTERNAL_EFFECT_AUTHORITY_ROOT,
        "session_id": digest_bytes(b"pending-external-effect-session"),
        "sources": [
            {
                "path": source_path,
                "sha256": source_sha256,
                "identity": source_identity,
            }
        ],
        "effect_rows": [],
        "approval_rows": [],
        "session_receipt": None,
        "resolution": {
            "resolved_set_sha256": digest_bytes(b"pending-external-effect-set"),
            "unknown_count": 0,
            "duplicate_count": 0,
            "ambiguous_count": 0,
            "conflict_count": 0,
            "unreferenced_count": 0,
            "final_session_revalidation": "required",
        },
    }
    authority = {
        "session_id": authority_session_id,
        "captured_at": "2026-07-13T00:30:00Z",
        "external_effect_authority": binding,
    }
    receipt_core = {
        "receipt_id": "EFFECT-SESSION-SYNTHETIC-ISSUE-COMMENT",
        "nonce": digest_bytes(b"synthetic-effect-session-nonce"),
        "issuer": "OWN-ULTRA-ROOT",
        "issued_at": "2026-07-13T00:29:58Z",
        "expires_at": "2026-07-13T00:45:00Z",
        "permitted_skew_seconds": 5,
        "repository_root_sha256": candidate["repository"]["canonical_root_sha256"],
        "candidate_id": candidate["candidate_id"],
        "authority_session_id": authority_session_id,
        "effect_authority_sources_sha256": effect_authority_source_set_digest(binding),
        "single_use": True,
        "usage_count": 0,
        "consumed_by_context_id": None,
    }
    receipt_row = {
        **receipt_core,
        "session_id": digest_bytes(canonical(receipt_core)),
    }
    receipt_document = {
        "schema_version": EXTERNAL_EFFECT_SESSION_RECEIPT_SCHEMA,
        "authority_owner": "OWN-ULTRA-ROOT",
        "source_kind": "current-external-effect-session-receipt",
        "receipts": [receipt_row],
    }
    receipt_path = EXTERNAL_EFFECT_SESSION_RECEIPT_ROOT + "/CURRENT-SYNTHETIC.json"
    receipt_sha256 = write_synthetic_authority_source(
        root, receipt_path, receipt_document
    )
    _, receipt_identity = read_external_effect_authority_source(root, receipt_path)
    resolved_receipt = {
        **receipt_row,
        "source_path": receipt_path,
        "source_sha256": receipt_sha256,
        "source_identity": receipt_identity,
        "row_sha256": digest_bytes(canonical(receipt_row)),
    }
    binding["session_receipt"] = {
        "namespace": EXTERNAL_EFFECT_SESSION_RECEIPT_ROOT,
        "receipt_id": receipt_row["receipt_id"],
        "sources": [
            {
                "path": receipt_path,
                "sha256": receipt_sha256,
                "identity": receipt_identity,
            }
        ],
        "resolved_receipt": resolved_receipt,
        "resolution": {
            "resolved_set_sha256": digest_bytes(canonical(resolved_receipt)),
            "unknown_count": 0,
            "duplicate_count": 0,
            "ambiguous_count": 0,
            "conflict_count": 0,
            "unreferenced_count": 0,
            "final_session_revalidation": "required",
        },
    }
    refresh_external_effect_authority_binding(root, scope, authority)
    return scope, authority, source_path, document


def refresh_synthetic_session_receipt_source(
    root: Path,
    binding: dict[str, Any],
    document: dict[str, Any] | None = None,
    *,
    refresh_effect_sources: bool = True,
    recompute_session_id: bool = True,
) -> dict[str, Any]:
    receipt_binding = binding["session_receipt"]
    source = receipt_binding["sources"][0]
    if document is None:
        document = json.loads((root / source["path"]).read_bytes())
    row = document["receipts"][0]
    if refresh_effect_sources:
        row["effect_authority_sources_sha256"] = effect_authority_source_set_digest(
            binding
        )
    if recompute_session_id:
        row["session_id"] = digest_bytes(
            canonical({key: value for key, value in row.items() if key != "session_id"})
        )
    source["sha256"] = write_synthetic_authority_source(
        root, source["path"], document
    )
    _, source["identity"] = read_external_effect_authority_source(
        root, source["path"]
    )
    resolved = {
        **row,
        "source_path": source["path"],
        "source_sha256": source["sha256"],
        "source_identity": source["identity"],
        "row_sha256": digest_bytes(canonical(row)),
    }
    receipt_binding["resolved_receipt"] = resolved
    receipt_binding["resolution"]["resolved_set_sha256"] = digest_bytes(
        canonical(resolved)
    )
    return document


def run_external_effect_positive(
    validator: jsonschema.Draft202012Validator, baseline: dict[str, Any]
) -> None:
    with tempfile.TemporaryDirectory(prefix="hul-effect-positive-") as temporary:
        root = Path(temporary).resolve(strict=True)
        probe = copy.deepcopy(baseline)
        _, repository_identity = resolve_canonical_repository(root)
        probe["candidate_identity"]["repository"] = repository_identity
        scope, authority, _, _ = prepare_synthetic_external_effect_authority(
            root,
            probe["candidate_identity"],
            probe["authority_binding"]["session_id"],
        )
        probe["scope"]["external_effects"] = copy.deepcopy(scope["external_effects"])
        probe["authority_binding"]["captured_at"] = authority["captured_at"]
        probe["authority_binding"]["external_effect_authority"] = copy.deepcopy(
            authority["external_effect_authority"]
        )
        probe["effects"]["external_systems"] = "approved_named_only"
        schema_errors = list(validator.iter_errors(probe))
        if schema_errors:
            raise EnvelopeError(
                "external_effect_positive_schema_failure",
                schema_errors[0].message,
            )
        mediator = ExternalEffectMediator()
        trusted_now = parse_timestamp(
            "2026-07-13T00:30:00Z", "synthetic_trusted_time_invalid"
        )
        token = mediator.initial(
            root,
            probe["scope"],
            probe["authority_binding"],
            candidate=probe["candidate_identity"],
            trusted_now=trusted_now,
        )
        mediator.final(
            root,
            probe["scope"],
            probe["authority_binding"],
            token,
            candidate=probe["candidate_identity"],
            trusted_now=trusted_now,
        )


def run_external_effect_red_case(case: dict[str, Any]) -> str:
    try:
        with tempfile.TemporaryDirectory(prefix="hul-effect-red-") as temporary:
            container = Path(temporary).resolve(strict=True)
            root = container / "repository"
            root.mkdir()
            repository_probe = open_canonical_repository(root)
            try:
                repository_identity = copy.deepcopy(
                    repository_probe["repository_identity"]
                )
                repository_object_identity = copy.deepcopy(
                    repository_probe["object_identity"]
                )
            finally:
                close_canonical_repository(repository_probe)
            candidate = {
                "repository": repository_identity,
                "candidate_id": digest_bytes(b"synthetic-external-effect-candidate"),
            }
            scope, authority, source_path, document = (
                prepare_synthetic_external_effect_authority(
                    root,
                    candidate,
                    digest_bytes(b"synthetic-main-authority-session"),
                )
            )
            binding = authority["external_effect_authority"]
            trusted_initial = parse_timestamp(
                "2026-07-13T00:30:00Z", "synthetic_trusted_time_invalid"
            )
            mediator = ExternalEffectMediator()

            def validate_probe(
                *,
                final_revalidation: bool = False,
                trusted_now: datetime = trusted_initial,
                mediation_token: object | None = None,
                repository_root: Path | None = None,
                candidate_identity: dict[str, Any] | None = None,
            ) -> object | None:
                current_root = root if repository_root is None else repository_root
                current_candidate = (
                    candidate
                    if candidate_identity is None
                    else candidate_identity
                )
                if final_revalidation:
                    mediator.final(
                        current_root,
                        scope,
                        authority,
                        mediation_token,
                        candidate=current_candidate,
                        trusted_now=trusted_now,
                    )
                    return None
                return mediator.initial(
                    current_root,
                    scope,
                    authority,
                    candidate=current_candidate,
                    trusted_now=trusted_now,
                )

            def rebind_effect_source_document() -> None:
                binding["sources"][0]["sha256"] = write_synthetic_authority_source(
                    root, source_path, document
                )
                _, binding["sources"][0]["identity"] = (
                    read_external_effect_authority_source(root, source_path)
                )
                refresh_synthetic_session_receipt_source(root, binding)

            def replace_single_authorization(
                effect_id: str, approval_id: str, target_seed: bytes
            ) -> None:
                scoped = scope["external_effects"][0]
                effect = document["effects"][0]
                approval = document["approvals"][0]
                scoped["effect_id"] = effect_id
                scoped["approval_id"] = approval_id
                scoped["target_sha256"] = digest_bytes(target_seed)
                effect["effect_id"] = effect_id
                effect["target_sha256"] = scoped["target_sha256"]
                approval["approval_id"] = approval_id
                approval["effect_id"] = effect_id
                rebind_effect_source_document()

            def reissue_receipt(
                receipt_id: str,
                nonce_seed: bytes,
                row_updates: dict[str, Any] | None = None,
            ) -> None:
                receipt_binding = binding["session_receipt"]
                receipt_source = receipt_binding["sources"][0]
                receipt_document = json.loads(
                    (root / receipt_source["path"]).read_bytes()
                )
                row = receipt_document["receipts"][0]
                receipt_binding["receipt_id"] = receipt_id
                row["receipt_id"] = receipt_id
                row["nonce"] = digest_bytes(nonce_seed)
                if row_updates:
                    row.update(row_updates)
                refresh_synthetic_session_receipt_source(
                    root, binding, receipt_document
                )

            def replace_repository_root_same_path() -> Path:
                moved_root = container / "retained-original-repository"
                root.rename(moved_root)
                root.mkdir()
                (moved_root / "docs").rename(root / "docs")
                return moved_root

            def restore_repository_root_same_path(moved_root: Path) -> None:
                (root / "docs").rename(moved_root / "docs")
                root.rmdir()
                moved_root.rename(root)

            setup = case["setup"]
            refresh_source_digest = False
            if setup == "same-root-caller-digest-substitution":
                mediation_token = validate_probe()
                assert mediation_token is not None
                validate_probe(
                    final_revalidation=True,
                    mediation_token=mediation_token,
                )
                candidate["repository"]["canonical_root_sha256"] = digest_bytes(
                    b"caller-substituted-same-root"
                )
                receipt_binding = binding["session_receipt"]
                receipt_source = receipt_binding["sources"][0]
                receipt_document = json.loads(
                    (root / receipt_source["path"]).read_bytes()
                )
                receipt_document["receipts"][0][
                    "repository_root_sha256"
                ] = candidate["repository"]["canonical_root_sha256"]
                refresh_synthetic_session_receipt_source(
                    root, binding, receipt_document
                )
                refresh_external_effect_authority_binding(root, scope, authority)
                fresh_mediator = ExternalEffectMediator()
                try:
                    fresh_mediator.initial(
                        root,
                        scope,
                        authority,
                        candidate=candidate,
                        trusted_now=trusted_initial,
                    )
                except EnvelopeError as error:
                    return error.code
                raise EnvelopeError(
                    "external_effect_red_fixture_unexpectedly_accepted",
                    case["case_id"],
                )
            elif setup == "same-root-caller-uri-substitution":
                mediation_token = validate_probe()
                assert mediation_token is not None
                validate_probe(
                    final_revalidation=True,
                    mediation_token=mediation_token,
                )
                candidate["repository"]["root_uri"] = "repo://caller-substitution"
                fresh_mediator = ExternalEffectMediator()
                try:
                    fresh_mediator.initial(
                        root,
                        scope,
                        authority,
                        candidate=candidate,
                        trusted_now=trusted_initial,
                    )
                except EnvelopeError as error:
                    return error.code
                raise EnvelopeError(
                    "external_effect_red_fixture_unexpectedly_accepted",
                    case["case_id"],
                )
            elif setup == "nonexistent-repository-root":
                try:
                    validate_probe(
                        repository_root=root / "nonexistent-repository-root"
                    )
                except EnvelopeError as error:
                    return error.code
                raise EnvelopeError(
                    "external_effect_red_fixture_unexpectedly_accepted",
                    case["case_id"],
                )
            elif setup == "non-directory-repository-root":
                non_directory_root = root / "non-directory-repository-root"
                non_directory_root.write_text("not a repository directory\n")
                try:
                    validate_probe(repository_root=non_directory_root)
                except EnvelopeError as error:
                    return error.code
                raise EnvelopeError(
                    "external_effect_red_fixture_unexpectedly_accepted",
                    case["case_id"],
                )
            elif setup == "special-file-repository-root":
                special_root = root / "special-file-repository-root"
                os.mkfifo(special_root)
                try:
                    validate_probe(repository_root=special_root)
                except EnvelopeError as error:
                    return error.code
                raise EnvelopeError(
                    "external_effect_red_fixture_unexpectedly_accepted",
                    case["case_id"],
                )
            elif setup == "symlink-alias-repository-root":
                alias_root = root / "repository-root-alias"
                alias_root.symlink_to(root, target_is_directory=True)
                try:
                    validate_probe(repository_root=alias_root)
                except EnvelopeError as error:
                    return error.code
                raise EnvelopeError(
                    "external_effect_red_fixture_unexpectedly_accepted",
                    case["case_id"],
                )
            elif setup == "lexical-alias-repository-root":
                alias_component = root / "repository-root-alias-component"
                alias_component.mkdir()
                alias_root = alias_component / ".."
                try:
                    validate_probe(repository_root=alias_root)
                except EnvelopeError as error:
                    return error.code
                raise EnvelopeError(
                    "external_effect_red_fixture_unexpectedly_accepted",
                    case["case_id"],
                )
            elif setup == "repository-root-swap-between-checks":
                mediation_token = validate_probe()
                assert mediation_token is not None
                other_root = root / "independent-repository-root"
                other_root.mkdir()
                try:
                    validate_probe(
                        final_revalidation=True,
                        mediation_token=mediation_token,
                        repository_root=other_root,
                    )
                except EnvelopeError as error:
                    return error.code
                raise EnvelopeError(
                    "external_effect_red_fixture_unexpectedly_accepted",
                    case["case_id"],
                )
            elif setup == "same-path-root-rename-replacement":
                mediation_token = validate_probe()
                assert mediation_token is not None
                replace_repository_root_same_path()
                try:
                    validate_probe(
                        final_revalidation=True,
                        mediation_token=mediation_token,
                    )
                except EnvelopeError as error:
                    return error.code
                raise EnvelopeError(
                    "external_effect_red_fixture_unexpectedly_accepted",
                    case["case_id"],
                )
            elif setup == "root-replacement-failure-does-not-consume":
                mediation_token = validate_probe()
                assert mediation_token is not None
                moved_root = replace_repository_root_same_path()
                try:
                    validate_probe(
                        final_revalidation=True,
                        mediation_token=mediation_token,
                    )
                except EnvelopeError as error:
                    if error.code != "canonical_repository_object_mismatch":
                        return error.code
                else:
                    raise EnvelopeError(
                        "external_effect_red_fixture_unexpectedly_accepted",
                        case["case_id"],
                    )
                restore_repository_root_same_path(moved_root)
                validate_probe(
                    final_revalidation=True,
                    mediation_token=mediation_token,
                )
                fresh_mediator = ExternalEffectMediator()
                repeated_token = fresh_mediator.initial(
                    root,
                    scope,
                    authority,
                    candidate=candidate,
                    trusted_now=trusted_initial,
                )
                try:
                    fresh_mediator.final(
                        root,
                        scope,
                        authority,
                        repeated_token,
                        candidate=candidate,
                        trusted_now=trusted_initial,
                    )
                except EnvelopeError as error:
                    return error.code
                raise EnvelopeError(
                    "external_effect_red_fixture_unexpectedly_accepted",
                    case["case_id"],
                )
            elif setup == "repository-mismatch-does-not-consume":
                mediation_token = validate_probe()
                assert mediation_token is not None
                original_uri = candidate["repository"]["root_uri"]
                candidate["repository"]["root_uri"] = "repo://mismatched-before-final"
                try:
                    validate_probe(
                        final_revalidation=True,
                        mediation_token=mediation_token,
                    )
                except EnvelopeError as error:
                    if error.code != "canonical_repository_identity_mismatch":
                        return error.code
                else:
                    raise EnvelopeError(
                        "external_effect_red_fixture_unexpectedly_accepted",
                        case["case_id"],
                    )
                candidate["repository"]["root_uri"] = original_uri
                validate_probe(
                    final_revalidation=True,
                    mediation_token=mediation_token,
                )
                fresh_mediator = ExternalEffectMediator()
                repeated_token = fresh_mediator.initial(
                    root,
                    scope,
                    authority,
                    candidate=candidate,
                    trusted_now=trusted_initial,
                )
                try:
                    fresh_mediator.final(
                        root,
                        scope,
                        authority,
                        repeated_token,
                        candidate=candidate,
                        trusted_now=trusted_initial,
                    )
                except EnvelopeError as error:
                    return error.code
                raise EnvelopeError(
                    "external_effect_red_fixture_unexpectedly_accepted",
                    case["case_id"],
                )
            elif setup == "fabricated-effect-without-authority":
                scope["external_effects"] = [
                    {
                        "effect_id": "EFFECT-UNAUTHORIZED-DELETE",
                        "kind": "external_write",
                        "provider": "github",
                        "action": "delete-repository",
                        "target_sha256": digest_bytes(b"fabricated-delete-target"),
                        "approval_id": "APPROVAL-DOES-NOT-EXIST",
                    }
                ]
                binding["sources"] = []
            elif setup == "missing-authority-source":
                binding["sources"] = []
            elif setup == "unknown-effect-id":
                scope["external_effects"][0]["effect_id"] = "EFFECT-UNKNOWN"
            elif setup == "unknown-approval-id":
                scope["external_effects"][0]["approval_id"] = "APPROVAL-UNKNOWN"
            elif setup == "duplicate-authority-row":
                document["effects"].append(copy.deepcopy(document["effects"][0]))
                refresh_source_digest = True
            elif setup == "ambiguous-authority-row":
                second_path = EXTERNAL_EFFECT_AUTHORITY_ROOT + "/SECOND-SYNTHETIC.json"
                second_sha256 = write_synthetic_authority_source(
                    root, second_path, document
                )
                _, second_identity = read_external_effect_authority_source(
                    root, second_path
                )
                binding["sources"].append(
                    {
                        "path": second_path,
                        "sha256": second_sha256,
                        "identity": second_identity,
                    }
                )
            elif setup == "revoked-effect":
                document["effects"][0]["status"] = "revoked"
                refresh_source_digest = True
            elif setup == "revoked-approval":
                document["approvals"][0]["status"] = "revoked"
                refresh_source_digest = True
            elif setup == "expired-approval":
                document["approvals"][0]["expires_at"] = "2026-07-13T00:20:00Z"
                refresh_source_digest = True
            elif setup == "not-yet-valid-approval":
                document["approvals"][0]["valid_from"] = "2026-07-13T00:40:00Z"
                refresh_source_digest = True
            elif setup == "provider-substitution":
                scope["external_effects"][0]["provider"] = "gitlab"
            elif setup == "action-substitution":
                scope["external_effects"][0]["action"] = "delete-repository"
            elif setup == "target-substitution":
                scope["external_effects"][0]["target_sha256"] = digest_bytes(
                    b"substituted-target"
                )
            elif setup == "kind-substitution":
                scope["external_effects"][0]["kind"] = "network"
            elif setup == "approval-link-substitution":
                document["approvals"][0]["effect_id"] = "EFFECT-OTHER"
                refresh_source_digest = True
            elif setup == "approval-reuse":
                second = copy.deepcopy(scope["external_effects"][0])
                second["effect_id"] = "EFFECT-SYNTHETIC-SECOND"
                scope["external_effects"].append(second)
            elif setup == "consumed-approval":
                document["approvals"][0]["usage_count"] = 1
                document["approvals"][0]["consumed_by_context_id"] = digest_bytes(
                    b"prior-context"
                )
                refresh_source_digest = True
            elif setup == "unreferenced-source-row":
                extra_effect = copy.deepcopy(document["effects"][0])
                extra_effect["effect_id"] = "EFFECT-UNREFERENCED"
                extra_approval = copy.deepcopy(document["approvals"][0])
                extra_approval["approval_id"] = "APPROVAL-UNREFERENCED"
                extra_approval["effect_id"] = "EFFECT-UNREFERENCED"
                document["effects"].append(extra_effect)
                document["approvals"].append(extra_approval)
                refresh_source_digest = True
            elif setup == "source-namespace-substitution":
                outside = "docs/context-proposal/FABRICATED-AUTHORITY.json"
                outside_sha256 = write_synthetic_authority_source(root, outside, document)
                outside_identity = external_effect_source_identity(
                    os.lstat(root / outside)
                )
                binding["sources"] = [
                    {
                        "path": outside,
                        "sha256": outside_sha256,
                        "identity": outside_identity,
                    }
                ]
            elif setup == "source-digest-substitution":
                document["captured_at"] = "2026-07-13T00:16:00Z"
                write_synthetic_authority_source(root, source_path, document)
            elif setup == "resolved-row-substitution":
                binding["effect_rows"][0]["row_sha256"] = digest_bytes(
                    b"substituted-row"
                )
            elif setup == "unreferenced-binding-row":
                extra = copy.deepcopy(binding["effect_rows"][0])
                extra["effect_id"] = "EFFECT-UNREFERENCED-BINDING"
                binding["effect_rows"].append(extra)
            elif setup == "future-authority-source":
                document["captured_at"] = "2026-07-13T00:40:00Z"
                refresh_source_digest = True
            elif setup == "final-session-authority-mutation":
                mediation_token = validate_probe()
                assert mediation_token is not None
                document["captured_at"] = "2026-07-13T00:16:00Z"
                write_synthetic_authority_source(root, source_path, document)
                try:
                    validate_probe(
                        final_revalidation=True,
                        mediation_token=mediation_token,
                    )
                except EnvelopeError as error:
                    return error.code
                raise EnvelopeError(
                    "external_effect_red_fixture_unexpectedly_accepted",
                    case["case_id"],
                )
            elif setup == "protected-hardlink-alias":
                os.link(
                    root / source_path,
                    root / EXTERNAL_EFFECT_AUTHORITY_ROOT / "AUTHORITY-ALIAS.json",
                )
            elif setup == "proposal-local-hardlink-alias":
                alias = root / "docs/context-proposal/AUTHORITY-ALIAS.json"
                alias.parent.mkdir(parents=True, exist_ok=True)
                os.link(root / source_path, alias)
            elif setup == "post-validation-hardlink-insertion":
                mediation_token = validate_probe()
                assert mediation_token is not None
                alias = root / "docs/context-proposal/POST-VALIDATION-ALIAS.json"
                alias.parent.mkdir(parents=True, exist_ok=True)
                os.link(root / source_path, alias)
                try:
                    validate_probe(
                        final_revalidation=True,
                        mediation_token=mediation_token,
                    )
                except EnvelopeError as error:
                    return error.code
                raise EnvelopeError(
                    "external_effect_red_fixture_unexpectedly_accepted",
                    case["case_id"],
                )
            elif setup == "rename-replacement":
                mediation_token = validate_probe()
                assert mediation_token is not None
                source = root / source_path
                data = source.read_bytes()
                moved = source.with_name("CURRENT-SYNTHETIC-MOVED.json")
                os.rename(source, moved)
                source.write_bytes(data)
                try:
                    validate_probe(
                        final_revalidation=True,
                        mediation_token=mediation_token,
                    )
                except EnvelopeError as error:
                    return error.code
                raise EnvelopeError(
                    "external_effect_red_fixture_unexpectedly_accepted",
                    case["case_id"],
                )
            elif setup == "mutate-restore":
                mediation_token = validate_probe()
                assert mediation_token is not None
                source = root / source_path
                data = source.read_bytes()
                original_mode = stat.S_IMODE(os.lstat(source).st_mode)
                source.write_bytes(data + b"mutation")
                source.write_bytes(data)
                os.chmod(source, 0o600)
                os.chmod(source, original_mode)
                try:
                    validate_probe(
                        final_revalidation=True,
                        mediation_token=mediation_token,
                    )
                except EnvelopeError as error:
                    return error.code
                raise EnvelopeError(
                    "external_effect_red_fixture_unexpectedly_accepted",
                    case["case_id"],
                )
            elif setup == "ancestor-symlink":
                authority_directory = root / EXTERNAL_EFFECT_AUTHORITY_ROOT
                moved = authority_directory.with_name("effect-authority-real")
                os.rename(authority_directory, moved)
                os.symlink(moved.name, authority_directory)
            elif setup == "leaf-symlink":
                source = root / source_path
                moved = source.with_name("CURRENT-SYNTHETIC-REAL.json")
                os.rename(source, moved)
                os.symlink(moved.name, source)
            elif setup == "special-file":
                source = root / source_path
                source.unlink()
                os.mkfifo(source, mode=0o600)
            elif setup == "oversized-source":
                source = root / source_path
                data = b"x" * (EXTERNAL_EFFECT_AUTHORITY_MAX_BYTES + 1)
                source.write_bytes(data)
                binding["sources"][0]["sha256"] = digest_bytes(data)
                binding["sources"][0]["identity"] = external_effect_source_identity(
                    os.lstat(source)
                )
            elif setup == "missing-session-receipt":
                binding["session_receipt"] = None
            elif setup == "unknown-session-receipt":
                binding["session_receipt"]["receipt_id"] = "EFFECT-SESSION-UNKNOWN"
            elif setup == "duplicate-session-receipt":
                receipt_binding = binding["session_receipt"]
                receipt_source = receipt_binding["sources"][0]
                receipt_document = json.loads(
                    (root / receipt_source["path"]).read_bytes()
                )
                receipt_document["receipts"].append(
                    copy.deepcopy(receipt_document["receipts"][0])
                )
                refresh_synthetic_session_receipt_source(
                    root, binding, receipt_document
                )
            elif setup == "ambiguous-session-receipt":
                receipt_binding = binding["session_receipt"]
                receipt_source = receipt_binding["sources"][0]
                receipt_document = json.loads(
                    (root / receipt_source["path"]).read_bytes()
                )
                second_path = (
                    EXTERNAL_EFFECT_SESSION_RECEIPT_ROOT + "/SECOND-SYNTHETIC.json"
                )
                second_sha256 = write_synthetic_authority_source(
                    root, second_path, receipt_document
                )
                _, second_identity = read_external_effect_authority_source(
                    root, second_path
                )
                receipt_binding["sources"].append(
                    {
                        "path": second_path,
                        "sha256": second_sha256,
                        "identity": second_identity,
                    }
                )
            elif setup == "session-receipt-source-substitution":
                receipt_binding = binding["session_receipt"]
                receipt_source = receipt_binding["sources"][0]
                receipt_document = json.loads(
                    (root / receipt_source["path"]).read_bytes()
                )
                outside = "docs/context-proposal/FABRICATED-TIME-RECEIPT.json"
                outside_sha256 = write_synthetic_authority_source(
                    root, outside, receipt_document
                )
                receipt_binding["sources"] = [
                    {
                        "path": outside,
                        "sha256": outside_sha256,
                        "identity": external_effect_source_identity(
                            os.lstat(root / outside)
                        ),
                    }
                ]
            elif setup == "backdated-session-receipt":
                document["captured_at"] = "2020-01-01T00:00:00Z"
                rebind_effect_source_document()
                authority["captured_at"] = "2020-01-01T00:00:02Z"
                receipt_binding = binding["session_receipt"]
                receipt_source = receipt_binding["sources"][0]
                receipt_document = json.loads(
                    (root / receipt_source["path"]).read_bytes()
                )
                row = receipt_document["receipts"][0]
                row["issued_at"] = "2020-01-01T00:00:00Z"
                row["expires_at"] = "2020-01-01T00:05:00Z"
                refresh_synthetic_session_receipt_source(
                    root, binding, receipt_document
                )
            elif setup == "future-session-receipt":
                authority["captured_at"] = "2026-07-13T00:40:00Z"
                receipt_binding = binding["session_receipt"]
                receipt_source = receipt_binding["sources"][0]
                receipt_document = json.loads(
                    (root / receipt_source["path"]).read_bytes()
                )
                row = receipt_document["receipts"][0]
                row["issued_at"] = "2026-07-13T00:40:00Z"
                row["expires_at"] = "2026-07-13T00:50:00Z"
                refresh_synthetic_session_receipt_source(
                    root, binding, receipt_document
                )
            elif setup in {
                "receipt-candidate-mismatch",
                "receipt-authority-session-mismatch",
                "receipt-repository-mismatch",
                "receipt-session-id-substitution",
                "receipt-effect-source-mismatch",
                "session-receipt-reuse",
            }:
                receipt_binding = binding["session_receipt"]
                receipt_source = receipt_binding["sources"][0]
                receipt_document = json.loads(
                    (root / receipt_source["path"]).read_bytes()
                )
                row = receipt_document["receipts"][0]
                if setup == "receipt-candidate-mismatch":
                    row["candidate_id"] = digest_bytes(b"substituted-candidate")
                elif setup == "receipt-authority-session-mismatch":
                    row["authority_session_id"] = digest_bytes(
                        b"substituted-authority-session"
                    )
                elif setup == "receipt-repository-mismatch":
                    row["repository_root_sha256"] = digest_bytes(
                        b"substituted-repository"
                    )
                elif setup == "receipt-session-id-substitution":
                    row["session_id"] = digest_bytes(b"substituted-session-id")
                elif setup == "receipt-effect-source-mismatch":
                    row["effect_authority_sources_sha256"] = digest_bytes(
                        b"substituted-effect-sources"
                    )
                else:
                    row["usage_count"] = 1
                    row["consumed_by_context_id"] = digest_bytes(
                        b"prior-receipt-context"
                    )
                refresh_synthetic_session_receipt_source(
                    root,
                    binding,
                    receipt_document,
                    refresh_effect_sources=setup != "receipt-effect-source-mismatch",
                    recompute_session_id=setup != "receipt-session-id-substitution",
                )
            elif setup == "receipt-skew-violation":
                authority["captured_at"] = "2026-07-13T00:30:30Z"
            elif setup == "approval-expired-at-trusted-time":
                document["approvals"][0]["expires_at"] = "2026-07-13T00:29:00Z"
                refresh_source_digest = True
            elif setup == "approval-expiry-between-checks":
                document["approvals"][0]["expires_at"] = "2026-07-13T00:31:00Z"
                rebind_effect_source_document()
                refresh_external_effect_authority_binding(root, scope, authority)
                mediation_token = validate_probe()
                assert mediation_token is not None
                try:
                    validate_probe(
                        final_revalidation=True,
                        trusted_now=parse_timestamp(
                            "2026-07-13T00:32:00Z",
                            "synthetic_trusted_time_invalid",
                        ),
                        mediation_token=mediation_token,
                    )
                except EnvelopeError as error:
                    return error.code
                raise EnvelopeError(
                    "external_effect_red_fixture_unexpectedly_accepted",
                    case["case_id"],
                )
            elif setup == "clock-rollback":
                mediation_token = validate_probe()
                assert mediation_token is not None
                try:
                    validate_probe(
                        final_revalidation=True,
                        trusted_now=parse_timestamp(
                            "2026-07-13T00:29:59Z",
                            "synthetic_trusted_time_invalid",
                        ),
                        mediation_token=mediation_token,
                    )
                except EnvelopeError as error:
                    return error.code
                raise EnvelopeError(
                    "external_effect_red_fixture_unexpectedly_accepted",
                    case["case_id"],
                )
            elif setup == "time-source-mutation":
                mediation_token = validate_probe()
                assert mediation_token is not None
                receipt_binding = binding["session_receipt"]
                receipt_source = receipt_binding["sources"][0]
                receipt_document = json.loads(
                    (root / receipt_source["path"]).read_bytes()
                )
                receipt_document["receipts"][0]["expires_at"] = (
                    "2026-07-13T00:44:00Z"
                )
                write_synthetic_authority_source(
                    root, receipt_source["path"], receipt_document
                )
                try:
                    validate_probe(
                        final_revalidation=True,
                        mediation_token=mediation_token,
                    )
                except EnvelopeError as error:
                    return error.code
                raise EnvelopeError(
                    "external_effect_red_fixture_unexpectedly_accepted",
                    case["case_id"],
                )
            elif setup == "final-observation-omitted":
                validate_probe()
                try:
                    validate_probe(final_revalidation=True)
                except EnvelopeError as error:
                    return error.code
                raise EnvelopeError(
                    "external_effect_red_fixture_unexpectedly_accepted",
                    case["case_id"],
                )
            elif setup == "rolled-back-final-observation-omitted":
                initial_at = parse_timestamp(
                    "2026-07-13T00:40:00Z", "synthetic_trusted_time_invalid"
                )
                validate_probe(trusted_now=initial_at)
                try:
                    validate_probe(
                        final_revalidation=True,
                        trusted_now=trusted_initial,
                    )
                except EnvelopeError as error:
                    return error.code
                raise EnvelopeError(
                    "external_effect_red_fixture_unexpectedly_accepted",
                    case["case_id"],
                )
            elif setup == "forged-prior-observation":
                mediation_token = validate_probe()
                assert mediation_token is not None
                forged_token = object.__new__(type(mediation_token))
                try:
                    validate_probe(
                        final_revalidation=True,
                        mediation_token=forged_token,
                    )
                except EnvelopeError as error:
                    return error.code
                raise EnvelopeError(
                    "external_effect_red_fixture_unexpectedly_accepted",
                    case["case_id"],
                )
            elif setup in {
                "prior-observation-candidate-mismatch",
                "prior-observation-session-mismatch",
                "prior-observation-receipt-mismatch",
            }:
                mediation_token = validate_probe()
                assert mediation_token is not None
                receipt_binding = binding["session_receipt"]
                receipt_source = receipt_binding["sources"][0]
                receipt_document = json.loads(
                    (root / receipt_source["path"]).read_bytes()
                )
                receipt_row = receipt_document["receipts"][0]
                if setup == "prior-observation-candidate-mismatch":
                    candidate["candidate_id"] = digest_bytes(
                        b"substituted-observation-candidate"
                    )
                    receipt_row["candidate_id"] = candidate["candidate_id"]
                elif setup == "prior-observation-session-mismatch":
                    authority["session_id"] = digest_bytes(
                        b"substituted-observation-authority-session"
                    )
                    receipt_row["authority_session_id"] = authority["session_id"]
                else:
                    receipt_row["nonce"] = digest_bytes(
                        b"substituted-observation-receipt"
                    )
                refresh_synthetic_session_receipt_source(
                    root, binding, receipt_document
                )
                refresh_external_effect_authority_binding(root, scope, authority)
                try:
                    validate_probe(
                        final_revalidation=True,
                        mediation_token=mediation_token,
                    )
                except EnvelopeError as error:
                    return error.code
                raise EnvelopeError(
                    "external_effect_red_fixture_unexpectedly_accepted",
                    case["case_id"],
                )
            elif setup == "prior-observation-after-current":
                mediation_token = validate_probe()
                assert mediation_token is not None
                try:
                    validate_probe(
                        final_revalidation=True,
                        trusted_now=parse_timestamp(
                            "2026-07-13T00:29:59Z",
                            "synthetic_trusted_time_invalid",
                        ),
                        mediation_token=mediation_token,
                    )
                except EnvelopeError as error:
                    return error.code
                raise EnvelopeError(
                    "external_effect_red_fixture_unexpectedly_accepted",
                    case["case_id"],
                )
            elif setup == "stale-prior-observation":
                mediation_token = validate_probe()
                assert mediation_token is not None
                try:
                    validate_probe(
                        final_revalidation=True,
                        trusted_now=parse_timestamp(
                            "2026-07-13T00:36:00Z",
                            "synthetic_trusted_time_invalid",
                        ),
                        mediation_token=mediation_token,
                    )
                except EnvelopeError as error:
                    return error.code
                raise EnvelopeError(
                    "external_effect_red_fixture_unexpectedly_accepted",
                    case["case_id"],
                )
            elif setup == "duplicate-final-observation-reuse":
                mediation_token = validate_probe()
                assert mediation_token is not None
                validate_probe(
                    final_revalidation=True,
                    mediation_token=mediation_token,
                )
                try:
                    validate_probe(
                        final_revalidation=True,
                        mediation_token=mediation_token,
                    )
                except EnvelopeError as error:
                    return error.code
                raise EnvelopeError(
                    "external_effect_red_fixture_unexpectedly_accepted",
                    case["case_id"],
                )
            elif setup == "same-mediator-authorization-reuse":
                mediation_token = validate_probe()
                assert mediation_token is not None
                validate_probe(
                    final_revalidation=True,
                    mediation_token=mediation_token,
                )
                second_token = validate_probe()
                assert second_token is not None
                try:
                    validate_probe(
                        final_revalidation=True,
                        mediation_token=second_token,
                    )
                except EnvelopeError as error:
                    return error.code
                raise EnvelopeError(
                    "external_effect_red_fixture_unexpectedly_accepted",
                    case["case_id"],
                )
            elif setup == "fresh-mediator-authorization-reuse":
                mediation_token = validate_probe()
                assert mediation_token is not None
                validate_probe(
                    final_revalidation=True,
                    mediation_token=mediation_token,
                )
                fresh_mediator = ExternalEffectMediator()
                second_token = fresh_mediator.initial(
                    root,
                    scope,
                    authority,
                    candidate=candidate,
                    trusted_now=trusted_initial,
                )
                try:
                    fresh_mediator.final(
                        root,
                        scope,
                        authority,
                        second_token,
                        candidate=candidate,
                        trusted_now=trusted_initial,
                    )
                except EnvelopeError as error:
                    return error.code
                raise EnvelopeError(
                    "external_effect_red_fixture_unexpectedly_accepted",
                    case["case_id"],
                )
            elif setup == "failed-final-does-not-consume":
                mediation_token = validate_probe()
                assert mediation_token is not None
                try:
                    validate_probe(
                        final_revalidation=True,
                        trusted_now=None,  # type: ignore[arg-type]
                        mediation_token=mediation_token,
                    )
                except EnvelopeError as error:
                    if error.code != "external_effect_trusted_time_unavailable":
                        return error.code
                else:
                    raise EnvelopeError(
                        "external_effect_red_fixture_unexpectedly_accepted",
                        case["case_id"],
                    )
                validate_probe(
                    final_revalidation=True,
                    mediation_token=mediation_token,
                )
                second_token = validate_probe()
                assert second_token is not None
                try:
                    validate_probe(
                        final_revalidation=True,
                        mediation_token=second_token,
                    )
                except EnvelopeError as error:
                    return error.code
                raise EnvelopeError(
                    "external_effect_red_fixture_unexpectedly_accepted",
                    case["case_id"],
                )
            elif setup == "distinct-authorization-proceeds":
                mediation_token = validate_probe()
                assert mediation_token is not None
                validate_probe(
                    final_revalidation=True,
                    mediation_token=mediation_token,
                )
                scope_row = scope["external_effects"][0]
                effect_row = document["effects"][0]
                approval_row = document["approvals"][0]
                scope_row["effect_id"] = "EFFECT-SYNTHETIC-DISTINCT"
                scope_row["approval_id"] = "APPROVAL-SYNTHETIC-DISTINCT"
                scope_row["target_sha256"] = digest_bytes(b"distinct-target")
                effect_row["effect_id"] = scope_row["effect_id"]
                effect_row["target_sha256"] = scope_row["target_sha256"]
                approval_row["approval_id"] = scope_row["approval_id"]
                approval_row["effect_id"] = scope_row["effect_id"]
                rebind_effect_source_document()
                receipt_binding = binding["session_receipt"]
                receipt_source = receipt_binding["sources"][0]
                receipt_document = json.loads(
                    (root / receipt_source["path"]).read_bytes()
                )
                receipt_binding["receipt_id"] = (
                    "EFFECT-SESSION-SYNTHETIC-DISTINCT"
                )
                receipt_document["receipts"][0]["receipt_id"] = receipt_binding[
                    "receipt_id"
                ]
                receipt_document["receipts"][0]["nonce"] = digest_bytes(
                    b"distinct-receipt-nonce"
                )
                refresh_synthetic_session_receipt_source(
                    root, binding, receipt_document
                )
                refresh_external_effect_authority_binding(root, scope, authority)
                distinct_token = validate_probe()
                assert distinct_token is not None
                validate_probe(
                    final_revalidation=True,
                    mediation_token=distinct_token,
                )
                fresh_mediator = ExternalEffectMediator()
                repeated_token = fresh_mediator.initial(
                    root,
                    scope,
                    authority,
                    candidate=candidate,
                    trusted_now=trusted_initial,
                )
                try:
                    fresh_mediator.final(
                        root,
                        scope,
                        authority,
                        repeated_token,
                        candidate=candidate,
                        trusted_now=trusted_initial,
                    )
                except EnvelopeError as error:
                    return error.code
                raise EnvelopeError(
                    "external_effect_red_fixture_unexpectedly_accepted",
                    case["case_id"],
                )
            elif setup in {
                "multiple-approval-atomic-first",
                "multiple-approval-atomic-second",
            }:
                second_scope = copy.deepcopy(scope["external_effects"][0])
                second_scope["effect_id"] = "EFFECT-SYNTHETIC-SECOND"
                second_scope["approval_id"] = "APPROVAL-SYNTHETIC-SECOND"
                second_scope["target_sha256"] = digest_bytes(b"second-target")
                scope["external_effects"].append(second_scope)
                second_effect = copy.deepcopy(document["effects"][0])
                second_effect["effect_id"] = second_scope["effect_id"]
                second_effect["target_sha256"] = second_scope["target_sha256"]
                document["effects"].append(second_effect)
                second_approval = copy.deepcopy(document["approvals"][0])
                second_approval["approval_id"] = second_scope["approval_id"]
                second_approval["effect_id"] = second_scope["effect_id"]
                document["approvals"].append(second_approval)
                rebind_effect_source_document()
                refresh_external_effect_authority_binding(root, scope, authority)
                mediation_token = validate_probe()
                assert mediation_token is not None
                try:
                    validate_probe(
                        final_revalidation=True,
                        trusted_now=None,  # type: ignore[arg-type]
                        mediation_token=mediation_token,
                    )
                except EnvelopeError as error:
                    if error.code != "external_effect_trusted_time_unavailable":
                        return error.code
                else:
                    raise EnvelopeError(
                        "external_effect_red_fixture_unexpectedly_accepted",
                        case["case_id"],
                    )
                validate_probe(
                    final_revalidation=True,
                    mediation_token=mediation_token,
                )
                keep = 0 if setup.endswith("first") else 1
                scope["external_effects"] = [scope["external_effects"][keep]]
                document["effects"] = [document["effects"][keep]]
                document["approvals"] = [document["approvals"][keep]]
                rebind_effect_source_document()
                refresh_external_effect_authority_binding(root, scope, authority)
                fresh_mediator = ExternalEffectMediator()
                repeated_token = fresh_mediator.initial(
                    root,
                    scope,
                    authority,
                    candidate=candidate,
                    trusted_now=trusted_initial,
                )
                try:
                    fresh_mediator.final(
                        root,
                        scope,
                        authority,
                        repeated_token,
                        candidate=candidate,
                        trusted_now=trusted_initial,
                    )
                except EnvelopeError as error:
                    return error.code
                raise EnvelopeError(
                    "external_effect_red_fixture_unexpectedly_accepted",
                    case["case_id"],
                )
            elif setup in {
                "consumed-approval-source-identity-substitution",
                "consumed-receipt-identity-substitution",
            }:
                mediation_token = validate_probe()
                assert mediation_token is not None
                validate_probe(
                    final_revalidation=True,
                    mediation_token=mediation_token,
                )
                if setup == "consumed-approval-source-identity-substitution":
                    document["captured_at"] = "2026-07-13T00:16:01Z"
                    rebind_effect_source_document()
                else:
                    receipt_binding = binding["session_receipt"]
                    receipt_source = receipt_binding["sources"][0]
                    receipt_document = json.loads(
                        (root / receipt_source["path"]).read_bytes()
                    )
                    receipt_document["receipts"][0]["nonce"] = digest_bytes(
                        b"substituted-consumed-receipt"
                    )
                    refresh_synthetic_session_receipt_source(
                        root, binding, receipt_document
                    )
                refresh_external_effect_authority_binding(root, scope, authority)
                fresh_mediator = ExternalEffectMediator()
                repeated_token = fresh_mediator.initial(
                    root,
                    scope,
                    authority,
                    candidate=candidate,
                    trusted_now=trusted_initial,
                )
                try:
                    fresh_mediator.final(
                        root,
                        scope,
                        authority,
                        repeated_token,
                        candidate=candidate,
                        trusted_now=trusted_initial,
                    )
                except EnvelopeError as error:
                    return error.code
                raise EnvelopeError(
                    "external_effect_red_fixture_unexpectedly_accepted",
                    case["case_id"],
                )
            elif setup == "receipt-id-revival-with-distinct-approval":
                mediation_token = validate_probe()
                assert mediation_token is not None
                validate_probe(
                    final_revalidation=True,
                    mediation_token=mediation_token,
                )
                original_receipt_id = binding["session_receipt"]["receipt_id"]
                replace_single_authorization(
                    "EFFECT-SYNTHETIC-RECEIPT-REVIVAL",
                    "APPROVAL-SYNTHETIC-RECEIPT-REVIVAL",
                    b"receipt-revival-target",
                )
                reissue_receipt(
                    original_receipt_id,
                    b"revived-receipt-nonce",
                    {"permitted_skew_seconds": 3},
                )
                refresh_external_effect_authority_binding(root, scope, authority)
                fresh_mediator = ExternalEffectMediator()
                revived_token = fresh_mediator.initial(
                    root,
                    scope,
                    authority,
                    candidate=candidate,
                    trusted_now=trusted_initial,
                )
                try:
                    fresh_mediator.final(
                        root,
                        scope,
                        authority,
                        revived_token,
                        candidate=candidate,
                        trusted_now=trusted_initial,
                    )
                except EnvelopeError as error:
                    return error.code
                raise EnvelopeError(
                    "external_effect_red_fixture_unexpectedly_accepted",
                    case["case_id"],
                )
            elif setup == "approval-id-revival-with-distinct-receipt":
                mediation_token = validate_probe()
                assert mediation_token is not None
                validate_probe(
                    final_revalidation=True,
                    mediation_token=mediation_token,
                )
                original_approval_id = scope["external_effects"][0]["approval_id"]
                replace_single_authorization(
                    "EFFECT-SYNTHETIC-APPROVAL-REVIVAL",
                    original_approval_id,
                    b"approval-revival-target",
                )
                reissue_receipt(
                    "EFFECT-SESSION-SYNTHETIC-APPROVAL-REVIVAL",
                    b"approval-revival-receipt",
                )
                refresh_external_effect_authority_binding(root, scope, authority)
                fresh_mediator = ExternalEffectMediator()
                revived_token = fresh_mediator.initial(
                    root,
                    scope,
                    authority,
                    candidate=candidate,
                    trusted_now=trusted_initial,
                )
                try:
                    fresh_mediator.final(
                        root,
                        scope,
                        authority,
                        revived_token,
                        candidate=candidate,
                        trusted_now=trusted_initial,
                    )
                except EnvelopeError as error:
                    return error.code
                raise EnvelopeError(
                    "external_effect_red_fixture_unexpectedly_accepted",
                    case["case_id"],
                )
            elif setup == "consumed-approval-protected-source-relocation":
                mediation_token = validate_probe()
                assert mediation_token is not None
                validate_probe(
                    final_revalidation=True,
                    mediation_token=mediation_token,
                )
                relocated_path = (
                    EXTERNAL_EFFECT_AUTHORITY_ROOT + "/RELOCATED-SYNTHETIC.json"
                )
                relocated_sha256 = write_synthetic_authority_source(
                    root, relocated_path, document
                )
                _, relocated_identity = read_external_effect_authority_source(
                    root, relocated_path
                )
                binding["sources"] = [
                    {
                        "path": relocated_path,
                        "sha256": relocated_sha256,
                        "identity": relocated_identity,
                    }
                ]
                reissue_receipt(
                    "EFFECT-SESSION-SYNTHETIC-RELOCATED",
                    b"relocated-receipt",
                )
                refresh_external_effect_authority_binding(root, scope, authority)
                fresh_mediator = ExternalEffectMediator()
                relocated_token = fresh_mediator.initial(
                    root,
                    scope,
                    authority,
                    candidate=candidate,
                    trusted_now=trusted_initial,
                )
                try:
                    fresh_mediator.final(
                        root,
                        scope,
                        authority,
                        relocated_token,
                        candidate=candidate,
                        trusted_now=trusted_initial,
                    )
                except EnvelopeError as error:
                    return error.code
                raise EnvelopeError(
                    "external_effect_red_fixture_unexpectedly_accepted",
                    case["case_id"],
                )
            elif setup in {
                "consumed-receipt-row-mutation",
                "consumed-receipt-session-mutation",
                "consumed-receipt-nonce-mutation",
            }:
                mediation_token = validate_probe()
                assert mediation_token is not None
                validate_probe(
                    final_revalidation=True,
                    mediation_token=mediation_token,
                )
                original_receipt_id = binding["session_receipt"]["receipt_id"]
                suffix = setup.removeprefix("consumed-receipt-").removesuffix(
                    "-mutation"
                ).upper()
                replace_single_authorization(
                    f"EFFECT-SYNTHETIC-{suffix}-MUTATION",
                    f"APPROVAL-SYNTHETIC-{suffix}-MUTATION",
                    setup.encode(),
                )
                row_updates: dict[str, Any] = {}
                if setup == "consumed-receipt-row-mutation":
                    row_updates["permitted_skew_seconds"] = 3
                elif setup == "consumed-receipt-session-mutation":
                    row_updates["issued_at"] = "2026-07-13T00:29:59Z"
                reissue_receipt(
                    original_receipt_id,
                    setup.encode(),
                    row_updates,
                )
                refresh_external_effect_authority_binding(root, scope, authority)
                fresh_mediator = ExternalEffectMediator()
                revived_token = fresh_mediator.initial(
                    root,
                    scope,
                    authority,
                    candidate=candidate,
                    trusted_now=trusted_initial,
                )
                try:
                    fresh_mediator.final(
                        root,
                        scope,
                        authority,
                        revived_token,
                        candidate=candidate,
                        trusted_now=trusted_initial,
                    )
                except EnvelopeError as error:
                    return error.code
                raise EnvelopeError(
                    "external_effect_red_fixture_unexpectedly_accepted",
                    case["case_id"],
                )
            elif setup == "cross-repository-non-overconsumption":
                mediation_token = validate_probe()
                assert mediation_token is not None
                validate_probe(
                    final_revalidation=True,
                    mediation_token=mediation_token,
                )
                with tempfile.TemporaryDirectory(
                    prefix="hul-effect-independent-repository-"
                ) as other_temporary:
                    other_root = Path(other_temporary).resolve(strict=True)
                    _, other_repository_identity = resolve_canonical_repository(
                        other_root,
                    )
                    other_candidate = {
                        "repository": other_repository_identity,
                        "candidate_id": candidate["candidate_id"],
                    }
                    other_scope, other_authority, _, _ = (
                        prepare_synthetic_external_effect_authority(
                            other_root,
                            other_candidate,
                            digest_bytes(b"synthetic-main-authority-session"),
                        )
                    )
                    other_mediator = ExternalEffectMediator()
                    other_token = other_mediator.initial(
                        other_root,
                        other_scope,
                        other_authority,
                        candidate=other_candidate,
                        trusted_now=trusted_initial,
                    )
                    other_mediator.final(
                        other_root,
                        other_scope,
                        other_authority,
                        other_token,
                        candidate=other_candidate,
                        trusted_now=trusted_initial,
                    )
                    repeated_mediator = ExternalEffectMediator()
                    repeated_token = repeated_mediator.initial(
                        other_root,
                        other_scope,
                        other_authority,
                        candidate=other_candidate,
                        trusted_now=trusted_initial,
                    )
                    try:
                        repeated_mediator.final(
                            other_root,
                            other_scope,
                            other_authority,
                            repeated_token,
                            candidate=other_candidate,
                            trusted_now=trusted_initial,
                        )
                    except EnvelopeError as error:
                        return error.code
                raise EnvelopeError(
                    "external_effect_red_fixture_unexpectedly_accepted",
                    case["case_id"],
                )
            elif setup == "concurrent-authorization-one-winner":
                first_mediator = ExternalEffectMediator()
                second_mediator = ExternalEffectMediator()
                first_token = first_mediator.initial(
                    root,
                    scope,
                    authority,
                    candidate=candidate,
                    trusted_now=trusted_initial,
                )
                second_token = second_mediator.initial(
                    root,
                    scope,
                    authority,
                    candidate=candidate,
                    trusted_now=trusted_initial,
                )
                barrier = threading.Barrier(3)
                results: list[str] = []

                def concurrent_final(
                    selected_mediator: Any, selected_token: object
                ) -> None:
                    barrier.wait()
                    try:
                        selected_mediator.final(
                            root,
                            scope,
                            authority,
                            selected_token,
                            candidate=candidate,
                            trusted_now=trusted_initial,
                        )
                    except EnvelopeError as error:
                        results.append(error.code)
                    else:
                        results.append("success")

                threads = [
                    threading.Thread(
                        target=concurrent_final,
                        args=(first_mediator, first_token),
                    ),
                    threading.Thread(
                        target=concurrent_final,
                        args=(second_mediator, second_token),
                    ),
                ]
                for thread in threads:
                    thread.start()
                barrier.wait()
                for thread in threads:
                    thread.join()
                if sorted(results) == [
                    "external_effect_authorization_reuse_rejected",
                    "success",
                ]:
                    return "external_effect_authorization_reuse_rejected"
                raise EnvelopeError(
                    "external_effect_concurrent_consumption_control_failed",
                    repr(results),
                )
            elif setup == "retired-public-builder-ledger-exploit":
                if any(
                    name in globals()
                    for name in (
                        "build_trusted_time_" + "observation",
                        "TrustedTime" + "ObservationLedger",
                    )
                ):
                    return "external_effect_retired_public_api_present"
                current_binding = _external_effect_validation_binding(
                    binding,
                    authority,
                    candidate,
                    binding["session_receipt"]["resolved_receipt"],
                    repository_identity,
                    repository_object_identity,
                )
                fabricated_state = {
                    "validation_binding": current_binding,
                    "observed_at": trusted_initial.isoformat(),
                    "initial_result_sha256": _external_effect_initial_result_digest(
                        current_binding, trusted_initial.isoformat()
                    ),
                }
                fresh_mediator = ExternalEffectMediator()
                try:
                    fresh_mediator.final(
                        root,
                        scope,
                        authority,
                        fabricated_state,
                        candidate=candidate,
                        trusted_now=trusted_initial,
                    )
                except EnvelopeError as error:
                    return error.code
                raise EnvelopeError(
                    "external_effect_red_fixture_unexpectedly_accepted",
                    case["case_id"],
                )
            elif setup == "cross-mediator-token":
                mediation_token = validate_probe()
                assert mediation_token is not None
                fresh_mediator = ExternalEffectMediator()
                try:
                    fresh_mediator.final(
                        root,
                        scope,
                        authority,
                        mediation_token,
                        candidate=candidate,
                        trusted_now=trusted_initial,
                    )
                except EnvelopeError as error:
                    return error.code
                raise EnvelopeError(
                    "external_effect_red_fixture_unexpectedly_accepted",
                    case["case_id"],
                )
            elif setup == "caller-created-mediator-state-injection":
                try:
                    ExternalEffectMediator({"pending": object()})  # type: ignore[call-arg]
                except TypeError:
                    fresh_mediator = ExternalEffectMediator()
                    try:
                        setattr(fresh_mediator, "pending_token", object())
                    except AttributeError:
                        return "external_effect_mediator_state_injection_rejected"
                raise EnvelopeError(
                    "external_effect_red_fixture_unexpectedly_accepted",
                    case["case_id"],
                )
            elif setup == "direct-token-construction":
                mediation_token = validate_probe()
                assert mediation_token is not None
                try:
                    type(mediation_token)(object())
                except TypeError:
                    return "external_effect_mediator_token_construction_rejected"
                raise EnvelopeError(
                    "external_effect_red_fixture_unexpectedly_accepted",
                    case["case_id"],
                )
            elif setup in {"token-shallow-copy", "token-deep-copy"}:
                mediation_token = validate_probe()
                assert mediation_token is not None
                try:
                    if setup == "token-shallow-copy":
                        copy.copy(mediation_token)
                    else:
                        copy.deepcopy(mediation_token)
                except TypeError:
                    return "external_effect_mediator_token_clone_rejected"
                raise EnvelopeError(
                    "external_effect_red_fixture_unexpectedly_accepted",
                    case["case_id"],
                )
            elif setup == "token-pickle-serialization":
                mediation_token = validate_probe()
                assert mediation_token is not None
                try:
                    pickle.dumps(mediation_token)
                except TypeError:
                    return "external_effect_mediator_token_serialization_rejected"
                raise EnvelopeError(
                    "external_effect_red_fixture_unexpectedly_accepted",
                    case["case_id"],
                )
            elif setup == "token-json-serialization":
                mediation_token = validate_probe()
                assert mediation_token is not None
                try:
                    json.dumps(mediation_token)
                except TypeError:
                    return "external_effect_mediator_token_serialization_rejected"
                raise EnvelopeError(
                    "external_effect_red_fixture_unexpectedly_accepted",
                    case["case_id"],
                )
            elif setup == "wrong-initial-result-token-substitution":
                validate_probe()
                fabricated = {
                    "initial_result_sha256": digest_bytes(
                        b"caller-supplied-initial-result"
                    )
                }
                try:
                    validate_probe(
                        final_revalidation=True,
                        mediation_token=fabricated,
                    )
                except EnvelopeError as error:
                    return error.code
                raise EnvelopeError(
                    "external_effect_red_fixture_unexpectedly_accepted",
                    case["case_id"],
                )
            elif setup == "missing-trusted-mediator-time":
                try:
                    mediator.initial(
                        root,
                        scope,
                        authority,
                        candidate=candidate,
                        trusted_now=None,
                    )
                except EnvelopeError as error:
                    return error.code
                raise EnvelopeError(
                    "external_effect_red_fixture_unexpectedly_accepted",
                    case["case_id"],
                )
            else:
                raise EnvelopeError(
                    "unknown_external_effect_fixture_setup",
                    f"unknown external-effect fixture setup: {setup}",
                )
            if refresh_source_digest:
                rebind_effect_source_document()
            try:
                validate_probe()
            except EnvelopeError as error:
                return error.code
            raise EnvelopeError(
                "external_effect_red_fixture_unexpectedly_accepted",
                case["case_id"],
            )
    finally:
        pass


def filesystem_probe_scope(proposed_path: str) -> dict[str, Any]:
    return {
        "artifact_paths": [{"path": proposed_path, "access": "propose_write"}],
        "semantic_symbols": ["filesystem-alias-probe"],
        "generated_outputs": [],
        "fixtures": [],
        "external_effects": [],
        "scratch_uris": [],
        "forbidden_paths": ["Cargo.toml"],
        "path_resolution": {
            "policy": "candidate-relative-no-follow-v2",
            "session_id": digest_bytes(b"filesystem-probe-session"),
            "resolved_scope_sha256": digest_bytes(b"filesystem-probe-scope"),
            "symlinks": "reject",
            "special_files": "reject",
            "post_freeze_revalidation": "required",
            "forbidden_overlap_policy": "exact-ancestor-descendant-casefold-reject-v1",
            "protected_write_policy": "git-host-root-authority-reject-v1",
            "proposed_write_object_policy": "lstat-no-follow-bounded-identity-set-v1",
            "proposed_write_object_set_sha256": digest_bytes(canonical([])),
            "proposed_write_object_count": 0,
            "max_existing_descendants": 4096,
        },
    }


def run_filesystem_red_case(case: dict[str, Any]) -> str:
    try:
        with tempfile.TemporaryDirectory(prefix="hul-alias-probe-") as temporary:
            root = Path(temporary)
            allowed = root / "allowed"
            allowed.mkdir()
            setup = case["setup"]
            proposed_path = case["proposed_path"]
            if setup in {"hardlink-target", "hardlink-descendant"}:
                protected = root / "Cargo.toml"
                protected.write_bytes(b"protected\n")
                alias = allowed / ("alias.toml" if setup == "hardlink-target" else "child.toml")
                os.link(protected, alias)
            elif setup == "symlink-descendant":
                protected = root / "Cargo.toml"
                protected.write_bytes(b"protected\n")
                os.symlink("../Cargo.toml", allowed / "child.toml")
            elif setup == "special-descendant":
                os.mkfifo(allowed / "child.fifo", mode=0o600)
            elif setup == "post-validation-hardlink-insertion":
                target = allowed / "target.toml"
                target.write_bytes(b"target\n")
                scope = filesystem_probe_scope(proposed_path)
                rows = proposed_write_identity_rows(root, scope)
                scope["path_resolution"]["proposed_write_object_set_sha256"] = digest_bytes(
                    canonical(rows)
                )
                scope["path_resolution"]["proposed_write_object_count"] = len(rows)
                validate_proposed_write_identity_binding(root, scope)
                os.link(target, root / "transient-alias.toml")
            else:
                raise EnvelopeError(
                    "unknown_filesystem_fixture_setup",
                    f"unknown filesystem fixture setup: {setup}",
                )
            scope = filesystem_probe_scope(proposed_path)
            try:
                proposed_write_identity_rows(root, scope)
            except EnvelopeError as error:
                return error.code
            raise EnvelopeError(
                "filesystem_red_fixture_unexpectedly_accepted",
                f"filesystem red fixture unexpectedly accepted: {case['case_id']}",
            )
    finally:
        pass


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
    valid_envelopes = {"valid_baseline": baseline}
    for variant in fixtures["valid_variants"]:
        candidate = copy.deepcopy(baseline)
        for mutation in variant["mutations"]:
            set_pointer(candidate, mutation["pointer"], mutation["value"])
        refresh_authority_binding(root, candidate)
        refresh_derived_bindings(root, candidate)
        variant_errors = list(validator.iter_errors(candidate))
        if variant_errors:
            raise EnvelopeError(
                "valid_variant_schema_failure",
                f"{variant['variant_id']}: {variant_errors[0].message}",
            )
        semantic_validate(root, candidate)
        valid_envelopes[variant["variant_id"]] = candidate
    run_external_effect_positive(validator, baseline)
    outcomes = []
    for case in fixtures["red_cases"]:
        candidate = copy.deepcopy(valid_envelopes[case.get("base", "valid_baseline")])
        for mutation in case["mutations"]:
            set_pointer(candidate, mutation["pointer"], mutation["value"])
        schema_errors = list(validator.iter_errors(candidate))
        expected_layer = case.get("expected_layer", "schema")
        semantic_error: EnvelopeError | None = None
        if expected_layer == "schema":
            if not schema_errors:
                raise EnvelopeError(
                    "red_fixture_wrong_rejection_layer",
                    f"red fixture did not fail schema as required: {case['case_id']}",
                )
        elif expected_layer == "semantic_resolver":
            if schema_errors:
                raise EnvelopeError(
                    "red_fixture_wrong_rejection_layer",
                    f"red fixture failed schema before semantic resolver: {case['case_id']}",
                )
            try:
                semantic_validate(root, candidate)
            except EnvelopeError as error:
                semantic_error = error
            if semantic_error is None:
                raise EnvelopeError(
                    "red_fixture_unexpectedly_accepted",
                    f"red fixture unexpectedly accepted: {case['case_id']}",
                )
            if semantic_error.code != case["expected_rejection"]:
                raise EnvelopeError(
                    "red_fixture_wrong_causal_rejection",
                    f"{case['case_id']}: expected {case['expected_rejection']}, observed {semantic_error.code}",
                )
        else:
            raise EnvelopeError(
                "red_fixture_invalid_expected_layer",
                f"unknown expected layer: {expected_layer}",
            )
        outcomes.append(
            {
                "case_id": case["case_id"],
                "rejected_by": expected_layer,
                "rejection": case["expected_rejection"],
            }
        )
    for case in fixtures["external_effect_authority_cases"]:
        observed = run_external_effect_red_case(case)
        if observed != case["expected_rejection"]:
            raise EnvelopeError(
                "red_fixture_wrong_causal_rejection",
                f"{case['case_id']}: expected {case['expected_rejection']}, observed {observed}",
            )
        outcomes.append(
            {
                "case_id": case["case_id"],
                "rejected_by": "semantic_resolver",
                "rejection": observed,
            }
        )
    for case in fixtures["filesystem_red_cases"]:
        observed = run_filesystem_red_case(case)
        if observed != case["expected_rejection"]:
            raise EnvelopeError(
                "red_fixture_wrong_causal_rejection",
                f"{case['case_id']}: expected {case['expected_rejection']}, observed {observed}",
            )
        outcomes.append(
            {
                "case_id": case["case_id"],
                "rejected_by": "semantic_resolver",
                "rejection": observed,
            }
        )
    if len(outcomes) != fixtures["expected_red_count"]:
        raise EnvelopeError(
            "red_fixture_count_mismatch",
            f"expected exactly {fixtures['expected_red_count']} red fixtures",
        )
    run_id = digest_bytes(
        canonical(
            {
                "schema_sha256": file_digest(root, str(SCHEMA_PATH.relative_to(root))),
                "fixture_sha256": file_digest(root, str(FIXTURE_PATH.relative_to(root))),
                "candidate_id": baseline["candidate_identity"]["candidate_id"],
                "outcomes": outcomes,
            }
        )
    )
    print(
        json.dumps(
            {
                "schema_version": "AgentTaskEnvelopeRedFixtureResult-v1",
                "valid_envelopes": sorted(valid_envelopes),
                "red_cases": outcomes,
                "red_rejected": len(outcomes),
                "red_total": len(outcomes),
                "schema_rejected": sum(
                    outcome["rejected_by"] == "schema" for outcome in outcomes
                ),
                "semantic_rejected": sum(
                    outcome["rejected_by"] == "semantic_resolver" for outcome in outcomes
                ),
                "run_id": run_id,
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
