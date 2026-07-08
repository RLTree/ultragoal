## Gate 92 - Full Local Observability Stack Integration And Non-Opaque Failure Law

Gate 92 is additive to Gates 1-91. It does not replace, reduce, defer, satisfy, or weaken any existing gate, stop condition, validation requirement, source/install/cache/app-registry separation requirement, CLI authority requirement, coverage law, Product Fitness law, Product Cohesion law, Product Success law, namespace law, final-packet proof, version sync, app-registry proof, or update_goal gate.

The Harness Ultragoal CLI and plugin must be a fully observability-instrumented enforcement product. Any law-bearing command, check, validator path, fixture path, receipt path, proof path, pass/fail output, metric, audit, package surface, claim guard, or update_goal eligibility path that can run without complete logs, metrics, traces, correlation, diagnostics, queryability, redaction, boundedness, and receipt binding fails the candidate.

This gate cannot be satisfied by better error messages, optional diagnostics, local JSON fallback, docs-only setup, Grafana-only inspection, checklist prose, packet text, claim-ceiling language, shell wrappers, row-shape compliance, hidden network calls, stale telemetry, wrong-digest telemetry, uncorrelated telemetry, unredacted telemetry, unbounded telemetry, or opaque failure output.

Gate 92 also rejects proxy proof. A parser test, schema-valid JSON document,
receipt path, generated inventory row, current-state projection, workflow-engine
output, cache key, timing field, or CLI pass line is not proof unless it
reconciles to actual current-candidate product behavior or to verified
current-input reuse with explicit claim limits. Same-candidate proof means the
same candidate digest, run/correlation identifiers, stdout, receipt, logs,
metrics, traces or wide events, and explain output reconcile for the product
behavior being claimed. Verified cache reuse from an older candidate is routine
acceleration evidence only, even when the input digests are equivalent. Every
claim-bearing row must identify the claim, the product behavior observed, the
proof surface, and the independent reconciliation surface. Unsupported
proof-shaped output must be reported as diagnostic evidence only and must block
the stronger claim.

Gate 92 is rooted in the actual foundational and additional research sources, not in repo-derived summaries alone. The OpenAI Harness Engineering article makes local worktree-scoped logs, metrics, traces, queryability, and agent legibility part of the engineering substrate. The OpenAI Codex repair-loop and agent-improvement-loop research makes structured review/repair/validate records, traces, feedback, evals, ranked changes, handoffs, and before/after validation mandatory loop components. The OpenAI Agents tracing guidance makes full workflow traces, tool/model/guardrail/custom spans, span parentage, immediate export for long-running work, and sensitive-data controls mandatory for any agent-runtime authority. Google SRE monitoring guidance makes freshness, purpose-built metrics, log/metric consistency, four golden signals, saturation/resource signals, and monitoring tests mandatory. Structured-event and high-cardinality research requires wide, ordered, trace-aware raw events with enough context to ask new questions, not write-time aggregate theater. OpenTelemetry semantic conventions require one stable telemetry vocabulary across logs, metrics, traces, resources, schemas, receipts, command inventory, and claim guards.

Therefore Gate 92 closure requires full research-to-law integration for every observability requirement, not a minimum set, sample set, or current-failure slice. Every requirement from every mandatory observability/improvement research source must be mapped through `docs/research-source-registry.json`, `docs/research-article-to-law-trace.json`, canonical law ids, standards rows, source obligations, foundational trace entries, schemas, validator check ids, red fixtures, green fixtures, tamper fixtures, receipt requirements, package inventory, setup/retrofit outputs, claim guards, final-packet fields, and update_goal blockers. Missing, stale, prose-only, alias-only, row-shape-only, checklist-only, package-omitted, fixture-incomplete, or setup/retrofit-omitted research mapping fails Gate 92 and Gate 93.

Required observability stack:

- Docker Compose is the default runtime. Use an existing Docker runtime if present. If Docker runtime is absent and Homebrew is available, install Colima, Docker CLI, and the Docker Compose plugin unless a real technical blocker prevents it. Docker Desktop is acceptable only if already installed or explicitly chosen by the user.
- If Compose cannot run on this machine, the CLI must mint a fail-closed blocker receipt that blocks Gate 92, readiness, release, completion, final packet, and update_goal. That blocker is not completion.
- Repo-owned stack files must exist for `dev/observability/compose.yml`, OpenTelemetry Collector config, Vector config, Grafana datasource provisioning, observability event/metric/trace/receipt/query-result schemas, validator checks for every schema, red/green/tamper fixtures, package inventory entries, standards rows, source-obligation rows, foundational trace entries, and CLI command docs generated from the command inventory.
- Compose must include VictoriaLogs, VictoriaMetrics, VictoriaTraces, OpenTelemetry Collector, Vector, and Grafana. Images must be pinned, not `latest`. Exposed ports must bind to `127.0.0.1`. Retention must be bounded. Volumes must be named. Every service must have a health check. No service may expose public network ports or require secrets. Grafana credentials are local-dev-only and must be documented in receipts as non-production auth.

Required CLI authority:

- Implement through `ultragoal`, not loose shell scripts as authority: `observe stack up`, `observe stack health`, `observe stack smoke`, `observe stack down`, `observe stack gc plan`, `observe stack gc dry-run`, `observe stack gc apply`, `observe logs query`, `observe metrics query`, `observe traces query`, `observe snapshot`, `observe prove`, `observe explain --next`, `observe explain-failure --run-id`, `observe explain-claim --claim-id`, `observe explain-check --check-id`, and `observe explain-law --law-id`.
- Shell scripts may exist only as implementation helpers. CLI receipts are the authority.
- A machine-readable command inventory must cover every current and future `ultragoal` command family, including package digest, source audit, red fixture report, schema validation, mandatory-law validation, standards-gardener, source-obligation validation, foundational trace validation, coverage, line caps, namespace, Product Fitness/Cohesion/Journey, fit-repo, review-round, review-target, archive, final-packet proof, registry probe, install audit, cache audit, transactional finalization, CLI self-law, update-goal eligibility, Rust DevX, GC, session-log hardening, target-repo audit, and observability commands.
- The validator must fail if any command inventory row lacks log instrumentation, metric instrumentation, trace instrumentation, pass output contract, fail output contract, receipt observability binding, focused tests, and claim impact mapping.
- The command inventory must include explicit observability fitting inventory for every law-bearing CLI command, validator check family, receipt/proof path, fixture/report path, and package/plugin surface. Command fitting and surface fitting are both mandatory and distinct. Each row must state `fitting_status` as `fitted`, `partially_fitted`, or `unfitted`, name the fitted surfaces, missing surfaces, validator check id, focused test ids, same-candidate receipt paths, live query proof paths, current owner surface, next unfitted surface, and claim impact. The inventory must also include a validator-checked `fitting_control_board` that recomputes totals by command, surface, operating-loop, and signal family, names the first incomplete row, names its next unfitted surface, and blocks claims when any row is partial, unfitted, stale, or row-shape-only. This inventory plus control board is the Gate 92 tracking surface; mutable checklist prose, side ledgers, adjacent command coverage, or a fitted neighbor cannot stand in for it. Fitted rows must dereference current same-candidate observability receipts and logs/metrics/traces query proof; row shape alone fails. `partially_fitted`, `unfitted`, missing, stale, wrong-digest, local-spool-only, or row-shape-only fitting rows fail Gate 92 and block completion-adjacent claims. A few fitted commands cannot substitute for unfitted commands, validator checks, receipts, fixtures, package resources, plugin surfaces, or claim guards elsewhere in the CLI or plugin.
- Existing `fitting_status` and fitting-board vocabulary is compatibility/status
  vocabulary for the Gate 92 inventory only. It must not justify source paths,
  module names, function names, helper names, test names, ids, or artifact path
  segments named `fitting`, `production_proof`, or equivalent goal/evidence
  labels. Implementation namespaces must describe the product behavior they own,
  such as command telemetry roundtrip, telemetry reconciliation, command
  inventory, receipt dereference, span parentage, cache invalidation, current
  state, or explain planning. New generated paths should use product-behavior
  names; any retained compatibility schema field must be encapsulated at the
  schema boundary and tracked as migration debt if it leaks into implementation
  naming.
