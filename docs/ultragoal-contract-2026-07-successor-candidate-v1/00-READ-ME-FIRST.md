# Harness Ultragoal Successor Contract Candidate v1

Status: candidate for independent GPT-5.6 Pro review  
Authored: 2026-07-09  
Authority: non-operative until explicitly adopted  
Scope: one integrated Harness Ultragoal product, including the plugin, the Rust `ultragoal` authority kernel, Harness-owned product tooling, and the complete operating lifecycle

## Runtime record

```yaml
expected_model: GPT-5.6 Sol
expected_reasoning: Extra High
runtime_configuration_verified: false
```

The client exposed the expected values only through the authoring prompt. It did not expose independently verifiable runtime metadata. This candidate therefore makes no claim that the expected runtime authored it.

## What this bundle is

This is the first coherent successor product and operational contract. It is a proposal, not implementation evidence. It does not replace `docs/ultragoal-contract-2026-07/`, modify the plugin, validate an installation, or prove runtime behavior.

The candidate resolves four structural problems in the current system:

1. It treats the plugin and its user journeys as at least co-equal with the CLI.
2. It replaces historical fixed lanes with an adaptive, dependency-aware work-package graph governed by an Ultra root agent.
3. It consolidates duplicate commands and evidence surfaces into one typed authority graph with explicit read/write behavior.
4. It separates product behavior, observations, reconciliation, and completion authority so that a receipt cannot prove itself.

## Reading order

1. `01-CURRENT-SYSTEM-ASSESSMENT.md` — what exists, what is useful, and what is misleading.
2. `02-PRODUCT-CONTRACT.md` — the binding integrated-product laws.
3. `03-PLUGIN-PRODUCT-CONTRACT.md` — acquisition through maintenance and real use.
4. `04-ULTRA-ORCHESTRATION-CONTRACT.md` — adaptive decomposition, ownership, review, recovery, and root authority.
5. `05-CLI-AND-CUSTOM-TOOLING-CONTRACT.md` — the authority kernel and definitive Harness-owned tool model.
6. `06-RESEARCH-AND-LAW-AUTHORITY.md` — source classes, adoption rules, and law provenance.
7. `07-PROOF-CLAIM-AND-EVIDENCE-CONTRACT.md` — claim-specific proof and false-pass rejection.
8. `08-IMPLEMENTATION-DEPENDENCY-ORDER.md` — dependency order and promotion criteria.
9. `09-MIGRATION-AND-RETIREMENT.md` — explicit retention, consolidation, replacement, and retirement.
10. `FOR_SOL_PRO_REVIEW.md` — self-contained independent-review packet.

Machine-readable companions are authoritative for inventory and trace joins:

- `CONTRACT_MANIFEST.json`
- `REQUIREMENT_TRACE.json`
- `PRODUCT_SURFACE_INVENTORY.json`
- `CUSTOM_TOOL_INVENTORY.json`
- `RESEARCH_SOURCE_REGISTRY.json`
- `ZIP_INCLUDE_MANIFEST.json`

`checklist/STATUS.md` is a concise candidate status view. It is not a progress log and contains no mutable receipts.

## Authority order

Until adoption:

1. explicit user instruction;
2. active repository instructions and the active contract;
3. current product source and behavior;
4. this candidate as a review proposal;
5. historical lane records and mutable evidence.

After explicit adoption, the successor product laws and machine-readable requirement trace become the product contract. Runtime truth still outranks documents about runtime truth.

## Stable law set

| Law | Name | Primary owner |
|---|---|---|
| HUL-PRODUCT-001 | Integrated lifecycle product | Product architecture |
| HUL-PLUGIN-001 | Progressive plugin closure | Plugin product |
| HUL-DISCOVERY-001 | Surface-specific discovery truth | Distribution and discovery |
| HUL-FIT-001 | Reconciled setup and retrofit | Repository fitting |
| HUL-STATE-001 | Immutable current-input snapshot | Authority kernel |
| HUL-INCREMENTAL-001 | Verified incremental legality | Incremental engine |
| HUL-AUTHORITY-001 | Single typed authority graph | Authority kernel |
| HUL-PROOF-001 | Independent claim reconciliation | Proof system |
| HUL-COVERAGE-001 | Routine and strict coverage intelligence | Test intelligence |
| HUL-OBSERVE-001 | Queryable semantic observability | Observability |
| HUL-REPAIR-001 | Exact repair and next action | Operator experience |
| HUL-GOAL-001 | Goal lineage without simulation | Goal integration |
| HUL-ORCHESTRATION-001 | Adaptive Ultra work graph | Orchestration |
| HUL-WORKER-001 | Typed worker ownership and acceptance | Orchestration |
| HUL-REVIEW-001 | Independent falsification | Review authority |
| HUL-SECURITY-001 | Confinement, permission, and secret safety | Security |
| HUL-SUPPLY-001 | Reproducible package integrity | Distribution security |
| HUL-CLI-001 | Discoverable, typed, non-surprising CLI | CLI product |
| HUL-EVAL-001 | Valid evaluations and failure harvesting | Agent quality |
| HUL-IMPROVEMENT-001 | Evidence-driven improvement loop | Product improvement |
| HUL-PERFORMANCE-001 | Fast routine work, honest strict proof | Performance |
| HUL-PRIVACY-001 | Telemetry minimization and retention | Privacy |
| HUL-MAINTENANCE-001 | Measured entropy and retirement | Maintenance |
| HUL-COMPLETION-001 | Honest stop and completion ceiling | Root claim authority |

Old gate and law identifiers remain migration aliases only. New implementation work must use product-semantic identifiers.

## Normative language

`MUST` and `MUST NOT` are completion-bearing. `SHOULD` is advisory unless promoted through the law process. `MAY` is optional. A candidate requirement becomes binding only after independent review, explicit adoption, an implementation owner, an executable validator, adversarial fixtures, independent product proof, and a claim guard are all assigned.

## Candidate claim ceiling

Allowed claim: **a successor contract candidate was authored and internally reviewed against current repository evidence and current primary sources.**

Disallowed claims include: implemented, installed, discoverable, fast, secure, runtime-correct, product-complete, or ready for release.
