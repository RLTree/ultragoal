# Independent GPT-5.6 Pro Review Packet

## Review request

Review this as the proposed successor product and operational contract for Harness Ultragoal. Do not review it as an implementation or as evidence that the current product works.

The review goal is to falsify the candidate’s coherence, completeness, authority boundaries, product usability, implementation order, proof model, and retirement safety before adoption and before GPT-5.6 Sol Ultra coordinates implementation in Codex.

## Runtime record

```yaml
expected_model: GPT-5.6 Sol
expected_reasoning: Extra High
runtime_configuration_verified: false
```

The authoring client did not expose independent runtime metadata. The prompt’s expected values were not treated as proof.

## Candidate status and allowed claim

Allowed claim: a successor contract candidate was authored and internally reviewed against an authoring-time repository inventory and current primary-source research.

Not claimed: implementation, installation, discovery, cache correctness, application behavior, runtime behavior, performance, security, release readiness, or product completion.

The current `docs/ultragoal-contract-2026-07/` remains active. This candidate is isolated under `docs/ultragoal-contract-2026-07-successor-candidate-v1/` and requires explicit adoption after review.

## Mission restatement

Harness Ultragoal is one integrated product:

1. a plugin containing discovery metadata, progressive skills, agents, templates, repository setup/retrofit, product guidance, observability, research, maintenance, and real-use journeys;
2. a Rust `ultragoal` CLI/library serving as the fast, typed, non-bypassable authority kernel;
3. Harness-owned product tooling for current inputs, product truth, affected-set legality, verified reuse, evidence reconciliation, repair, and claims;
4. the complete lifecycle from acquisition through repository fitting, routine work, diagnosis, strict proof, improvement, maintenance, and honest goal completion.

The CLI supports and enforces the plugin. It is not the whole product.

## Current-system evidence reviewed

The author inspected the routed active contract and the requested repository families, including plugin manifests, skills, agents, custom agents, templates, schemas, validator source, scripts, install guidance, observability development stack, research registries/traces, source snapshots, fixture catalogs/generators/schemas, representative red/green/tamper fixtures, and historical lane state.

Authoring-time observations:

- 5,011 tracked files;
- 1,178 Rust files and roughly 141,102 Rust lines;
- roughly 3,499 tracked fixture files;
- roughly 10,050 lines in the routed active contract;
- 84 schemas, 68 templates, 14 skills, 7 Markdown agents, and 7 custom-agent TOML definitions;
- 1,382 resources declared in the draft plugin manifest;
- about 9.6 GB of mutable validation artifacts and about 32 GB of Rust build cache on disk;
- broad custom tooling exists, but authority is fragmented among audit, product, standards, coverage, package, observability, review, final-packet, and gate-specific surfaces;
- many fixtures validate declared Boolean fields or row shape rather than independently exercised product behavior;
- the plugin has broad assets but no single reconciled acquisition-to-completion journey;
- the active contract contains long mutable history and fixed lane/orchestration assumptions;
- source, package, install, cache, application, and runtime distinctions are present in doctrine but not closed through one executable identity chain;
- the development observability stack is useful optional substrate but should not be required for routine local correctness.

One concrete command-effect defect was observed during authoring: the read-shaped `ultragoal --root . package digest` wrote `validation_artifacts/observability/package-digest.json`. That newly written artifact was excluded from assessment evidence and was not cleaned up because the task forbade artifact mutation. The successor therefore makes query side-effect freedom structural, not conventional.

## Proposed law set