- The command inventory must also include an observability operating-loop inventory and signal inventory. Gate 92 treats observability as the repair operating system, not a receipt family. The required loop is: current digest first; run the highest-authority failing command once; query logs, metrics, and traces by run id/correlation id; explain the failure through CLI output before manual artifact inspection; repair the smallest root cause; rerun the narrow command; compare before/after telemetry; and run broad source audit only after the narrow observable proof passes. The required signal classes are CLI-translated latency, traffic, errors, saturation, freshness, correlation, redaction, and boundedness. Each loop stage and signal class must have the same `fitting_status`, fitted/missing surfaces, validator check id, tests, current receipt paths, live query proof paths, and claim impact as command and surface rows. Fitted loop/signal rows must dereference same-candidate telemetry. Partial, unfitted, stale, wrong-digest, or row-shape-only loop/signal rows fail Gate 92 and block completion-adjacent claims.
- Gate 92 must add `observe explain --next` as the tactical repair planner for
  the fitting board. It reads the validator-owned fitting control board, selects
  the first incomplete row by dependency order, and emits a concrete repair plan:
  row id, owner surface, next unfitted surface, missing proof class, implicated
  command or family, stale/wrong-digest evidence if present, exact narrow command
  to run, required logs/metrics/traces query commands, required explain command,
  fixture or source gap, claim impact, and forbidden broad/final actions. It may
  not mark rows fitted, mint readiness, substitute for production proof, or use
  checklist prose as authority.
- Every law-bearing failure stdout is a first-class legibility surface. It must
  include run_id, correlation_id, trace/span ids when available, candidate
  digest, failed law/check/claim ids, where_failed, why_failed, failure_class,
  next_repair, narrow_rerun, claim_impact, receipt path, and exact observe
  logs/metrics/traces query hints. Generic "inspect receipts", "validation
  failed", or wall-of-failures output without priority and narrow repair fails
  Gate 92 even when the receipt shape is valid.
- Gate 92 must expose a compact typed current-state read model. The canonical
  surface should be `ultragoal current-state --json`; a bounded
  `validation_artifacts/current-state.json` snapshot is allowed only as a
  current-candidate read model generated from typed sources such as package
  digest, git status, fitting board, coverage receipt, source audit, red report,
  claim guards, and stale-evidence checks. Current-state output is not proof by
  itself, must name its source receipts and candidate digest, must fail closed on
  stale/wrong-digest inputs, and is the feed consumed by `ultragoal next` and the
  future Agent Cockpit.
- Gate 92 must also expose the parent-owned routine live loop:
  `ultragoal loop run --tier hot --cache-mode verified-local --jobs auto`.
  This is the default source-local agent repair loop, not a final proof command.
  It must compute current digest/package boundary, derive changed files, resolve
  affected law/check/schema/fixture/receipt/query nodes, execute only legally
  sufficient affected work, emit stdout with run/correlation/trace ids and the
  first blocker, query its own logs/metrics/traces when telemetry is available,
  call or link the applicable explain command, emit current-state, and print the
  exact narrow rerun plus claim ceiling. It must be implemented through typed
  task classes and deterministic joins, not a broad serial shell wrapper.
- `ultragoal loop run` must distinguish tiers and proof levels. `--tier hot`
  supports fast repair. `--tier focused` supports one law/check repair path.
  `--tier standard` supports affected source-local validation. `--tier strict`
  or explicit strict commands support broad proof boundaries. `--cache-mode
  verified-local` is the default live loop, while `--cache-mode none` is required
  for clean strict proof when the claim requires it. Warm-cache timing, affected-
  only runs, and current-state snapshots cannot satisfy strict no-cache,
  readiness, release, final-packet, install/cache, app-registry, reviewer,
  completion, or update_goal claims.
