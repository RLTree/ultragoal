# Harness Ultragoal Successor Contract — Read First

## Status and claim ceiling

This is the final **proposed successor contract and Codex Ultra implementation handoff**, produced from an independently verified repository snapshot and current primary-source research. It is normative inside this handoff, but adoption and implementation in the live repository are **not verified**. The only valid claim from this bundle is that a snapshot-based review and handoff were produced.

The archived ZIP was read beginning with its `00-READ-ME-FIRST.md`. ZIP CRC and path safety passed. The candidate contract's 17 listed files and content-set digest passed; all 5,011 tracked source digests passed; all 3,499 fixture digests passed. No supplied archive-wide manifest covered 249 copied metadata/evidence/research/legacy entries, so those remain unverified context. See `SNAPSHOT-INTEGRITY-REPORT.json`.

## Runtime record

```json
{
  "expected_model_family": "GPT-5.6",
  "expected_mode": "Pro",
  "runtime_configuration_verified": true,
  "basis": "The client exposed the model family and Pro mode to this review runtime. This record does not attest the later Codex Ultra runtime."
}
```

This record applies only to the review runtime exposed by the client. It does not attest the later local Codex project. The Ultra root must record only runtime metadata exposed there and lower its claim ceiling if the requested configuration cannot be confirmed.

## Authority order

1. Human external authority and destructive-action decisions.
2. The adopted live copy of this successor contract and its machine-readable manifest.
3. Stable product laws in `01-AUTHORITY-AND-PRODUCT-LAWS.md`.
4. Granular requirements in `REQUIREMENT_TRACE.json`.
5. Product-surface, tool, dependency, migration, source, and claim registries.
6. Generated projections, documentation, reports, receipts, and telemetry, which never author higher authority.

A validator's presence does not make a law binding. Four states are separate: **normative adoption**, **implementation**, **proof eligibility**, and **claim decision**.

## Load order

1. `CONTRACT_MANIFEST.json`
2. `01-AUTHORITY-AND-PRODUCT-LAWS.md`
3. `PRODUCT_SURFACE_INVENTORY.json`
4. `CUSTOM_TOOL_INVENTORY.json`
5. `CLAIM_REGISTRY.json`
6. `IMPLEMENTATION_DEPENDENCY_GRAPH.json`
7. `REQUIREMENT_TRACE.json`
8. `MIGRATION-AND-RETIREMENT.md`
9. `PRO-ADVERSARIAL-REVIEW.md`
10. `OPEN-DECISIONS.md`
11. `CODEX-ULTRA-BOOTSTRAP.md` and `CODEX-ULTRA-GOAL.md`

## Non-negotiable boundaries

- The plugin is the product; the Rust CLI is its enforcement and acceleration kernel.
- The ZIP and this review are context, not live repository or runtime proof.
- Source, package, installed bytes, host discovery, runtime, product behavior, release, and completion are separate truth surfaces.
- Read/help/version/parse/inspect/next/query operations have zero hidden writes.
- Tests, receipts, generated rows, documentation, telemetry, signatures, or provenance cannot substitute for named behavior.
- Required Harness-owned custom tools precede claims that depend on them.
- The Ultra root alone owns shared authority, integration, merge/rebase, claim decisions, release, and completion.
- Write workers require explicit disjoint semantic leases and independent review.
- Routine dirty-tree work is fast and conservative; strict proof is exact and claim-boundary-specific.
- External authority and destructive decisions stop only dependent claims; no prompt text can manufacture authority or runtime metadata.

## Start implementation

Read `CODEX-ULTRA-BOOTSTRAP.md`, open the actual live Harness Ultragoal repository, and paste the exact content of `CODEX-ULTRA-GOAL.md` without rewriting it.
