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

## Phase Order

### Phase 0 - Stabilize WIP And Candidate Boundary

Classify dirty files, finish or shelve incomplete Gate 93-97/Gate 92 source
scaffolds, and prevent generated receipts or dependency installs from changing
candidate truth invisibly.

Exit requires: current digest, dirty-state inventory, no ambiguous partial
source scaffolds, package/evidence boundary plan, and no broad audit loop.

### Phase 1 - Package Inventory, Namespace, And Generated-Artifact Boundary

Repair package/resource/classification failures before chasing receipts. Resolve
`node_modules`, promptfoo artifacts, root lock/config files, package inventory
closure, exact-once listing, namespace classes, and generated/proof artifact
boundaries.

Exit requires: focused package/namespace tests, no broad orphan inventory
explosion, schema/catalog paths listed, current receipt, and claim guard.

### Phase 2 - Research-Rooted Full Gate 92 Observability And Agent Legibility

Gate 92 is now the top source-local priority after the current dirty slice is
unambiguous. Do not treat it as a current-blocker-only rescue, a representative
sample, or a later fitting backlog.

First bind the observability work to the foundational papers and additional
research already governed by Gate 93: OpenAI Harness Engineering, OpenAI Codex
repair loops, OpenAI Agents observability/tracing, Google SRE monitoring and
four golden signals, structured-event/high-cardinality doctrine, OpenTelemetry
semantic conventions, OpenAI improvement-loop research, and the self-improving
domain-agent article where it affects traces/evals/feedback loops. If any
observability requirement is not mapped through the research-source registry,
article-to-law trace, source obligations, foundational trace, standards rows,
validators, fixtures, package inventory, claim guards, and setup/retrofit
outputs, repair that mapping before claiming Gate 92 progress.

Complete observability fitting for every CLI/plugin production path: every
command and subcommand, validator check family, receipt/proof path,
fixture/report path, package/plugin/setup/retrofit surface, operating-loop
stage, signal class, long-running path, external/live path, and claim guard.
No minimum-surface, sample-based, current-failure-only, or adjacent-surface
substitution is allowed.

Phase 2 has two internal closures:

- Phase 2A closes operational observability. It fits every command/check/
  receipt/fixture/claim-guard row, proves the four-channel model of logs,
  metrics, traces-or-wide-events, and evals where applicable, and makes the
  repair loop executable through query and explain commands.
- Phase 2B closes observability doctrine hardening. It proves the two-plane
  SRE/agent-quality model, expanded agent/tool/repo/eval telemetry envelope,
  high-cardinality metric-label limits, redaction, boundedness, span parentage,
  resource saturation, before/after repair anchors, and query/explain usefulness.

Phase 2A without Phase 2B is not Gate 92 closure. A command that emits records
but cannot explain the failure in agent-actionable terms is only partially
fitted. A stack-health pass without per-command and per-claim fitting is only
environment telemetry. A fitted neighbor command cannot satisfy an unfitted row.

Required repair loop for every opaque failure encountered: digest -> run the
failing command once -> query logs/metrics/traces by run/correlation/digest ->
explain failure through CLI -> repair smallest cause -> rerun narrow command ->
compare telemetry -> only then broad audit.

Exit requires a passing fitting control board with every inventory row fitted on
same-candidate query proof, every mandatory research requirement current and
mapped to the Gate 92 law surface, full logs/metrics/traces/explain coverage for
all law-bearing command families and plugin surfaces, both observability planes
queryable, cardinality/redaction/boundedness enforced, focused tests,
red/green/tamper fixtures, source inspection, current digest, concise checklist
status updates, and a source-local/not-readiness commit.

### Phase 3 - Gates 93-97 Remaining Touched-Surface Closure

After full Gate 92 closure, close remaining research, improvement-loop, OpenAI,
promptfoo, and HALO surfaces already touched by WIP. Fix schema enum drift,
standards TSV/JSON drift, source obligations, red fixture schema/digests, valid
fixtures, package inventory, observability binding, and claim guards.

This phase must also integrate the GPT-5.5 Pro synthesis and the gold-standard
stack developer-experience archive as Gate 93 synthesis source cards. Decompose
every adopted requirement into canonical law ids, standards rows, source
obligations, foundational trace entries, schemas, validator check ids,
red/green/tamper fixtures, receipt requirements, package inventory, setup/
retrofit outputs, claim guards, final-packet fields, and update_goal blockers.
Prompt-only, checklist-only, row-shape-only, or summary-only adoption fails this
phase.

Exit requires focused tests and receipts for 93-97 source-local claims only.

### Phase 3.5 - Product Usage Fitness And CLI Discoverability

