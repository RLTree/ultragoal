# Harness Ultragoal Full Compliance Execution Spine - 2026-06-30

This document controls execution order only. The canonical scope remains:

- `docs/parent-session-full-ultragoal-compliance-prompt-2026-06-25.md`
- `docs/parent-session-full-ultragoal-compliance-checklist-2026-06-25.md`

The spine exists to prevent receipt churn, broad-gate theater, premature lanes,
and readiness claims before the source-local proof graph is stable.

## Non-Negotiable Operating Rule

Work by dependency-closed slices, not by gate-number theater.

Exactly one broad slice may be active at a time. A slice may cross gates, but it
must close one claim-bearing production path. Do not start a second broad slice
while the first has dirty source, stale receipts, current failing focused tests,
no red/green/tamper proof where applicable, or no same-candidate receipt.

Every slice ends with:

1. recomputed package digest;
2. focused tests;
3. red/green/tamper or stale/wrong-surface proof where applicable;
4. current same-candidate receipt;
5. claim guard;
6. targeted manual source/runtime inspection when the slice closes or changes a
   claim-bearing validator, schema, fixture, receipt, claim guard, product
   claim, external-AI adapter, final packet, install/cache/app-registry surface,
   update_goal surface, or suspicious CLI pass;
7. existing checklist row updates only;
8. commit marked source-local/not readiness when a coherent checkpoint exists.

Checklist text is progress tracking only, never law evidence.

## Anti-Self-Validation Rule

The Ultragoal CLI is the system under development. Until final self-law
compliance is proven by the full contract, CLI `pass` output is a subject-under-
test signal, not claim authority by itself.

Manual validation is mandatory at claim-boundary points, not as a universal
receipt family stamped onto every mandatory-law row. Use it when a gate is being
marked complete, when a validator/check/schema/claim guard is added or changed,
when red/green/tamper semantics change, when Product Fitness/Product Success or
external-AI authority is involved, when final packet/update_goal/readiness/
install/cache/app-registry surfaces are touched, or when a CLI pass looks
suspicious.

Routine inner-loop checks may be tool-driven: fmt/build, focused unit tests,
line-cap, exact coverage, schema validation, package digest, targeted receipts,
and focused red/green/tamper tests. At slice boundaries, inspect representative
source paths, receipt JSON, schema or validator paths, fixture paths, and live
runtime/query output for the changed claim path. Full manual E2E dogfooding is
required before completion/readiness/release/update_goal claims.

Manual validation tunes and challenges the CLI. It does not replace the CLI, and
it must not become row-shape/manual-receipt theater. If manual inspection finds
the CLI passed too broadly, passed with stale evidence, passed a row-shape
substitute, or passed while a dependent law surface remains incomplete, repair
the validator/check/fixture/claim guard before the claim can move.

## Genuine Proof And Proxy-Claim Ban

Claim-bearing proof must prove the product behavior that the claim names. A
schema-valid receipt, generated inventory row, current-state projection,
workflow-engine output, parser/unit test, local spool record, timing field,
cache-key calculation, or CLI `pass` line is not proof by itself. It is only an
observation unless it dereferences one of these proof paths:

1. actual command or runtime behavior on the current candidate, with stdout,
   exit status, receipt/artifact paths, logs/metrics/traces where applicable,
   source/runtime inspection, and explicit claim impact; or
2. verified same-candidate reuse of a prior result, with current input digests,
   validator/law/schema/fixture versions, arguments, environment class, cache
   key, prior result digest, replayed output digest, equivalence status, and
   invalidation proof.

All other proxy surfaces are diagnostic only and must carry a claim ceiling of
`observation_only` or `source_local_diagnostic_only`. The CLI must fail closed
when a claim-bearing row has `work_unit_count=0`, no command/result digest, no
same-candidate cache equivalence, no observability reconciliation, stale or
wrong-digest evidence, generic fail text, or a proof surface that cannot explain
what product behavior was actually observed.

