## Checklist Addition: Gate 92 - Full Local Observability Stack Integration And Non-Opaque Failure Law

This checklist section is a tracking surface only. It does not weaken Gate 92 and does not replace Gates 1-91, source/install/cache/app-registry separation, CLI authority, CLI self-law, coverage, Product Fitness, Product Cohesion, Product Success, namespace law, final-packet proof, version sync, or update_goal gates. Do not check an item unless the implementation is represented in standards rows, source-obligation rows, foundational trace entries, schemas, validator checks, red/green/tamper fixtures, package inventory, claim guards, CLI receipts, live stack receipts, and current same-candidate evidence.

### Gate 92.1: Doctrine And Stop Conditions

- [ ] Gate 92 is represented in the canonical prompt, checklist, required claim graph, stop condition 104, standards rows, source-obligation rows, foundational trace, schema catalog, validator checks, fixtures, receipts, package inventory, claim guards, and final packet fields.
  - Evidence:
  - Command:
  - Candidate digest:
  - Claim impact:
  - Status: in progress

- [ ] Gate 92 implementation is rooted in the actual mandatory research sources, not only repo-derived summaries. OpenAI Harness Engineering, OpenAI Codex repair loops, OpenAI Agents observability/tracing, Google SRE monitoring and four golden signals, structured-event/high-cardinality doctrine, OpenTelemetry semantic conventions, OpenAI agent-improvement loop, and self-improving domain-agent research are all current in the research-source registry and article-to-law trace before any Gate 92 progress is checked.
  - Status: not started

- [ ] No Gate 92 row may be checked from a minimum surface, sample source, representative command, current-failure-only proof, stack-health-only proof, query-only proof, or adjacent fitted surface. Every command, validator check family, receipt/proof path, fixture/report path, package/plugin/setup/retrofit surface, operating-loop stage, signal class, long-running path, external/live path, and claim guard must be fitted or must explicitly block Gate 92 and all dependent claims.
  - Evidence:
  - Command inventory:
  - Fitting control board:
  - Candidate digest:
  - Claim impact:
  - Status: in progress

- [ ] No law-bearing command, check, validator path, fixture path, receipt path, proof path, pass/fail output, metric, audit, package surface, claim guard, or update_goal eligibility path can run without complete logs, metrics, traces, correlation, diagnostics, queryability, redaction, boundedness, and receipt binding.
  - Evidence:
  - Command inventory:
  - Validator check:
  - Candidate digest:
  - Claim impact:
  - Status:

- [ ] Local JSON fallback, Grafana-only inspection, docs-only setup, checklist prose, packet text, claim-ceiling language, shell wrapper output, row-shape compliance, and stale/wrong-digest telemetry cannot satisfy Gate 92 or any completion-adjacent claim.
  - Evidence:
  - Red fixtures:
  - Candidate digest:
  - Claim impact:
  - Status:

### Gate 92.2: Repo-Owned Stack And Runtime Setup

- [ ] Docker/Compose runtime detection is CLI-routed, receipt-bound, and blocks Gate 92 when Compose cannot run.
  - Evidence:
  - Command:
  - Receipt:
  - Candidate digest:
  - Claim impact:
  - Status:

- [ ] Repo-owned stack files exist and are package-included: `dev/observability/compose.yml`, `dev/observability/otel-collector/config.yaml`, `dev/observability/vector/vector.yaml`, and `dev/observability/grafana/provisioning/datasources/datasources.yml`.
  - Evidence:
  - Package inventory entries:
  - Validator check:
  - Candidate digest:
  - Status:

- [ ] Observability schemas exist and are validator-owned: `schemas/observability-event.schema.json`, `schemas/observability-metric.schema.json`, `schemas/observability-trace.schema.json`, `schemas/observability-receipt.schema.json`, and `schemas/observability-query-result.schema.json`.
  - Evidence:
  - Schema catalog:
  - Validator check:
  - Candidate digest:
  - Status:

