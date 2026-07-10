# Implementation, Validation, and Quality-in-Use

## Required implementation sequence

1. N00-LIVE-BASELINE: Live baseline and contract adoption
2. N01-CONTEXT-EFFECT: Live context and effect model
3. N02-INVENTORY: Canonical inventory and generated authority
4. N03-CLI-STATE: CLI grammar, typed state, diagnostics, and capture
5. N04-DISTRIBUTION: Package, marketplace, installation, and discovery
6. N05-FIT: Fresh setup and retrofit reconciliation
7. N06-ROUTINE: Routine impact, capture, fixtures, and verified reuse
8. N07-OBSERVABILITY: Local observability, diagnosis, and optional export
9. N09-AGENTS-REPO: Read-only agents and agent-first repository guidance
10. N08-PLUGIN-PRODUCT: Plugin front door, canonical skills, and journeys
11. N10-ORCHESTRATION: Adaptive orchestration and write leases
12. N11-EVAL-RESEARCH: Evaluation, improvement, and research promotion
13. N12-CLAIMS: Claim graph and proof reconciliation
14. N13-REAL-JOURNEYS: Representative product and quality-in-use journeys
15. N14-MIGRATION: Migration routes and compatibility
16. N15-RETIREMENT: Old-surface retirement and authority cleanup
17. N16-RELEASE: Release candidate proof
18. N17-COMPLETION: Integrated goal completion reconciliation

Parallelize only dependency-ready nodes with non-overlapping leases. Implement the twelve Harness-owned tools in tool topological order where dependencies require it: `HCT-CONTEXT -> HCT-CAPTURE -> HCT-INVENTORY -> HCT-FIXTURES -> HCT-DISTRIBUTION -> HCT-IMPACT -> HCT-STATE -> HCT-CLAIMS -> HCT-FIT -> HCT-OBSERVE -> HCT-MIGRATE -> HCT-EVAL`. A downstream claim remains blocked until each required tool's behavior and negative proof passes.

## Validation dimensions

- **Static:** schemas, references, inventories, public routes, ownership, dependency graphs, no duplicate authority.
- **Behavioral:** representative commands, plugin selection, fit, routine work, diagnosis, observability, migration, and product journeys.
- **Negative:** stale, missing, tampered, wrong identity, wrong truth surface, bypass, reward hacking, denied permission, no capability, dirty tree, race, cancellation, and recovery.
- **Quality in use:** fresh user/agent comprehension, time-to-correct-route, repair usefulness, maintainability, and minimal cognitive/ceremonial burden.
- **Security/supply:** confinement, local-work preservation, secrets, trust, deterministic packaging, provenance/signature expectations, install/discovery identity.
- **Claim:** candidate-bound evidence, independent reviewer, exact ceiling, false-pass guard, repair, and rerun.

## Real completion conditions

Completion is realistic only when the live product can be acquired, installed, discovered, entered, fitted to fresh and existing repositories, used routinely on dirty work, diagnosed, observed, evaluated, improved, migrated, released, and maintained; the CLI and custom tools enforce those journeys; old authority is routed or retired; and independent proof for every required claim passes. Missing external authority preserves work and lowers only affected claims.

## Snapshot limitations

This review did not run Rust/Cargo validation because Cargo/Rust were unavailable in the review environment. It did not inspect a live branch, current installed plugin, live Codex project, or current CLI behavior. The Ultra implementation must recompute and test all of those surfaces.