- Gate 92 inventory and command telemetry roundtrip must be generated from
  canonical specs, not hand-authored row churn. Add canonical
  `CommandObservabilitySpec` and `SurfaceObservabilitySpec` registries that
  generate command inventory rows, telemetry reconciliation rows, stdout
  contracts, receipt expectations, query proof paths, fixture packs, generated
  roundtrip tests, control-board status, `observe explain --next`,
  current-state inputs, and `ultragoal next` inputs. Add a real command
  telemetry roundtrip runner such as `ultragoal observe fit --command <id>` and
  `ultragoal observe fit --family <family>` that runs the production command or
  surface, captures stdout/receipt, queries logs/metrics/traces, runs explain,
  reconciles same-candidate proof, and refuses fitted status without production
  proof. The public `observe fit` spelling may remain as a compatibility command
  only if its implementation routes into product-semantic modules and functions.
  Row-specific manual edits to generated inventory are debt unless they are
  temporary generated output with source specs and validation.
- The live-loop/audit engine must be a verified incremental query graph. It must
  share per-run package digest, parsed JSON, schema catalog, package inventory,
  source-obligation indexes, foundational trace indexes, namespace file sets,
  coverage/source-tree digest maps, and semantic fixture indexes through a typed
  `AuditContext`. It must cache only by current input digests, validator digest,
  law/schema/fixture versions, args, environment class, and cache mode; every hit
  must record key, hit/miss, invalidation reason, cache class, duration, and
  claim impact. Persistent validator daemon or worker state is allowed only as an
  acceleration layer that verifies current inputs on every live run and never
  becomes authority.
- Gate 92 performance proof must report the observed routine-loop baseline and
  current timing against the 20x live-loop target. The current known scheduled
  audit shape is approximately 181 seconds of work dominated by broad package
  checks and red fixtures, making the routine live-loop 20x target approximately
  9.1 seconds. If that baseline changes, recompute it from current receipts. A
  run can claim the speedup only from live command execution with current digest
  or from verified current-input cache reuse that proves output equivalence and
  declares its routine-only claim ceiling. Same-candidate production proof is
  still required for command fitting, Gate 92 row fitting, and any claim-boundary
  observability row.
  Dry-run estimates, stale receipts, synthetic no-op paths, current-state reads,
  generated row materialization, cache-key construction, graph scheduling
  overhead, local JSON shape checks, or workflow-worker reports are not speed
  proof.
- Every speed-bearing node must record `proof_kind=executed` or
  `proof_kind=verified_cache_hit`. Executed nodes must record command argv,
  exit status, work_unit_count, actual_work_duration_ms, graph_overhead_ms,
  stdout/stderr digests where applicable, result digest, candidate digest,
  worker/task/queue state, receipt/artifact paths, and telemetry run/
  correlation ids. Verified cache hits must record cache key, current input
  digests, validator/law/schema/fixture versions, arguments, environment class,
  prior result digest, replayed output digest, equivalence status, invalidation
  proof, original candidate digest, current candidate digest, original run/
  correlation ids if available, replay telemetry binding or explicit telemetry
  replay gap, and claim impact. Missing fields, `work_unit_count=0` without
  verified cache equivalence, `cache_hit=false` without execution, no result
  digest, or timing from graph overhead alone fails the speed claim.