- [ ] Compose stack includes VictoriaLogs, VictoriaMetrics, VictoriaTraces, OpenTelemetry Collector, Vector, and Grafana with pinned images, `127.0.0.1` port bindings, bounded retention, named volumes, service health checks, no public ports, and no production secrets.
  - Evidence:
  - Command:
  - Receipt:
  - Candidate digest:
  - Claim impact:
  - Status:

### Gate 92.3: CLI Observability Commands

- [ ] `ultragoal observe stack up`, `health`, `smoke`, `down`, `gc plan`, `gc dry-run`, and `gc apply` are implemented, typed, receipt-bound, and validator-enforced.
  - Evidence:
  - Command inventory:
  - Receipts:
  - Candidate digest:
  - Status:

- [ ] `ultragoal observe logs query`, `metrics query`, `traces query`, `snapshot`, `prove`, `explain-failure --run-id`, `explain-claim --claim-id`, `explain-check --check-id`, and `explain-law --law-id` are implemented, typed, bounded, receipt-bound, and validator-enforced.
  - Evidence:
  - Command inventory:
  - Receipts:
  - Candidate digest:
  - Status:

- [ ] Shell scripts, raw Docker commands, raw Grafana inspection, and local JSON spool output cannot act as Harness claim authority without CLI receipts.
  - Evidence:
  - Red fixtures:
  - Validator check:
  - Candidate digest:
  - Claim impact:
  - Status:

### Gate 92.4: Typed Telemetry, Logs, Metrics, And Traces

- [ ] Every observability event, metric sample, trace span, query result, and observability receipt carries typed fields for schema, run/correlation/span ids, command, operation, surface, law/check/claim ids, candidate digest, artifact/receipt paths, status, failure class, why/where/next repair, claim impact, timestamp, duration, exporter, redaction status, bounded output status, and query hints.
  - Evidence:
  - Schemas:
  - Validator check:
  - Candidate digest:
  - Status: in progress

- [ ] Unknown authority fields, freeform authority blobs, missing fields, wrong digest, wrong correlation id, unredacted secrets, and unbounded output fail validation.
  - Evidence:
  - Red fixtures:
  - Tamper fixtures:
  - Candidate digest:
  - Status:

- [ ] Every command and every check emits structured logs to VictoriaLogs and bounded local JSONL fallback, with failure logs naming failed law/check, pointer/path, digest, reason, blocked claim, next repair, and query hints.
  - Evidence:
  - Live query:
  - Local spool:
  - Receipt:
  - Candidate digest:
  - Status:

- [ ] Every command and check emits bounded VictoriaMetrics metrics for commands, durations, check failures, law failures, receipt dereferences, stale receipts, digest mismatches, claim blocks, red fixtures, proof graph cycles, registry unsupported events, exporter retries/drops, stack health, and stack smoke.
  - Evidence:
  - Live query:
  - Metric snapshot:
  - Receipt:
  - Candidate digest:
  - Status:

- [ ] Every CLI command opens a root span, validator/schema/receipt/fixture/claim/packet/surface/exporter operation creates child spans, and broken parentage fails Gate 92.
  - Evidence:
  - Live query:
  - Trace bundle:
  - Receipt:
  - Candidate digest:
  - Status:

### Gate 92.5: Command Inventory And Output Contracts

- [ ] Machine-readable command inventory covers every current `ultragoal` command family and fails future commands until inventory, instrumentation, tests, and claim-impact mapping are added.
  - Evidence:
  - Inventory path:
  - Validator check:
  - Candidate digest:
  - Status:

- [ ] Machine-readable observability fitting inventory tracks every law-bearing CLI command, validator check family, receipt/proof path, fixture/report path, package surface, and plugin surface as `fitted`, `partially_fitted`, or `unfitted`, names the current owner surface and next unfitted surface for each row, includes a validator-recomputed `fitting_control_board`, and the validator fails every partial, unfitted, missing, stale, adjacent-surface-substituted, count-mismatched, pass-shaped-control-board, or row-shape-only fitting row.
  - Status: in progress; `final-packet prove` and `update-goal eligibility` are fitted, `self update-goal eligibility` remains partially fitted, and the first incomplete command row is `self update-goal eligibility`.
  - Claim impact: partial/unfitted rows mechanically block Gate 92, readiness, release, completion, final-packet correctness, and `update_goal()` eligibility.