Performance and speedup claims have the same standard. Timing graph overhead,
cache-key construction, dry-run planning, current-state reads, generated row
materialization, local JSON shape checks, workflow worker reports, or synthetic
no-op paths cannot prove speed. A speed claim is legal only when every included
node records `proof_kind=executed` or `proof_kind=verified_cache_hit`, separates
actual work duration from scheduler/graph overhead, records result and output
digests, and reconciles same-candidate telemetry. Missing proof blocks the speed
claim even when the displayed ratio exceeds the target.

## Current State Assumption To Recompute

On every resume, recompute current package digest with the canonical CLI command.
Treat all receipts not bound to that digest as stale.

Do not trust older source-audit, red-report, coverage, Gate 92, Product/Fit,
Rust/GC, OpenAI, promptfoo, HALO, final-packet, install/cache, or update_goal
receipts until they are rebound or explicitly stale-marked.

## Builder-Contract Versus Package Boundary

The parent-session prompt, checklist, and execution spine are agent-governing
builder contracts for this work session. They are not package resources, plugin
product surfaces, coverage targets, package digest inputs, shipped law evidence,
valid fixture dependencies, product receipts, review/archive contents, install
inputs, cache inputs, registry inputs, or update_goal evidence.

The Ultragoal CLI and plugin package must not depend on these parent-session
files in any capacity. Editing them may change agent instructions and execution
order, but it must not stale package digest, coverage, source audit, Product/Fit,
review-target, archive, install/cache, registry, final-packet, or update_goal
receipts. Any current package manifest, package inventory, coverage manifest,
fixture, receipt, or validator check that treats these files as package-owned
must be repaired as a package-boundary bug before the affected claim can close.

## Gold-Standard Harness Product Doctrine

The attached GPT-5.5 Pro synthesis is adopted as a synthesis input for execution
planning. It is not primary factual authority for exact external version pins or
current vendor claims, but its product doctrine is binding for this spine:

- The harness is the product.
- Languages, models, receipts, plugins, MCP tools, eval tools, and dashboards
  serve the harness.
- Observability is the agent sensory system, not a decorative log stream.
- Evals and trace feedback are the learning loop, not a later report.
- Repository structure, command discoverability, setup/retrofit, and runtime
  proof are product surfaces.
- SRE discipline applies to agent quality as well as system health.

This doctrine changes execution, not just wording. A slice that adds command
fitting, telemetry, evals, setup/retrofit, product usage, or final packet fields
must state which product capability is improved and how an agent would know it
is working. A slice that only adds row shape, receipt shape, adjacent command
coverage, or vague proof text does not close a product claim.

Operationalized objectives:

1. Harness Product Doctrine
   - Done when an un-oriented agent can discover the routine path, run it,
     diagnose a failure through CLI observability, repair the smallest cause,
     rerun narrowly, and understand the remaining claim ceiling without reading
     every parent-session document.
   - Downstream: Product Usage Fitness, Gate 92, Gate 94, Gate 99, Gate 105, the
     final packet, and update_goal blockers must prove product utility.

2. Gate 92 As Agent Sensory System
   - Done when every law-bearing command/check has logs, metrics, traces or wide
     events, eval linkage where behavioral quality is involved, bounded query
     proof, and useful explain output.
   - Downstream: local JSONL/spool proof remains transition/debug evidence only;
     live stack ingestion/query proof or an explicit fail-closed blocker is
     required for Gate 92 closure.

3. Two Observability Planes
   - Done when Plane A system health and Plane B agent quality are both queryable.
     Plane A covers latency, traffic, errors, saturation, freshness, retry/
     backoff, cache state, and resource pressure. Plane B covers task completion,
     first-pass success, repair iterations, failure class, human escalation, bad
     repair/packet rate, post-merge regression, eval trend, tool misuse, docs
     drift, architecture violations, and claim-theater escapes.
   - Downstream: Gate 105 measured improvement and later worktree lane proof must
     use both planes.

4. Expanded Telemetry Envelope
   - Done when command/check telemetry carries run/correlation/trace/span ids,
     candidate digest, law/check/claim ids, artifact/receipt paths, failure
     class, why/where/next repair, duration, worker/task/queue state, cache
     mode, retry/backoff, saturation/resource state, redaction/boundedness,
     before/after repair anchors, plus agent/tool/repo/eval attributes.
   - Downstream: observability schemas, receipts, improvement-loop records,
     OpenAI/promptfoo/HALO adapters, setup/retrofit, and final packet fields must
     validate these attributes where applicable.