- The routine loop must absorb the checks and audits that agents actually run on
  every repair path. It is non-compliant to make `ultragoal loop run` fast while
  leaving `scripts/check`, `scripts/check-coverage-full`,
  `scripts/check-coverage-fast`, `ultragoal coverage prove`, source audit, red
  fixture report, line-cap check, namespace check, schema validation,
  mandatory-law validation, source-obligations check, foundational-trace check,
  package inventory scans, focused Rust tests, fmt/build checks, and touched
  fixture/report paths as slow side channels. Each high-frequency validation
  surface must be represented as a typed node in the verified incremental query
  graph, with a canonical full-command baseline, current measured latency,
  affected-set legality proof, cache/input-digest proof, worker/task/queue/cache
  telemetry, and a same-candidate observability receipt. The fast-loop slice
  cannot close while any routinely-run source-local check remains outside the
  loop without a typed serial/destructive/external-live reason and a fail-closed
  claim blocker.
- Every routinely-run source-local check or audit node must meet the live-loop
  law: execute current-candidate affected work or replay a verified
  current-input cache hit, record actual work and graph overhead separately, and
  keep the complete routine loop at or below the current 20x whole-loop target,
  approximately 9.1 seconds unless recomputed from current receipts.
  High-frequency nodes should be at least 20x faster than their canonical
  full-command baselines in routine mode when a legal affected-set or
  cache-equivalent path exists. If a node cannot legally meet that target, it
  must emit a typed blocker or strict-boundary-only reason with claim impact
  instead of being hidden outside the loop.
- Exact coverage is split by proof surface. `scripts/check-coverage-full` and
  strict `ultragoal coverage prove` remain the authoritative full-clean
  claim-boundary proof for 100 percent coverage and `uncovered_records=[]`; they
  do not have to satisfy the routine-loop 20x target and must not be run after
  every small edit by default. Routine coverage feedback must use a separate
  retained-artifact, affected-set, or verified-local mode that proves current
  input equivalence to the authoritative coverage boundary or lowers the claim
  ceiling to `routine_repair_only`. Warm-cache, affected-only, or verified-local
  timing may support only routine repair-loop speed claims, never clean-proof,
  readiness, release, final-packet, install/cache, app-registry, reviewer,
  completion, or update_goal claims.

Required fast-loop speed architecture and dependency order:

1. Speed law arithmetic and baseline provenance must be repaired before any
   faster result can be claimed. The CLI must stop storing a lossy integer
   `speedup_ratio` as authority. It may emit a displayed fixed-point ratio, but
   the authority fields are `baseline_duration_ms`, `product_latency_ms`,
   `actual_work_duration_ms`, `graph_overhead_ms`, `telemetry_reconciliation_ms`,
   `baseline_proof_kind`, and `baseline_invalidation_proof`. A same-command
   baseline cannot support a speedup claim. Boundary baselines must be stored as
   digest-bound artifacts keyed by node id, canonical command, validator digest,
   law/schema/fixture versions, command arguments, environment class, and cache
   mode. If any key component changes, the node fails closed as
   `baseline_stale` until a boundary baseline is regenerated. Red fixtures must
   cover integer truncation, same-command baseline reuse, fabricated baselines,
   stale baselines, and displayed ratios computed from proxy timing.
2. Rust/cache receipt honesty must be repaired with the speed law. Rust receipts
   must record the effective Cargo incremental/cache state observed for the
   command, not a hardcoded desired value. If incremental compilation, target
   directories, sccache, Cargo registry/git cache, or coverage instrumentation
   artifacts influence timing, the receipt must declare them and set the claim
   ceiling accordingly. Hidden cache use blocks no-cache and clean-proof claims.
3. Coverage must be split into strict boundary mode and routine mode before the
   loop can be fast. Strict boundary mode keeps the authoritative full-clean
   exact coverage behavior and remains the only mode that can support complete
   coverage, readiness, release, final-packet, completion, or update_goal
   claims. Routine mode may use documented retained-artifact coverage workflows
   such as `--no-clean` only after a boundary lineage is established, and its
   receipt must include source-tree digest, coverage manifest digest, coverage
   command digest, toolchain and `cargo-llvm-cov` version, flags, target dir,
   boundary lineage digest, cache class, current candidate digest, coverage
   percent, uncovered record count, equivalence status, and
   `claim_ceiling=routine_repair_only`. `scripts/check-coverage-fast` must be
   deleted or made an honest routine-mode alias; a command that runs the full
   pipeline cannot be named or routed as fast.