| Law | Binding outcome |
|---|---|
| HUL-PRODUCT-001 | Plugin, CLI, tools, and lifecycle are one completion graph |
| HUL-PLUGIN-001 | Concise front door and progressively disclosed journey closure |
| HUL-DISCOVERY-001 | Separate and reconcile source/package/install/cache/app/runtime truth |
| HUL-FIT-001 | Inspect, plan, explicitly apply, preserve, and verify setup/retrofit |
| HUL-STATE-001 | Every authority operation consumes one immutable AuditContext |
| HUL-INCREMENTAL-001 | Affected subsets and cache reuse require verified dependency legality |
| HUL-AUTHORITY-001 | One typed graph owns findings, repair, state, next, and claims |
| HUL-PROOF-001 | Completion evidence requires independent congruent reconciliation |
| HUL-COVERAGE-001 | Routine and strict coverage remain separate scopes and claims |
| HUL-OBSERVE-001 | Local semantic observability is zero-dependency; export is separately proven |
| HUL-REPAIR-001 | Every blocker has a smallest repair, exact rerun, effect, and ceiling |
| HUL-GOAL-001 | Integrate platform goal durability without simulating platform state |
| HUL-ORCHESTRATION-001 | Root derives adaptive product-semantic work packages from current state |
| HUL-WORKER-001 | Workers have bounded ownership; root independently accepts and reconciles |
| HUL-REVIEW-001 | Material claims receive non-authoring falsification |
| HUL-SECURITY-001 | Effects, paths, permissions, secrets, and external boundaries fail closed |
| HUL-SUPPLY-001 | Source/build/package/install/application identity is reproducibly reconciled |
| HUL-CLI-001 | Typed, conventional, discoverable commands have structural effect safety |
| HUL-EVAL-001 | Evaluation task/test/scoring quality precedes authority |
| HUL-IMPROVEMENT-001 | Failures become scoped, evaluated, reviewed, reversible improvements |
| HUL-PERFORMANCE-001 | Routine work is affected-set fast; strict proof reports real scope/reuse |
| HUL-PRIVACY-001 | Telemetry is minimized, classified, bounded, redacted, retained, and deleted |
| HUL-MAINTENANCE-001 | Obsolete authority and entropy retire only after replacement proof |
| HUL-COMPLETION-001 | Root issues the minimum current reconciled ceiling or an honest stop |

Every entry in `REQUIREMENT_TRACE.json` maps source -> principle -> stable law -> owner -> implementation surface -> validator -> fixtures -> product proof -> claim guard -> completion blocker.

## Plugin contract summary

The plugin is treated as the user- and agent-facing product operating system, not a resource container. The candidate specifies these behavioral journeys:

- PJ-ACQUIRE-001 — find the intended origin/version;
- PJ-INSTALL-001 — install and reconcile bytes;
- PJ-DISCOVER-001 — register and discover in the running application;
- PJ-ENTER-001 — understand the first action without internal jargon;
- PJ-FIT-FRESH-001 — minimally fit a new repository;
- PJ-FIT-RETROFIT-001 — preserve and reconcile an existing repository;
- PJ-ROUTINE-001 — perform fast legal checks on a dirty tree;
- PJ-DIAGNOSE-001 — explain failure and compile exact repair;
- PJ-PROVE-001 — execute strict surface-appropriate proof;
- PJ-ORCHESTRATE-001 — coordinate and recover adaptive multi-package work;
- PJ-IMPROVE-001 — turn real failure into independently promoted improvement;
- PJ-RESEARCH-001 — refresh primary authority and propose traced product law;
- PJ-MAINTAIN-001 — upgrade, clean, migrate, and retire safely;
- PJ-COMPLETE-001 — issue an honest integrated ceiling or stop.

Each journey has an entry, behavior, independent proof, false-pass cases, and repair. Installation/discovery uses a non-substitutable ladder: source -> package -> installed tree -> host cache -> application registry -> new-task front door -> selected skill load -> representative runtime journey.

The proposed skill topology consolidates overlapping current skills into product capabilities for entry, repository fitting, routine work, proof, observation/diagnosis, improvement, and maintenance. The proposed agent topology uses one canonical role definition with generated host presentations. Historical model pins and fixed lane instructions retire.

## CLI contract summary

Proposed public surface:

```text
ultragoal status [--json]
ultragoal next [--json]
ultragoal fit inspect|plan|apply|verify
ultragoal check [--changed|--all] [--json]
ultragoal prove --surface SURFACE [--json] [--emit PATH]
ultragoal package snapshot|verify|diff
ultragoal observe query|explain
ultragoal goal inspect|reconcile
ultragoal maintain plan|apply|verify
ultragoal help [COMMAND]
ultragoal version [--json]
```

The key architectural rule is compile-time command-effect separation. A `read_only` handler cannot obtain repository/artifact/install/cache/external writers. Mutation appears through an explicit verb or `--emit` path and a typed plan. Internal gate-specific validators remain library selectors or compatibility views, not competing completion commands.

Routine `check` and strict `prove` declare product-managed build/query cache writes; `prove --emit` additionally declares an artifact write. Cargo target output, coverage profiles, fixture state, and subprocess mutations cannot be laundered through a read-only parent. Status, next, observe queries/explanations, package verification/diff, goal inspection, and verification views remain structurally read-only.

The candidate recommends a mature parser such as `clap`, Cargo/nextest/rustc/llvm-cov for execution substrate, Serde for contracts, and tracing/OpenTelemetry for instrumentation. Harness-owned code remains responsible for product surfaces, affected legality, cache validity, evidence congruence, repair, and claim authority.

## Required Harness-owned tools

The definitive inventory contains 19 tools:

1. CT-AUDIT-CONTEXT — immutable current-input snapshot;
2. CT-SURFACE-SPEC — product-surface input specifications;
3. CT-INCREMENTAL-GRAPH — verified incremental query graph;
4. CT-NODE-CACHE — node-local cache/invalidation authority;
5. CT-SEMANTIC-INVENTORY — component/symbol/dependency inventory;
6. CT-PACKAGE-SNAPSHOT — normalized package truth;
7. CT-COVERAGE — routine and strict coverage intelligence;
8. CT-IMPACTED-TESTS — affected test legality;
9. CT-FIXTURE-SCHEDULER — isolated behavioral fixture execution;
10. CT-COMMAND-CAPTURE — uninterpreted structured subprocess facts;
11. CT-TELEMETRY-RECONCILE — local/export roundtrip reconciliation;
12. CT-ARTIFACT-DEREF — typed evidence identity and congruence;
13. CT-CURRENT-STATE — one read model;
14. CT-NEXT — deterministic exact next action;
15. CT-OBSERVE-REPAIR — bounded query/explain/repair compiler;
16. CT-PLUGIN-DISCOVERY — package/install/cache/app/runtime reconciliation;
17. CT-FIT-RECONCILE — setup/retrofit inspect-plan-apply-verify;
18. CT-AGENT-EVAL — eval quality and failure harvesting;
19. CT-CLAIM-GRAPH — single completion authority graph.

For each, `CUSTOM_TOOL_INVENTORY.json` names current status, product role, current surfaces, substrate, independent proof, duplicates/replacements, retirement work, and owner.

## Proof model summary

Validation, product behavior, artifact integrity, observability, speed, and completion are separate truths. Every completion-bearing claim specifies:

- product behavior;
- evidence surface;
- independent reconciler;
- false-pass example;
- fail-closed diagnostic;
- stale/wrong-surface rejection;
- smallest repair;
- exact rerun;
- maximum ceiling.

Defined claims cover source, package, install, cache, application discovery, runtime journey, repository fitting, routine development, strict coverage, observability, orchestration, evaluation/improvement, real product journeys, and integrated completion.

Claim ceilings are:

```text
description_only
source_present
observation_only
routine_repair
surface_reconciled
journey_reconciled
strict_candidate
integrated_completion_candidate
released_product
```

The integrated ceiling is the minimum of every applicable required claim. Unknown, blocked, stale, invalid, or lower-ceiling claims cannot be averaged away.

## Ultra orchestration summary

The Sol Ultra root agent is the sole integration and claim authority. It:

1. freezes a discovery context;
2. evaluates product laws and surfaces;
3. groups current findings into semantic work packages;
4. computes prerequisites, conflicts, and shared authority;
5. uses read-only reconnaissance/adversarial review while contracts are unstable;
6. permits write workers only after ownership and acceptance stabilize;
7. serializes shared authority and external state;
8. independently accepts worker output;
9. recomputes the graph after material changes;
10. reconciles, reviews, verifies, cleans up, retires, and issues the minimum ceiling.

Safety classes:

- `parallel_read_only`;
- `parallel_disjoint_write`;
- `serial_shared_authority`;
- `serial_external_state`;
- `root_only_claim`.

Path separation is insufficient when two workers affect the same schema, generator, public command, package inventory, semantic authority, or evidence identity. Worker statuses cannot include accepted/integrated/complete. Root acceptance records diff review, ownership reconciliation, reproduced acceptance, invalidations, merge order, and claim effect.

Recovery is explicit for stale, blocked, drifting, incomplete, conflicting, and interrupted-root cases. Historical lane state is never required to reconstruct current work.

## Implementation dependency order

1. Independent review/adoption and canonical schemas.
2. AuditContext and surface input specs.
3. Semantic inventory and verified query graph.
4. Findings, repairs, current state, and next.
5. Command capture, artifact dereference, and verified cache.
6. Consolidated CLI and claim graph.
7. Package/distribution truth.
8. Install/discovery and repository fitting.
9. Routine checks, impacted tests, coverage, and behavioral fixtures.
10. Local observability and optional export reconciliation.
11. Adaptive orchestration and recovery.
12. Plugin journeys and real-use validation.
13. Evaluation/improvement loop.
14. Strict proof and integrated completion authority.
15. Migration, cleanup, and retirement.

This is a semantic dependency graph. The root chooses package instances and parallelism from current state; it does not generate fixed historical lanes.

## Migration summary

Pre-89 gates map by semantic family into the 24 laws, with individual active rows required to join before retirement. Gates 89–105 have explicit mappings:

- 89 -> typed CLI/authority/proof;
- 90 -> semantic authority/naming/maintenance;
- 91 -> CLI/performance/maintenance;
- 92 -> observability/repair/state/incremental/performance;
- 93 -> research-law provenance;
- 94 -> evaluation/improvement;
- 95 -> generic external-tool security/privacy/eval boundary;
- 96 -> optional evaluation adapter, not Promptfoo authority;
- 97 -> optional ranking adapter, not HALO authority;
- 98 -> evaluated domain improvement without self-certification;
- 99 -> one setup/retrofit engine;
- 100 -> per-repository reconciled rollout;
- 101 -> detected-stack fitting, not universal language-guide law;
- 102 -> centralized privacy/security/observability policy;
- 103 -> core wrong-surface rejection;
- 104 -> one requirement graph with generated views;
- 105 -> measured improvement and entropy control.

`LANE_REGISTRY.json` and `.codex/lane-registry/**` become history. Duplicate claim commands, mutable checklist history, embedded script policy, Boolean-only law fixtures, hidden receipt writers, obsolete model pins, and mandatory vendor adapters retire only after replacement proof and one compatibility period.

## Research posture

The candidate refreshed first-party sources for OpenAI Harness Engineering, Codex goals/subagents/skills/plugins, GPT-5.6/Ultra, Symphony, self-improving tax agents, and coding-evaluation data quality; Google SRE monitoring; OpenTelemetry semantic conventions and baggage; Cargo/nextest/rustc/llvm-cov; clap/Rust CLI practices; SLSA; and Sigstore.

The bundle separates primary authoritative facts, primary empirical findings, secondary advice, repository observations, experimental hypotheses, and rejected ideas. Package-authored summaries do not inherit primary-source authority. Drift-prone host/model/tool facts must be refreshed before release.