- [ ] Machine-readable operating-loop and signal inventory tracks whether observability is actually usable as the repair loop: current digest first, failing command capture, logs/metrics/traces query by run id, CLI explanation before manual artifact inspection, smallest repair, narrow rerun, before/after telemetry comparison, broad-audit gating, freshness, and the CLI-translated latency/traffic/error/saturation/freshness/correlation/redaction/boundedness signal model.
  - Status: in progress
  - Remaining gap: complete before/after repair comparison and full command/surface/signal coverage are still incomplete.
  - Claim impact: partial/unfitted operating-loop or signal rows mechanically block Gate 92, readiness, release, completion, final-packet correctness, and `update_goal()` eligibility.

- [ ] Every pass stdout states what was proven, candidate digest, receipt path, observability run id, supported claims, and explicitly unsupported claims.
  - Evidence:
  - Focused tests:
  - Validator check:
  - Candidate digest:
  - Status:

- [ ] Every fail stdout states failed law/check ids, candidate digest, run/correlation ids, trace/span ids where available, where_failed, why_failed, failure_class, claim impact, next repair action, exact narrow rerun, receipt path, and exact observe query commands for logs, metrics, and traces.
  - Evidence:
  - Focused tests:
  - Validator check:
  - Candidate digest:
  - Status:

- [ ] `observe explain --next` turns the fitting control board's first incomplete row into a concrete Gate 92 repair plan without marking the row fitted or substituting for production proof.
  - Working means the output names the row id, owner surface, next unfitted surface, missing proof class, implicated command or family, stale/wrong-digest evidence if present, exact narrow command, required logs/metrics/traces query commands, required explain command, source or fixture gap, claim impact, and forbidden broad/final actions.
  - Validation:
  - Production proof:
  - Candidate digest:
  - Status: not started

- [ ] A compact typed current-state read model exists, preferably `ultragoal current-state --json`, with optional bounded `validation_artifacts/current-state.json` generated only from real typed authority surfaces.
  - Working means current-state reports current package digest, git dirty/stale state, Gate 92 fitting-board summary, coverage status, first blockers, source-audit/red-report status, claim ceiling, and the source receipts it read, while failing closed on stale, wrong-digest, missing, checklist-derived, or unbounded inputs.
  - Validation:
  - Production proof:
  - Candidate digest:
  - Status: validated current

- [ ] `ultragoal loop run --tier hot --cache-mode verified-local --jobs auto` exists as the parent-owned routine live repair command.
  - Working means one command computes the current candidate and package boundary, detects changed files, resolves affected law/check/schema/fixture/receipt/query nodes, runs only legally sufficient impacted work, emits stdout with run/correlation/trace ids and first blocker, queries same-candidate logs/metrics/traces when available, invokes or links the applicable explain command, emits current-state, and prints exact next repair, exact narrow rerun, broad-rerun allowance, forbidden actions, and claim ceiling.
  - Validation means parser/help/schema/cache-key/bounds/redaction/unit tests pass.
  - Production proof means a real current-candidate command run reconciles stdout, receipt, logs, metrics, traces, explain output, current-state, source inspection, and explicit source-local claim ceiling.
  - Status: implemented, pending validation

