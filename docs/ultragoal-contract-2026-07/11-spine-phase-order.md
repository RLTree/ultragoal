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

Semantic namespace repair is part of this phase, not a Gate 92 subtask. Before
Gate 92 resumes, repo-owned product surfaces must not use goal-work, phase,
slice, evidence-purpose, or session-history names in source paths, modules,
functions, helper names, test names, ids, receipt/artifact path segments, or
generated/package inventory paths. Names must identify product behavior or
domain responsibility: for example command telemetry roundtrip, telemetry
reconciliation, command inventory, namespace topology, cache invalidation, or
receipt dereference. Names such as `fitting`, `production_proof`, `phase4`,
`slice`, `workstream`, `checkpoint`, `progress`, `todo`, `wip`, and generic
`helpers`/`utils`/`common` fail when they stand in for product behavior.

Phase 1 also owns purpose-backed active surface enforcement. Every package-owned
binary, command, module, function, helper, test, type, schema, fixture, receipt
producer, generated artifact, package resource, setup/retrofit output, claim
guard, final-packet blocker, and update_goal blocker must have one typed product
role and one authority level. Redundant surfaces, duplicate entrypoints,
coverage-only wrappers, dead fallback branches, history/progress labels, and
compatibility aliases without external contract and sunset are law violations.
For example, `ultragoal` and `ultragoal-validator` cannot both remain as
undifferentiated product authorities. Either `ultragoal-validator` is a typed,
claim-limited compatibility alias that delegates to canonical `ultragoal`, or
it is removed.

Validation for this phase proves the mechanics: schema checks, parser/help/unit
tests, duplicate-detector tests, namespace checks, line caps, coverage, and
red/green/tamper fixtures. Proof for this phase proves current product behavior:
real CLI/source-audit execution on the current candidate detects actual
package-owned surfaces, rejects unregistered or redundant authority, reconciles
claim guards and receipts where applicable, and source inspection confirms the
registry is not row-shape theater.

Exit requires: focused package/namespace tests, product-semantic symbol checks,
purpose-backed active surface inventory, duplicate-authority/compatibility-alias
proof, red/green/tamper fixtures for opaque path/module/function/surface names,
no broad orphan inventory explosion, schema/catalog paths listed, current
receipt, and claim guard.

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

Before continuing broad command-by-command fitting, insert a parent-owned
acceleration slice inside Phase 2. The dynamic workflow engine is currently a
tooling dependency under review, not the owner of this work. The parent may use
it only for bounded design assistance after inspecting and supplementing the
generated plan; the actual source changes, commands, telemetry proof, and claim
ceilings remain parent-owned and must route through canonical `ultragoal`
surfaces. Workflow output, worker output, or generated plans cannot satisfy Gate
92, Product Usage, Phase 4, readiness, release, final packet, worktree
eligibility, or update_goal claims.

The acceleration slice is: Gate 92 Fast Loop, Verified Incremental Audit, And
Command Telemetry Roundtrip/Reconciliation. It must produce one routine
live-loop command and one spec-driven command telemetry roundtrip path before
the parent spends more cycles manually proving individual rows:

This acceleration work is dependency ordered. Do not jump directly to broad
Gate 92 row fitting while the speed-law architecture is dishonest or the routine
commands remain slow side channels. The required internal order is:

1. Close or explicitly block Phase 1A active-surface/namespace work.
2. Repair speed-law arithmetic, baseline provenance, and Rust/cache receipt
   honesty. Same-command baselines and integer-truncated ratios cannot support
   a speed claim.
3. Split coverage into strict full-clean boundary proof and routine warm/
   retained-artifact repair proof. Routine coverage must be claim-limited and
   must never satisfy strict/no-cache or completion-adjacent claims.
4. Introduce one product-surface input spec that drives affected-set detection,
   cache keys, invalidation proof, and command telemetry roundtrip rows.
5. Route loop measurement/execution through typed task classes and in-process
   validator nodes sharing `AuditContext`, while keeping cargo/build/test/fmt
   target-dir contention typed and isolated or serial for a stated reason.
6. Compute package digest and shared source indexes once per immutable process
   snapshot.
7. De-duplicate broad source audit and red fixture work through shared read-only
   inventories, deterministic parallelism, and isolated mutated fixture state.
8. Bound observability I/O: indexed local spool, bounded retention, batched
   exporter flushes, and compact hot-path timing receipts.
9. Consider crate/workspace splitting only after measured residual test/build
   cost proves it is still needed.

Each sub-slice must have focused tests, red/green/tamper fixtures for the
semantics it changes, source inspection, measured before/after timing, explicit
proof-vs-validation labels, concise checklist status updates only, and a
source-local/not-readiness commit when coherent.