4. One product-surface input spec must drive both affected-set detection and
   cache keys. Each spec declares path/content rules, law/schema/fixture/
   validator versions, arguments, environment class, cache mode, output digest
   expectations, claim surface, and invalidation reasons. The same spec must
   feed `loop run`, `loop measure`, `scripts/check` delegation, source audit
   affected paths, red fixture affected paths, and command telemetry roundtrip
   rows. Separate coarse tables that can disagree are forbidden. Tamper fixtures
   must prove that mutating a covered input invalidates the row and mutating an
   unrelated input preserves only verified current-input cache reuse with a
   bounded claim ceiling.
5. The execution model must use typed task classes instead of a serial shell
   wrapper. Cargo-touching nodes such as build, test, and fmt must either run as
   a typed serial chain with a reason or in isolated target/cache namespaces.
   Validator-owned nodes such as line caps, namespace, schema validation,
   package inventory, source-obligation checks, foundational trace checks, and
   receipt dereferences must run as in-process typed nodes sharing `AuditContext`
   unless a specific product reason requires a subprocess. In-process nodes must
   record `execution_class=in_process_validator_node`, function id, args,
   source digests, output/result digest, and CLI-equivalence proof; they must
   not fake a shell argv. Remaining subprocesses should avoid login-shell
   overhead unless the command explicitly requires it.
6. Package digest and shared source indexes must be computed once per immutable
   process snapshot and threaded through `AuditContext`. If the tree mutates
   after the snapshot is created, the command must fail closed or create a new
   snapshot; cached digests must never outlive their source boundary. Validation
   must include an internal digest-computation counter or equivalent proof that
   routine commands do not recompute the whole package digest per node/row.
7. Broad audit and red fixture performance must be repaired through shared
   read-only inventories and deterministic parallelism. Source audit text-check
   families must consume one shared parsed-source inventory instead of
   re-walking/re-reading the tree per family. Red fixture execution must share
   only bundle-independent semantic indexes and parse packets once; row-mutated
   bundle state must remain isolated. Acceptance requires byte-equivalent
   pass/fail matrices before/after and tamper fixtures proving no cross-row
   leakage.
8. Observability I/O must be bounded as part of speed compliance. Local spools
   must be segmented/indexed by run/correlation ids with retention bounds,
   exporter calls must be batched or flushed at run boundaries without
   spawn-and-forget loss, timing receipts must retain current candidate plus a
   declared bounded history, and hot-path artifacts must avoid pretty-printed
   bulk output unless a human-readable projection is explicitly requested.
9. Crate/workspace splitting is an endgame optimization, not the first repair.
   It may proceed only after the speed law, coverage modes, input specs,
   execution model, digest snapshot, and audit/red-fixture de-duplication have
   measured residual test/build cost. A spike must split one product-semantic
   leaf crate, preserve namespace law, update coverage/package manifests, and
   measure before/after edit-class timings before broad crate churn is allowed.

Gold-standard observability doctrine required by the synthesis:

- Gate 92 must implement the four-channel model: metrics for alerting, traces
  and wide events for investigation, logs for local detail and forensic
  reconstruction, and evals for behavioral quality. A command can be partially
  fitted without all four channels only when the row declares why a channel is
  not applicable and the validator agrees. Channel absence without typed reason
  fails the row.
- Gate 92 must implement two observability planes. Plane A is product/system
  health: latency, traffic, errors, saturation, freshness, retry/backoff, cache
  state, external/live probe health, and resource pressure. Plane B is agent
  quality: task completion, first-pass success, repair iterations, validation
  failure class, human escalation, bad repair or bad packet rate, post-merge
  regression, eval trend, tool misuse, docs drift, architecture violations, and
  claim-theater escapes.
- Every command/check row must identify which plane and which channel each event,
  metric, log, span, eval, and receipt binding supports. Stack health alone is
  environment proof. Agent quality alone is behavioral proof. Neither can
  complete Gate 92 without per-command fitting and claim binding.