- [ ] Live-loop performance proof demonstrates the 20x routine source-local speed target without confusing live repair with strict no-cache proof.
  - Working means the command records duration_ms, worker_count, task_count, queue_depth, critical path, affected node count, skipped node count, cache mode, cache hit rate, invalidation reasons, cold/warm timing class, candidate digest, and claim impact. The known baseline is approximately 181 seconds scheduled audit work, so 20x routine live-loop target is approximately 9.1 seconds unless current receipts recompute a different baseline. The measured loop must include the routinely-run source-local validation surfaces, not merely a wrapper around current-state.
  - Validation means timing fields, per-node baselines, per-node 20x thresholds, fail-closed timing classes, and slow-hidden-side-channel rejection are covered by focused tests and fixtures.
  - Production proof means measured live command runs on current digest, includes all required high-frequency routine nodes, proves each included node either executed affected current-candidate work or replayed verified current-input cache equivalence with explicit routine-only claim limits, proves the whole routine loop completes at or below the current 9.1 second target, and does not use dry-run estimates, stale receipts, synthetic no-op paths, workflow-engine output, or warm-cache timing as clean proof. Strict full-clean coverage, full source audit, full red fixture report, and final source-local proof remain separate claim-boundary commands and must not be treated as the ordinary edit loop.
  - Status: implemented, pending validation

- [ ] Live-loop speed claims prove actual work or verified reuse, not proxy timing.
  - Working means every speed-bearing node records `proof_kind=executed` or `proof_kind=verified_cache_hit`, actual_work_duration_ms, graph_overhead_ms, work_unit_count, command argv when executed, result digest, output digest where applicable, candidate digest, worker/task/queue state, receipt/artifact paths, and telemetry run/correlation ids. Verified cache hits additionally record cache key, current input digests, validator/law/schema/fixture versions, arguments, environment class, prior result digest, replayed output digest, equivalence status, invalidation proof, and claim impact.
  - Validation means red/green/tamper fixtures fail synthetic/key-only timing, current-state-only timing, graph-overhead-only timing, generated-row timing, workflow-output-as-proof, `cache_hit=false` with no execution, `work_unit_count=0` with no verified cache equivalence, missing result digest, stale/wrong-digest cache reuse, and speedup ratio computed from proxy duration.
  - Production proof means a real current-candidate loop run shows the displayed speedup is computed from executed work on the current candidate or verified current-input cache reuse with explicit claim limits for every included node. Same-candidate stdout/receipt/logs/metrics/traces/explain/current-state reconciliation is required for executed nodes and for any row that claims production command fitting. Cache reuse from an older candidate is routine acceleration evidence only, not same-candidate production proof.
  - Status: implemented, pending validation

- [ ] Speed-law arithmetic and baseline provenance are honest.
  - Working means `speedup_ratio` is not a lossy precomputed integer authority field; the authoritative fields are baseline_duration_ms, product_latency_ms, actual_work_duration_ms, graph_overhead_ms, telemetry_reconciliation_ms, baseline_proof_kind, and baseline_invalidation_proof. Boundary baselines are digest-bound by node id, canonical command, validator digest, law/schema/fixture versions, args, environment class, and cache mode. Same-command baseline reuse, fabricated baselines, stale baselines, and proxy-duration baselines fail closed.
  - Validation means focused unit tests and red/tamper fixtures cover integer-truncation regressions, same-command baseline reuse, stale baseline reuse, fabricated baseline rows, narrow-command divergence from canonical behavior, and displayed speedup computed from graph overhead or current-state projection.
  - Production proof means at least one real current-candidate `loop measure --node <id> --tier hot --cache-mode verified-local` run emits nonzero honest ratio projection from valid baseline and product latency fields, while failing nodes identify exact baseline repair.
  - Status: not started

- [ ] Rust and Cargo cache receipts report effective cache truth.
  - Working means Rust receipts record the effective Cargo incremental state, target dir, coverage target dir, sccache/RUSTC_WRAPPER state, Cargo registry/git cache class, warm/no-cache class, and hidden-cache claim impact from observed command/runtime state rather than hardcoded desired values.
  - Validation means focused tests and fixtures fail hardcoded `cargo_incremental`, warm-cache proof used as no-cache proof, missing target/cache namespace, hidden Cargo cache dependency, and receipt/cache state mismatch.
  - Production proof means a real Rust/coverage/live-loop command receipt matches observed local cache state and lowers claim ceiling when cache use cannot support no-cache or clean-proof claims.
  - Status: not started