All 15 entries in the active repository research registry have an explicit successor disposition. Provider/practitioner sources and attached guides remain advisory or stack-conditional; the two local gold-standard syntheses are routing/history, not substitutes for their primary sources or current product proof.

## Required review method

1. Validate every JSON artifact parses and every referenced law/source/surface/tool exists.
2. Search the bundle for duplicate definitions of readiness, current state, next action, or completion.
3. Attempt to construct a valid-shape receipt that passes without product behavior.
4. Attempt source/package/install/cache/app/runtime substitution.
5. Attempt a read-only command with a hidden writer or transparent artifact mutation.
6. Attempt an affected-set false negative and unverifiable cache replay.
7. Attempt a plugin journey that shortcuts application discovery or repository guidance.
8. Attempt semantic worker conflict across disjoint paths and worker self-acceptance.
9. Attempt stale, blocked, drifting, incomplete, and interrupted-root recovery.
10. Attempt secret/private path/high-cardinality leakage through local and exported telemetry.
11. Attempt package integrity proof with wrong runtime behavior and runtime behavior with wrong package identity.
12. Attempt improvement promotion with a broken eval, score-only evidence, or adjacent regression.
13. Check implementation order for cycles, missing prerequisites, or retirement before replacement.
14. Check whether the contract is usable by an operator without learning historical gates or receipt locations.
15. Check whether plugin journeys receive at least equal analytical and proof attention to the CLI.

## Specific questions for GPT-5.6 Pro

Return material findings first, with severity, affected law/artifact, false-pass or user harm, and smallest contract repair.

1. Is any binding law too broad to validate independently or too narrow to own its product behavior?
2. Does any source fact overreach the cited authority or convert advice into law without product justification?
3. Is any product family or lifecycle transition absent from the product-surface inventory?
4. Can any receipt, generated row, test, telemetry event, signature, or worker report still prove itself?
5. Can routine mode remain fast and useful while dirty and partially observable without weakening strict proof?
6. Are command effects structurally enforceable, including transparent cache population and subprocess side effects?
7. Is the proposed CLI small enough, or does it still expose internal architecture?
8. Does the plugin contract specify acquisition, install, discovery, fitting, guidance, recovery, maintenance, and real use deeply enough to guide implementation?
9. Can the adaptive work graph deadlock, oscillate, or falsely serialize independent work?
10. Are semantic conflicts and generated-source ownership defined tightly enough for concurrent writers?
11. Are worker output, root acceptance, independent review, recovery, and claim authority truly separated?
12. Are security, supply-chain, privacy, telemetry, and external-tool boundaries complete?
13. Does the custom-tool inventory duplicate ecosystem primitives without measured necessity or omit a Harness-owned semantic tool?
14. Is every legacy authority safely mapped or does migration risk silent loss/duplicate authority?
15. Is the minimum-ceiling completion model sufficient for real release/install/application/runtime truth?

## Reviewer response schema

```json
{
  "reviewer_runtime": {
    "model": "observed or null",
    "reasoning": "observed or null",
    "verified": false
  },
  "candidate_identity": "sha256 from ZIP_INCLUDE_MANIFEST or null",
  "verdict": "revise|reviewable_after_minor_repairs|adoption_candidate",
  "findings": [
    {
      "finding_id": "PRO-001",
      "severity": "blocker|high|medium|low",
      "law_ids": ["HUL-..."],
      "artifacts": ["..."],
      "problem": "...",
      "false_pass_or_user_harm": "...",
      "smallest_contract_repair": "...",
      "required_recheck": "..."
    }
  ],
  "traceability_gaps": [],
  "duplicate_authorities": [],
  "missing_product_surfaces": [],
  "unsafe_retirements": [],
  "advisory_improvements": [],
  "claim_ceiling": "review_observation_only"
}
```

Reviewer approval is not adoption and does not prove product implementation. Material findings must be revised in the candidate, rechecked, and explicitly accepted by the product owner before the contract becomes active.