- High-cardinality correlation fields such as run id, trace id, span id, full
  candidate digest, artifact path, receipt path, file path, branch, PR number,
  prompt hash, or user id may be queryable in traces, wide events, logs, and eval
  records, but must not become unbounded metric labels. Metric labels must be
  bounded, redacted, and purpose-built. Digest/path/run-cardinality metric
  labels fail unless represented by a bounded class or hash category approved by
  schema.
- `observe query` and `observe explain` must be agent-legible. For failures they
  must identify the law/check/claim, failed invariant, observed value, expected
  value, where the failure occurred, why it matters, the repair class, the
  smallest likely source surface, exact narrow rerun command, affected claims,
  telemetry gaps, redaction/boundedness state, and before/after comparison path.
  Generic health summaries, stack pings, opaque fail output, or raw log dumps are
  not sufficient.
- Product truth, observability truth, and artifact truth remain distinct.
  Product truth is source/package/runtime behavior. Observability truth is logs,
  metrics, traces, wide events, evals, and queryable records. Artifact truth is
  receipts, manifests, reports, review targets, archives, and final packets.
  These surfaces may cross-reference one another but may not substitute for one
  another unless a law explicitly allows it.
- Gate 92 is working when an agent can execute: digest -> run the failing command
  once -> query logs/metrics/traces by run/correlation/digest -> explain the
  failure through CLI output -> repair the smallest root cause -> rerun the
  narrow command -> compare before/after telemetry -> run broad audit once.
  Any step requiring unstated human archaeology is a fitting gap.

Typed telemetry model:

- Every log event, metric sample, trace span, query result, and observability receipt must carry typed fields for schema, run_id, correlation_id, trace_id, span_id, parent_span_id, command, subcommand, operation, surface, law_id, check_id, claim_id, candidate_digest, target_revision, artifact_path, receipt_path, status, failure_class, why_failed, where_failed, next_repair, claim_impact, timestamp, duration_ms, exporter, redaction_status, bounded_output_status, query_hint_logql, query_hint_promql, and query_hint_traceql.
- Every log event, metric sample, trace span, query result, and observability
  receipt must also carry applicable agent/tool/repo/eval attributes. Required
  attributes include agent system/workflow/step/role/model where an agent or
  automated flow is involved; tool name/type/risk tier/approval/duration/status
  where a tool is invoked; repo name/branch/sha/worktree or target mode where a
  repo surface is involved; and eval suite/case/score/judge/regression status
  where behavioral quality or improvement claims are involved.
- Unknown authority fields, freeform authority blobs, missing fields, wrong digest, wrong correlation id, unredacted secrets, and unbounded output fail.
- Every command/check must emit a wide structured event carrying enough contextual dimensions to diagnose unknown failures without new instrumentation: command family, argument surface, task class, worker/task/queue state, cache key/mode/hit status, filesystem/package surface, receipt/schema/law graph digests, source/install/cache/app surface, retry/backoff state, resource saturation state, and before/after comparison anchors where a repair loop is in progress. Aggregated metrics may be derived from events, but aggregate-only telemetry cannot satisfy observability or repair-loop claims.
- Every command and every check must emit structured logs to VictoriaLogs through the live stack and to a bounded local JSONL spool for fallback/forensics. Local JSONL fallback is transition evidence only and cannot complete Gate 92 without live VictoriaLogs ingestion and query proof.
- Every command and check must emit VictoriaMetrics metrics for command totals/durations, check totals/failures, law failures, receipt dereferences, stale receipts, digest mismatches, claim blocks, red fixture totals/failures, proof graph cycles, registry unsupported events, observability emit failures, exporter retries/drops, stack health, and stack smoke. Labels must be bounded and may not leak secrets or unbounded paths.
- Every CLI command opens a root span. Validator checks, schema parses, receipt dereferences, fixture runs, claim-ceiling calculations, final-packet dereferences, registry/install/cache probes, and exporter calls create child spans. Broken parentage fails Gate 92.