- [ ] Coverage has separate strict-boundary and routine-repair modes.
  - Working means strict boundary mode keeps full-clean authoritative exact coverage and remains the only mode that can support complete coverage or completion-adjacent claims. Routine mode may use retained-artifact or warm coverage only with source-tree digest, coverage manifest digest, coverage command digest, toolchain and cargo-llvm-cov version, flags, target dir, boundary lineage digest, cache class, current candidate digest, coverage percent, uncovered record count, equivalence status, and `claim_ceiling=routine_repair_only`. `scripts/check-coverage-fast` is deleted or becomes an honest routine-mode alias; it may not run the full pipeline while pretending to be fast.
  - Validation means timed clean-vs-routine tests, schema checks, and red fixtures fail warm coverage presented as strict proof, stale coverage cache, wrong package/source/changed-files digest, coverage-fast substituted for exact coverage, generated-code or exclusion gaming, and missing coverage cache class.
  - Production proof means full-clean coverage still proves 100 percent with `uncovered_records=[]` at the claim boundary, while routine coverage accelerates repair loops and explicitly blocks readiness/release/completion/final/update_goal claims.
  - Status: not started

- [ ] Per-surface input specs are the single authority for affected sets and cache keys.
  - Working means each high-frequency node has a product-surface input spec declaring path/content rules, law/schema/fixture/validator versions, args, environment class, cache mode, output digest expectations, claim surface, and invalidation reasons. The same spec drives `loop run`, `loop measure`, `scripts/check` delegation, source audit affected paths, red fixture affected paths, cache keys, and command telemetry roundtrip rows. Candidate digest remains in receipts for reconciliation and enters a cache key only for surfaces that truly depend on the whole package boundary.
  - Validation means tamper fixtures prove covered-input mutation misses cache, unrelated-input mutation preserves only verified current-input cache reuse with a bounded claim ceiling, stale/wrong-digest rows fail, affected-set/cache-key divergence fails, and broad `_ => true` fallback cannot hide missing specs.
  - Production proof means a real two-edit sequence records expected hit/miss behavior, invalidation reasons, current input digests, result/output digests, and claim ceilings in the live-loop timing receipt.
  - Status: not started

- [ ] Loop execution uses typed task classes and in-process validator nodes.
  - Working means cargo-touching nodes run as a typed serial chain or isolated target/cache namespaces, while validator-owned nodes such as line caps, namespace, schema validation, package inventory, source-obligation checks, foundational trace checks, and receipt dereferences run as in-process typed nodes sharing `AuditContext` unless a product reason requires a subprocess. In-process rows record execution_class, function id, args, source digests, output/result digest, and CLI-equivalence proof. Remaining subprocesses avoid login-shell overhead unless required.
  - Validation means scheduler/unit/fixture tests fail hidden global serial locks, unbounded concurrency, nondeterministic joins, shared validation_artifacts worker writes, in-process node output divergence from CLI behavior, missing execution_class, and fake argv for non-subprocess work.
  - Production proof means measured loop wall time drops from serial sum toward the critical path while preserving deterministic results, stdout/receipt/logs/metrics/traces/explain reconciliation, and explicit typed serial reasons.
  - Status: not started

- [ ] Package digest and shared source indexes are computed once per immutable command snapshot.
  - Working means `AuditContext` owns package digest, parsed JSON, schema catalog, package inventory, source-obligation indexes, foundational trace indexes, namespace file sets, source-tree digest maps, and semantic fixture indexes for one immutable snapshot. If source mutates after snapshot creation, the command fails closed or starts a new snapshot.
  - Validation means tests or counters prove routine commands do not recompute whole-package digest per node/row and fail if cached snapshot state survives source mutation.
  - Production proof means a real loop run emits digest-computation count or equivalent telemetry, uses one snapshot digest across nodes, and reconciles receipts to that digest.
  - Status: not started

