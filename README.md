# Harness Ultragoal Plugin Proposal Package

This package is a proposal bundle plus the Rust-backed `ultragoal` CLI for a future plugin that turns the strongest practices from the `codex-workflow-rs` repo and the recent large goal run into reusable, reproducible goal-run infrastructure.

The skills, agents, connectors, and dogfood rollout remain proposal-stage. The included Rust validator is implemented for package/schema validation, exactly-once inventory closure, static semantic fixture checks, target-repo fixtures, semantic-classification receipts, and red-fixture proof.

## Package Map

- `REPORT.md`: detailed rationale, evidence, architecture, review method, and recommended plugin composition.
- `plugin-manifest-draft.json`: proposed plugin inventory and relationships.
- `.codex-plugin/plugin.json`: Codex plugin manifest for local marketplace testing.
- `install/personal-marketplace.example.json`: personal marketplace entry shape.
- `skills/`: proposed skill contracts.
- `agents/`: proposed review personas and supporting agent contracts.
- `custom-agents/`: Codex custom-agent TOML files intended for app-visible subagents; app visibility still requires install/reload proof.
- `templates/`: proposed repo-local contract templates.
- `templates/agent-standards/`: decomposed standards modules routed by
  `templates/AGENT_STANDARDS.md`.
- `schemas/`: proposed JSON schemas plus `schema-catalog.json` for offline resolver binding.
- `connectors/CONNECTORS.md`: connector stance and optional adapter boundaries.
- `docs/source-article-synthesis.md`: how the foundation articles shaped the proposal.
- `docs/plugin-resource-map.md`: product-cohesion map for when agents should
  use each skill, persona, template, receipt, and gate.
- `docs/codex-worktree-environment.md`: Codex app worktree environment setup,
  generated `.codex-worktree/` state, and toolbar action contract.
- `docs/observability-stack.md`: optional observability setup and proof surface
  for runtime claims that cite logs, metrics, traces, or agent context.
- `docs/repo-patterns-extracted.md`: reusable standards extracted from `codex-workflow-rs`.
- `docs/agent-first-repo-shape.md`: fresh repo and retrofit baseline shape.
- `docs/product-cohesion-gate.md`: conditional product/UX cohesion gate for consumer-facing changes.
- `docs/install-and-visibility.md`: Codex local plugin and custom-agent install route.
- `docs/implementation-roadmap.md`: staged implementation plan and acceptance gates.
- `docs/schema-resolver.md`: offline schema resolver contract.
- `docs/review-target-and-archive.md`: detached review-target and deterministic zip receipt procedure.
- `docs/review-loop-record.md`: compact review-history index; typed review
  receipt fixtures live under `fixtures/review-round/`.
- `validator/`: Rust canonical validator crate. Invoke with `cargo run --offline -- --root <package> audit --receipt <receipt.json>`.
- `Cargo.toml`: workspace entrypoint for the Rust validator.
- `validation_artifacts/ultragoal-audit/`: generated validator receipt and red-fixture report from the current package.

## Design Intent

The plugin should create high-trust goal runs where:

- the goal is a real runtime object, not a chat convention;
- long work is bound to restartable ExecPlans;
- lanes are macro-sized, isolated, non-overlapping, and proof-bound;
- the parent acts as orchestrator and reconciler unless explicitly registered as a lane;
- every completion claim maps to a typed claim id and evidence surface;
- consumer-facing product claims map to a product journey, user promise, UI proof, and human-attention policy;
- skipped checks become lane-owed, root-owed, externally-blocked, or withheld-claim;
- live beneficial end-to-end proof is required for feature completion claims;
- stale sessions, stale worktrees, stale receipts, and hand-written readiness are rejected;
- standards are enforced mechanically, with prose serving as routing and explanation.
- operating law is progressively disclosed through a compact standards router
  and semantically named standards modules, not one always-loaded mega
  document.

## Reviewer Instructions

Review this package as a contract and product spec. The highest-value feedback is not copyediting. Look for any place an agent could:

- treat a weak run as complete;
- hide unverified work behind generic status text;
- substitute one proof surface for another;
- treat engine/runtime proof as a substitute for product journey proof;
- overuse human handoff states instead of preserving agent autonomy;
- leave stale worktrees or sessions behind;
- launch tiny or overlapping lanes;
- drift from live repo truth into memory or chat lore;
- avoid installing, provisioning, escalating, or running checks it can actually run;
- produce a bulky process artifact that agents will ignore.

The included review loop record should be considered part of the evidence packet, not proof that the proposal is final. Current sign-off must bind to the detached review-target digest and current validator receipt described in `docs/review-target-and-archive.md`. The current validator pass is package/static proof only; local install visibility and real multi-lane dogfood require separate receipts before V1 claims.
