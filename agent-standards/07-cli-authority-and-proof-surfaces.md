# CLI Authority And Proof Surfaces

## Authority Kernel

Harness law claims are governed by the canonical CLI authority kernel.

- A claim is supported only when the CLI computes or verifies it from typed,
  current inputs and names the claim ceiling.
- Prose, checklist status, reviewer agreement, copied receipts, packet
  existence, source-only proof, install proof, cache proof, local JSON, or
  historical memory cannot close a different surface.
- If the CLI fails, the claim stays failed or blocked until the smallest
  production cause is repaired and the CLI proof passes on the current
  candidate.
- Unknown status strings, catch-all authority states, partial enum values, and
  untyped claim ceilings are authority failures.

## Proof Surface Separation

Proof surfaces are not interchangeable.

- Source, installed plugin, cache package, package sync, app registry,
  reviewer exposure, review target, archive, final packet, live runtime, and
  update-goal eligibility each require same-surface proof.
- Source-local proof may support source-local claims only. It does not imply
  install success, cache freshness, app visibility, marketplace readiness,
  reviewer exposure, daily-driver readiness, release readiness, completion, or
  update-goal eligibility.
- Receipts bind candidate digest, command, operation id, artifact paths,
  tool/runtime identity, claim ids, proof surface, generated time, and claim
  ceiling. Stale, wrong-digest, wrong-surface, copied, missing, or private-path
  receipts block affected claims.

## Coverage Authority

Coverage is source-law proof, not a confidence note.

- Material source claims require exact current coverage unless the claim
  ceiling is withheld.
- Exact coverage means the authoritative command runs on the current candidate,
  every required dimension is at 100 percent, and `uncovered_records = []`.
- Coverage scope is owned by the manifest, repo walk, changed-file coupling,
  freshness binding, policy mutation gate, and fast/full gate separation.
- Fast checks support iteration only. They cannot substitute for full exact
  coverage in material claims.
- Generated, vendor, external, or unreachable exclusions must be mechanically
  justified and must not count as covered.
- Delete or refactor dead fallback code. Do not write tests that preserve
  unreachable product states merely to satisfy hit counts.

## Agent-Readable Failures

CLI and validator failures should reduce investigation, not create archaeology.

- Failures name the failed law/check/claim, current candidate, implicated
  path or receipt, stale or wrong-surface evidence, smallest next repair,
  narrow rerun, and blocked claims.
- Generic output such as "validation failed" or "inspect receipt" is not
  sufficient for law-bearing paths.
- Broad audits are boundary proof. Inner loops use the narrow failing command,
  targeted source inspection, and current telemetry when available.