- [ ] Broad audit and red fixture execution reuse safe shared read-only inventories.
  - Working means source-audit text families consume one shared parsed-source inventory with deterministic parallel merge, and red fixtures share only bundle-independent semantic indexes while keeping row-mutated bundle state isolated. Packets are parsed once per row and touched-fixture reports select actual affected subsets from the per-surface input specs.
  - Validation means before/after pass-fail matrices are byte-equivalent, tamper fixtures prove no cross-row leakage, and hidden broad re-walk/re-read behavior is measured or blocked.
  - Production proof means source audit and red fixture report timing improves with worker/task/queue metrics and no change in verdicts.
  - Status: not started

- [ ] Observability I/O is bounded on the hot path.
  - Working means local spools are segmented and indexed by run/correlation ids with retention bounds, exporter emissions flush at run boundaries without spawn-and-forget loss, timing receipts retain current candidate plus declared bounded history, and hot-path artifacts avoid pretty-printed bulk output unless a human projection is explicitly requested.
  - Validation means fixtures fail unbounded spool reads, unbounded retention, missing truncation markers, lost exporter flush, hot-path pretty bulk artifacts, and deletion of records still referenced by active receipts.
  - Production proof means observe query and loop-run latency improve while query output remains bounded, redacted, and reconciled to current run/correlation ids.
  - Status: not started

- [ ] High-frequency validation surfaces are inside the verified incremental loop rather than slow side channels.
  - Working means `scripts/check`, `scripts/check-coverage-full`, `scripts/check-coverage-fast`, `ultragoal coverage prove`, source audit, red fixture report, line caps, namespace, schema validation, mandatory-law validation, source-obligations, foundational trace, package inventory scans, focused Rust tests, fmt/build checks, and touched fixture/report paths are represented as typed query-graph nodes with affected-set resolution, cache/input-digest keys, worker/task/queue/cache telemetry, source-local claim impact, and exact narrow rerun commands.
  - Validation means parser/unit/table tests and red fixtures fail closed when a routinely-run check is missing from the loop, left as an external slow helper without typed reason, has no baseline, lacks a typed routine target or strict-boundary-only blocker, accepts stale/wrong-digest cache, hides serial execution, omits affected fixtures, substitutes coverage-fast for strict exact coverage, or reports wrapper-only timing.
  - Production proof means a real current-candidate loop run executes current-candidate work or legally reuses verified current-input cache proof with explicit routine-only claim limits for every required high-frequency node, reconciles stdout/receipt/logs/metrics/traces/explain/current-state for executed nodes, and names any typed serial/destructive/external-live exception as a blocker rather than fitting the row.
  - Status: not started

- [ ] Exact coverage is split into strict boundary proof and fast routine feedback.
  - Working means `scripts/check-coverage-full` and strict `ultragoal coverage prove` remain authoritative full-clean boundary proof with canonical baselines, 100 percent coverage, and `uncovered_records=[]` as the only passing strict exact-coverage condition. Routine coverage feedback has a separate retained-artifact, affected-set, or verified-local node with current input digests, boundary lineage, equivalence status, cache class, and `claim_ceiling=routine_repair_only`. The strict coverage command is not required to satisfy the routine-loop 20x target, and it must not be run after every small edit by default.
  - Validation means tests/fixtures fail closed for missing coverage baseline, stale coverage cache, wrong package/source/changed-files digest, coverage-fast substituted for full exact coverage, generated-code/exclusion gaming, coverage timing without worker/task/queue/cache fields, and warm-cache proof presented as strict no-cache proof.
  - Production proof means the current loop run includes routine coverage feedback when coverage is relevant to the edit class, emits/query-proves coverage latency and cache honesty, reconciles to the current exact coverage boundary lineage or lowers the claim ceiling, and keeps strict no-cache final coverage proof separate from routine verified-local speed proof.
  - Status: not started

