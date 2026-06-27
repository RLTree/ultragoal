# Canonical Types

This document names the schema-owned types used across the package. Prose files should reference these types instead of repeating enum definitions.

## ClaimStatus

Defined in `schemas/common-defs.schema.json`.

Positive and non-proven subsets are also defined there. Positive claim ceilings may include only `proven_live` and narrowly scoped `proven_static`.

## ClaimSurface

Defined in `schemas/common-defs.schema.json`.

Surfaces are not interchangeable. CLI proof, API proof, UI proof, package proof, installed-app proof, and root-integration proof carry different claim ceilings.

## EvidenceKind

Defined in `schemas/common-defs.schema.json`.

`live_beneficial_e2e` is required for feature completion claims. `static_check` only supports static claims. `red_fixture` supports validator negative proof.
`product_cohesion_receipt` supports consumer-facing product claims only when it is paired with UI journey evidence.

## ProductCohesionReceipt

Defined in `schemas/product-cohesion-receipt.schema.json`.

Product-cohesion receipts connect a product promise, primary user, user job,
critical journey, surface map, UI/runtime evidence, human-attention policy, and
claim ceiling. They are required when a claim sets
`requires_product_cohesion`, uses `claim_kind: product_cohesion`, or uses
`claim_surface: product_cohesion`.

## GoalBinding

Defined in `schemas/goal-binding.schema.json`.

`bound` requires real goal-tool receipt references. `unavailable` or `blocked` removes goal-bound claims from the claim ceiling.

## Lane Registry

Defined in `schemas/lane-registry.schema.json`.

`LANE_REGISTRY.json` is canonical. Markdown lane registry views are human-readable projections only.

## Verification Backlog

Defined in `schemas/verification-backlog.schema.json`.

Every non-proven claim needs a backlog row with owner, proof requirement, attempt/repro, next action, evidence, and claim-ceiling impact.

## Validator Checks

Defined in `schemas/common-defs.schema.json` as `requiredValidatorCheckId`.

Validator receipts must name the complete required check set. Counting checks is insufficient; `checks` is keyed by check id and every required key must be present.

## Red Fixtures

Defined in `schemas/red-fixture.schema.json` and `schemas/red-fixtures-catalog.schema.json`.

The catalog must include every required red fixture id and each row must point at a concrete packet file with a digest. Red packets are executable JSON Patch specs against their declared valid base fixture; the validator/test runner materializes the bad bundle before semantic validation. Validator receipts key red fixture results by fixture id so duplicate rows cannot fake coverage.

## Semantic Validator Boundary

JSON Schema enforces artifact shape and many local invariants. Cross-document and cross-field obligations are validator-owned: claim/evidence surface matching, product-cohesion proof coupling, backlog row joins, generated ready-artifact provenance, derived amendment deltas, target-repo audit receipts, and live-beneficial non-fixture status. The package validator now covers the included fixtures; non-package/live plugin claims remain blocked until their own receipts and dogfood evidence exist.

## Contract Amendments

Defined in `schemas/contract-amendment.schema.json`.

Clarifying and strengthening amendments can preserve monotonicity. Weakening amendments require explicit user approval and backlog or withheld-claim accounting.
