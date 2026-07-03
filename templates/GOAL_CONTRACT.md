# Goal Contract

## Objective

State the objective in observable terms.

## Claim Ceiling

The claim ceiling is typed. Keep this block aligned with `schemas/schema-authority-primitives.schema.json#/$defs/claimCeilingEntry`.

```json
[
  {
    "claim_id": "CLAIM-001",
    "status": "contract_only",
    "claim_surface": "static",
    "effect": "blocked",
    "evidence_ids": ["EV-001"],
    "source_manifest_path": "COMPLETION_MANIFEST.json"
  }
]
```

## Required Claim IDs

- `CLAIM-001`: ...

## Contract Bundle

- `GOAL_CONTRACT.md`
- `LANE_REGISTRY.json`
- `VERIFICATION_BACKLOG.json`
- `COMPLETION_MANIFEST.json`
- `AMENDMENTS.jsonl`
- `RED_FIXTURES.json`
- lane ExecPlans

## Generated Evidence, Not Authority

- validator receipts
- ready-for-merge receipts
- red fixture reports

## Hashes

- `contract_bundle_hash`: TBD
- `required_claim_ids_hash`: TBD

## Goal Tool Binding

Use `schemas/goal-binding.schema.json`.

- status:
- goal id:
- contract path:
- get_goal receipt path and digest:
- create_goal receipt path and digest:
- gap reason if unavailable or blocked:

## Completion Requirements

- all required claims classified;
- positive claims backed by acceptable evidence;
- verification backlog closed or claim ceiling reduced;
- no stale worktrees/sessions;
- validator schema and semantic checks pass; red fixtures fail for the intended reasons;
- final report written.

## Amendments

Authoritative amendments live only in `AMENDMENTS.jsonl` and validate against `schemas/contract-amendment.schema.json`. This section may contain only the current amendment-log digest or a generated summary.

- latest_amendment_log_digest: TBD