After full Gate 92 closure and before Phase 4 evidence rebinding, close one
dependency-closed source-local product-usage slice.

The CLI/plugin must be usable as a product for plugin-activated repositories, not
only as a self-audit machine. Required validation paths must be obvious, simple,
and hard to skip. Preserve individual advanced entrypoints, but provide one
routine CLI entrypoint for ordinary required validation and self-contained help
that tells an un-oriented agent or user what to run, when, why, which proof
surface is affected, and which claims remain unsupported.

This phase must apply the AGENTS.md-as-routing-table doctrine and the concrete
routine command surface doctrine. Wrappers such as scripts/check, just recipes,
or shell helpers may remain only when they delegate to the canonical CLI
authority kernel, preserve telemetry and receipt binding, and declare whether
they are narrow helpers or routine validation. Product Usage Fitness is not
closed until the fit-repo first-use path and plugin-activated target-repo path
are both visible.

The routine surface should converge toward the stack command model:
`ultragoal stack fast`, `ultragoal stack standard`,
`ultragoal stack clean-proof --cache-mode none`, `ultragoal stack watch`,
`ultragoal stack resources prove`, and `ultragoal gc plan/dry-run/apply/verify`.
Only the source-local parts needed for the current claim boundary should be
implemented here; release/install/cache/app-registry proof remains forbidden
until later phases.

This phase must also add an Agent State / Next Action Contract. The canonical
surface is `ultragoal next` plus `ultragoal next --json`; aliases such as
`ultragoal stack status` or `ultragoal stack next` are permitted only when they
delegate to the same CLI authority kernel. This command is the bridge between
Gate 92 sensory proof and product usability. It consumes typed authority
surfaces and tells an un-oriented agent or Tree the current candidate digest,
dirty/stale state, active phase, strict claim ceiling, first legal blocker, why
that blocker comes first, dependency chain, stale or wrong-digest evidence,
exact next repair, exact narrow rerun, observe query and explain commands when
available, broad rerun only when allowed, forbidden actions, Gate 92 fitting
summary, Product Usage status, worktree eligibility, and Agent Cockpit state.
`--json` is the future cockpit feed; stdout is the human/agent hot path and must
be bounded, redacted, and decisive.

Done means the harness can answer "where am I, what is blocked, why is it
blocked, what exact command proves the next repair, and what claims remain
forbidden" without forcing manual archaeology through parent prompts, stale
checklist rows, raw receipts, command inventory walls, source-audit dumps, or
hidden command knowledge. A generic "inspect receipts" answer, broad-audit-first
answer, dashboard-only answer, or checklist-derived answer is not product
fitness.

Validation and proof are separate. Validation covers parser behavior,
self-contained help, JSON schema shape, bounded output, redaction, dirty/stale
state handling, forbidden-action guards, and fail-closed fixtures. Product proof
requires a real `ultragoal next` run on the current candidate, stdout and JSON
that identify the real current blocker and correct next narrow repair, receipt
binding to the same candidate/run/correlation, logs/metrics/traces query proof
for that run when telemetry exists, useful observe explain output when blocked,
and source inspection proving the command reads typed evidence instead of
markdown, checklist prose, or last-command heuristics.

Exit requires focused help/routine-path/next-action tests and red fixtures for
missing routine entrypoint, non-navigable help, leaf-only validation
substitution, hidden fit-repo path, omitted target-repo/plugin-activated path,
`scripts/check` failing to delegate or declare itself a narrow helper, missing
`ultragoal next`, generic next-action output, no exact next command, stale digest
accepted as current, checklist prose accepted as authority, broad audit
recommended before narrow observable repair, worktrees marked eligible while
Gate 92/Product Usage/Phase 4 are incomplete, readiness/update_goal implied from
source-local proof, cockpit/UI proof substituted for CLI proof, private path
leakage, and unbounded JSON output. Update checklist rows with progress statuses
only, then commit as source-local/not readiness.

### Phase 4 - Rebind Gates 0-91 Acceptance Spine

Only after Gate 92 is fully fitted and Phase 3.5 closes or is explicitly
source-local blocked, rerun or stale-mark coverage, line caps, namespace/maximal
factoring, typed boundaries, Product/Fit/Journey, Rust/GC, standards, source
obligations, foundational trace, source audit, and red report on one digest.

Exit requires current source-local audit/red/coverage spine or named failures.

### Phase 5 - Gate 104 Closure For Gates 93-97

Prove Gates 93-97 exist across all mandatory law surfaces, not only files/rows.

Exit requires Gate 104 focused tests and source-audit coverage for Gates 93-97.

### Phase 6 - Gates 98-103