5. Cardinality Doctrine
   - Done when high-cardinality values stay in traces, wide events, logs, and
     eval records, while metrics keep bounded labels.
   - Downstream: red fixtures must fail unbounded metric labels, private path
     leakage, and fake boundedness.

6. Trace To Feedback To Eval To Repair To Promotion
   - Done when repeated failures become typed feedback, eval cases, regression
     fixtures, validator laws, standards rows, or explicit claim blockers.
   - Downstream: Gate 94 is not closed by a loop receipt unless before/after
     telemetry and promoted protection exist for the claimed repair class.

7. AGENTS.md Routing Table
   - Done when setup/retrofit produces concise routing docs that point to
     canonical commands, docs, telemetry, evals, architecture, security, privacy,
     and quality gates without duplicating the entire law system.
   - Downstream: Product Usage Fitness and Gate 99 must reject encyclopedia-style
     always-loaded docs and hidden proof surfaces.

8. Tool Contract And Risk Tiers
   - Done when each tool/adaptor declares owner, risk tier, idempotency, auth
     scope, schemas, approval need, telemetry, pre/postconditions, failure
     semantics, and claim impact.
   - Downstream: external/live paths, setup/retrofit, MCP/plugin surfaces, and
     future lanes must explain why a tool was allowed, blocked, serialized, or
     excluded from claim support.

9. Agent Cockpit Direction
   - Done later when a product surface can show active runs, repair state, plan,
     changed files, validation, eval deltas, trace waterfall, tool calls,
     approvals, screenshots/videos where applicable, logs by run id, PR status,
     docs touched, architecture lints, cost, latency, and token summaries.
   - Downstream: this is a future product surface, not a Phase 2 blocker unless
     explicitly scoped. The final packet must state status or blocker honestly.

10. Concrete Routine Command Surface
   - Done when fast validation, full source-local validation, observe query,
     observe explain, repair loop, eval, architecture check, security check,
     setup, and retrofit are discoverable through canonical CLI help or wrappers
     that delegate to the CLI authority kernel.
   - Downstream: Product Usage Fitness cannot close on leaf-only commands or
     helper scripts that bypass telemetry, receipts, or claim ceilings.

11. Entropy Cleanup Loop
   - Done when docs drift, dead code, unused dependencies, missing tests,
     duplicate abstractions, telemetry drift, architecture violations, stale
     evals, flaky or slow tests, large files, uncited assumptions, and security
     drift have a standards-gardener or claim-blocking path.
   - Downstream: long-term product quality is part of Gate 105 and later
     hardening, not a side note.

12. Validation Gate Levels
   - Done when local-fast, full source-local, merge/review, install/cache, live
     app/registry, and update_goal proof levels cannot be confused in command
     output, receipts, checklist statuses, or final packets.
   - Downstream: focused checks remain repair aids and cannot substitute for
     final or live-surface proof.

13. Pedagogical Validator Output
   - Done when failures include law id, check id, invariant, observed value,
     expected value, where/why, repair class, rerun command, affected claims,
     severity, redaction, and boundedness.
   - Downstream: `observe explain` must make manual spelunking a verification
     step, not the only way to discover the next patch.

14. Portable Adapter Stack
   - Done when Rust, TypeScript, Python, data stores, telemetry stores,
     workflow/infra adapters, and setup/retrofit integration are treated as
     explicit adapter families with support status and claim impact.
   - Downstream: exact version pins from the synthesis require official-source
     verification before becoming hard package law.

15. Supply-Chain And Security Baseline
   - Done when dependency selection, frozen lockfiles where applicable,
     provenance/SBOM/signing where supported, audits, package age policy where
     supported, and image digest pinning where applicable are represented in
     setup/retrofit and install/cache proof.
   - Downstream: agent-installed dependencies without validation cannot support
     readiness, release, or update_goal claims.