- [ ] Verified incremental audit query engine backs the routine loop.
  - Working means a shared `AuditContext`, query-DAG check engine, verified content-addressed cache, decomposed package/text/coverage/check/audit nodes, grouped red fixtures with shared semantic indexes, minimal or copy-on-write isolated fixture roots, deterministic result ordering, and optional persistent validator daemon/worker state all verify current input digests before reuse.
  - Fail-closed cases include stale or wrong-digest cache hit, cache hit without invalidation reason, warm-cache timing used as clean proof, hidden worker cap, hidden global serial lock, unbounded concurrency, nondeterministic result ordering, omitted affected fixture, omitted high-frequency check, slow helper hidden outside the loop, and parallel worker writing shared `validation_artifacts/**`.
  - Status: implemented, pending validation

- [ ] Gate 92 fitting compiler and runner replace hand-authored row churn.
  - Working means canonical `CommandObservabilitySpec` and `SurfaceObservabilitySpec` registries generate command inventory rows, fitting rows, stdout contracts, receipt expectations, logs/metrics/traces query paths, fixture packs, generated fitting tests, fitting control-board status, `observe explain --next`, current-state inputs, and `ultragoal next` inputs. A real runner such as `ultragoal observe fit --command <id>` and `ultragoal observe fit --family <family>` runs the production command/surface, captures stdout/receipt, queries telemetry, runs explain, reconciles same-candidate proof, and refuses fitted status without production proof.
  - Validation means generated table-driven tests and fixture semantics pass.
  - Production proof means at least one command family is fitted through the compiler/runner without manual generated-row edits, with same-candidate telemetry and explain reconciliation.
  - Status: implemented, pending validation

- [ ] Gate 92 command-roundtrip implementation obeys Gate 90 product-semantic namespace law.
  - Working means any compatibility vocabulary such as `fitting_status`, `fitting_control_board`, or public `observe fit` remains contained at schema/parser/status boundaries, while source paths, modules, functions, helpers, tests, ids, and artifact path segments use product-behavior names such as command roundtrip, telemetry reconciliation, command inventory, receipt dereference, span parentage, cache invalidation, current state, or explain planning.
  - Validation means namespace checks and red/green/tamper fixtures fail goal-work implementation names such as `observe/fitting`, `production_proof`, `fit_command`, `fit_path`, and `validation_artifacts/observability/fitting/...`, while allowing product vocabulary such as `fit-repo` and behavior names such as `command_roundtrip`.
  - Production proof means the real `observe fit` compatibility command, if retained, routes into product-semantic implementation modules and emits product-legible stdout/receipt paths without goal-work artifact namespaces.
  - Status: validated current

- [ ] Workflow-engine use is explicitly bounded for Gate 92 acceleration work.
  - Working means the parent owns execution, source changes, command runs, telemetry proof, and claim ceilings. The dynamic workflow engine may design or suggest a workflow only when the parent inspects, supplements, and verifies it through canonical `ultragoal` surfaces. Workflow output, worker output, or generated plans cannot satisfy Gate 92, Product Usage, Phase 4, readiness, release, final packet, worktree eligibility, or update_goal claims.
  - Status: not started

### Gate 92.6: Receipt Binding And Agent-Queryable Proof

- [ ] Every law-bearing receipt references observability receipt path, log stream digest, metric snapshot digest, trace bundle digest, query examples, redaction proof, retention/bounds proof, candidate digest, and run/correlation ids.
  - Evidence:
  - Validator check:
  - Red fixtures:
  - Candidate digest:
  - Claim impact:
  - Status:

- [ ] Agent proof comes from CLI queries against VictoriaLogs, VictoriaMetrics, and VictoriaTraces; Grafana inspection is not claim authority.
  - Status: in progress; current update-goal failure is queryable through CLI logs, metrics, traces, and explain output; self update-goal same-candidate live query proof remains pending.
  - Claim impact: source-local query observation only; no Grafana/manual proof is used, and full Gate 92 remains blocked by unfitted command, plugin surface, operating-loop, and signal inventory rows.

- [ ] Required query proof covers failed run by run_id, failed law by law_id, failed check by check_id, blocked claim by claim_id, command duration metrics, stale receipt counters, full command trace, and current proof-graph failure across logs, metrics, and traces.
  - Status: in progress; current-run logs, metrics, and explain proof exist, but trace query proof and the complete required query matrix across all law-bearing paths do not.