Implement domain-agent pattern, setup/retrofit, active-repo rollout, TypeScript
DevX, privacy/data minimization, and surface separation.

This phase is where future product surfaces from the synthesis begin becoming
implementation work: portable adapter stack status, supply-chain/security
baseline, AGENTS routing templates, tool contract/risk tiers, active-repo
rollout proof, HU-STACK laws, stack receipt/staleness model, cache/no-cache
honesty, resource discipline, GC classification, CI/local parity, migration
phases, and Agent Cockpit status or explicit blocker. Do not pull these forward
to block Phase 2 unless the current Gate 92 implementation depends on them.

Worktrees may start only after Phase 4 is committed and parent owns all
`validation_artifacts/**` writes.

### Phase 7 - Gate 92 Regression And Propagation Guard

Full Gate 92 fitting is no longer deferred here. This phase only revalidates
that later Gates 93-103 work did not regress any observability inventory row,
research mapping, query proof, pass/fail output contract, trace parentage,
metric/log binding, help discoverability, or setup/retrofit propagation.

Exit requires the fitting control board to remain pass on same-candidate query
proof after later source changes.

### Phase 8 - Gate 105 Measured Improvement

Add baselines, current values, regression guards, telemetry comparison,
standards-gardener promotion, and claim guards for improvement claims.
Gate 105 must use both observability planes: product/system health and agent
quality. It must measure whether the harness reduces time-to-diagnosis,
time-to-repair, rerun count, stale/wrong-digest recurrence, opaque-failure
recurrence, claim-theater escape count, source-audit failure recurrence,
manual-spelunking burden, and worktree/lane regression rates.

### Phase 9 - Final Source-Local Proof

Run exact coverage, line-cap scan, focused tests, source audit, red report, CLI
self-law, update-goal eligibility, and final source-local claim ceiling.

### Phase 10 - Distribution Surfaces

Only after Phase 9 passes: install/cache refresh, version bump, package sync,
final packet, reviewer/app-registry exposure proof or unsupported-claim blocking,
and update_goal eligibility.

## Lane Rules

No lanes before Phase 4 is committed.

After Phase 4:

- Parent owns package digest, source audit, red report, receipts, final packet,
  package sync, version bump, and update_goal.
- No lane writes `validation_artifacts/**`.
- Lanes must own disjoint source paths.
- Candidate lanes:
  - Lane A: Gate 98 domain-agent pattern.
  - Lane B: Gate 99 setup/retrofit.
  - Lane C: Gate 100 active-repo rollout.
  - Lane D: Gate 101 TypeScript DevX.
  - Lane E: Gate 102 privacy/data minimization.
- Parent retains Gates 92, 94, 104, 105, and final synthesis.

## Forbidden Actions

Until Phase 9 passes: no install/cache refresh, version bump, final packet
finalization, app-registry/reviewer exposure claim, readiness/release/completion
claim, update_goal call, or worktree lane launch unless explicitly allowed by
this spine.

If any forbidden action happens, stop and repair the execution violation before
continuing.

## Parallel-First Default

Every safe CLI, plugin, validator, fixture, package scan, setup/retrofit,
observability, shell-helper, and proof path must use available parallelism by
default. Serial behavior is allowed only when the path is a typed
`shared_authority_write_serial`, `destructive_or_mutating_serial`, or externally
bounded live phase with an explicit reason.

The canonical scheduler task classes are `pure_read_parallel`,
`isolated_temp_write_parallel`, `external_live_bounded_parallel`,
`shared_authority_write_serial`, and `destructive_or_mutating_serial`. Safe
multi-item work must use the scheduler/executor or emit a fail-closed reason
showing why parallelization is impossible. Default workers are
`available_parallelism - 1`, minimum `1`, with bounded `--jobs N` where exposed.
Worker results must be deterministically ordered, fixture workers must use
isolated temp roots, and no worker may write shared `validation_artifacts/**`.

Scheduler/performance evidence must record worker count, task count, queue
depth, wall time, CPU time when available, memory/IO when available, cache mode,
resource-measurement status, candidate digest, and claim impact. Missing timing
or concurrency metadata blocks speed, routine-usability, product-readiness,
release, and update_goal claims.

## Disobedience Detection

A parent action is invalid if it:

- runs a broad audit repeatedly without implementation or evidence semantics
  change;
- treats coverage, red report, source audit, or fail-closed receipts as
  completion;
- uses stale or wrong-digest receipts as current;
- lets local JSON, Grafana, packet text, or checklist text substitute for CLI
  proof;
- launches lanes before Phase 4;
- writes shared receipts from lanes;
- claims source/install/cache/app-registry/reviewer surfaces interchangeably.

Invalid actions must become validator/CLI guard work, not prose.