- `ultragoal loop run --tier hot --cache-mode verified-local --jobs auto` is the
  routine live source-local repair command. It computes the current candidate
  and package boundary, detects changed files, resolves affected law/check/
  schema/fixture/receipt/query nodes, runs only legally sufficient impacted
  work, queries emitted telemetry when available, explains the first blocker,
  emits current-state, and prints exact next repair, exact narrow rerun, broad
  rerun allowance, forbidden actions, and claim ceiling.
- The live-loop engine must be a verified incremental query graph, not a serial
  script. It must use a shared `AuditContext`, verified content-addressed cache,
  deterministic joins, decomposed package/text checks, grouped red fixtures with
  shared semantic indexes, minimal or copy-on-write isolated fixture roots, and
  optional persistent validator daemon/worker state that verifies current inputs
  on every run.
- `--cache-mode verified-local` is the default live-loop cache mode. Cache hits
  are legal only when bound to current input digests, validator digest, law/
  schema/fixture versions, arguments, environment class, and cache mode. Cache
  concealment, stale cache acceptance, dry-run timing presented as live timing,
  and warm-cache proof substituted for clean proof all fail. `--cache-mode none`
  remains required for strict no-cache proof boundaries.
- The live-loop proof target is measured command latency, not architecture
  optimism. Use the current audit receipt baseline when available; the known
  scheduled audit shape is about 181 seconds, so a 20x routine live-loop target
  is about 9.1 seconds. Recompute the baseline if current receipts differ. A
  strict no-cache final proof may be slower and must be reported separately.
- The live-loop speed claim must prove actual validation work or verified reuse,
  not proxy execution. Measuring cache-key construction, generated row
  materialization, current-state projection, graph scheduling overhead, workflow
  output, dry-run planning, or a no-op wrapper is diagnostic telemetry only.
  `verified-local` timing is claim-bearing only when each affected node records
  `proof_kind=executed` or `proof_kind=verified_cache_hit`, result/output
  digests, current input digests, validator/law/schema/fixture versions, cache
  key and invalidation proof when reused, actual work duration, graph overhead,
  and same-candidate stdout/receipt/log/metric/trace reconciliation. If a node
  has `work_unit_count=0`, `cache_hit=false`, no executed command, no verified
  cache equivalence, or no result digest, the speed row fails regardless of the
  displayed speedup ratio.
- The live-loop target applies to the validation work agents actually run, not
  only to the wrapper command. `scripts/check`, `scripts/check-coverage-full`,
  `scripts/check-coverage-fast`, `ultragoal coverage prove`, source audit, red
  fixture report, line caps, namespace, schema validation, mandatory-law
  validation, source-obligations, foundational trace, package inventory scans,
  focused Rust tests, fmt/build checks, and touched fixture/report paths must be
  routed into the verified incremental query graph as typed nodes before this
  acceleration slice closes. Each node needs its own current full-command
  baseline, current verified-local timing, affected-set equivalence proof,
  cache/input-digest proof, worker/task/queue/cache telemetry, and claim impact.
  A node that remains slow, serial, hidden outside the loop, or unmeasured blocks
  the fast-loop slice unless it has a typed serial/destructive/external-live
  reason and emits a fail-closed blocker.
- The 20x target is a routine-loop law, not a universal strict-proof law. Every
  high-frequency source-local node must either run current-candidate affected
  work fast enough for the verified-local routine loop, replay a verified
  current-input cache hit with explicit routine-only claim limits, or emit a
  typed blocker/strict-boundary-only reason. The whole routine loop including
  those nodes must complete within the current whole-loop target of about 9.1
  seconds unless the baseline is recomputed from current receipts.
- Coverage is explicitly split by proof surface. Routine coverage feedback must
  have verified current-input equivalence to authoritative exact coverage or a
  `routine_repair_only` claim ceiling. Strict `check-coverage-full` and strict
  `coverage prove` remain full-clean claim-boundary proof for 100 percent
  coverage and may remain slower within the strict coverage budget. They cannot
  be used as the ordinary edit loop, and their cost cannot excuse missing
  routine coverage feedback when coverage is relevant to the edit class.
- Add a canonical `CommandObservabilitySpec` and `SurfaceObservabilitySpec`
  registry. Generated artifacts must derive command inventory rows, telemetry
  reconciliation rows, stdout contracts, receipt expectations, query proof paths,
  fixture packs, generated roundtrip tests, control-board status, `observe
  explain --next`, current-state inputs, and `ultragoal next` inputs from those
  specs.
- Add a command telemetry roundtrip runner such as `ultragoal observe fit --command <id>` and
  `ultragoal observe fit --family rust|observe-query|gc|external-live|
  claim-guard|package|coverage|product|install-cache|final-control`. The runner
  must execute the real command or surface, capture stdout/receipt, query
  logs/metrics/traces, run the applicable explain command, reconcile
  same-candidate proof, and refuse fitted status when production proof is
  missing. Row-specific hand edits to generated inventory are debt unless they
  are temporary generated output with a source spec and validation.

