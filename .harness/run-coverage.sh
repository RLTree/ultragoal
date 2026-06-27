#!/usr/bin/env bash
set -euo pipefail

root="${1:-${CODEX_WORKTREE_PATH:-$PWD}}"
cd "$root"

mkdir -p validation_artifacts/coverage

report="validation_artifacts/coverage/llvm-cov-full.json"
missing_report="validation_artifacts/coverage/missing-lines.txt"
receipt="validation_artifacts/coverage/coverage-receipt.json"
started_at="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

cargo llvm-cov clean --workspace
cargo llvm-cov --workspace --all-features --json \
  --output-path "$report" --offline
cargo llvm-cov report --text --show-missing-lines \
  --output-path "$missing_report" --offline

completed_at="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

python3 - "$started_at" "$completed_at" "$report" "$receipt" <<'PY'
import hashlib
import json
import subprocess
import sys
from pathlib import Path

started_at, completed_at, report_path, receipt_path = sys.argv[1:5]
root = Path.cwd().resolve()
manifest_path = Path(".harness/coverage-manifest.json")
command_path = Path(".harness/coverage-command")
report_path = Path(report_path)
receipt_path = Path(receipt_path)

def sha_file(path):
    h = hashlib.sha256()
    with open(path, "rb") as fh:
        for chunk in iter(lambda: fh.read(1024 * 1024), b""):
            h.update(chunk)
    return "sha256:" + h.hexdigest()

def digest_files(paths):
    h = hashlib.sha256()
    for raw in sorted(set(paths)):
        path = (root / raw).resolve()
        h.update(raw.encode())
        h.update(b"\0")
        with open(path, "rb") as fh:
            for chunk in iter(lambda: fh.read(1024 * 1024), b""):
                h.update(chunk)
        h.update(b"\0")
    return "sha256:" + h.hexdigest()

def ignored(raw, manifest):
    if raw in {
        ".harness/coverage-manifest.json",
        ".harness/coverage-command",
        "templates/.harness/coverage-manifest.json",
        "templates/.harness/coverage-command",
    }:
        return True
    for pattern in manifest.get("source_discovery_rules", {}).get("ignore", []):
        if pattern.endswith("/**"):
            prefix = pattern[:-3]
            if raw == prefix or raw.startswith(prefix + "/"):
                return True
        elif raw == pattern:
            return True
    return False

def source_files(manifest):
    out = []
    for target in manifest["required_target_paths"]:
        target_path = (root / target).resolve()
        if target_path.is_file():
            out.append(target)
            continue
        for path in target_path.rglob("*"):
            if path.is_file():
                raw = path.relative_to(root).as_posix()
                if not ignored(raw, manifest):
                    out.append(raw)
    return out

with open(manifest_path, encoding="utf-8") as fh:
    manifest = json.load(fh)
with open(report_path, encoding="utf-8") as fh:
    report = json.load(fh)

totals = report["data"][0]["totals"]
files = report["data"][0].get("files", [])
uncovered = []
for item in files:
    summary = item.get("summary", {})
    lines = summary.get("lines", {})
    if lines.get("percent", 100) < 100:
        uncovered.append({
            "path": str(Path(item["filename"]).resolve().relative_to(root)),
            "reason": f"line coverage {lines.get('percent', 0):.2f}%",
            "owner": "repo-owner",
            "blocker_or_debt_id": "coverage-100-plugin-self-law"
        })

pkg = subprocess.run(
    [
        "cargo",
        "run",
        "--offline",
        "--bin",
        "ultragoal-validator",
        "--",
        "--root",
        ".",
        "package-digest",
    ],
    text=True,
    stdout=subprocess.PIPE,
    stderr=subprocess.DEVNULL,
)
target_value = pkg.stdout.strip() if pkg.returncode == 0 else "unavailable"

dimensions = sorted({
    dim
    for row in manifest["required_measured_dimensions_per_root"]
    for dim in row["dimensions"]
})

receipt = {
    "schema": "harness-ultragoal.coverage-receipt.v1",
    "claim_id": "CLAIM-001",
    "command": "bash .harness/run-coverage.sh",
    "tool": "cargo-llvm-cov",
    "source_tree_digest": digest_files(source_files(manifest)),
    "coverage_manifest_digest": sha_file(manifest_path),
    "coverage_command_digest": sha_file(command_path),
    "changed_files_digest": digest_files(
        manifest["changed_file_coupling_policy"]["changed_files"]
    ),
    "tool_version": subprocess.check_output(
        ["cargo", "llvm-cov", "--version"], text=True
    ).strip(),
    "workspace_root": str(root),
    "target_revision": {
        "kind": "package_digest",
        "value": target_value
    },
    "command_started_at": started_at,
    "command_completed_at": completed_at,
    "command_exit": 0,
    "machine_readable_report": {
        "path": report_path.as_posix(),
        "digest": sha_file(report_path)
    },
    "generated_by": "coverage-command",
    "percent_source": "machine_readable_report",
    "target_paths": manifest["required_target_paths"],
    "measured_dimensions": dimensions,
    "coverage": {
        "percent": totals["lines"]["percent"],
        "floor_percent": 100,
        "policy": "100_percent_required",
        "owner": "repo-owner",
        "reason": "Harness Ultragoal plugin self-law requires exact 100% coverage.",
        "blocker_or_debt_id": "coverage-100-plugin-self-law"
    },
    "uncovered_records": uncovered,
    "exclusions": manifest.get("exclusions", []),
    "generated_at": completed_at,
    "claim_ceiling": (
        "supports_complete_claim"
        if totals["lines"]["percent"] == 100 and not uncovered
        else "withheld_or_blocked"
    )
}

receipt_path.write_text(json.dumps(receipt, indent=2) + "\n", encoding="utf-8")
PY