Pass/fail output and receipt binding:

- Every pass stdout states what was proven, candidate digest, receipt path, observability run id, supported claims, and explicitly unsupported claims.
- Every fail stdout states failed law/check ids, why it failed, where it failed, claim impact, next repair action, receipt path, run_id/correlation_id, and exact observe query commands for logs, metrics, and traces.
- Every law-bearing receipt must reference observability receipt path, log stream digest, metric snapshot digest, trace bundle digest, query examples, redaction proof, retention/bounds proof, candidate digest, and run_id/correlation_id.
- A receipt without current same-candidate observability binding cannot support any Harness claim. Stale, wrong-digest, unqueryable, forgeable, or local-spool-only telemetry fails.

Agent-queryable proof:

- Grafana is a human dashboard only. Agent proof must come from CLI query commands against VictoriaLogs, VictoriaMetrics, and VictoriaTraces.
- Required query proof includes failed run by run_id, failed law by law_id, failed check by check_id, blocked claim by claim_id, command duration metrics, stale receipt counters, full command trace, and the current `final_packet_proof_source_audit_target_digest_mismatch` failure across logs, metrics, and traces.
- The current blocker must be visible through stdout, source audit receipt, final-packet proof receipt, VictoriaLogs, VictoriaMetrics, VictoriaTraces, `observe logs query`, `observe metrics query`, `observe traces query`, and `observe explain-failure --run-id`.

Security, redaction, and boundedness:

- Telemetry must not leak API keys, tokens, cookies, Authorization headers, database URLs, private local proof paths except typed local-dev category evidence, raw private session logs, or full user home paths in public/package claims.
- Validator checks must inspect logs, metric labels, traces, receipts, query output, and local spool for leaks.
- Every query must have row limit, byte limit, timeout, retention bound, cardinality guard, output truncation marker, and claim impact when truncated.
- Hidden background exporters, spawn-and-forget telemetry tasks, unmanaged child processes, unbounded retention, unbounded queues, and public port binding fail.

Fixtures and law surfaces:

- Add red fixtures for missing log event, missing metric, missing trace, missing or mismatched correlation id, wrong candidate digest, stale telemetry, unredacted secret, private path leak, public port binding, unbounded retention, unbounded query, hidden network endpoint, missing repair hint, opaque pass output, opaque fail output, receipt without observability binding, forged log bundle, forged metric bundle, forged trace bundle, metric/log/trace digest mismatch, broken span parentage, command missing from inventory, command inventory row without instrumentation proof, and local JSON fallback used as completion proof.
- Add green fixtures for complete live-stack observability and tamper fixtures for forged telemetry.
- Gate 92 must have same-law-id enforcement across agent standards, source obligations, foundational trace, schemas, validator check ids, red fixtures, green fixtures, tamper fixtures, receipts, package inventory, claim guards, and final packet fields.
- Observability configs and schemas are package resources. Source stack proof does not imply installed plugin observability proof. Installed plugin proof does not imply app-registry/reviewer exposure. Cache proof does not imply live stack proof. Every surface must identify which surface emitted telemetry.

Required Gate 92 validation:

- Run and record runtime detection, stack up, stack health, stack smoke, query logs, query metrics, query traces, explain current failure, focused observability tests, observability red/green/tamper fixtures, exact coverage, line-cap scan, source audit, red fixture report, package digest, git status, and checkpoint commit.
- Run and record research-source registry validation and article-to-law trace validation for every mandatory Gate 92 research source before claiming any Gate 92 progress. A Gate 92 receipt that lacks same-candidate research-source and article-to-law proof is incomplete even if stack health, smoke, and query commands pass.
- Do not refresh install/cache, bump version, finalize packet, claim registry/reviewer exposure, claim readiness/release/completion, launch parallel lanes, or call update_goal until Gate 92 and all prior gates pass on the same candidate digest.