### Gate 92.7: Security, Redaction, Boundedness, And Resource Discipline

- [ ] Logs, metrics labels, traces, receipts, query output, and local spool reject API keys, tokens, cookies, Authorization headers, database URLs, private local proof paths except typed local-dev category evidence, raw private session logs, and full user home paths in public/package claims.
  - Evidence:
  - Red fixtures:
  - Validator check:
  - Candidate digest:
  - Status: in progress

- [ ] Every query has row limit, byte limit, timeout, retention bound, cardinality guard, truncation marker, and claim impact when truncated.
  - Evidence:
  - Focused tests:
  - Query receipts:
  - Candidate digest:
  - Status:

- [ ] Hidden background exporters, spawn-and-forget telemetry tasks, unmanaged child processes, unbounded queues, unbounded retention, and public port binding fail Gate 92.
  - Evidence:
  - Red fixtures:
  - Validator check:
  - Candidate digest:
  - Status:

### Gate 92.8: Fixtures, Standards, Traceability, And Package Integration

- [ ] Observability red fixtures cover missing log, metric, trace, correlation id, wrong digest, stale telemetry, leaks, public ports, unbounded retention/query, hidden endpoints, missing repair hint, opaque pass/fail output, missing receipt binding, forged bundles, digest mismatches, broken span parentage, missing command inventory row, missing instrumentation proof, and local JSON fallback used as completion proof.
  - Evidence:
  - Red fixture ids:
  - Red report:
  - Candidate digest:
  - Status:

- [ ] Green fixtures prove complete live-stack observability and tamper fixtures reject forged telemetry.
  - Evidence:
  - Green fixtures:
  - Tamper fixtures:
  - Candidate digest:
  - Status:

- [ ] Gate 92 has same-law-id enforcement across agent standards, source obligations, foundational trace, schemas, validator checks, red fixtures, green fixtures, tamper fixtures, receipts, package inventory, claim guards, and final packet fields.
  - Evidence:
  - Standards rows:
  - Source obligations:
  - Trace entries:
  - Candidate digest:
  - Status:

- [ ] Observability configs and schemas are package resources, and source, installed plugin, cache, live stack, app-registry, and reviewer exposure observability proofs remain separate and non-substitutable.
  - Evidence:
  - Package inventory:
  - Validator check:
  - Candidate digest:
  - Claim impact:
  - Status:

### Gate 92.9: Stack Health, Smoke, Current Failure, And Final Validation

- [ ] Stack health proves VictoriaLogs, VictoriaMetrics, VictoriaTraces, OpenTelemetry Collector, Vector, and Grafana are running and healthy.
  - Status: validated current for source-local stack health only; no final packet, readiness, release, registry/reviewer exposure, completion, or `update_goal()` claim.

- [ ] Stack smoke proves log ingestion/query, metric ingestion/query, trace ingestion/query, and one correlated ultragoal CLI run visible in all three stores.
  - Status: validated current for source-local stack smoke/query only; no final packet, readiness, release, registry/reviewer exposure, completion, or `update_goal()` claim.

- [ ] The current proof-graph failure is visible through stdout, source audit receipt, final-packet proof receipt, VictoriaLogs, VictoriaMetrics, VictoriaTraces, observe query commands, and `observe explain-failure --run-id`.
  - Status: in progress
  - Next blocker: live-loop `fmt_check` misses the 20x speed target after same-candidate logs, metrics, traces, and explain reconciliation.
  - Claim impact: no final packet correctness, registry/reviewer exposure, readiness, release, completion, or `update_goal()` eligibility claim.

- [ ] Gate 92 validation commands run: runtime detection, stack up, stack health, stack smoke, query logs, query metrics, query traces, explain current failure, focused observability tests, observability red/green/tamper fixtures, exact coverage, line-cap scan, source audit, red fixture report, package digest, git status, and checkpoint commit.
  - Evidence:
  - Commands:
  - Receipts:
  - Candidate digest:
  - Status: in progress