Fast-loop slice exit requires: current package digest, source inspection of the
query-DAG/cache/observability-spec/high-frequency-node path, focused tests, red/green/
tamper fixtures for cache dishonesty, stale/wrong-digest hits, hidden worker
caps, hidden serial locks, unbounded concurrency, nondeterministic ordering,
omitted affected fixtures, omitted high-frequency checks, workflow-output-as-
proof, tests-without-production-fitting, coverage-fast-substituted-for-exact-
coverage, synthetic/key-only timing, wrapper-only timing, current-state-only
timing, cache_hit-false-with-no-execution, no-result-digest speed claims, and
slow-check-hidden-outside-loop; measured live-loop timing with separated
actual-work and graph-overhead durations plus worker/task/queue/cache metrics;
per-node 20x proof for each routinely-run
source-local check/audit; whole-loop timing at or below the current 9.1 second
target; current-state and explain output from a real run; concise checklist
status updates only; and a source-local/not-readiness commit. This slice may
close as acceleration/product infrastructure before every Gate 92 row is fitted,
but it may not claim Gate 92 closure by itself.

Complete observability fitting for every CLI/plugin production path: every
command and subcommand, validator check family, receipt/proof path,
fixture/report path, package/plugin/setup/retrofit surface, operating-loop
stage, signal class, long-running path, external/live path, and claim guard.
No minimum-surface, sample-based, current-failure-only, or adjacent-surface
substitution is allowed.

Gate 92 must also install the lower-level agent-legibility primitives that make
the fitting board usable instead of merely visible:

- Every law-bearing failure stdout must carry run_id, correlation_id,
  trace/span ids where available, current candidate digest, failed law/check/
  claim ids, where_failed, why_failed, failure_class, claim_impact, receipt
  path, exact next_repair, exact narrow_rerun, and observe logs/metrics/traces
  query hints.
- `observe explain --next` must turn the fitting control board's first
  incomplete row into a concrete repair plan: row id, owner surface, next
  unfitted surface, missing proof class, implicated command/family, stale or
  wrong-digest evidence, narrow command, query/explain commands, source or
  fixture gap, claim ceiling, and forbidden actions. It is tactical Gate 92
  repair guidance, not row fitting, readiness proof, or checklist projection.
- A compact typed current-state read model must exist, preferably
  `ultragoal current-state --json`, with an optional bounded
  `validation_artifacts/current-state.json` snapshot. It is derived from real
  receipts and typed authority surfaces: package digest, git dirty state,
  fitting board, coverage status, source audit, red report, claim guards, and
  stale-evidence checks. It is not proof by itself and must fail closed on stale
  or wrong-digest inputs.

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

AGENTS.md and generated repo-entry docs must be routing tables in the literal
operator sense: "If doing X, run Y, inspect Z, stop on W." They should route to
canonical CLI commands, current-state, observe query/explain, evals, security,
architecture, setup/retrofit, and claim-ceiling surfaces without duplicating the
law corpus. Encyclopedia prose, stale checklist summaries, undocumented helper
routes, or hidden author memory fail this phase.

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
surfaces, including the current-state read model and `observe explain --next`
when Gate 92 is the active blocker. It tells an un-oriented agent or Tree the
current candidate digest, dirty/stale state, active phase, strict claim ceiling,
first legal blocker, why that blocker comes first, dependency chain, stale or
wrong-digest evidence, exact next repair, exact narrow rerun, observe query and
explain commands when available, broad rerun only when allowed, forbidden
actions, Gate 92 fitting summary, Product Usage status, worktree eligibility,
and Agent Cockpit state. `--json` is the future cockpit feed; stdout is the
human/agent hot path and must be bounded, redacted, and decisive.

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
for that run when telemetry exists, useful `observe explain --next` or
target-specific observe explain output when blocked, current-state reconciliation
against its source receipts, and source inspection proving the command reads
typed evidence instead of markdown, checklist prose, or last-command heuristics.

Exit requires focused help/routine-path/next-action tests and red fixtures for
missing routine entrypoint, non-navigable help, leaf-only validation
substitution, hidden fit-repo path, omitted target-repo/plugin-activated path,
`scripts/check` failing to delegate or declare itself a narrow helper, missing
`ultragoal next`, generic next-action output, no exact next command, stale digest
accepted as current, checklist prose accepted as authority, missing or stale
current-state source binding, `observe explain --next` returning generic repair
advice, broad audit recommended before narrow observable repair, worktrees marked
eligible while Gate 92/Product Usage/Phase 4 are incomplete, readiness/
update_goal implied from source-local proof, cockpit/UI proof substituted for CLI
proof, private path leakage, and unbounded JSON output. Update checklist rows
with progress statuses only, then commit as source-local/not readiness.

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