16. Truth-Surface Separation
   - Done when product truth, observability truth, and artifact truth are
     cross-linked but non-substitutable.
   - Downstream: receipts cannot replace runtime behavior, spool files cannot
     replace live observability, source proof cannot replace install/cache proof,
     and final packets cannot mint authority.

17. Synthesis Source Card
   - Done when the synthesis is represented as a Gate 93 source id such as
     `agentic-gold-standard-stack-synthesis-2026-07-01` and every adopted
     requirement maps through the mandatory law surfaces.
   - Downstream: prompt-only or checklist-only adoption fails Gate 93 and Gate
     104.

18. Final Packet Product Shape
   - Done when final packet and final response surfaces include Harness Product
     Doctrine, Observability Planes, Agent Quality Metrics, Trace-to-Eval-to-
     Repair Loop, Tool Risk and Approval Surface, Data/Privacy Boundary, Product
     Usage/CLI Surface, Active Repo Rollout, Stack Adapter Status,
     Supply-Chain/Security Baseline, Agent Cockpit status or blocker, and
     unsupported live surfaces.
   - Downstream: reviewers can see exactly which claims are source-local,
     install/cache, app-registry, live reviewer, final-packet, external/live, or
     update_goal-supported.

## Gold-Standard Stack Developer Experience Addendum

The archive
`/Users/terrynoblin/Downloads/harness_ultragoal_gold_standard_stack_markdown_and_laws.zip`
is a second synthesis input for execution planning. It is not primary authority
for exact external version pins, but its HU-STACK laws, command loops,
proof-surface separation, cache/resource/GC discipline, supply-chain baseline,
CI/local parity, and migration plan are binding execution guidance.

Use a Gate 93 source id such as
`gold-standard-stack-developer-experience-governance-2026-07-01`. The source row
must record the zip path and per-entry digests, and must mark external version
and vendor claims as requiring official-source verification before package,
install/cache, release, readiness, or update_goal support.

Stack classifications:

- `REQUIRED`: must exist for applicable repo surfaces and route through CLI
  receipts.
- `DEFAULT_ON`: enabled by default when safe; absence is explicit and
  claim-limited.
- `GOVERNED_ADAPTER`: supported but not canonical authority without declared
  config, schemas, receipts, and claim limits.
- `OPTIONAL_LOCAL`: local productivity only, no claim support.
- `REJECT`: forbidden for claim-bearing work.

HU-STACK law integration:

- `HU-STACK-001 Agent-First Harness`: repo, runtime harness, validation,
  observability, evals, and cleanup are product surfaces.
- `HU-STACK-002 Cross-Language Claim Authority`: Rust, TypeScript, Python, SQL,
  infrastructure, telemetry, and agent tools emit observations only; `ultragoal`
  converts observations into claims.
- `HU-STACK-003 Clean-Checkout Discoverability`: fresh agents discover setup,
  validation, local runtime, evals, and release proof from repo files only.
- `HU-STACK-004 Same-Surface Full-Stack Proof`: every surface claim is proven on
  that same surface.
- `HU-STACK-005 Lockfile Sovereignty`: lockfiles and provider locks are governed
  truth surfaces; drift blocks dependent claims.
- `HU-STACK-006 Telemetry Schema`: telemetry/eval attributes are registered with
  type, owner, cardinality, privacy, and allowed surfaces.
- `HU-STACK-007 Agent Tool Boundary`: every tool/MCP action has schemas, risk,
  approval, idempotency, telemetry, and cleanup.
- `HU-STACK-008 Product-Cockpit`: agent work becomes inspectable through a
  cockpit or equivalent runtime surface, without replacing CLI proof.
- `HU-STACK-009 Full-Stack GC`: every artifact/cache/state/receipt/packet/lock/
  pid/port/tempdir is classed before cleanup; deletion requires dry-run plan and
  receipt.
- `HU-STACK-010 Update Goal Eligibility`: update_goal remains forbidden until
  current goal state, active receipts, stack claim ceiling, product proof
  eligibility, and cleanup/protection status are verified.

Command loop implications:

- `ultragoal stack fast` is the hot inner-loop command. It supports only fast
  feedback and changed-scope structural observations.
- `ultragoal stack standard` is the serious source-local repair proof loop.
  It runs language standards, DB migration checks, observability semantic checks,
  tool inventory, red/green/tamper fixtures, receipt verification, and claim
  ceiling computation.
- `ultragoal stack release` is the full release/product proof loop. It adds
  Rust/TypeScript/Python/data/observability/workflow/infra/package/install/cache/
  Product Fitness/Product Cohesion/Product Success/GC/final-packet proof where
  applicable. It does not override the existing claim ceiling.
- `ultragoal stack clean-proof --cache-mode none` proves clean checkout and no
  hidden local cache dependence.
- `ultragoal stack watch` emits observations only.
- `ultragoal stack resources prove` proves memory, queues, processes, pools,
  browser artifacts, workflow backlog, and agent tool cleanup are bounded.
- `ultragoal gc plan/dry-run/apply/verify` governs cleanup. No blind cleanup,
  broad deletion, or unclassified artifact removal can support claims.

Stack proof discipline:

- Stack receipts stale on dirty state, lockfile drift, schema/law/fixture/
  migration/telemetry/model/MCP/provider/Kubernetes/package/install/runtime/
  observability config changes.
- Cache use is legal. Cache concealment is illegal. Warm-cache speed is not
  clean-checkout proof. Cache presence is not correctness proof.
- Product Fitness requires correct runtime-surface proof. Product Cohesion
  requires architecture, telemetry, DB truth model, UI/framework boundaries,
  package surfaces, docs, security, and operational model to fit. Product Success
  requires the user outcome on the runtime product surface.
- CI YAML is orchestration, not authority. CI must run the same `ultragoal`
  command surfaces available locally.
- Optional local tools can help investigation but cannot support claims.
- A stack law is theater if prose, reviewer agreement, stale evidence,
  source-only proof for runtime claims, package-only proof for install/runtime,
  install/cache proof for product success, fixture names without execution, or a
  lowered claim ceiling alone can satisfy it.

## Carry-Forward Control Loop Requirements

The checklist is a progress surface, not a receipt ledger. Use only concise row
statuses: `not started`, `in progress`, `implemented, pending validation`,
`validated current`, and `stale due to source change`. Do not add progress-ledger
sections or churn receipt paths/digests through checklist rows during active
implementation.

Receipts are minted or refreshed only when they support a claim boundary: slice
closure, a phase gate requiring same-candidate evidence, final source-local proof
assembly, or an in-scope package/install/cache/final-packet/update_goal proof.
Otherwise use focused tests, stdout, direct source inspection, logs, metrics,
traces, and explain output as inner-loop evidence.

Validation is tiered:

- Inner loop: fmt/build, focused unit tests, line-cap, package digest, schema
  check, targeted receipt, and targeted red/green/tamper tests.
- Slice boundary: current digest, focused tests, touched red/green/tamper proof,
  same-candidate receipts, claim guard, targeted source/runtime inspection,
  checklist status updates only, and a source-local/not-readiness commit when
  coherent.
- Broad boundary: exact coverage, source audit, red fixture report, standards,
  source-obligation, and foundational trace closure.
- Completion boundary: full E2E/manual dogfood, CLI self-law, update_goal
  eligibility, final packet, install/cache/app-registry, and reviewer surfaces.

Do not rerun broad source audit/red report loops unless implementation or
evidence semantics changed. For repeated failures, run the narrow failing command
once, query telemetry by run/correlation/current digest, explain the failure,
repair the smallest production cause, rerun the narrow command, verify changed
telemetry, and only then run broad audit once.

Current dependency order is strict: if the red-fixture scheduler/
parallelization slice is dirty, close it immediately with focused proof and a
source-local/not-readiness commit, or explicitly stale-mark/shelve it without
claiming closure. After the current dirty slice is no longer ambiguous, full
research-rooted Gate 92 observability and agent legibility is the next broad
slice. Product Usage Fitness, Phase 4 rebinding, final proof assembly, and
worktree lanes wait until Gate 92 is fully fitted and committed.
