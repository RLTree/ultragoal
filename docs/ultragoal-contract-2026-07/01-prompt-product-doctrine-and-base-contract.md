# Parent Session Prompt: Full Harness Ultragoal Compliance

Paste or reference this in the parent session.

```text
Rebind to the active Harness Ultragoal June 25 candidate contract and finish the plugin repo itself into full Harness Ultragoal compliance with zero exceptions.

Source root:
`/Users/terrynoblin/Projects/harness-ultragoal-plugin-proposal`

Use the side-thread audit as required evidence, but verify everything live before editing. Current known violations from side audit:

- Current source audit is failing: `validation_artifacts/ultragoal-audit/validator-receipt.json` showed `status: fail`, `36/43` checks passing.
- Current red fixture report is failing/stale: `validation_artifacts/ultragoal-audit/red-fixture-report.json` showed `208/458` passing and `250/458` failing, mostly `validator_receipt_not_runtime_provenance`.
- `templates/agent-standards/enforcement.json` still contains unmechanized/backlogged laws:
  - `documentation-freshness`
  - `runtime-tool-identity`
  - `product-live-surface-receipts`
  - `transcript-quality-reuse-gates`
- Plugin self-law coverage is not compliant: prior live proof showed no root `.harness/coverage-command`, failed `cargo llvm-cov --workspace --all-features --summary-only --fail-under-lines 100 --offline`, and total line coverage around `6.68%`.
- Session-log hardening proof is stale or mismatched: current audit reported `session_log_hardening_candidate_version_mismatch`.
- Product Fitness receipt is stale: current audit reported `product_fitness_receipt_stale`.
- Fit-repo/source/install/cache proof has drift: current audit reported `fit_repo_receipt_target_digest_mismatch`, wrong plugin version, and wrong cache package.
- Package inventory contains private local proof paths: current audit reported `manifest_owned_private_local_path` for coverage artifacts.
- Live registry/app/reviewer exposure remains unsupported unless freshly proven on the same surface.
- Product Fitness review ownership is not fail-closed: there is no dedicated material-review subagent, and current Product/Simplicity reviewer prompts do not explicitly name Product Fitness / Quality-In-Use ownership. Implied ownership through Product Fitness receipts or `harness-product-simplicity-falsifier` is insufficient. Product Fitness must become a first-class, typed, validator-enforced review obligation, either owned explicitly by the existing review team or enforced by a dedicated Product Fitness falsifier.
- Namespace law is a repeat-offender class and must be proven first-class and fail-closed by live validation, not inferred from row presence. `docs/source-article-synthesis.md` carries the foundational article requirement that the filesystem is an agent-facing interface, and `templates/agent-standards/01-namespace-and-progressive-disclosure.md` defines the Namespace Law. A standards row, foundational trace entry, validator path, or fixture catalog entry is not enough unless non-compliant package/repo paths actually fail completion, review, package, readiness, and release claims.
- Foundational law enforcement-center drift remains a violation: every foundational law must be enforced by the central standards validator, claim gates, red fixtures, receipts, and traceability. A law being enforced only by one package-audit path, one prose row, one schema, or one reviewer persona is insufficient.
- `docs/source-obligation-matrix.json` still permits weak dispositions such as `partial`, `backlog`, `reviewer`, `reviewer_and_backlog`, `future`, or "missing receipt" language for foundational laws. That is not allowed for this candidate. Every foundational law must be either deterministically enforced or explicitly non-goal in a way that blocks every related claim.
- `templates/agent-standards/enforcement.json` can still overstate mechanization when a row only proves ledger shape or gate-path existence. Mechanized must mean the non-compliant behavior fails through a validator, schema, red fixture, and receipt path.
- Runtime/tool identity, product live-surface receipts, transcript-quality reuse gates, clean-checkout command discovery, restartable ExecPlans, memory/wiki context-only boundaries, source-card freshness, namespace/progressive disclosure, Product Fitness, line caps, typed parsing, and exact 100% coverage are all mandatory law surfaces. None may remain advisory, reviewer-only, partial, stale-source-backed, or claim-ceiling-only.
- Parse-don't-validate violations remain when authority is inferred from freeform text or substring heuristics instead of typed fields and parsed receipt objects. Text scans may be fail-closed backstops only; typed fields must carry authority.
- Source-card freshness is law-bearing for this candidate. A foundational source card with `not_refreshed` cannot support current law-completeness or current-source claims.
- Memory/wiki/Chronicle/session summaries are context and audit inputs, never current implementation proof unless paired with live same-surface files, commands, digests, receipts, or runtime artifacts.
- Foundational article traceability is incomplete: there is no complete article requirement -> standards law -> validator check -> red fixture -> receipt -> claim-ceiling ledger.
- The repo/plugin is not exempt from its own laws: parsing/typed boundaries, exact 100% coverage, line caps, Product Fitness, proof surfaces, and standards rows must all be enforced against the plugin itself.
- Coverage is only one mandatory law. The parent session must not fixate on coverage while leaving architecture topology, quality-score/taste gates, standards promotion, autonomy-loop proof, orchestrator state-machine behavior, scheduler/runner boundaries, subagent/custom-agent approval inheritance, skill progressive-disclosure metadata, plugin install-surface metadata, ExecPlan no-handback/prototype gates, or semantic domain-type naming weak or unenforced.
- Early implementation debt is not grandfathered. If full compliance requires restructuring validator modules, schemas, fixtures, package inventory, custom-agent contracts, installed/cache layouts, or the codebase namespace itself, do the restructuring. Existing code shape, historical convenience, or fear of churn is not an exemption from Harness Ultragoal law.
- Additional article-law gaps must be treated as first-class mandatory laws, not nice-to-have hardening: validator failures must be agent-remediating, third-party dependency use must be typed and legible, repo knowledge/core-beliefs indexes must be verified, workflow template parsing/rendering/reload must fail closed, workspace command confinement and lifecycle cleanup must be proven, plugin bundled component graphs must be closed, instruction precedence/nested `AGENTS.md` routing must be enforced, ExecPlans must be plain-language and expected-output complete, guardrails must be fast/isolation-safe/cache-honest, and secrets/tokens must never leak to subagents or dynamic tools.
- Additional missed article-law gaps are also mandatory: generated/proof artifacts must have deterministic provenance and cannot be hand-fabricated, every human/agent review finding must have a typed disposition with proof, coverage must prove behavior rather than hit-count theater, fresh environments must be one-command and concurrency-safe, observability must be queryable by agents with correlation and redaction, subagent orchestration must be explicit and reconciled, skill catalogs must obey context-budget/omission-warning law, workspace-sharing/public/local distribution claims must be separated, and authority surfaces must be total types with impossible states unconstructible.
- Second-pass missed law gaps are mandatory too: plugin flow graph/product journey authority, portable non-prescriptive adapter boundaries, derived authority recomputation, offline schema-catalog resolution, batch fan-out job discipline, raw-private artifact handling, and active setup-to-idle orchestration must all be enforced as fail-closed law surfaces, not buried inside adjacent categories.
- Current plugin-state audit gaps are mandatory too, even where the repo has partial mechanics. `connector-capability-discovery` and `target-repo-audit-capability` appear in schema/validator/cohesion surfaces, but they must also be promoted to central mandatory law, red-fixture, receipt, and claim-ceiling enforcement. Security/trust-boundary coverage also appears in the active coverage ExecPlan, but it must be enforced as a fail-closed standards law with abuse-path and failure-path proof, not only as planning prose.
- Source-obligation registry parity is mandatory. A foundational law being present only in `docs/source-obligation-matrix.json`, `docs/foundational-law-traceability.json`, or `validator/src/audit/source_obligations.rs` is not enough if it is hidden under an adjacent standards row, a generic validator check, a minimum valid fixture, or a broad umbrella gate. Every source-obligation law must either have first-class same-law enforcement or a typed parent/child enforcement relationship that proves the child law fails independently.
- Hybrid human-audit dispositions are not allowed to close mandatory law compliance. Current live repo state still has `deterministic_with_human_audit` rows in `docs/source-obligation-matrix.json`, while the weak-disposition validators only name reviewer/prose/backlog/future-style terms. Any such row must be decomposed into deterministic enforcement plus a typed judgment-only review surface with claim blocking; it cannot count as full compliance.
- Capability-gap promotion is mandatory. The foundational harness article says an agent failure should identify what capability is missing and make that capability legible/enforceable. Current repo state has connector-specific and target-repo-specific capability gates, but no generalized fail-closed law for missing tools, context, runtime legibility, validators, receipts, adapters, commands, permissions, reviewer routes, doc indexes, or underspecified environments discovered during agent work. A missing capability cannot terminate as user handback, retry-hard, prose blocker, or claim-ceiling downgrade without a typed promotion path.
- Goal-contract amendment authority and closed claim-id mapping are mandatory. The Ultragoal contract says canonical authority lives in the contract bundle and `AMENDMENTS.jsonl`, not in markdown projections alone, and every advertised claim must map to a closed required claim id or explicit informational non-claim. Current repo state has amendment schema/fixture/validator fragments, but this parent contract did not yet require side-thread additions, checklist gates, final-packet claims, or newly discovered laws to be reconciled into the canonical contract bundle, amendment log, completion manifest, required-claim-id hash, and claim ceiling.
- Forward-only state transition integrity is mandatory. `templates/agent-standards/02-boundaries-validation-and-enforcement.md` defines a distinct law: approval or closure cannot silently re-enter queued, active, or needs-review states without a typed reopen transition. Current contract coverage through issue/tracker lifecycle and orchestrator state-machine gates is not enough because it does not explicitly require append-only transition history, source/destination states, actor/reason/evidence, transition provenance, and claim-blocking revalidation when approved/closed/verified surfaces regress.
- Validator source namespace topology is a live repeat-offender violation and must be treated as first-class law evidence, not as a cosmetic refactor. Side-thread live inspection found `validator/src` with 155 top-level Rust files, 105 top-level `internal_*.rs` files, and 16 top-level `internal_coverage*.rs` files. `docs/namespace-law-exceptions.json` currently contains a broad `repeated-prefix-validator-src-internal` exception for `directory = "validator/src"`, `prefix = "internal"`, and `applies_to = ["validator/src/internal*"]`, which blesses the exact prefix-as-directory smell the namespace law is supposed to reject. `plugin-manifest-draft.json` also lists those top-level internal test files as package resources. This proves the current namespace and semantic repo laws are not strict enough. The parent must remove this escape hatch, restructure validator source/test topology into routed semantic subdirectories, and enforce recurrence mechanically.

Required context loading before edits:

1. Read:
   - `README.md`
   - `REPORT.md`
   - `.codex-plugin/plugin.json`
   - `plugin-manifest-draft.json`
   - `docs/review-loop-record.md`
   - `docs/review-target-and-archive.md`
   - `docs/codex-custom-agent-registry-preflight.md`
   - `docs/product-fitness-and-quality-in-use.md`
   - `docs/source-obligation-matrix.md`
   - all active ExecPlans under `docs/exec-plans/active/`
   - `templates/agent-standards/enforcement.json`
   - `templates/agent-standards/enforcement.tsv`
   - `templates/agent-standards/enforcement-audit.tsv`
   - `templates/PRODUCT_FITNESS.md`
   - `templates/PRODUCT_FITNESS_RECEIPT.json`
   - all related schemas and validator/report surfaces
2. Use Chronicle/session logs for June 25 `harness-ultragoal` / `0.0.10` evidence, including:
   - Product Fitness work around `2026-06-25T04:49Z`
   - source/install/cache drift around `2026-06-25T06:50Z`
   - registry/review-round proof blocker around `2026-06-25T17:06Z`
   - packet/session-log gap around `2026-06-25T18:47Z` and `19:06Z`
   - raw Codex session logs for June 25 matching `harness-ultragoal`, `0.0.10`, `Product Fitness`, `coverage`, `line-cap`, `typed`, `parse`, `stale`, `overclaim`, `not enforced`, `weak`, `missing`, `blocker`.

Implementation requirements:

Gold-Standard Harness Product Doctrine Integration:

The attached GPT-5.5 Pro synthesis is now a required synthesis input for this
parent contract. It is not primary factual authority for external product
versions, pricing, or current vendor capabilities. Any exact external version
pin, vendor capability claim, or hosted-service claim from that synthesis must be
verified against the applicable official source before becoming a hard package
or install/cache law. Its architectural doctrine, operating-loop structure,
observability model, agent-legibility model, and downstream product implications
are binding guidance for this contract unless a stronger live repo law conflicts.

Core product thesis:

- The Harness Ultragoal product is not "Rust plus TypeScript plus validators."
  The product is an agent-legible repository harness with isolated execution,
  deterministic validation, repair/eval loops, high-cardinality observability,
  mechanically enforced architecture, and production SRE discipline.
- For agentic engineering, the harness is the product. Languages, models,
  plugins, MCP tools, eval tools, receipts, and dashboards serve the harness.
- A compliance pass that does not make agents faster, less confused, more
  repair-capable, and less likely to overclaim is not product completion.
- The final product must let an agent and Tree diagnose, repair, harden, and
  validate work from first principles without independent archaeology through
  stale receipts, scattered markdown, raw logs, or hidden command knowledge.
- The parent must preserve the existing claim ceiling: source-local proof does
  not imply install/cache, app-registry, reviewer exposure, release, readiness,
  completion, or update_goal eligibility.

Mandatory integration objectives from the synthesis:

1. Harness Product Doctrine
   - Rule: Add and enforce the doctrine that Harness Ultragoal is a productized
     agentic engineering harness, not a compliance checklist, validator bundle,
     or receipt generator. Every law-bearing surface should improve agent
     execution, verification, observability, repair, or product reliability.
   - You know it is working when: a new agent can open a plugin-activated repo,
     discover what to run, execute the routine validation path, observe a
     failure, query/explain it, patch the smallest root cause, rerun narrowly,
     compare before/after telemetry, and understand the remaining claim ceiling
     without reading this parent prompt end to end.
   - Downstream impact: Product Usage Fitness, Gate 92, Gate 94, Gate 99, Gate
     100, Gate 105, final packet fields, and update_goal blockers must prove
     product utility and agent legibility, not only file presence, schema shape,
     command existence, or receipt generation.
   - Evidence and confidence: Based on the synthesis thesis that the harness is
     the product and on existing Gate 92/Gate 105 direction. Confidence 96%.

2. Gate 92 As The Agent Sensory System
   - Rule: Gate 92 is the agent sensory system for the entire CLI/plugin. It is
     not stack health, query-only proof, current-failure rescue, or representative
     command fitting. It must operationalize four channels: metrics for alerting,
     traces and wide events for investigation, logs for local detail and forensic
     reconstruction, and evals for behavioral quality.
   - You know it is working when: every law-bearing command/check emits enough
     structured data for `observe query` and `observe explain` to tell the agent
     what failed, where it failed, why it failed, which law/claim is affected,
     what to inspect next, what repair class is likely, and what narrow rerun
     proves the repair.
   - Downstream impact: command inventory, fitting control board, telemetry
     schema, receipt schemas, red/green/tamper fixtures, final packet fields,
     setup/retrofit outputs, active-repo rollout, and update_goal blockers must
     distinguish local spool evidence from live stack ingestion/query proof.
   - Evidence and confidence: Based on the synthesis observability architecture
     and current Gate 92 command inventory. Confidence 93%.

3. Two Observability Planes
   - Rule: Observability must have two explicit planes. Plane A is product/system
     health: latency, traffic, errors, saturation, freshness, retry/backoff,
     cache state, resource pressure, and external/live probe health. Plane B is
     agent quality: task completion, first-pass success, repair iterations,
     validation failure class, human escalation, bad PR or bad packet rate,
     post-merge regression, eval trend, tool misuse, docs drift, architecture
     violations, and claim-theater escapes.
   - You know it is working when: the CLI can answer both "is the system healthy"
     and "is the agent getting better or worse" from current queryable records,
     with same-candidate binding and bounded output.
   - Downstream impact: Gate 105 measured-improvement proof must include both
     planes. Worktree/lane launch should later use the same planes to prove
     parallelism is increasing throughput without increasing bad repairs,
     overclaims, stale evidence, or manual-spelunking burden.
   - Evidence and confidence: Based on the synthesis SRE/agent-quality split and
     current Gate 105 metrics. Confidence 91%.

4. Expanded Telemetry Envelope
   - Rule: The Gate 92 telemetry envelope must include agent, tool, repo, eval,
     and claim attributes in addition to the existing command/check fields. At
     minimum it must model agent system/workflow/step/role/model, tool
     name/type/risk tier/approval/duration/status, repo name/branch/sha/worktree
     or target mode, eval suite/case/score/judge/regression status, and claim
     ids/claim impact.
   - You know it is working when: a failed command can be grouped by agent
     workflow stage, model/tool class, repo state, validator family, eval case,
     failure class, and claim impact without manually joining unrelated files.
   - Downstream impact: observability schemas, logs, spans, wide events, receipt
     bindings, improvement-loop traces, OpenAI/promptfoo/HALO adapters, tool
     contracts, setup/retrofit templates, final packet, and update_goal blockers
     must carry or validate these attributes.
   - Evidence and confidence: Based on the synthesis agent telemetry schema and
     current typed telemetry model. Confidence 94%.

5. Cardinality Doctrine
   - Rule: High-cardinality values belong in traces, wide structured events,
     logs, and eval records. They must not become unbounded metric labels.
     Dangerous metric labels include raw run id, trace id, span id, user id,
     prompt hash, full file path, full candidate digest, PR number, unbounded
     branch name, raw artifact path, or raw private/local path.
   - You know it is working when: agents can still query by run/correlation/digest
     through events/spans/logs, while metrics stay bounded, aggregatable, and
     safe for alerting and dashboards.
   - Downstream impact: telemetry schemas, exporter code, redaction checks,
     bounded-output checks, red fixtures, source audit, Gate 102 privacy, and
     final packet observability sections must fail unbounded metric labels and
     private high-cardinality leakage.
   - Evidence and confidence: Based on the synthesis cardinality section and
     structured-event/high-cardinality research already bound by Gate 92.
     Confidence 95%.

6. Trace To Feedback To Eval To Repair To Promotion Loop
   - Rule: Gate 94 must be a real improvement loop. Failed traces, opaque
     failures, human corrections, bad tool calls, slow workflows, incidents,
     security near misses, wrong repairs, stale-evidence escapes, and claim
     theater must become typed feedback records, eval cases, regression fixtures,
     validator laws, or explicit claim blockers.
   - You know it is working when: a repeated failure class is no longer solved by
     memory or reviewer reminder. It is harvested from trace/eval evidence,
     reproduced by a fixture or eval, repaired, validated narrowly, compared
     before/after through telemetry, promoted into the standards/law surfaces,
     and protected against recurrence.
   - Downstream impact: improvement-loop registry, promptfoo suites, HALO ranked
     change records, Codex handoff templates, standards-gardener cadence, red
     fixtures, eval schemas, final packet, and update_goal blockers must all
     represent this loop.
   - Evidence and confidence: Based on the synthesis repair/eval loops and
     current Gate 94. Confidence 90%.

7. AGENTS.md As Routing Table
   - Rule: Plugin setup/retrofit must treat AGENTS.md and equivalent repo-entry
     docs as routing tables, not encyclopedias. They should point agents to
     canonical docs, exec plans, command surfaces, evals, telemetry, architecture,
     security, privacy, and quality gates without duplicating the whole law
     system.
   - Required shape: routing entries must be actionable operator routes, not
     essays. A fresh agent should see "If doing X, run Y, inspect Z, stop on W"
     for setup, routine validation, Gate 92 diagnosis, current-state inspection,
     eval promotion, security/privacy checks, architecture checks, source-local
     proof, install/cache/live proof, and update_goal eligibility. The file
     should route to canonical CLI authority, `observe` query/explain surfaces,
     `ultragoal current-state --json`, `ultragoal next`, deeper docs, and active
     ExecPlans; it must not duplicate the full law corpus or turn stale prose
     into authority.
   - You know it is working when: an un-oriented agent can find the routine
     command, proof surfaces, claim ceiling, observability query/explain path,
     and active work contract from concise routing docs without loading every
     standard into the hot context.
   - Downstream impact: Gate 99 setup/retrofit, Product Usage Fitness, repo
     knowledge index/core-beliefs, instruction precedence/nested AGENTS routing,
     active-repo rollout, and final packet discoverability fields must enforce
     this routing shape.
   - Evidence and confidence: Based on the synthesis repo architecture guidance
     and current progressive-disclosure laws. Confidence 87%.

8. Tool Contract And Risk Tier Law
   - Rule: Every tool, MCP surface, plugin adapter, dynamic helper, model-call
     adapter, live probe, setup/retrofit helper, and external writer must declare
     owner, risk tier, idempotency, auth scope, input schema, output schema,
     approval requirement, telemetry attributes, preconditions, postconditions,
     failure semantics, and claim impact.
   - Required risk tiers: read-only, local write, local execution,
     external-live bounded, external write, destructive or mutating, and
     sensitive/secret-bearing.
   - You know it is working when: the CLI can explain why a tool was allowed,
     blocked, serialized, approval-gated, redacted, retried, or excluded from
     claim support.
   - Downstream impact: MCP/plugin surfaces, OpenAI/promptfoo/HALO adapters,
     setup/retrofit, worktree lanes, external/live proof, claim guards,
     telemetry, and final packets must stop treating tools as generic commands.
   - Evidence and confidence: Based on the synthesis MCP/tool boundary model and
     existing subagent/custom-agent approval laws. Confidence 89%.

9. Agent Cockpit Future Product Surface
   - Rule: The final product direction should include an Agent Cockpit or
     equivalent operational product surface, but it must not block the immediate
     Gate 92 source-local closure unless explicitly scoped. The cockpit is a
     future product multiplier, not a substitute for CLI proof. The cockpit must
     render CLI-governed state, receipts, telemetry, evals, and claim ceilings;
     it may not invent status, hide blockers, or convert observations into claims.
   - You know it is working when: a user can see active runs, repair-loop state,
     plan, changed files, validation status, eval deltas, trace waterfall, tool
     calls, approvals, screenshots/videos where applicable, logs by run id, PR
     status, docs touched, architecture lints, cost, latency, and token summaries.
   - Downstream impact: Gate 99/100/105, active-repo rollout, product usage,
     future UI/plugin surfaces, and final packet should reserve a place for
     cockpit capability status or an explicit fail-closed "not implemented"
     product-surface blocker.
   - Evidence and confidence: Based on the synthesis agent cockpit section.
     Confidence 82% as future product requirement and 60% as immediate blocker.

10. Concrete Routine Command Surface
   - Rule: Product Usage Fitness must define a canonical routine command surface
     for ordinary validation. Compatibility scripts, `just` recipes, shell
     wrappers, or narrow helpers may exist only if they delegate to the canonical
     CLI authority kernel and preserve telemetry, receipts, and claim ceilings.
   - Required discoverable capabilities: fast validation, full source-local
     validation, observability query, observability explain, repair-loop
     execution, eval execution, architecture check, security check, setup, and
     retrofit.
   - Required legibility stack: this feature is a layered product loop, not a
     collection of disconnected tools. Failure stdout is the atomic command
     diagnostic. `observe explain --next` is the Gate 92 tactical repair planner
     for the fitting board's first incomplete row. `ultragoal current-state
     --json` is the typed read model and optional bounded
     `validation_artifacts/current-state.json` snapshot generated from real
     receipts. `ultragoal next` is the top-level harness navigator that consumes
     current-state and Gate 92 explain output. AGENTS.md routes agents to those
     surfaces without becoming an encyclopedia.
   - Required Agent State / Next Action Contract: Product Usage Fitness must add
     a canonical CLI-governed next-action surface, `ultragoal next` and
     `ultragoal next --json`, after full Gate 92 closure and before Phase 4
     evidence rebinding. Optional aliases such as `ultragoal stack status` or
     `ultragoal stack next` may exist only when they delegate to this authority
     kernel and preserve telemetry, receipts, and claim ceilings.
   - Rule: `ultragoal next` is not another status dump, dashboard, checklist
     projection, or source-audit wrapper. It is the operator-facing synthesis of
     current digest, dirty/stale state, active spine phase, strict claim ceiling,
     first legal blocker, why that blocker comes first, dependency chain, stale
     or wrong-digest evidence, exact next repair, exact narrow rerun, observe
     logs/metrics/traces query commands when a run exists, observe explain
     command when relevant, current-state source bindings, broad rerun only when
     allowed, forbidden actions, Gate 92 fitting-board summary, Product Usage
     status, worktree eligibility, and cockpit state. `--json` is the future
     Agent Cockpit feed; stdout is the human/agent hot path and must remain
     bounded, redacted, and decisive.
   - Rule: `ultragoal current-state --json` and
     `validation_artifacts/current-state.json` are read models, not claim proof.
     They must list the typed source receipts and authority surfaces they read,
     bind to the current candidate digest, report dirty/stale state, summarize
     Gate 92 board status, coverage status, first blockers, and claim ceiling,
     and fail closed when a source receipt is stale, wrong-digest, missing, or
     checklist-derived. They may feed `ultragoal next` and the future cockpit,
     but may not replace source audit, coverage, red report, production
     telemetry, or same-surface proof.
   - You know it is working when: from a clean checkout or plugin-activated target
     repo, the expected path is obvious, command help is self-contained, advanced
     commands remain available, and helper scripts cannot pretend to be final
     proof. Done feels like the harness can answer, "where am I, what is
     blocked, why is it blocked, what exact command proves the next repair, and
     what claims are still forbidden" without making an agent or Tree spelunk
     parent prompts, stale checklist rows, command-inventory walls, raw receipts,
     source-audit dumps, or hidden command knowledge. If done correctly, a fresh
     agent should be able to enter a clean checkout, read the compact route, run
     one next-action command, follow one exact narrow repair command, and know
     which claims remain forbidden in under a minute of orientation.
   - Validation versus proof: parser tests, help tests, JSON schema checks,
     redaction/bounds tests, and red fixtures prove feature mechanics only. They
     do not prove product usefulness. Production proof requires running the real
     `ultragoal next` command on the current candidate, verifying stdout and JSON
     identify the real current blocker and correct narrow next repair, querying
     logs/metrics/traces by the same run/correlation/current digest, reconciling
     those records with the command receipt, running `observe explain --next` or
     the target-specific observe explain command when blocked, reconciling
     current-state with its source receipts, and inspecting source to confirm the
     command reads typed authority surfaces rather than markdown, checklist
     prose, or last-command heuristics.
   - Required fail-closed cases: missing `ultragoal next`, generic "inspect
     receipts" output, no exact next command, missing claim ceiling, stale digest
     accepted as current, current-state snapshot treated as proof, current-state
     generated from stale or checklist-derived inputs, checklist prose accepted as
     authority, `observe explain --next` producing generic repair text, broad
     audit recommended before narrow observable repair, worktrees marked eligible
     while Gate 92/Product Usage/Phase 4 are incomplete, readiness/update_goal
     implied from source-local proof, cockpit/UI proof substituted for CLI proof,
     private path leakage, and unbounded JSON output.
   - Downstream impact: Phase 3.5 Product Usage Fitness, clean-checkout command
     discovery, setup/retrofit, active-repo rollout, Agent Cockpit, Gate 105,
     final packet, and update_goal blockers must enforce command discoverability,
     delegation, next-action usefulness, and proof-surface separation.
   - Evidence and confidence: Based on the synthesis command-surface section and
     current Product Usage Fitness slice. Confidence 88%.

11. Entropy Cleanup Loop
   - Rule: After core source-local closure, the harness must include an entropy
     cleanup loop for docs drift, dead code, unused dependencies, missing tests,
     duplicate abstractions, telemetry drift, architecture violations, stale
     evals, flaky or slow tests, large files, uncited assumptions, and security
     drift.
   - You know it is working when: repeated drift is not rediscovered by human
     frustration. It is scheduled, detected, promoted into standards or fixtures,
     and claim-blocked when material.
   - Downstream impact: standards-gardener, Gate 105, targeted refactor/debt
     removal, active-repo rollout, final packet, and long-term maintenance must
     treat entropy as a governed product risk.
   - Evidence and confidence: Based on the synthesis entropy cleanup loop and
     current standards-gardener/debt-removal law. Confidence 84%.

12. Validation Gate Levels
   - Rule: The product must separate local-fast, full source-local, merge/review,
     and live distribution proof levels. Focused checks can support repair.
     Broad source-local proof can support source-local claims. Install/cache,
     app-registry, reviewer, release, completion, and update_goal claims require
     same-surface proof.
   - You know it is working when: command output, receipts, final packets, and
     checklist statuses make it impossible to confuse a focused test, a broad
     source audit, a clean-room proof, an installed-cache proof, and a live
     external proof.
   - Downstream impact: command help, validation modes, claim ceilings, final
     packet, checklist statuses, worktree lane entry, and update_goal eligibility
     must all preserve these proof levels.
   - Evidence and confidence: Based on the synthesis validation gate model and
     current source/install/cache/app-surface separation laws. Confidence 86%.

13. Pedagogical Validator And Linter Output
   - Rule: Validator, schema, fixture, architecture, package, source/install/cache,
     and claim-ceiling failures must teach the agent how to repair. Opaque output
     such as "Architecture violation" is non-compliant for law-bearing failures.
   - Minimum useful failure output: law id, check id, failed invariant, observed
     value, expected value, where failed, why failed, repair class, likely file
     or surface, narrow rerun command, affected claims, severity, redaction
     status, and bounded output status.
   - You know it is working when: `observe explain` and normal failed command
     stdout make the next repair step clear enough that manual source spelunking
     is verification, not discovery.
   - Downstream impact: Gate 89 remediation quality, Gate 92 explain, architecture
     topology checks, red fixtures, final packet blockers, Product Usage Fitness,
     and worktree lane autonomy must enforce agent-actionable output.
   - Evidence and confidence: Based on the synthesis architecture/linter guidance
     and current agent-remediating failure laws. Confidence 93%.

14. Stack Policy As Portable Adapter Families
   - Rule: Adopt the synthesis stack direction as portable adapter families, not
     blind prescriptive pins. Rust is the core CLI/validator authority path.
     TypeScript is the UI/tooling/plugin surface path where applicable. Python is
     appropriate for AI/research/eval sidecars behind explicit interfaces.
     PostgreSQL-style stores hold relational product truth, ClickHouse-style or
     Honeycomb-style stores hold high-cardinality observability, OpenTelemetry is
     the common telemetry protocol, and Nix/Bazel/OpenTofu/Kubernetes-class tools
     are higher-maturity adapters where justified.
   - You know it is working when: setup/retrofit can detect repo type, install or
     fail-close appropriate adapters, and explain why an adapter is unsupported,
     out of scope, or claim-blocking.
   - Downstream impact: Gate 99, Gate 100, Gate 101, active-repo rollout,
     product usage docs, setup templates, adapter schemas, and final packet
     product-readiness sections must treat stack choices as explicit adapter
     contracts.
   - Evidence and confidence: Based on the synthesis stack sections. Confidence
     80% for direction and 45-60% for exact external version pins until verified.

15. Supply-Chain And Security Baseline
   - Rule: The product must include frozen lockfiles where applicable, dependency
     review, minimum package-age policy where supported, provenance/SBOM where
     supported, signed artifacts where supported, audit equivalents for Rust and
     JavaScript/TypeScript package managers, image digest pinning if containers
     enter scope, and no agent-installed production dependency without
     validation.
   - You know it is working when: install/cache/readiness proof can show how
     dependencies were selected, pinned, audited, bounded, and prevented from
     becoming unreviewed agent convenience.
   - Downstream impact: Gate 95 external-AI use, Gate 99 setup/retrofit, Gate 101
     developer experience, Gate 102 privacy/data, install/cache proof, final
     packet, and release/update_goal blockers must include supply-chain status.
   - Evidence and confidence: Based on the synthesis supply-chain section and
     existing security/dependency laws. Confidence 85%.

16. Product Truth, Observability Truth, And Artifact Truth Separation
   - Rule: Product truth, observability truth, and artifact truth are distinct.
     Product truth lives in source/package/runtime behavior. Observability truth
     lives in logs, metrics, traces, events, evals, and queryable records.
     Artifact truth lives in receipts, manifests, reports, archives, and final
     packets. They may cross-reference each other, but none may substitute for
     another without an explicit law.
   - You know it is working when: a local JSONL spool cannot complete Gate 92, a
     receipt cannot replace runtime behavior, source proof cannot replace
     install/cache proof, and a final packet cannot create authority that did not
     exist before packet generation.
   - Downstream impact: Gate 92, source/install/cache separation, final packet,
     review target, archive, package inventory, app-registry proof, and
     update_goal blockers must enforce non-substitution.
   - Evidence and confidence: Based on the synthesis data/observability
     architecture and current claim-ceiling doctrine. Confidence 92%.

17. GPT-5.5 Pro Synthesis Source Card
   - Rule: Create or update a Gate 93 research-source registry entry for the
     synthesis as `agentic-gold-standard-stack-synthesis-2026-07-01` or an
     equivalent stable id. It is a synthesis source. It may introduce doctrine,
     architecture requirements, and operating-loop requirements, but its
     external factual claims require primary-source verification before they
     become hard law.
   - You know it is working when: every adopted synthesis requirement maps through
     research-source registry, article-to-law trace, canonical law ids, standards,
     source obligations, foundational trace, schemas, validator ids, fixtures,
     package inventory, setup/retrofit outputs, claim guards, final-packet
     fields, and update_goal blockers.
   - Downstream impact: Gate 93 and Gate 104 must treat these additions as
     source-mapped law, not chat-only steer or prompt-only claims.
   - Evidence and confidence: Based on the synthesis source list and current Gate
     93 research mapping law. Confidence 91%.

18. Final Packet Product Shape
   - Rule: Final packet and final response surfaces must report product readiness
     by proof surface, not compliance vibes. Add explicit sections for Harness
     Product Doctrine, Observability Planes, Agent Quality Metrics, Trace to Eval
     to Repair Loop, Tool Risk and Approval Surface, Data/Privacy Boundary,
     Product Usage/CLI Surface, Active Repo Rollout, Stack Adapter Status,
     Supply-Chain/Security Baseline, Agent Cockpit status or blocker, and
     remaining unsupported live surfaces.
   - You know it is working when: a reviewer can tell exactly which surfaces are
     source-local, install/cache, app-registry, live reviewer, final-packet,
     external/live, or update_goal-supported, and exactly which product claims
     remain blocked.
   - Downstream impact: final packet, review target, candidate archive, claim
     ceiling, update_goal eligibility, Product Fitness, Product Success, and
     completion response fields must include these product-shape sections.
   - Evidence and confidence: Based on the synthesis final formulation and
     current final-packet/claim-ceiling laws. Confidence 89%.

19. Parent-Owned Fast Loop, Verified Incremental Audit, And Fitting Compiler
   - Rule: The next acceleration path is parent-owned, not workflow-engine-owned,
     until the workflow engine can prove it produces useful, typed, current
     candidate execution plans for this repo. The parent may use the engine for
     bounded design assistance only when its output is inspected, supplemented,
     and executed through canonical `ultragoal` authority. Worker or workflow
     output is evidence, never proof, and cannot substitute for current command
     runs, source inspection, logs/metrics/traces, receipts, or claim guards.
   - Rule: The routine live validation product shape is one command over a
     verified incremental engine:
     `ultragoal loop run --tier hot --cache-mode verified-local --jobs auto`.
     That command computes the current candidate/package boundary, detects
     changed files, resolves affected law/check/schema/fixture/receipt/query
     nodes, runs only legally sufficient impacted checks and fixtures, queries
     emitted telemetry, explains the first blocker, emits the current-state read
     model, and prints the exact next repair, exact narrow rerun, claim ceiling,
     and whether broad proof is required.
   - Rule: The command is one UX surface, but internally it must be a dependency
     graph, not a giant serial shell script. Required implementation components
     are a per-run shared `AuditContext`, verified content-addressed local cache,
     query-DAG check engine, decomposed package/text checks, grouped red-fixture
     execution with shared semantic indexes, minimal or copy-on-write isolated
     fixture roots, deterministic result ordering, and optional persistent
     validator daemon/worker state guarded by current input digests.
   - Rule: Verified cache is default for live repair loops. Cache concealment is
     illegal. Warm-cache speed is not clean-checkout proof. Cache presence is not
     correctness proof. Every cache hit must record input digests, validator
     digest, law/schema/fixture versions, cache key, cache mode, hit/miss state,
     invalidation reason, worker/task/queue state, timing class, and claim
     impact. Cache reuse across an older package candidate is verified
     current-input equivalence for routine acceleration only, not same-candidate
     production proof. `--cache-mode none` remains mandatory for strict proof
     boundaries.
   - Required speed-recovery order: first fix speed-law arithmetic and
     digest-bound baseline provenance so same-command baselines and integer
     truncation cannot fake failure or success; repair Rust/Cargo cache receipt
     honesty so effective cache state is observed instead of hardcoded; split
     coverage into strict full-clean boundary proof and routine warm/retained
     repair proof; introduce one product-surface input spec that drives both
     affected-set detection and cache keys; route loop work through typed task
     classes and in-process validator nodes sharing `AuditContext`; compute
     package digest and shared source indexes once per immutable snapshot;
     de-duplicate source-audit and red-fixture work with safe read-only indexes;
     bound observability I/O; and consider crate/workspace splitting only after
     measured residual test/build cost proves it is still needed.
   - Rule: The parent must also build a Gate 92 fitting compiler and runner
     before continuing command-by-command fitting churn. The canonical
     `CommandObservabilitySpec` and `SurfaceObservabilitySpec` registry must
     drive command inventory rows, stdout contracts, receipt expectations,
     logs/metrics/traces query proof paths, fixture packs, claim ceilings,
     generated fitting tests, fitting control-board status, `observe explain
     --next`, `current-state`, and `ultragoal next`.
   - Required fitting runner shape: `ultragoal observe fit --command <id>` and
     `ultragoal observe fit --family rust|observe-query|gc|external-live|
     claim-guard|package|coverage|product|install-cache|final-control` run the
     real command or surface, capture stdout and receipt, query logs/metrics/
     traces, run the applicable explain command, reconcile same-candidate proof,
     emit a fitting receipt/proposal, and refuse fitted status when production
     proof is missing. Fit by family where semantics are shared. Do not hand-edit
     generated inventory rows as the normal path.
   - Required latency targets for live source-local iteration:
     hot edit-check loop <= 5s p95; focused repair loop <= 15s p95; standard
     affected source-local loop <= 30s p95; strict source-local proof target <=
     60s and hard ceiling <= 180s. The routine live loop should be about 20x
     faster than the current full-world audit baseline for ordinary dirty-tree
     repair work; the current known baseline is about 181 seconds, making the
     current routine target about 9.1 seconds when affected-set and
     verified-cache assumptions hold. Recompute that target from current
     receipts when the baseline changes.
   - Rule: The 20x target applies to the routine verified-local product loop,
     not to every strict no-cache boundary proof. The routine live loop must
     include legally sufficient high-frequency validation work agents repeatedly
     need during repair: line caps, namespace, schema validation, package
     inventory scans, focused Rust tests, fmt/build checks, source-obligation and
     foundational-trace affected checks, affected source-audit families, affected
     red/green/tamper fixtures, receipt dereferences, `scripts/check`
     delegation, and routine coverage when the edit class requires coverage
     feedback. Each included node must either execute current-candidate work or
     record verified current-input cache equivalence with explicit
     routine-only claim limits. A node that cannot legally run fast must emit a
     typed blocker or strict-boundary-only reason; it may not hide outside the
     loop as a slow side channel.
   - Rule: Strict full-clean coverage, full source audit, full red fixture
     report, and final source-local proof remain claim-boundary surfaces. They
     may be slower than the routine loop, within their strict budgets, and must
     not be run after every small edit as the default iteration path. They also
     cannot be used to excuse a missing fast routine equivalent for ordinary
     repair. Coverage must therefore have two honest modes: strict full-clean
     exact coverage for claim boundaries, and routine coverage feedback only when
     it can prove verified current-input equivalence or lower the claim ceiling.
   - Required speed evidence: receipts and telemetry must report duration_ms,
     worker_count, task_count, queue_depth, critical path, affected node count,
     skipped node count, cache mode, cache hit rate, invalidation reasons,
     cold/warm timing class, candidate digest, claim impact, and whether the run
     can support only repair-loop progress or a stricter proof boundary.
   - You know it is working when: `ultragoal loop run --tier hot --cache-mode
     verified-local --jobs auto` gives a fresh agent one bounded command that
     runs the real impacted validation, emits useful stdout with run/correlation/
     trace ids, records same-candidate observability, explains the first blocker,
     produces current-state, and lands under the live-loop latency target without
     hiding cache, skipping required affected work, or implying readiness.
   - Validation versus proof: unit tests, table-driven fitting tests, cache-key
     tests, parser tests, schema tests, redaction/bounds tests, and fixture
     mechanics prove implementation mechanics only. Production proof requires a
     real current-candidate `ultragoal loop run`, same-candidate stdout/receipt/
     logs/metrics/traces/explain/current-state reconciliation, source inspection
     of the query-DAG and cache invalidation path, red/green/tamper fixture
     coverage for stale/wrong-digest/cache-substitution failures, and an explicit
     claim ceiling. Performance proof requires measured live runs, not dry-run
     estimates.
   - Required fail-closed cases: hidden worker cap, hidden global serial lock,
     unbounded concurrency, nondeterministic result ordering, cache hit with
     stale or wrong digest, cache hit without invalidation reason, warm-cache
     timing used as clean proof, dry-run timing used as live timing, affected
     fixture omitted, generated inventory hand-edited around the spec registry,
     workflow-engine output accepted as proof, `current-state` treated as proof,
     broad audit recommended before narrow observable repair, and fitted status
     from tests without production round trip.
   - Evidence and confidence: Based on the current audit timing shape of about
     181 seconds scheduled work, where broad package checks and red fixtures
     dominate, plus primary implementation patterns from incremental query
     engines, build daemons, persistent workers, action caches, dependency
     graphs, and file watchers. Confidence 97% that full-world recomputation and
     hand-authored fitting are the root causes; 88% that verified incremental
     live loops can deliver 20x routine-loop speedup; 84% that the fitting
     compiler/runner can deliver 20x repeated Gate 92 family-fitting speedup;
     38% that strict no-cache final proof can deliver 20x without deeper
     architectural evidence, so strict final proof remains a separate
     claim-boundary budget rather than the routine-loop speed target.

20. Builder-Contract Modularization Without Semantic Loss
   - Rule: The parent prompt, checklist, and spine are builder contracts. They
     are overloaded enough that a governed split is now warranted, but the split
     is a source-local documentation-architecture migration, not implementation
     proof, package evidence, source audit proof, Gate 92 proof, Product Usage
     proof, final-packet proof, readiness proof, or update_goal evidence. Editing
     these files or their successor modules must not stale package digest unless
     a package-boundary bug incorrectly includes parent-session builder
     contracts in package evidence.
   - Required product shape: create one new root roadmap/connective-tissue
     contract that every parent agent loads first. That root file defines
     precedence, package-boundary doctrine, first action on resume, phase order,
     claim ceilings, checklist-status discipline, receipt discipline, how to
     load each module by task, how modules cross-reference one another, and what
     old files are retained as compatibility shims. The root must be short
     enough to be used, but exact enough that agents cannot miss stop conditions.
   - Required module families: split stable law/operating content into logical
     `.md` files, for example `00-roadmap-and-loading-order.md`,
     `01-operating-contract-and-claim-ceilings.md`,
     `02-phase-spine-and-dependency-order.md`, `03-gates-000-091.md`,
     `04-gate-092-observability-and-fast-loop.md`,
     `05-gate-093-research-source-authority.md`,
     `06-product-usage-and-agent-legibility.md`,
     `07-performance-cache-and-incremental-engine.md`,
     `08-final-proof-surfaces-and-update-goal.md`, and
     `checklist/phase-*.md` or `checklist/gate-*.md`. The exact names may change
     if the root map is clearer, but the split must be by durable authority
     boundary, not by arbitrary line count.
   - Required migration behavior: preserve the original three files until the
     root roadmap and successor modules pass semantic parity. The old prompt,
     checklist, and spine may become generated or hand-maintained shims only
     after they point to the new root contract and no longer contain unique
     hidden authority. Do not delete, rename, or hollow them out in a dirty
     source slice or while current evidence is stale.
   - Required semantic parity proof: before claiming the split complete, build a
     migration index mapping every old heading, gate, stop condition, phase,
     validation command, final response field, claim ceiling, receipt rule,
     checklist-status rule, package-boundary rule, Gate 92 requirement, Gate 93
     source requirement, Product Usage requirement, HU-STACK requirement,
     performance/cache rule, and update_goal blocker to exactly one successor
     module or an explicit retired/duplicate disposition. Missing, duplicate,
     ambiguous, or orphaned authority fails the migration.
   - Required usability proof: a fresh agent must be able to load the root,
     answer which files govern the current task, identify the first legal next
     action, identify forbidden actions, and find proof versus validation
     requirements without reading every module. The split fails if it creates a
     library of docs that humans and agents avoid because the entrypoint is not
     obvious.
   - Required validation versus proof: validation is markdown/link/schema/heading
     checks, `rg` parity, old-to-new migration-index coverage, no duplicate
     active authority, and checklist status rows. Proof is a real dry parent
     orientation run on current docs: start from the root, classify the active
     phase, find the Gate 92 fast-loop/fitting-compiler requirements, find the
     Product Usage/Agent State requirements, find the Phase 4/worktree blockers,
     and verify that the root routes to exact module paths without stale hidden
     authority in the old files. This proof is builder-contract proof only.
   - Required fail-closed cases: old file retains unique authority not in the
     root/module map; root omits first-action/package-boundary/claim-ceiling
     doctrine; checklist becomes a receipt ledger; module cross-links are
     broken; one law exists in two active places with conflicting wording;
     builder-contract edit is treated as package evidence; package digest stales
     from parent-session doc edits; agents must read every module to know the
     next legal action; final packet/update_goal/worktree eligibility is inferred
     from the split.
   - Evidence and confidence: Based on the current size and mixed authority of
     the three binding docs, repeated agent misses, and the product doctrine that
     harness contracts must reduce cognitive load. Confidence 92% that a
     governed modular split improves agent legibility; 55% if done casually
     without migration parity; 95% that old files should remain compatibility
     shims until parity proof exists.

Gold-Standard Stack Developer Experience Addendum:

The zip archive
`/Users/terrynoblin/Downloads/harness_ultragoal_gold_standard_stack_markdown_and_laws.zip`
adds concrete developer-experience, stack, command-loop, receipt, cache,
resource, garbage-collection, supply-chain, product, CI/local-parity, and
HU-STACK law guidance. Treat it as an additional Gate 93 synthesis source,
alongside the earlier GPT-5.5 Pro thesis. Use a stable source id such as
`gold-standard-stack-developer-experience-governance-2026-07-01`. The archive
contains:

- `harness_ultragoal_gold_standard_stack_developer_experience.md`;
- `AGENTS.md`;
- `harness_ultragoal_codex_gold_standard_stack_laws_AGENTS.md`.

The archive's exact external version claims are snapshot candidates, not
unchecked package law. Rust, TypeScript, Python, Bun, Node, SvelteKit,
SolidStart, Vite, Tailwind, PostgreSQL, NATS, Restate, Nix, Bazel, OpenTofu,
Kubernetes, OpenTelemetry, and other external version pins must be verified
against official sources before they support package, install/cache, release,
readiness, or update_goal claims. Their roles, surface separations, command
loops, receipt requirements, and law shapes are binding guidance now.

Stack classification vocabulary:

- `REQUIRED`: part of the gold-standard stack when the repo/app uses the
  applicable surface. It must route through the CLI, produce receipts, and have
  claim guards.
- `DEFAULT_ON`: enabled by default when safe and available. Absence must be
  explicit and claim-limited.
- `GOVERNED_ADAPTER`: useful but not canonical authority. It requires declared
  config, tool identity, schemas, receipts, and claim-surface limits.
- `OPTIONAL_LOCAL`: allowed for local productivity only. It cannot support
  claims.
- `REJECT`: forbidden for claim-bearing work because it creates hidden state,
  weak proof, wrong abstraction, unnecessary risk, or product substitution.

Stack surface decisions to preserve:

- Rust is the correctness-critical core/control-plane language for the CLI,
  validators, parsers, receipt binding, claim ceiling, workers, services, and
  performance-sensitive law paths. Raw Cargo output is observation only.
- TypeScript is the UI/frontend and agent-cockpit language. Runtime parser proof,
  browser proof, accessibility proof, visual proof where applicable, and bundle
  inventory are mandatory because TypeScript types erase at runtime.
- Python is for AI, evals, notebooks, research, trace analysis, and model
  experiments. Python cannot become the correctness-critical law engine or claim
  authority without an explicit governed adapter.
- Bun/Node are TypeScript package/script/test and compatibility substrates.
  Frozen lockfiles, cache honesty, package-manager identity, and security
  scanning are required where those surfaces are used.
- SvelteKit is the default product-UI framework for SSR/product dashboards and
  docs-heavy surfaces when the repo has product UI. SolidStart/SolidJS is
  default-on for agent cockpit, trace explorers, graph editors, and
  high-interactivity surfaces. Vite is the frontend build substrate. Tailwind v4
  is a governed design-system substrate only when its browser baseline is
  accepted and tested.
- PostgreSQL is durable relational truth. ClickHouse is high-cardinality
  observation/analytics truth. DuckDB is local analytical investigation. Object
  storage is artifact storage. NATS JetStream is movement/event fanout. Restate
  is durable execution. Temporal is an adapter. Valkey/Dragonfly are cache/
  ephemeral state. Cache is never durable truth.
- OpenTelemetry APIs, collector, and semantic conventions are the cross-language
  telemetry vocabulary. High-cardinality agent/run/tool/eval fields belong in
  traces/events/logs/eval records, not unbounded alert metric labels.
- Codex, OpenAI Agents SDK, and MCP tools are execution/orchestration surfaces.
  Their output is observation until `ultragoal` parses, validates, and binds it.
- Nix is the clean-checkout reproducible devshell substrate. Bazel is a governed
  monorepo build-graph adapter only when scale justifies it. OpenTofu is
  infrastructure-as-code baseline where infrastructure enters scope. Kubernetes
  is a runtime adapter only when orchestration complexity is justified. Docker/
  Compose is default-on for local ephemeral services and per-worktree harnesses.

HU-STACK laws from the archive:

1. `HU-STACK-001: Agent-First Harness Law`
   - Rule: The repository, runtime harness, validation system, observability
     stack, eval suite, and artifact cleanup system are product surfaces.
     Language tools serve the harness; they do not define compliance.
   - You know it is working when: stack-specific proof improves agent execution,
     validation, repair, runtime inspection, or cleanup rather than merely
     recording that a tool ran.
   - Downstream impact: Gate 92, Gate 99, Gate 101, Product Usage Fitness,
     Product Fitness, Product Cohesion, Product Success, and final packet must
     report stack surfaces as governed product surfaces.

2. `HU-STACK-002: Cross-Language Claim Authority Law`
   - Rule: Rust, TypeScript, Python, SQL, infrastructure, telemetry, and agent
     tools may emit observations. Only `ultragoal` may convert observations into
     claims.
   - You know it is working when: raw `cargo`, `bun`, `uv`, `pytest`, `sqlx`,
     `tofu`, `kubectl`, `playwright`, promptfoo, OpenAI, HALO, or MCP output can
     never raise a claim ceiling without CLI parsing, same-surface binding,
     freshness checks, and receipt validation.
   - Downstream impact: claim guards, receipt schemas, final packet, update_goal
     eligibility, and all setup/retrofit templates must reject raw-tool proof.

3. `HU-STACK-003: Clean-Checkout Discoverability Law`
   - Rule: A fresh agent must discover setup, validation, local runtime, evals,
     and release proof from repo files only. Hidden local state cannot support
     claims.
   - You know it is working when: a clean checkout or isolated worktree can reach
     the documented fast/standard/clean/release command surface without author
     memory, editor state, private aliases, or untracked scripts.
   - Downstream impact: Product Usage Fitness, setup/retrofit, Nix/devshell,
     command registry, AGENTS.md routing, CI/local parity, and worktree-lane
     entry rules must enforce this.

4. `HU-STACK-004: Same-Surface Full-Stack Proof Law`
   - Rule: A claim about a surface must be proven on that same surface. Source
     claims need source proof; package claims need package inventory; install
     claims need install tree proof; runtime API claims need runtime API proof;
     browser claims need browser journeys; database claims need migration/query
     proof; workflow claims need scenario/replay proof; observability claims need
     emitted/queryable telemetry; product claims need user-outcome proof.
   - You know it is working when: package install cannot satisfy runtime success,
     Kubernetes rollout cannot satisfy product success, trace existence cannot
     satisfy eval success, and reviewer agreement cannot satisfy compliance.
   - Downstream impact: final packet, Product Fitness, Product Cohesion, Product
     Success, install/cache, app-registry, reviewer, active-repo rollout, and
     update_goal blockers must stay surface-specific.

5. `HU-STACK-005: Lockfile Sovereignty Law`
   - Rule: `Cargo.lock`, `bun.lock`, `uv.lock`, `flake.lock`, OpenTofu provider
     locks, and equivalent package/toolchain locks are governed truth surfaces.
     Lock drift blocks dependency, build, package, release, and update_goal
     claims until verified.
   - You know it is working when: dependency/toolchain receipts name lockfile
     digests, stale locks fail, frozen installs are enforced, and warm local
     caches cannot masquerade as clean proof.
   - Downstream impact: Gate 101, supply-chain/security, clean-checkout proof,
     install/cache, final packet, and update_goal eligibility.

6. `HU-STACK-006: Telemetry Schema Law`
   - Rule: No trace, metric, log, event, or eval record may introduce
     unregistered attributes. Every attribute has type, owner, cardinality class,
     privacy class, and allowed surfaces.
   - You know it is working when: telemetry schema drift fails before claims,
     high-cardinality values stay out of alert metric labels, and query/explain
     output can rely on stable attribute names.
   - Downstream impact: Gate 92, Gate 102, command inventory, fitting control
     board, source audit, red fixtures, final packet, and Agent Cockpit.

7. `HU-STACK-007: Agent Tool Boundary Law`
   - Rule: Every tool/MCP action requires typed schemas, risk class, approval
     policy, idempotency class, telemetry span, and cleanup policy. Tool use
     without policy cannot support claims.
   - You know it is working when: the CLI can explain the allowed action, risk,
     approval, inputs, outputs, idempotency, cleanup, trace, and claim impact for
     every agent/tool operation.
   - Downstream impact: OpenAI/Agents SDK/MCP integration, setup/retrofit,
     external/live surfaces, worktree lanes, privacy, security, final packet, and
     update_goal blockers.

8. `HU-STACK-008: Product-Cockpit Law`
   - Rule: Agent work must be inspectable through an internal cockpit or
     equivalent runtime surface showing plans, diffs, validations, traces, evals,
     tool calls, approval records, artifacts, and claim ceilings.
   - You know it is working when: Tree or a future agent can inspect the state of
     a run without spelunking raw files, and the cockpit remains observation
     until CLI receipts bind claims.
   - Downstream impact: future Agent Cockpit, Product Usage Fitness, Gate 100,
     Gate 105, active-repo rollout, final packet, and live product UX.

9. `HU-STACK-009: Full-Stack GC Law`
   - Rule: Every generated artifact, cache, package, install copy, database data
     directory, trace, screenshot, video, receipt, packet, lock, pid, port, and
     tempdir must be classed before cleanup. Deletion requires dry-run plan and
     deletion receipt.
   - You know it is working when: cleanup can distinguish protected current
     proof from stale junk, never deletes active baselines/receipts/signatures/
     lockfiles, and records exactly what was removed or preserved.
   - Downstream impact: Rust/GC, workspace/artifact/cache cleanup, worktrees,
     install/cache, final packet, release artifacts, and update_goal blockers.

10. `HU-STACK-010: Update Goal Eligibility Law`
    - Rule: `update_goal` is forbidden unless the CLI verifies current goal
      state, active receipts, stack claim ceiling, product proof eligibility, and
      cleanup/protection status.
    - You know it is working when: `update_goal` cannot pass with stale stack
      receipts, hidden cache dependence, missing product proof, missing cleanup
      protection, or unsupported live surfaces.
    - Downstream impact: final source-local proof, release/distribution surfaces,
      final packet, install/cache, app-registry, reviewer exposure, and
      completion response.

Gold-standard command matrix:

- `ultragoal stack fast`
  - Purpose: fastest meaningful local feedback for agent iteration.
  - Required steps: quick toolchain verification, changed-scope workspace
    topology, Rust fmt/check/focused tests when Rust changed, TypeScript
    typecheck/lint/focused tests when UI changed, Python ruff/pytest focused
    when evals changed, schema/namespace/line-cap changed-scope checks, DB
    migration syntax when migrations changed, trace/eval schema check when
    telemetry/evals changed.
  - Claim support: `FastFeedbackObservation` and
    `ChangedScopeStructuralPass` only.
  - Cannot support: review-ready, release-ready, product-success, or
    update_goal.

- `ultragoal stack standard`
  - Purpose: serious local proof before claiming a repair.
  - Required steps: Rust standard, TypeScript standard, Python standard,
    database empty/fixture migration verification, observability semantic
    convention check, agent tool-inventory verification, required red/green/
    tamper fixtures, current receipt verification, and claim-ceiling
    computation.
  - Claim support: `StandardRepairProof` and, only where explicitly allowed by
    claim guards, `ReviewReadyCandidate`.
  - Cannot support: release-ready unless release loop also passes.

- `ultragoal stack release`
  - Purpose: full law proof for release readiness.
  - Required steps: standard loop, Rust coverage/security/supply-chain/memory,
    TypeScript coverage/browser/accessibility/visual/bundle, Python eval
    regression, database migration/backup/restore/query-plan/RLS, ClickHouse
    ingest/query, NATS stream/replay, Restate workflow scenario, OpenTelemetry
    local/prod config, OpenTofu plan/policy, Kubernetes manifest/rollout where
    applicable, package inventory, install proof, cache separation, Product
    Fitness, Product Cohesion, Product Success, GC dry-run, and final packet.
  - Claim support: release/product/final-packet claims only when all same-surface
    receipts are current and the existing strict claim ceiling permits them.

- `ultragoal stack clean-proof --cache-mode none`
  - Purpose: prove clean checkout and no hidden local cache dependency.
  - Required properties: fresh checkout or clean worktree snapshot, isolated
    Cargo/Bun/uv caches, isolated target/node_modules/.venv directories, Nix
    devshell or recorded tool bootstrap, no editor/watcher state, no global env
    except allowlist, local services boot from empty/fixture state, full standard
    loop.

- `ultragoal stack watch`
  - Purpose: continuous developer feedback.
  - Rule: watch emits observations only. It cannot mint completion claims unless
    followed by current receipt verification and claim-ceiling computation.

- `ultragoal stack resources prove`
  - Purpose: prove bounded memory, no leaks, bounded queues, cleanup on
    cancellation/error, and stable long-running control plane.
  - Required checks: Rust long-running service memory, Node/Bun frontend build
    memory budget, Python eval memory budget, Postgres pool limits, ClickHouse
    retention/memory limits, NATS consumer lag/stream limits, Restate workflow
    backlog limits, browser memory/trace artifact limits, and agent tool child
    process cleanup.

- `ultragoal gc plan`, `ultragoal gc dry-run`, `ultragoal gc apply`, and
  `ultragoal gc verify`
  - Purpose: safe cleanup of stale artifacts.
  - Required scope: Cargo target dirs, Bun node_modules/cache, uv virtualenv/
    cache, Nix store roots, Bazel output base, database test dirs, ClickHouse
    local data, NATS streams, Restate state, Playwright traces/videos/
    screenshots, coverage artifacts, receipts, final packets, worktrees, locks,
    pids, ports, and tempdirs.

Stack receipt and staleness model:

- Stack receipts must record command, goal id, git commit/dirty/worktree id,
  toolchain identities, source/package/install/cache/runtime/observability
  surface digests, component receipts, claim support, claim exclusions, issued
  time, and expiry.
- Stack receipts stale on changes to git commit or dirty state, lockfiles,
  schema registry, law registry, validator binary, fixture suite, migration
  files, telemetry semantic-convention registry, model config, MCP tool
  registry, OpenTofu provider lock, Kubernetes manifests, package artifact
  digest, install tree digest, runtime config digest, or observability backend
  config.
- A stale stack receipt can support diagnosis only. It cannot support completion,
  product, review, release, install/cache, final packet, or update_goal claims.

Cache/no-cache honesty model:

- Cache use is legal. Cache concealment is illegal.
- Warm-cache speed is not clean-checkout proof.
- Cache presence is not correctness proof.
- Cache proof supports only cache-surface claims unless a same-surface law says
  otherwise.
- Governed cache classes include Cargo target/registry/git/sccache, Bun install
  cache, node_modules, Vite cache, Playwright browser cache, uv cache, Python
  virtualenv, Nix store, Bazel output base, Docker layer cache, Postgres test
  data, ClickHouse local data, NATS JetStream data, Restate state, browser
  storage state, and agent-run cache.

Resource discipline and full-stack cleanup:

- Across Rust, TypeScript/browser, Node/Bun tooling, Python, databases, caches,
  event buses, workflows, and agents: no unbounded queues, no unbounded caches,
  no spawn-and-forget tasks, no child process without owner/kill/reap policy, no
  tempdir without cleanup policy, no browser session without cleanup policy, no
  long-running worker without shutdown path, no large-file load without size
  bound or streaming justification, no DB pool without max size and timeout, and
  no workflow without timeout/retry/cancel policy.
- Artifact classes must include source/generated source, build outputs,
  node_modules/Bun/uv/Nix/Bazel/Docker caches, database/event/workflow state,
  Playwright traces/videos/screenshots, coverage reports, eval results, agent
  traces, current/stale receipts, review packets, final packets, release
  artifacts, SBOMs, provenance, signatures, lockfiles, pid files, port
  reservations, and tempdirs.
- Protected artifacts must never be deleted without replacement proof: current
  receipts supporting active claims, current final packets, release artifacts,
  SBOMs, provenance, signatures, lockfiles, law/schema registries, fixtures,
  active eval baselines, active performance baselines, active trace exemplars,
  `Cargo.lock`, `bun.lock`, `uv.lock`, `flake.lock`, and OpenTofu provider
  locks.

Full-stack product laws:

- Product Fitness requires the intended user problem to be proven on the correct
  runtime surface. API behavior needs runtime API request/response proof. UI
  behavior needs browser journey proof. Agent cockpit behavior needs trace/eval/
  run scenario proof. Database behavior needs migration plus runtime persistence
  journey. Workflow behavior needs durable replay/idempotency scenario.
  Observability behavior needs emitted and queryable trace/log/metric proof.
- Product Cohesion requires architecture, module boundaries, telemetry schema,
  database truth model, UI framework boundaries, package surfaces, docs,
  security, and operational model to fit together.
- Product Success requires same-surface proof that the actual user outcome works.
  For this stack, that usually means runtime API proof, browser journey proof,
  database persistence proof, workflow completion proof where async, trace/event
  proof, SLO/golden-signal non-regression observation, accessibility proof for
  user-facing UI, and security/secret boundary proof.
- Invalid substitutes include Rust tests pass, TypeScript builds, Python eval
  passes, database migration runs, ClickHouse table exists, NATS stream exists,
  Kubernetes rollout succeeds, package published, reviewer approval, or agent
  says complete.

Supply-chain and security baseline:

- Required controls include committed lockfiles, frozen installs in CI,
  dependency review, cargo-deny/audit/vet, Bun security scanner/audit,
  Python advisory scan where Python exists, SBOM generation for Rust/TypeScript/
  Python/container artifacts, Sigstore/cosign signing for release artifacts
  where applicable, container image digest pinning, Kubernetes admission/policy
  validation where applicable, Gitleaks and optional TruffleHog, minimum package
  age for JavaScript where supported, and eval-backed dependency-update PRs.
- Agent-specific prohibitions: agents cannot add production dependencies without
  dependency receipt, weaken CI to pass, disable tracing to hide failures, change
  approval policy without human review, modify secrets, deploy production
  without release gate, or substitute package publication for product success.

CI/local parity:

- CI must run the same `ultragoal` commands available locally. CI YAML is
  orchestration, not authority.
- Required lanes include stack-fast, stack-standard, stack-clean-proof,
  rust-coverage, frontend-browser, python-evals, migration-proof,
  observability-proof, security-supply-chain, infra-plan, release-dry-run,
  scheduled memory-resource, scheduled fuzz/property, and scheduled gc-dry-run
  where those surfaces exist.

Migration plan for existing gold-stack repos:

- Phase 0 Inventory: stack, Rust, TypeScript, Python, database, observability,
  and infrastructure inventory in report-only mode.
- Phase 1 Toolchain and lockfiles: add/verify Rust toolchain/Cargo lock,
  package manager lock, Python lock, Nix flake lock, and OpenTofu provider locks
  where applicable.
- Phase 2 Command routing: introduce fast, standard, clean-proof, and release
  stack commands; raw tools become implementation details.
- Phase 3 Architecture and namespace: refactor Rust module tree, TypeScript
  feature/surface modules, Python eval package structure, DB migration naming,
  telemetry attribute registry, and infra module boundaries.
- Phase 4 Typed boundaries: replace raw `serde_json::Value`, unchecked
  TypeScript `any`/`unknown`, Python dict authority, raw SQL authority, untyped
  telemetry, and untyped MCP inputs/outputs with typed parsers, schemas,
  newtypes, and receipt-bound validators.
- Phase 5 Red/green/tamper fixtures: every validator and law gets red, green,
  tamper, expected failure schema, and receipt proof.
- Phase 6 Product proof: add browser/API/database/workflow/observability
  journeys for Product Fitness, Cohesion, and Success.
- Phase 7 GC and release packets: classify artifacts, protect active proof,
  implement dry-run cleanup, and build final packets.

Operational addenda that must not hide inside larger buckets:

- Vite frontend build substrate is governed. Required receipts include version,
  framework adapter, mode, config digest, env allowlist digest, bundle manifest,
  chunk inventory, and source-map policy. Vite build does not prove browser
  behavior, accessibility, product success, or security absence.
- Tailwind/design-system substrate is governed when used. Required receipts
  include version, browser baseline, CSS entrypoints, style policy, and visual
  baseline. It does not prove accessibility, product success, or legacy browser
  support without explicit tests.
- OpenAI SDKs and model aliases are governed. Model aliases must live in config,
  not scattered scripts/prompts. Model changes require eval gates, instruction
  digests, SDK/version identity, before/after eval scores, and trace/eval links.
- Task runner and command discovery are governed. `ultragoal` is required;
  `just` may be default-on only as an alias layer; Nix devshell is required;
  raw shell scripts as authority are rejected. Each recipe must delegate to
  `ultragoal`.
- Optional local-only tools such as editor lenses, editor-only diagnostics,
  local DB GUIs, manual ClickHouse queries, terminal aliases, personal direnv,
  ad hoc browser devtools, manual profilers, and notebook-only analysis may help
  investigation but cannot support claims.
- Rejected practices include raw tool output as final proof, warm cache proof as
  clean proof, package install as product success, Kubernetes rollout as product
  success, DB migration success as UI success, trace existence as eval success,
  metric presence as SLO compliance, cache existence as durable truth,
  unbounded cache/queue/workflow backlog, unclassified generated artifacts,
  untyped JSON authority, hard-coded model ids in scripts, MCP tools without
  schema/risk/approval policy, secret-bearing local config in repo, blind cleanup
  deletion, and reviewer agreement as compliance.

Final strictness test:

- A full-stack law is theater if it can be satisfied by prose, reviewer
  agreement, row presence, stale evidence, source-only proof for runtime claim,
  package-only proof for install/runtime claim, install/cache proof for product
  success, fixture names without execution, red-only impossible requirements,
  green-only validators without tamper tests, hidden local cache, editor state,
  warm watcher state, or lowered claim ceiling alone.
- A full-stack law is real only if it has law id, typed inputs, typed outputs,
  deterministic validator, red fixtures, green fixtures, tamper fixtures,
  same-surface proof rule, receipt schema, digest binding, staleness policy,
  claim impact, repair class, and self-law compliance.

Spine-level ordering implied by the synthesis:

- Phase 2A: Complete Gate 92 operational observability, including command/check/
  receipt/fixture/claim-guard fitting, logs/metrics/traces/evals channel model,
  repair-loop execution, and useful query/explain output.
- Phase 2B: Harden observability doctrine: cardinality rules, agent/tool/repo/
  eval attributes, SRE plane, agent-quality plane, redaction, boundedness,
  parent/child span integrity, and query/explain quality proof.
- Phase 3: Close research doctrine integration, including the synthesis source
  card and every adopted requirement mapped through all law surfaces.
- Phase 3.5: Close Product Usage Fitness and CLI discoverability, including
  routine command surface, AGENTS routing-table doctrine, clean-checkout command
  discovery, and plugin-activated target-repo first-use path.
- Phase 4: Rebind Gates 0-91 only after Gate 92 and Product Usage are
  source-local coherent or explicitly blocked.
- Later phases: close trace/eval/repair improvement, setup/retrofit,
  active-repo rollout, Rust/TypeScript/Python/stack adapters, supply-chain,
  privacy, Agent Cockpit status, entropy cleanup, and final live surfaces.

Do not stop at a packet, issue list, blocker list, or claim ceiling downgrade. Implement deterministic enforcement until the plugin repo itself passes its laws.

Concrete required repairs:

All numbered items below are mandatory completion gates. The numbering is for reference and verification traceability only. It is not a priority order, not a sequencing excuse, not a deferral mechanism, and not permission to ship a partial subset. Anything short of dictator-level fail-closed enforcement and adherence for every listed law is failure.

1. Make standards fail-closed:
   - No `backlogged`, `blocked`, empty `gate_or_fixture_path`, or unmechanized standards row may pass material/completion/review claims.
   - Either mechanize each currently backlogged row or move it into an explicit non-goal exclusion schema that itself blocks related claims.
   - Add red fixtures proving each formerly optional law fails non-compliance.
   - Add every foundational law row to the central standards required-id set, including `namespace-progressive-disclosure`. A row enforced only through a side audit path but absent from the central required set is non-compliant.
   - Forbid "mechanized" rows that only prove row shape, audit TSV alignment, or gate-path existence. Each mechanized row must name a behavior validator, at least one red fixture for actual non-compliance, a valid fixture or receipt path, and a claim-ceiling failure mode.

2. Add foundational-law traceability:
   - Create/extend a machine-readable registry mapping each foundational article requirement to:
     - source artifact/digest
     - law id
     - standards row id
     - validator check id
     - red fixture id
     - valid fixture id when applicable
     - receipt requirement
     - claim ceiling impact
   - Validator must fail unmapped laws or laws without red fixtures/enforcement.
   - Validator must fail any foundational obligation whose disposition is `partial`, `backlog`, `blocked`, `reviewer`, `reviewer_and_backlog`, `future`, "missing receipt", "future receipt", "future validator", or equivalent weak/deferral language unless the row is typed as explicit non-goal and blocks every related claim.
   - Refresh or explicitly fail-close every foundational source card. `not_refreshed` sources may support historical background only and must block current law-completeness, current-source, release, readiness, and full-compliance claims.

3. Fix Product Fitness completely:
   - Regenerate Product Fitness receipt against the current package digest.
   - Ensure documentation-only Product Fitness, install success, package publication, first run, smoke test, fixture pass, reviewer agreement, and happy-path proof all fail as product success.
   - Keep real user/daily-driver/release/product-readiness claims impossible unless same-surface Product Fitness evidence exists.

4. Fix source/install/cache/app-registry separation:
   - Regenerate source, installed plugin, and versioned cache audit receipts from the exact same candidate digest.
   - Remove private local paths from package inventory.
   - Do not let disk install/cache proof imply app registry, Plugins UI, marketplace, install-button, launcher runtime, or reviewer exposure.
   - If live active-registry proof is possible, produce a fresh same-surface receipt.
   - If not possible, enforce that no current reviewer-ready/app-registry claim can be emitted anywhere.

5. Fix coverage self-law:
   - Add root `.harness/coverage-command` and any required coverage manifest/receipt surfaces for this plugin repo.
   - Bring plugin source coverage to exactly 100% for declared repo-owned scope, or do not finish.
   - A typed coverage receipt must prove `100%` and `uncovered_records = []`.
   - Test pass counts, smoke tests, fixtures, mocks, reviewer approval, and package audit pass must fail as coverage substitutes.

6. Fix stale receipt/red fixture propagation:
   - Refresh all valid fixture validator provenance after current validator/source changes.
   - Refresh red fixture catalog/report until all red fixtures fail for the intended reason and the report passes.
   - Validator must fail stale generated receipts, stale package digests, stale candidate versions, stale red fixture counts, and stale runtime provenance.

7. Enforce typed parsing/boundaries:
   - Audit all file/JSON/schema/receipt/CLI/env/runtime boundaries in the validator and package scripts.
   - Ensure parsing preserves typed authority instead of ad hoc string validation.
   - Add tests/red fixtures for malformed JSON, missing schema fields, path traversal, private local paths, stale digest fields, wrong candidate version, wrong cache path, and wrong receipt schema.
   - Replace freeform-text authority with typed fields for all law-bearing gates. Required examples include `requires_product_fitness`, `requires_runtime_tool_identity`, `requires_live_surface_receipt`, `requires_transcript_quality_receipt`, `current_source_claim_allowed`, and `memory_context_only`.
   - Text and substring scans may remain only as fail-closed additional detectors. They must not be the primary authority that allows a claim to pass.

8. Enforce namespace and progressive disclosure as a first-class fail-closed law:
   - Add or tighten a dedicated standards row such as `namespace-progressive-disclosure`, backed by `templates/agent-standards/01-namespace-and-progressive-disclosure.md`, the foundational article synthesis, validator checks, red fixtures, valid fixtures, and receipt requirements.
   - Add a dedicated foundational trace entry for namespace / filesystem-as-agent-interface. It must not be hidden under documentation freshness, purpose-backed files, or generic authority binding.
   - Validator must inspect repo-owned and packaged paths, not just standards metadata. It must fail junk-drawer directories or files such as `utils`, `helpers`, `misc`, vague domain-logic `common`, historical shims without an external compatibility contract, root-level clutter without routing purpose, and path names that do not reveal domain responsibility.
   - Validator must fail repeated filename prefixes across more than two files when they indicate a missing subdirectory, unless a typed exception names the external contract or generated/mechanical reason.
   - Validator must fail repo-managed files that have no active purpose recorded in a manifest, codemap, package inventory, generated receipt, fixture catalog, schema catalog, active ExecPlan, or adjacent README.
   - Validator must fail progressive-disclosure violations: bloated always-loaded docs, standards duplicated into multiple places without a routing owner, hidden proof surfaces, and task-critical files that cannot be discovered from the repo's routing documents.
   - Add red fixtures for junk drawer names, vague `common` domain logic, repeated prefixes without a subdirectory, root clutter without route/purpose, compatibility exception without external contract, generated/mechanical exception without generator proof, orphan repo file, and namespace law present only as prose.
   - Add valid fixtures proving accepted namespace exceptions are narrow, typed, routed, and claim-limited.
   - Completion, review, package, readiness, and release claims must fail when namespace law fails. This law is not advisory and cannot be waived by reviewer agreement, Product Fitness pass, coverage pass, install proof, package proof, or a broad foundational trace pass.
   - Remove capped or partial namespace reporting that can hide additional orphan files or path violations. All namespace violations must be surfaced or summarized with exact counts plus representative paths and must still fail the gate.
   - Extend namespace enforcement beyond `utils`, `helpers`, `misc`, and vague `common` to cover vague `shared`, vague `lib`, generic `services` where a domain directory is required, excessive depth, mixed-domain folders, root-level clutter, generated/mechanical exceptions without generator proof, and compatibility exceptions without external contract proof.
   - This gate is not satisfied while top-level `validator/src/internal_*.rs`, `validator/src/internal_coverage*.rs`, typo variants such as `iinternal_*`, or other prefix-as-directory validator source clusters remain accepted by blanket exceptions. The parent must treat those files as concrete non-compliant examples until Gate 90 is implemented and verified.

9. Enforce line caps:
   - Run line-cap checks over plugin source, especially `validator/src`.
   - Split files over the active cap or add a validator-enforced explicit exception only if generated/mechanical and justified.
   - Completion cannot rely on “currently okay” unless the check is durable.

10. Fix review packet correctness:
   - Supersede any packet that was created before the actual session-log/Chronicle audit and hardening.
   - Final packet must be a record of implemented findings, verification, exact claim support, and unsupported claims removed or fail-closed.
   - It must not merely say blockers remain.

11. Fix Product Fitness review-team ownership as a fail-closed law:
   - Encode Product Fitness as a first-class material-review obligation. It is not optional, advisory, implied by adjacent review, or satisfied by prose.
   - Existing four-person review may remain only if the contract explicitly promotes Product/Simplicity to the named Product Fitness / Quality-In-Use owner and assigns typed Product Fitness responsibilities across all relevant personas. If that cannot be enforced without ambiguity, add a dedicated Product Fitness falsifier and update agents, custom-agent TOML, manifest surfaces, schemas, fixtures, reports, and validator routing.
   - Update `agents/product-simplicity-falsifier.md` and `custom-agents/harness-product-simplicity-falsifier.toml` so Product/Simplicity explicitly owns Product Fitness and Quality-In-Use review for audience, job, context, outcome, accessibility, cognitive load, recovery burden, continuance, real-use evidence, and product-success substitution rejection.
   - Update Contract/Claim, Security/Trust, and Orchestration/Recovery reviewer contracts so they cannot ignore Product Fitness-adjacent claim, privacy, trust, live-surface, stale-proof, recovery, repeatability, or substitution risks.
   - Update the review-round schema, fixtures, reports, and validator checks with typed Product Fitness fields. At minimum: `product_fitness_required`, `product_fitness_owner`, `product_fitness_disposition`, `product_fitness_receipt_digest`, `product_fitness_claim_ids`, and `substitution_rejections_reviewed`.
   - Validator must fail any product-impacting review round where Product Fitness is required but the owner, disposition, current receipt binding, claim binding, or substitution review is missing, stale, wrong-surface, wrong-version, or assigned to an undeclared reviewer.
   - Explicitly reject Product Fitness substitutes: generic Product/Simplicity approval, Product Cohesion proof, install success, package publication, first use, smoke test, fixture pass, reviewer agreement, Product Fitness receipt alone without review disposition, or any material product sign-off that lacks same-candidate Product Fitness review binding.
   - Ensure Contract/Claim must reject product success, release, readiness, daily-driver, or material product sign-off claims without both a current Product Fitness receipt and current Product Fitness review disposition.
   - Add red fixtures proving review rounds fail for missing owner, missing disposition, stale receipt, wrong owner, undeclared fifth reviewer, generic approval substitution, Product Cohesion substitution, receipt-only substitution, four-person round without Product Fitness disposition, and any product-impacting claim without Product Fitness claim binding.

12. Enforce runtime, live-surface, transcript, clean-checkout, ExecPlan, source-card, and memory laws:
   - Runtime/tool identity: browser, UI, runtime, live-surface, model/tool, and app proof claims must require `runtime_tool_identity_receipt` evidence with tool identity, version, binary/path when relevant, workspace/root, artifact digests, and claim ceiling. Add red fixtures for missing tool identity, stale version, wrong workspace, missing binary/path when relevant, and digest mismatch.
   - Product live-surface receipts: live UI, live recording, replay, transcript alignment, video alignment, parent-operation coordination, and user-facing runtime claims must require same-surface live receipts. CLI checks, package proof, install proof, smoke tests, and fixtures must fail as substitutes.
   - Transcript-quality reuse gates: transcript reuse, replay, narration, summary, or word-trust claims must fail without final post-stop transcription, cleanup, alignment, quality receipt, and stale-proof guards. Disabled or missing finalization is a hard failure.
   - Clean-checkout command discovery: install/setup/runtime/check commands must be discoverable and runnable from a clean checkout or installed package without author memory. Add a typed receipt and red fixtures for missing root check, prose-only command, non-runnable command, local-state dependency, missing installed command, and source/install/cache command drift.
   - Restartable ExecPlans: active ExecPlans must be self-contained living contracts with purpose, user outcome, steps, progress, discoveries, decisions, validation commands, idempotence, recovery, and demonstrably working behavior. Reviewer-only ExecPlan validation is not enough; add deterministic checks and red fixtures for each omitted section or stale/prose-only plan state.
   - Source-card freshness: every foundational source card used for current law claims must be freshly refreshed or must hard-block current-source, full-compliance, readiness, release, and law-completeness claims.
   - Memory/wiki/Chronicle context-only: memory, wiki, Chronicle summaries, packet summaries, and session summaries may guide audit, but cannot be accepted as current implementation proof unless joined to live same-surface files, command outputs, digests, receipts, or runtime artifacts. Add claim-evidence red fixtures for memory-only and summary-only proof.

13. Bump plugin version after all hardening:
   - Update `.codex-plugin/plugin.json`, `plugin-manifest-draft.json`, installed plugin, cache path/version, receipts, and packet references consistently.

14. Enforce architecture dependency topology as a first-class fail-closed law:
   - Add or tighten a machine-readable architecture boundary registry that names repo/package layers, domains, public entrypoints, allowed dependency edges, forbidden edges, and typed exceptions.
   - Validator must fail forbidden cross-layer imports, deep imports across ownership boundaries, circular dependencies, mixed-domain modules, unregistered public APIs, and dependency-direction drift.
   - Structural tests, schemas, red fixtures, valid fixtures, receipts, foundational trace entries, and standards rows must prove actual non-compliant dependency behavior fails. A prose architecture document, namespace pass, line-cap pass, or reviewer agreement is not enough.
   - Add red fixtures for forbidden edge, missing layer map, circular dependency, deep import bypass, broad shared/common escape, mixed-domain folder, stale architecture registry, and exception without external contract.

15. Enforce Quality Score and taste invariants as typed gates:
   - `templates/QUALITY_SCORE.md` and any generated quality receipt must be schema-bound to changed/claimed surfaces, not a summary humans can fill in after the fact.
   - Validator must fail stale quality grades, missing quality rows for changed domains, unsupported "looks good" grades, quality categories without evidence digests, and quality rows that pass while Product Fitness, namespace, line-cap, coverage, runtime, security, or architecture laws fail.
   - Quality/taste claims must include current evidence for boundaries/types, tests/coverage, code shape, security, latency/efficiency, observability, product cohesion/fitness, docs/architecture fit, receipts, and residual gaps.
   - Add red fixtures for missing quality score, stale score, evidence-free category pass, product-fitness substitution, architecture-law failure hidden by quality pass, and broad reviewer approval used as a quality substitute.

16. Enforce feedback-to-rule promotion without backlog escape:
   - Every repeated user correction, reviewer finding, Chronicle/session-log signal, or prior "weak/not enforced/missing/overclaim/stale" class must map to a deterministic validator, schema, fixture, receipt, standards row, or typed non-goal that blocks all related claims.
   - "Backlog", "future", "follow-up", "blocked", "reviewer agreed", "claim ceiling lowered", or "documented only" is not a permitted terminal disposition for a material repeated finding.
   - Add a feedback-promotion receipt linking source artifact/session id, failure class, affected surface, implemented enforcement, red fixture, valid fixture/receipt, and claim-ceiling impact.
   - Validator must fail repeated findings that have no deterministic enforcement or claim-blocking non-goal.

17. Enforce the full autonomy-loop proof law:
   - For bugfix, runtime, product, install, registry, review-packet, validator, and evidence repairs, require a typed autonomy-loop receipt when the claim depends on behavior changing.
   - Receipt must include current-state validation, reproduced failure or stale-proof signal, before artifact, implemented repair, after validation by the relevant runtime/same surface, build/test remediation status, resolution artifact, and any human escalation reason.
   - Validator must fail repairs that only show code changes, passing unit tests, or packet text when the law requires before/after behavioral proof.
   - Add red fixtures for missing reproduction, missing before artifact, missing after runtime proof, CLI substituted for live surface, build failure ignored, and escalation without judgment-only reason.

18. Enforce orchestrator state-machine invariants:
   - Orchestration claims must prove bounded concurrency, one authoritative orchestrator state, deterministic per-issue/per-lane workspaces, state-transition rules, stop-on-ineligible state changes, retry/backoff policy, restart recovery, and observability events.
   - Validator must fail multiple authoritative state sources, unbounded concurrency, no backoff, no stop-on-state-change, non-deterministic workspace naming, missing restart recovery, and prose-only observability.
   - Add schemas, red fixtures, receipts, and foundational trace entries for every state-machine invariant.

19. Enforce scheduler/runner/tracker mutation boundaries:
   - Scheduler, runner, and tracker-reader roles must not directly perform external tracker/ticket writes unless a typed tool authority and approval boundary permits it.
   - Successful orchestration handoff must not be treated as "Done" unless a coding agent or authorized tool produced the required implementation and verification receipts.
   - Validator must fail direct scheduler ticket writes, handoff-as-done claims, missing tracker write authority, missing approval proof, and external mutation without provenance.

20. Enforce subagent and custom-agent sandbox/approval inheritance:
   - Custom-agent and subagent launch receipts must record parent sandbox, child effective sandbox, approval policy, runtime/tool overrides, model, workspace/root, and approval-request behavior.
   - Validator must fail child permissions broader than parent, missing sandbox/approval fields, non-interactive approval gaps hidden by reviewer output, parent runtime overrides not reapplied, or reviewer/custom-agent proof without same-round launch receipt.
   - Add red fixtures for broadened sandbox, missing approval inheritance, stale agent TOML, undeclared reviewer, wrong model, and inactive-thread approval request treated as success.

21. Enforce skill progressive-disclosure metadata and load routing:
   - Every packaged skill must have specific discoverable metadata, bounded always-loaded content, valid local references/scripts/assets, and a routing path that lets an agent load only what the task requires.
   - Validator must fail vague skill descriptions, bloated `SKILL.md` files, broken relative references, hidden required docs outside the package, duplicated standards in always-loaded files, and skill behavior that depends on undocumented author memory.
   - Add red fixtures for vague metadata, giant skill file, broken reference, missing script asset, hidden dependency, and progressive-disclosure bypass.

22. Enforce plugin install-surface metadata, cache semantics, and enable-state proof:
   - Source/install/cache proof must validate not only digest equality but also plugin manifest identity, publisher/developer/interface metadata, install-surface copy, personal marketplace entry, installed cache path, local-vs-versioned semantics, enablement/config state, and installed-load proof.
   - Validator must fail marketplace/app/install/enable claims when `.codex-plugin/plugin.json`, `plugin-manifest-draft.json`, personal marketplace example, installed copy, cache package, and enable-state receipt disagree.
   - Add red fixtures for missing publisher/interface metadata, wrong cache version path, local package misrepresented as versioned, missing config enablement, stale installed copy, marketplace copy mismatch, and installed-load proof substituted by source audit.

23. Enforce ExecPlan no-handback and prototype promotion/discard laws:
   - Active ExecPlans must not ask the user for generic next steps when the next milestone is knowable from the plan. They must proceed through the next milestone or record a true explicit blocker with claim impact.
   - Every stopping point must update progress, discoveries, decision log, outcomes, validation, idempotence, recovery, and next milestone.
   - Any prototype, proof-of-concept, spike, toy implementation, or experimental path must have typed promotion/discard criteria, owner, cleanup path, evidence requirement, and claim ceiling.
   - Validator must fail ask-user-next-steps language, stale stopping-point sections, missing decision log, prototype without promotion/discard criteria, and active plan state that cannot restart a fresh agent.

24. Enforce semantic domain-type naming on authority surfaces:
   - Law-bearing exported types, schema fields, receipt fields, validator authority objects, and claim/evidence identifiers must use semantic domain names that encode what the value means.
   - Validator/lint must fail vague authority names such as `Data`, `Info`, `Record`, `Item`, `Value`, `Thing`, `Result`, broad `Record<string, unknown>`, or generic catch-all fields when a domain-specific type or enum is required.
   - Typed exceptions are allowed only for local generic algorithms, generated code, or external compatibility contracts, and the exception must be narrow, documented, and claim-limited.
   - Add red fixtures for generic authority type, broad record in receipt, stringly typed claim state, generic evidence field, and exception without contract.
   - File and module names are authority surfaces too. Semantic domain-type naming must fail generic module names, historical-wave names, coverage-wave names, and repeated prefix names when the filesystem should instead expose a typed domain directory and descriptive module names. `internal_coverage_waveNN_tests.rs` style names are not semantic domain names; they are coverage-chase history leaking into the repo interface.

25. Enforce agent-remediating validator failure messages:
   - Every validator, linter, schema failure, red fixture failure, and audit failure must emit enough typed remediation context for an agent to repair the issue without guessing.
   - Failure output must include law id, source obligation/foundational trace id, affected path or claim id, violated condition, expected evidence, exact repair class, claim-ceiling impact, and whether the failure is stale-proof, wrong-surface, missing-proof, bad-shape, or behavioral non-compliance.
   - Validator must fail vague or non-actionable errors, failures without law ids, failures without repair action, failures without claim impact, and fixture failures that do not identify intended-failure reason.
   - Add red fixtures for vague error text, missing law id, missing source obligation, missing repair instruction, missing claim-ceiling impact, missing affected surface, and fixture failure reason mismatch.

26. Enforce third-party dependency legibility and typed adapter boundaries:
   - Dependency use must be legible to agents through a dependency registry or equivalent typed record naming owner, purpose, upstream docs/source, allowed call sites, adapter path, typed request/response boundary, timeout/retry policy, cache behavior, and claim impact.
   - External clients, file formats, network APIs, package registries, browser/runtime APIs, and tool outputs must enter through typed adapters. Direct third-party calls from arbitrary modules are forbidden unless a typed exception names the external contract and claim ceiling.
   - Validator must fail untyped external clients, direct third-party API bypasses, broad `serde_json::Value`/stringly typed upstream use on authority surfaces, undocumented dependency behavior, no timeout/retry policy where applicable, and data probing without parsed boundary types.
   - Add red fixtures for opaque upstream use, missing dependency registry entry, direct API bypass, untyped adapter, missing timeout, cache behavior without invalidation, and unvalidated data probing.

27. Enforce repo knowledge index and core-beliefs verification:
   - Law-bearing docs, source cards, standards, templates, schemas, skills, custom agents, receipts, and generated reports must be discoverable from a repo knowledge index or routed manifest.
   - Core beliefs, architectural decisions, product laws, standards laws, and proof-surface laws must be indexed with owner, freshness, source artifact, validator/receipt path, and claim-ceiling impact.
   - Validator must fail law-bearing docs that are not indexed, stale indexed docs, duplicated law text without a routing owner, missing core-belief/source linkage, and docs that are discoverable only by author memory.
   - Add red fixtures for unindexed law doc, stale index entry, duplicate law without owner, missing source-card binding, missing validator path, and hidden proof surface.

28. Enforce workflow template parsing, strict rendering, and dynamic reload:
   - Workflow contracts and orchestration templates must parse into typed records. Unknown variables, unknown filters, malformed front matter/YAML/TOML/JSON, unsafe path expansion, missing required fields, and stale rendered prompts must fail closed.
   - Repo-owned workflow files such as `WORKFLOW.md`, active ExecPlans, lane registries, automation prompts, and launch templates must support deterministic reload or explicitly record why reload is not applicable with claim impact.
   - Validator must fail non-map front matter, unknown template variable, unknown filter, missing workflow source, stale rendered prompt, unsafe path normalization, rendered prompt without source digest, and runtime using stale workflow policy.
   - Add red fixtures for malformed workflow front matter, unknown variable, unknown filter, path traversal in template input, missing source digest, stale render, and reload not applied.

29. Enforce workspace command confinement and lifecycle cleanup:
   - Every lane/workspace/runtime command claim must prove command confinement to the intended workspace/root and allowed output paths.
   - Workspace lifecycle receipts must record creation, command cwd, allowed writes, environment identity, cleanup/teardown when terminal, stale workspace detection, and contamination checks before reuse.
   - Validator must fail commands run outside the declared workspace, writes outside allowed roots, stale workspace reuse, missing cleanup after terminal state, missing environment identity, and artifact paths that cannot be tied to the workspace.
   - Add red fixtures for wrong cwd, outside-root write, stale workspace reused as fresh, missing teardown, missing environment identity, and artifact from undeclared workspace.

30. Enforce plugin bundled component graph and hook/app/MCP safety:
   - The plugin package graph must enumerate every bundled skill, custom agent, app, MCP server, hook, script, template, asset, schema, fixture, receipt, generated report, and install/config file.
   - Every component must have package-relative paths, purpose, owner, source manifest entry, load/install surface, safety boundary, and claim impact. Paths escaping the package or pointing to private local proof are forbidden.
   - Validator must fail missing component files, unlisted bundled components, private local paths, package-root escapes, hook/app/MCP declarations without shipped implementation, stale component digests, and installable package claims with graph gaps.
   - Add red fixtures for missing hook, missing app, missing MCP server, unlisted asset, private local component path, path escaping package root, stale component digest, and component graph/manifest drift.

31. Enforce instruction precedence and nested `AGENTS.md` routing:
   - Instruction precedence must be represented as a typed route across root `AGENTS.md`, nested `AGENTS.md`, plugin skills, custom-agent prompts, templates, BOOT/reference docs when applicable, and repo law documents.
   - Validator must fail conflicting laws with no precedence resolution, nested `AGENTS.md` ignored for edited subtrees, too-much-reading instructions without progressive routing repair, stale root routing, duplicate law definitions with no owner, and custom-agent prompts that weaken repo law.
   - Any instruction conflict must resolve to the strictest applicable law or explicitly block all affected claims.
   - Add red fixtures for nested instruction conflict, root law overriding stricter local law, duplicate standard without owner, hidden mandatory doc, stale route, and custom-agent prompt weakening a law.

32. Enforce ExecPlan plain-language, expected-output, and interface completeness:
   - Active ExecPlans must define terms of art, inputs, outputs, dependencies, interfaces, exact commands, expected command outputs or receipt paths, observable acceptance criteria, artifacts, idempotence, recovery, and user/product outcome.
   - Validator must fail undefined terms, missing expected output, missing interface/dependency section, acceptance criteria stated only as code shape, missing artifact evidence, missing recovery path, and plan text that a fresh agent cannot execute without author memory.
   - Add red fixtures for undefined acronym/term, command without expected output, missing artifact path, missing interface dependency, internal-only acceptance, no recovery section, and hidden author-memory dependency.

33. Enforce guardrail speed, isolation, and cache honesty:
   - Guardrails required for routine development must have runtime budgets, bounded input scope, deterministic sampling when needed, and isolation from concurrent workspaces.
   - Final readiness/compliance checks must state whether cached data was used, what cache keys/invalidation rules apply, and when a no-cache or clean-checkout run is required.
   - Validator must fail unbounded guardrail runtime, hidden cache dependency, stale cache pass, cache key without invalidation, shared cache contamination across lanes, final proof relying on fast/cached checks when full/no-cache proof is required, and performance regressions that make strict gates impractical.
   - Add red fixtures for slow guardrail without budget, cached proof misrepresented as fresh, missing invalidation, cross-workspace cache collision, fast-check substituted for final proof, and no-cache-required claim without no-cache evidence.

34. Enforce secret/token boundaries for subagents, dynamic tools, hooks, and receipts:
   - Secrets, tokens, credentials, private raw prompts, private transcripts, raw message payloads, and sensitive local paths must not be exposed to subagents, custom-agent prompts, dynamic tools, hooks, logs, generated receipts, fixtures, review packets, or package inventory.
   - Tool authority must route through typed capability/proxy records that expose only permitted operations and redacted evidence. Raw credentials must never be used as proof.
   - Validator must fail secret-like values in prompts, env captures, logs, receipts, fixtures, package manifests, component graphs, review packets, and child-agent context.
   - Add red fixtures for leaked token in child env, secret in prompt, raw credential in receipt, private raw transcript in packet, unredacted message payload, sensitive local path in package inventory, and dynamic tool proof without capability boundary.

35. Enforce generated/proof artifact provenance and anti-fabrication:
   - Every generated receipt, report, packet, manifest snapshot, fixture report, archive, review target, schema-derived output, and package inventory artifact must identify the command/tool, version, cwd, inputs, input digests, output path, output digest, timestamp, candidate version, and claim ceiling.
   - Handwritten or manually edited generated/proof artifacts must fail unless a typed manual-edit exception is explicitly allowed, scoped, provenance-preserving, and claim-limited.
   - Validator must fail proof artifacts without generator provenance, artifacts whose content digest does not match recorded inputs, copied proof from another candidate/surface, hand-edited generated outputs, and generated outputs not reproducible by the named command.
   - Add red fixtures for fabricated receipt, copied package proof, missing generator command, missing input digest, stale output digest, wrong cwd, wrong candidate version, and manually edited generated report.

36. Enforce review feedback disposition and same-round satisfaction:
   - Every human review comment, agent review finding, reviewer-round issue, Chronicle/session-log review signal, and parent/side-thread steering correction must have a typed disposition: fixed with evidence, already enforced with evidence, non-goal with claim block, or invalid with proof.
   - Reviewer agreement, silence, stale review, or a later packet cannot close a finding without a same-round or superseding disposition record tied to the candidate digest.
   - Validator must fail unaddressed review findings, missing disposition evidence, stale reviewer satisfaction, cross-round issue carryover without closure, and any material sign-off that ignores open review feedback.
   - Add red fixtures for unresolved reviewer finding, stale satisfaction, missing disposition, reviewer silence treated as pass, finding closed by prose only, and side-thread correction not entered into feedback promotion.

37. Enforce behavior-example coverage and coverage anti-gaming:
   - Coverage must demonstrate behavior for every line in declared scope, not merely execute lines. Tests must include meaningful assertions or executable examples tied to the code path and claim.
   - Unreachable code, dead branches, vacuous wrappers, trivial no-op tests, blanket coverage exclusions, generated-code misclassification, and test-only code paths used to inflate coverage must fail.
   - Coverage receipts must distinguish behavior-proving tests from hit-only execution and must keep `uncovered_records = []`, `excluded_records` justified, and changed-file coupling current.
   - Add red fixtures for trivial hit-only test, unreachable code retained, blanket exclusion, generated-code false label, test-only branch, unasserted coverage, and changed line without behavior example.

38. Enforce one-command fresh environment bootstrap and concurrent resource allocation:
   - A fresh source checkout or installed/cache package must expose a one-command bootstrap/check path that can create or validate a working environment without author memory or manual tinkering.
   - Environment bootstrap must be fast enough for routine use, copy or bind local config safely, install dependencies deterministically, and allocate ports, database names, caches, background jobs, temp dirs, logs, and runtime resources without cross-workspace collisions.
   - Validator must fail multi-step undocumented setup, manual config dependency, missing local-config safety boundary, non-deterministic dependency install, hardcoded port/database/cache/job names, and environment bootstrap that cannot run concurrently.
   - Add red fixtures for missing bootstrap command, manual `.env` dependency, hardcoded port, shared cache, shared job queue, non-deterministic install, slow bootstrap without budget, and concurrent environment collision.

39. Enforce agent-queryable observability surfaces:
   - Logs, metrics, traces, screenshots, DOM/browser state, runtime events, and workflow/validator events used as proof must be queryable by agents through typed, bounded, redacted interfaces.
   - Observability receipts must include operation name, stable entity ids, correlation/request/run id, duration when applicable, outcome, error category, query command or API, output digest, retention/teardown rule, and sensitive-data redaction proof.
   - Validator must fail observability claims backed by raw unbounded logs, uncorrelated screenshots, missing query provenance, missing redaction, missing retention/teardown, missing runtime identity, and metrics/traces that cannot be joined to the claim.
   - Add red fixtures for raw log dump, missing correlation id, unbounded query, screenshot without runtime identity, trace without claim binding, metric without query provenance, and unredacted sensitive event.

40. Enforce subagent orchestration explicitness, token/model cost, and result reconciliation:
   - Subagents may be used only when explicitly requested or when the repo contract requires named reviewer/custom-agent lanes. Each launch must record why subagents are required, expected outputs, model, reasoning/tool budget, token/cost risk, parent synthesis owner, and stop condition.
   - Parent synthesis must wait for required subagent results or fail closed, reconcile expected vs observed outputs, classify missing/orphaned/failed subagents, and prevent child output from becoming proof without live verification.
   - Validator must fail implicit subagent spawning, unbounded subagent fanout, missing token/model/cost budget, orphaned child result, missing parent synthesis, child output treated as proof, and failed subagent ignored.
   - Add red fixtures for implicit spawn, missing budget, missing result, orphaned agent, partial synthesis, child-proof substitution, and subagent launch without explicit requested/contracted role.

41. Enforce skill catalog context-budget and omission-warning law:
   - The packaged skill catalog must fit within the configured context budget and preserve discoverability for required skills. Skill descriptions must be concise enough to survive truncation and specific enough to route correctly.
   - When installed skill count or description length causes omission, shortening, or warning behavior, the package must record a typed warning/claim ceiling and must not claim all skills are initially visible.
   - Validator must fail oversized skill catalogs, descriptions that crowd out task context, required skills omitted without warning, descriptions too vague after truncation, duplicate names that confuse routing, and no claim-ceiling guard for omitted skills.
   - Add red fixtures for catalog over budget, omitted required skill without warning, duplicate skill name, vague truncated description, context-heavy description, and discoverability claim without catalog proof.

42. Enforce distribution and sharing-surface claim separation:
   - Local source, personal marketplace, repo marketplace, installed plugin, versioned cache, Codex app Plugin UI, workspace-shared plugin, public Plugin Directory, share link, and organization-bound access are separate surfaces.
   - Claims about workspace sharing, public availability, teammate access, install-button success, plugin directory listing, or organization boundary must require same-surface proof and must not be inferred from source, cache, local install, or personal marketplace proof.
   - Validator must fail workspace/public sharing claims without app-surface receipt, public directory claims from local marketplace proof, share-link claims without current app proof, organization-boundary claims without account/workspace proof, and admin-disabled sharing ignored.
   - Add red fixtures for local proof used as workspace sharing, personal marketplace used as public directory, stale share link, missing workspace/account id, admin-disabled sharing overclaim, and teammate access claim without same-surface receipt.

43. Enforce total authority types and impossible-state elimination:
   - Law-bearing authority surfaces must encode invariants in types so invalid or impossible states cannot be constructed after parsing. Runtime revalidation of already-validated impossible cases is not enough.
   - Validator and schema layers must reject nullable authority where non-empty values are required, partial enums with catch-all strings, optional fields that become required later, unchecked `unwrap`/`expect`/panic-style authority paths, and authority functions that can return impossible states.
   - Typed constructors or parser outputs must carry proof of non-empty lists, current candidate binding, same-surface binding, valid digest, permitted claim state, and complete disposition where those invariants matter.
   - Add red fixtures for nullable required authority, empty list accepted then rechecked, optional candidate digest, catch-all claim state, panic on malformed authority input, unchecked unwrap on receipt field, and impossible state represented only by convention.

44. Enforce agent-authored source, tooling, and documentation provenance:
   - Any source, validator, schema, fixture, docs, script, hook, receipt, package, or tooling change that supports a law-bearing claim must have agent-run provenance: changed-file inventory, command transcript pointer, candidate digest before/after, authoring session/thread identifier where non-private, and reviewer/parent source when the change came from review feedback.
   - The package must not imply that law-bearing code, docs, tests, scripts, generated artifacts, or review responses were produced out-of-band without an agent-visible receipt. Manual/user edits can only remain if they are explicitly classified as external input, bound to a current digest, and barred from supporting automated-compliance claims until revalidated.
   - Validator must fail law-bearing claims from untracked local changes, unproven generated source, stale post-validation edits, missing changed-file digest, missing transcript pointer, generated code altered without regeneration proof, and docs/tooling edits that lack the same provenance standard as product code.
   - Add red fixtures for untracked claim source, hand-edited validator without provenance, stale changed-file digest, generated artifact modified after validation, docs-only compliance claim without command evidence, and review response emitted without source finding binding.

45. Enforce stable identifier, normalization, and collision law:
   - Candidate ids, package versions, session ids, thread ids, turn ids, issue ids, workspace keys, cache keys, install ids, review-round ids, receipt ids, archive ids, and registry ids are distinct typed identifiers with canonical normalized forms. Human-readable titles, prose labels, file names, or display strings are not authority identifiers.
   - Identifier normalization must be deterministic, path-safe, case-stable, collision-checked, and bound to the authority surface it names. A normalized workspace/cache/install key must not be reusable across different candidates, surfaces, repos, or package versions without a typed equivalence proof.
   - Validator must fail malformed ids, path traversal through ids, title-as-id authority, case-only collisions, digest/id mismatch, cache-key reuse across candidates, session id without thread/turn binding, receipt id detached from artifact digest, and state normalization that changes semantic meaning.
   - Add red fixtures for unsanitized workspace key, duplicate ids after normalization, stale id reused for a new candidate, display title treated as authority, receipt id with mismatched digest, and mixed-case state accepted as a distinct lifecycle state.

46. Enforce agent session telemetry, token accounting, and rate-limit handling:
   - Every agent/reviewer/custom-agent/automation run that contributes law-bearing evidence must record bounded, redacted runtime telemetry: session id, thread id, turn id where available, model, reasoning-effort/model class when available, start/end timestamps, last event timestamp, input/output/total token counts or explicit unavailable field, runtime seconds, retry/backoff events, and rate-limit snapshot/impact when surfaced by the runtime.
   - Parent packets must separate live-session telemetry from summarized session logs and must not use stale summaries as proof of current liveness, budget, model selection, reviewer exposure, or rate-limit safety.
   - Validator must fail orphaned live-session evidence, missing token accounting without explicit unavailable status, stale last-event timestamp, rate-limit exhaustion ignored as success, model/reasoning drift hidden by prose, private raw payload leakage, and reviewer results accepted without reconciling session telemetry.
   - Add red fixtures for missing token totals, stale live-session timestamp, reviewer output with no session id, rate-limit hit but claim remains green, model mismatch between TOML and receipt, and private transcript payload embedded in a public packet.

47. Enforce config precedence, defaults, and environment indirection:
   - All config that affects law-bearing behavior must declare typed sources and precedence: defaults, repo files, plugin manifest, installed copy, cache package, environment variables, CLI args, user/app settings, and generated receipts. Unknown config keys and silent fallback across authority surfaces are forbidden.
   - Environment-variable indirection must be explicit, typed, redacted, and validated before dispatch. Missing required env/config values must fail closed with agent-remediating messages instead of silently downgrading proof, changing paths, widening scope, or switching surfaces.
   - Validator must fail wrong precedence, unknown config accepted, env var reference unbound, secret printed in config evidence, source/install/cache config drift hidden by defaults, CLI override not recorded, and public claim produced from private/local config.
   - Add red fixtures for default overriding repo policy, env-var typo treated as empty string, unknown config accepted, secret leaked in receipt, cache config diverging from source, CLI arg changing candidate id without receipt binding, and app config claim inferred from repo config.

48. Enforce fresh-init versus retrofit mode separation:
   - Target-repo operations must carry an explicit typed mode: fresh initialization, existing-repo retrofit, source-only audit, installed-plugin audit, cache-package audit, registry/app proof, or review-packet assembly. Each mode must declare permitted writes, expected files, idempotency behavior, merge strategy, rollback/recovery evidence, and non-goal surfaces.
   - Fresh-init proof cannot satisfy retrofit proof, install/cache proof cannot satisfy target-repo setup proof, and source-only proof cannot satisfy live app/registry proof. The validator must require same-mode receipts for claims about repo setup, migration safety, idempotency, and package usability.
   - Validator must fail omitted mode, fresh-init fixture used for retrofit claim, retrofit modifying unrelated user files, installed package missing init/retrofit parity proof, setup mode changing during a run without a receipt, and rollback/idempotency claims without evidence.
   - Add red fixtures for fresh init used as retrofit, retrofit writes outside allowed paths, mode omitted from receipt, cache audit used as target-repo setup proof, idempotency claim without second run, and rollback claim with no restored digest.

49. Enforce issue/tracker lifecycle, eligibility, and terminal-state law:
   - Goals, review findings, issue inventory rows, standards rows, review-round comments, package tasks, and repair actions must use normalized lifecycle states with typed eligibility rules: discovered, classified, accepted, in-progress, retrying, blocked-by-external-authority only when claim-blocking, fixed, verified, non-goal, superseded, and rejected-with-evidence. Prose-only status is not authority.
   - Dispatch, reviewer readiness, packet inclusion, and `update_goal()` eligibility must depend on current typed state plus candidate binding, not on stale summaries, human-readable labels, prior receipt existence, or the word `complete`.
   - Validator must fail case-mismatched states, unrecognized states, terminal state without verification evidence, ineligible item dispatched, stale claim marked fixed, blocked state that still permits a supported claim, non-goal without evidence, and issue inventory row omitted from packet disposition.
   - Add red fixtures for `Done` accepted as verified, blocked row counted as supported, stale review issue marked fixed without candidate digest, non-goal without rationale, state case collision, and `update_goal()` allowed while accepted issue remains unverified.

50. Enforce targeted refactor, debt-removal, and standards-gardener cadence:
   - Repeated deviations, session-log signals, reviewer findings, standards rows, article-law gaps, and workaround patterns must enter a typed debt/remediation queue with owner surface, enforcing gate, evidence target, due cadence, current status, and claim impact. Accumulated early-codebase debt cannot be exempted because it predates the current candidate.
   - Recurring cleanup must update quality grades, law traces, fixtures, and validator coverage. A passing audit must not hide known debt that blocks any foundational law, and packets must prove debt was either eliminated, mechanized into a failing gate, or classified as a typed non-goal with no supported claim depending on it.
   - Validator must fail repeated findings with no deterministic enforcement, stale debt rows, quality grade unchanged after material repairs, debt labeled backlog/future while claims remain green, standards-gardener cadence missed, and recurring cleanup evidence detached from source/install/cache candidate digest.
   - Add red fixtures for repeated namespace violation not promoted to a check, stale debt row, quality grade not updated after hardening, backlog debt supporting a claim, recurring cleanup receipt with wrong candidate digest, and standards-gardener cadence omitted.

51. Enforce plugin flow graph, package dependency closure, and plugin product journey authority:
   - The plugin must include a machine-readable flow graph such as `docs/plugin-cohesion-manifest.json` that declares every skill, authorable template, setup-required script, setup-required schema, setup-required fixture group, setup-required receipt, validator check id, standards row id, custom agent, package/cache/install surface, required edge, flow entrypoint, completion receipt, and claim ceiling.
   - Every shipped skill, script, template, schema, fixture, custom agent, automation prompt, manifest entry, package/cache/install path, and validator reference must have all local references closed inside the claimed package surface. A file required by setup but absent from the plugin manifest, cache package, or installed package surface being claimed is a hard failure.
   - The plugin itself is a product surface and must produce a digest-bound plugin product journey receipt proving the agent journey: invoke plugin entrypoint, classify fresh versus retrofit, install or verify required setup surfaces, run standards enforcement, run coverage authority, run worktree setup, run target-repo setup validation, emit fit-repo receipt, produce claim ceiling, and refuse unsupported live/install/marketplace/runtime claims.
   - Validator must fail missing or malformed flow graph, missing entrypoint, missing required edge, setup file not packaged, skill/template/script/schema/fixture/reference gap, source/cache/install/app registry flow mismatch, product journey receipt missing, journey error path omitted, and flow claim surface unproven.
   - Add red fixtures for missing `fit-repo` entrypoint, missing flow graph, missing required edge, script references unshipped file, validator references unshipped fixture, source file absent from shipped package, product journey receipt missing, unsupported runtime claim emitted, and plugin flow manifest/cache mismatch.

52. Enforce portable non-prescriptive adapter and implementation-choice law:
   - Strict enforcement must not hard-code one task tracker, one UI tool, one repo command set, one programming language stack, one workflow engine, one VCS workflow, one observability backend, or one deployment surface unless the claim explicitly names that adapter and same-surface proof exists.
   - Every optional integration or implementation choice must be represented as a typed adapter/capability with availability, authority surface, required receipts, fallback behavior, and claim ceiling. Missing optional adapters must fail only the claims that depend on them, while required adapters for an emitted claim must fail closed when absent.
   - Validator must fail Linear-only, GitHub-only, browser-only, `codex-workflow-rs`-only, LogQL/PromQL/Grafana-only, language-stack-only, or VCS-specific assumptions in generic plugin/package/readiness claims unless a typed adapter contract and claim limitation proves the assumption is intended.
   - Add red fixtures for exact tracker required by generic claim, UI-tool proof required for non-UI repo, language-specific setup forced on nonmatching repo, workflow-engine-specific lane numbering treated as law, optional observability backend treated as mandatory, missing adapter fallback, and adapter absence still allowing dependent claim.

53. Enforce derived authority recomputation and named-authority fallback refusal:
   - Proof-bearing derived authority values must be recomputed from canonical current inputs or bound to a digest derived from those inputs. Content digest or path privacy alone is not enough when an artifact encodes authority such as rankings, resolver output, package inventory, install alignment, archive anchors, source-card status, moving values, or runtime/app authority.
   - When the user, contract, or packet names a full plugin, installed package, app registry, marketplace, process, runtime, or other authority, local/source/cache/reference fallbacks are forbidden unless the receipt explicitly records the named authority, fallback boundary, affected claims, and withheld claim ceiling.
   - Validator must fail derived authority copied from stale inputs, digest detached from canonical inputs, fallback to local source when installed/app authority was named, moving-value proof without recomputation, archive/source-card/package inventory derived from stale data, and named authority claim emitted after fallback.
   - Add red fixtures for stale derived package inventory, copied archive anchors, source-card currentness inferred from old digest, installed-plugin claim backed by source fallback, app-registry claim backed by cache fallback, moving-value digest mismatch, and fallback boundary missing claim withholding.

54. Enforce offline schema catalog and resolver portability:
   - `schemas/schema-catalog.json` or its successor must be part of the package contract. Validators must preload the catalog, resolve every local schema id to a package-relative file, resolve relative `$ref` values through the same catalog, and never depend on undocumented local resolver state.
   - Remote schema fetches, network schema resolution, absolute local schema paths, missing catalog entries, catalog/schema drift, renamed schemas without same-change catalog updates, and installed/cache/source schema-catalog mismatch must fail closed.
   - Offline validation of canonical templates, fixtures, receipts, manifests, and generated examples is a package acceptance gate. Passing only on the author's machine, through network access, or through an undeclared resolver cache is non-compliant.
   - Add red fixtures for remote schema fetch attempt, absolute local schema path, missing catalog entry, stale schema id, relative `$ref` escaping package root, source/cache catalog drift, installed package missing schema, and valid fixture passing only with network resolver.

55. Enforce batch fan-out, custom-agent job schema, and worker-result discipline:
   - Any CSV/batch fan-out or many-worker subagent workflow used or claimed by the plugin must declare typed source rows, stable item ids, worker prompt template variables, output schema, output artifact path, `max_concurrency`, `max_runtime_seconds`, runtime state root, and parent synthesis owner.
   - Each worker must report exactly one structured result conforming to the output schema. Missing results, duplicate results, orphan workers, malformed result JSON, timeout without status, unbounded fan-out, and parent synthesis that ignores errored rows must fail closed.
   - Custom-agent files used in these workflows must satisfy the required TOML schema (`name`, `description`, `developer_instructions`) and must record optional inherited settings such as model, reasoning effort, sandbox, MCP servers, skills, max thread/depth policy, and override behavior when they matter to a claim.
   - Add red fixtures for missing `id_column`, placeholder references unknown CSV column, missing output schema, worker reports twice, worker exits without result, output CSV missing job metadata, unbounded concurrency, timeout ignored, malformed custom-agent TOML, and built-in/custom-agent name collision without explicit precedence.

56. Enforce raw-private artifact handling and category-only evidence law:
   - Raw private transcripts, audio, prompts, message payloads, session logs, browser state, private local paths, secrets, credentials, unpublished research data, and other sensitive artifacts must not be copied into package artifacts, fixtures, review packets, public receipts, generated reports, or child-agent prompts.
   - Session-log/Chronicle audits may inspect required local evidence, but durable outputs must carry only bounded, redacted, category-level findings, source artifact identifiers when safe, timestamps/session ids when non-sensitive, evidence digests, and claim impacts. Raw content is not review or fixture material.
   - Validator must fail raw private payload persistence, unredacted absolute private paths in packaged artifacts, sensitive snippets in red fixtures, private transcript text in packet summaries, child-agent prompt leakage, review evidence that requires opening raw private material, and leak summaries that omit category/severity/claim impact.
   - Add red fixtures for raw transcript in packet, raw message payload in fixture, absolute private path in package inventory, secret-like value in child prompt, private session log copied to artifact root, reviewer evidence requiring raw private file, and redaction summary without claim impact.

57. Enforce active setup-to-idle orchestration and thread-bound heartbeat law:
   - Ultragoal orchestration must distinguish active setup from idle continuation. Active setup must bind the goal, prepare the repo, write and validate macro-lane ExecPlans, create or verify branches, create or verify Codex app worktree threads, launch first-wave lane owners, install or verify thread-bound heartbeat automation, emit a transition receipt, and only then go idle.
   - A reminder, detached automation, stale thread, generic monitor, unbound heartbeat, or chat-only statement cannot substitute for the transition receipt. Wakeups must inspect current evidence cursors and emit `DONT_NOTIFY`, `STEER`, or `ESCALATE` with typed reason and next action.
   - Validator must fail setup-complete claims before transition receipt, heartbeat without target thread binding, automation without evidence cursor inspection, idle transition without first-wave launch proof, stale worktree thread accepted as current, wakeup result without typed outcome, and parent/lane status claims that skip reconciliation.
   - Add red fixtures for multi-lane setup complete without transition receipt, detached reminder substituted for heartbeat, missing thread id, missing first-wave launch receipt, automation tick without cursor inspection, stale lane thread, and wakeup that emits prose-only status.

58. Enforce connector capability discovery and same-surface capability authority:
   - Any claim that a connector, plugin, tool, app, Model Context Protocol server, browser, runtime, registry, or external capability exists or can be used must require a current `connector-capability-discovery` receipt bound to the candidate, claim id, account/workspace boundary when relevant, and exact authority surface.
   - Capability receipts must identify requested capability, discovered source, tool/plugin/app id, installed/enabled status, version or unavailable marker, auth/permission state without secrets, same-surface proof, fallback boundary, unsupported capability handling, checked timestamp, and affected claim ceiling.
   - Local docs, source files, installed cache, plugin manifest entries, skill descriptions, or adjacent connector availability cannot prove a live app/connector/tool capability. If the capability is unavailable, disabled, unauthenticated, wrong account/workspace, stale, or only reachable through fallback, every dependent claim must be withheld.
   - Validator must fail capability claims without discovery, stale discovery, disabled/uninstalled connector, wrong account/workspace boundary, auth/permission not proven, source/cache proof substituted for app capability, plugin-adjacent capability inferred, capability substitution, and fallback emitted as supported capability.
   - Add red fixtures for connector claim without discovery, stale discovery, disabled connector, wrong account/workspace, local docs used as live connector proof, missing auth boundary, cache/source substituted for app connector, fallback emitted as supported, and capability claim not linked to current candidate/claim id.

59. Enforce target-repo audit capability and target-scope support boundary:
   - Before claiming target repo fitness, setup readiness, retrofit safety, fresh-init support, audit completeness, Product Fitness applicability, package usability, or generic repo support, the plugin must prove `target-repo-audit-capability` for the exact target class, target surfaces, and candidate package surface.
   - Target capability receipts must name target mode, detected languages/build systems/runtime/test/doc/product surfaces, supported and unsupported surfaces, required adapters, missing commands, unsupported target ceiling, current target digest, and whether evidence came from source, installed package, cache package, registry/app proof, or review packet mode.
   - Fixture proof, source-only package proof, a successful install, or a generic README claim cannot prove live target support. Unsupported target classes must fail dependent claims instead of being silently treated as partial support.
   - Validator must fail target claims when classification is missing, unsupported target is marked fit, unknown language/runtime is accepted, required command/audit capability is absent, fixture/static package proof is used as live target evidence, target mode changes without receipt, product/runtime/app claims are emitted without detected surface, and broad "works for repos" claims lack target capability evidence.
   - Add red fixtures for unsupported repo marked fit, target classification missing, target command missing but success emitted, fixture used as live target, unknown language/runtime accepted, product surface not detected but product claim emitted, mode substitution, and generic repo-support overclaim.

60. Enforce trust-boundary abuse-path and failure-path coverage:
   - Any security/trust-boundary, authorization, permission, path containment, resolver, archive/package, install/cache, registry/app, model/tool authority, external input, dependency, network/file/shell, destructive action, secret/token, raw-private artifact, or sensitive-data change must include abuse-path and failure-path tests, fixtures, or receipts.
   - Coverage and review claims must prove malicious/malformed/denied/stale/forged/path-escape/fallback/failure cases, not only line coverage, happy-path tests, reviewer agreement, package audit pass, smoke tests, or source inspection.
   - Abuse/failure proof must be bound to the changed trust boundary, expected rejection mode, error category, claim ceiling, and receipt/fixture id. If the boundary cannot be exercised locally, the claim must be mechanically withheld rather than passed by prose.
   - Validator must fail trust-boundary changes with only happy-path coverage, missing malformed input coverage, missing permission-denied coverage, missing path traversal/escape coverage, missing forged digest/receipt coverage, missing unsafe authority/fallback test, missing secret/raw-private leak test, no exploit/failure path in reviewer finding, and Product Fitness/trust claims without burden/failure recovery proof.
   - Add red fixtures for trust-boundary change without abuse path, resolver bypass missing test, path traversal missing negative test, permission denied untested, forged digest accepted, stale registry proof accepted, secret leak untested, destructive-action ambiguity untested, and review signoff without failure-path evidence.

61. Enforce source-obligation parity and anti-bundling law:
   - Every id required by `validator/src/audit/source_obligations.rs` and every law-bearing row in `docs/source-obligation-matrix.json` must be represented by first-class, same-law-id enforcement across `docs/mandatory-law-surfaces.json`, `templates/agent-standards/enforcement.json`, `docs/foundational-law-traceability.json`, schemas/check enums, red fixtures, valid fixtures or receipts, and claim-ceiling guards.
   - A source-obligation law may be grouped under a parent row only when a typed parent/child relationship names the child id, child failure modes, child red fixtures, child receipt/evidence requirement, child claim ceiling, and proves the parent cannot pass while the child fails. Generic aliases such as `authority-source-binding`, `documentation-freshness`, `std-plans-001`, `plugin-product-cohesion-authority`, `schema-valid`, or a minimum valid fixture cannot silently satisfy a distinct law.
   - The current audit found source-obligation laws that are trace/audit visible but not sufficiently first-class in the central mandatory/standards surfaces unless repaired: `compact-agents-routed-standards`, `conditional-observability-proof`, `distinct-proof-surfaces-claim-ceilings`, `fresh-retrofit-repo-shape`, `generated-ready-completion-receipts`, `lane-worktree-isolation-cleanup`, `live-beneficial-e2e`, `product-cohesion-product-claims`, `purpose-backed-active-files`, `skill-local-reference-closure`, `source-card-freshness-ceiling`, `standards-gardener-promotion`, `typed-records-over-prose`, `worktree-lane-owner-cost-policy`, plus any `clean-checkout-command-discovery`, `memory-wiki-context-only`, or `restartable-execplans` parity drift found live.
   - Validator must fail any source-obligation row marked deterministic/covered when it lacks same-law central representation, independent child-law failure proof, red fixture coverage, valid fixture/receipt binding, or claim-ceiling impact. Validator must also fail drift between `source_obligations.rs`, `docs/source-obligation-matrix.json`, `docs/foundational-law-traceability.json`, `docs/mandatory-law-surfaces.json`, `templates/agent-standards/enforcement.json`, and receipt/check-id enums.
   - Add red fixtures for source-obligation law aliased to unrelated row, deterministic-covered row without mandatory-law surface, foundational trace row without same-law standards row, umbrella standards row passing while child law fails, minimum-goal valid fixture used for unrelated law, claim ceiling missing for source obligation, source-obligation list drift, and row marked covered with no independent red fixture.

62. Enforce human-audit disposition decomposition and judgment-only claim blocking:
   - Mandatory law compliance cannot be closed by `deterministic_with_human_audit`, "human audit", "manual audit", reviewer agreement, subjective signoff, or any hybrid label that lets a non-deterministic residue ride inside a deterministic status.
   - Every current `deterministic_with_human_audit` source-obligation row must be split into two typed surfaces: a deterministic enforceable law surface with validator/schema/red-fixture/receipt proof, and a judgment-only review surface with owner, scope, decision rubric, evidence inputs, disposition enum, same-candidate binding, and explicit claim ceiling. The deterministic surface must fail independently without the reviewer.
   - Judgment-only review may inform semantic, product, security, orchestration, or claim-ceiling decisions, but it cannot mark a law as fully enforced, cannot raise a claim above the deterministic evidence ceiling, cannot satisfy source-obligation parity by itself, and cannot be the only proof for package/readiness/release/full-compliance claims.
   - Validator must fail `deterministic_with_human_audit` dispositions in mandatory/source-obligation law rows, human-audit rows without deterministic split, judgment review without owner/rubric/evidence/disposition/candidate binding, reviewer signoff used as deterministic proof, and any full-compliance claim while a law still has unresolved judgment-only residue.
   - Add red fixtures for hybrid disposition accepted, human audit closing mandatory law, judgment review without deterministic sibling, reviewer signoff used as validator proof, semantic review raising claim ceiling without evidence, source-obligation row with `deterministic_with_human_audit`, and full-compliance claim with judgment-only residue.

63. Enforce capability-gap extraction and harness-capability promotion:
   - Any agent failure, blocker, repeated handback, stale retry loop, or incomplete run caused by missing tool access, missing context, missing runtime legibility, missing validator/schema/fixture/receipt, missing adapter, missing command, missing target surface, missing reviewer route, missing permission, missing doc index, missing install/cache/app surface, or underspecified environment must produce a typed capability-gap record.
   - Capability-gap records must include source artifact/session id, timestamp when available, affected workflow/law/claim, missing capability class, owner surface, blocked package surface, deterministic repair target, chosen promotion artifact, current claim ceiling, required evidence, and disposition.
   - Permitted promotion artifacts are validator checks, schemas, fixtures, receipts, skill/tool metadata, typed adapters, routed docs indexes, install/cache/app receipts, command manifests, or explicit claim-blocking non-goal records where no related claim remains supported. Prose-only notes, retry-hard instructions, user handback, reviewer agreement, generic "blocked", generic "unsupported", or lowering a claim ceiling without remediation are not terminal dispositions.
   - Validator must fail repeated missing-capability signals without promotion, capability-gap records without owner/repair/evidence/claim impact, stale or unowned capability gaps, capability gaps closed by prose, missing runtime/tool/doc-index/permission/adaptor gaps hidden by adjacent gates, and any full-compliance/readiness/review claim while an open capability gap affects the claimed surface.
   - Add red fixtures for missing capability with no record, repeated tool gap with no promotion, missing context fixed by prose only, missing runtime legibility while live-surface claim stays green, missing adapter but generic support claim emitted, permission/app capability unavailable but reviewer-ready claim emitted, user handback treated as repair, stale unowned capability gap, and claim-supported non-goal capability gap.

64. Enforce goal-contract amendment authority and closed required-claim-id mapping:
   - Every post-launch scope change, side-thread steering addition, parent instruction that adds/clarifies/strengthens/weakens/removes requirements, newly discovered law gap, checklist gate, validation obligation, and final-packet claim must be reconciled into the canonical contract bundle through a validated append-only `AMENDMENTS.jsonl` row. Markdown prompt files, checklist files, ExecPlan prose, packet text, side-thread summaries, and chat instructions are evidence inputs or generated projections only until the canonical bundle records them.
   - Contract bundle authority must bind `GOAL_CONTRACT.md`, `LANE_REGISTRY.json`, `VERIFICATION_BACKLOG.json`, `COMPLETION_MANIFEST.json`, lane ExecPlans, `AMENDMENTS.jsonl`, `RED_FIXTURES.json`, `agent-standards/enforcement.*`, mandatory law surfaces, source-obligation rows, generated receipts, and claim ceilings through current digests. `contract_bundle_hash`, `required_claim_ids_hash`, amendment-log digest, candidate version, and package/source/install/cache surface must be recomputed whenever scope or law surfaces change.
   - Every advertised claim, repair gate, validation result, packet statement, review-ready claim, release/readiness claim, and final response status must map to exactly one current `required_claim_ids` entry or an explicit non-required informational claim that cannot support completion. Omitted required claims, duplicate claim ids, stale zero hashes, broad umbrella claim ids, markdown-only gates, and final-packet claims not represented in `COMPLETION_MANIFEST.json` are hard failures.
   - Amendment rows must classify changes as strengthening, clarification, weakening, removal, supersession, or correction. Weakening/removal requires explicit user approval, verification-backlog and claim-ceiling updates, affected-claim blocking, and proof that no full-compliance/readiness/release claim depends on the removed requirement. A scope-changing "clarification" is a mislabeled strengthening or weakening and must fail.
   - Validator must fail side-thread additions not appended to `AMENDMENTS.jsonl`, checklist gates not represented in required claim ids, stale contract bundle or required-claim-id hashes after any gate/law change, markdown projection treated as canonical authority, final-packet claim outside the completion manifest, weakening without explicit approval, amendment-log non-append-only mutation, duplicate/omitted claim ids, and any `update_goal()` eligibility while contract/amendment/claim-id drift exists.
   - Add red fixtures for side-thread requirement without amendment, checklist gate absent from required claim ids, stale contract bundle hash after law addition, stale required-claim-id hash, markdown prompt treated as authority, final packet claim absent from completion manifest, weakening amendment without approval, scope-changing clarification, amendment log rewritten instead of appended, duplicate required claim id, broad umbrella claim id hiding child gates, and requirement removal without claim blocking.

65. Enforce forward-only state transition integrity and silent-reopen prevention:
   - Every law-bearing state surface must have append-only transition history, not only a current status string. This includes goals, lanes, ExecPlans, verification backlog rows, issue inventory rows, review findings, review rounds, approvals, completion manifests, readiness gates, validator receipts, Product Fitness reviews, install/cache/app-registry proof, automation ticks, packet claims, and package/release claims.
   - Transition events must record entity id, source state, destination state, actor, timestamp, trigger/source artifact, reason, evidence path, evidence digest, affected claim ids, revalidation obligations, previous transition hash, and resulting claim ceiling.
   - Approved, signed-off, verified, fixed, merged, archived, released, superseded, rejected, non-goal, or otherwise terminal states cannot silently re-enter queued, active, in-progress, retrying, needs-review, review-ready, supported, or claim-green states. Any reopen, regression, supersession, or rollback must be typed, append-only, provenance-bound, claim-blocking, and paired with fresh validation obligations before positive claims can resume.
   - Validators and claim gates must check transition history and provenance, not status strings alone. Markdown summaries, packet text, reviewer prose, receipt filenames, or current enum values may be projections, but they cannot overwrite transition history or close reopened work without evidence.
   - Validator must fail missing transition history, missing source/destination state, transition without actor/reason/evidence, terminal-to-active transition without typed reopen, status string contradicting transition history, approval reused after reopen, reopened claim counted as verified, automation tick silently resetting state, stale receipt reactivating claim support, and `update_goal()` while any reopened/regressed law-bearing item lacks fresh validation.
   - Add red fixtures for terminal row edited back to active, approval re-entering needs-review without reopen, status string says verified but transition missing, final packet changes blocked to fixed by prose, reviewer signoff reused after reopen, automation tick resets lane state, package/install receipt advances stale state, transition event missing previous hash, transition event missing actor/evidence, reopened gate still green, and `update_goal()` allowed after a reopened required claim remains unverified.

66. Enforce initiation-time Product Success Contract authority:
   - Product success, Product Fitness, Product Cohesion, release readiness, daily-driver readiness, user value, operator value, marketplace readiness, reviewer readiness, install usefulness, app-registry usefulness, or any product-impacting claim must be governed from Ultragoal initiation by a canonical Product Success Contract. Product success cannot be added as a review afterthought, markdown note, optional appendix, reviewer preference, or prose-only rubric.
   - Implement first-class schema/template/package surfaces for `schemas/product-success-contract.schema.json`, `templates/PRODUCT_SUCCESS_CONTRACT.md`, generated `PRODUCT_SUCCESS_CONTRACT.json`, `validation_artifacts/harness/product-success-contract-receipt.json`, package inventory/manifest entries, and validator/report surfaces. No substitute artifact name can satisfy this gate unless the schema, validator, manifest, package inventory, source/install/cache receipts, and review packet all name the replacement explicitly through an append-only contract amendment.
   - The Product Success Contract must be required by Ultragoal initiation, fresh-init, retrofit, fit-repo, `GOAL_CONTRACT`, lane registry, lane ExecPlans, verification backlog, completion manifest, review packet, review target, archive, source/install/cache validation, and claim-ceiling calculation for every product-impacting goal or lane.
   - Required typed fields are `product_success_contract_id`, `contract_version`, `goal_id`, `target_revision`, `claim_ids`, `product_surface_classification`, `target_user_or_operator`, `job_to_be_done`, `context_of_use`, `desired_user_outcome`, `business_or_mission_outcome`, `critical_journey_id`, `critical_journey_steps`, `first_value_event`, `quality_in_use_dimensions`, `accessibility_gate`, `cognitive_load_gate`, `recovery_burden_gate`, `trust_burden_gate`, `human_attention_policy`, `continuance_required`, `evidence_ladder_required_level`, `forbidden_substitutions`, `non_goal_product_claims`, `claim_ceiling`, `producer_actor_id`, `review_owner`, `generated_at`, and `receipt_digest`.
   - `quality_in_use_dimensions` must include effectiveness, efficiency, satisfaction, freedom from risk, and context coverage. Empty, placeholder, generic, `TBD`, wrong-goal, stale, unsigned, actorless, digestless, markdown-only, or optional fields are hard failures.
   - Validator must fail product-impacting initiation, planning, lane launch, review-packet generation, or completion when the Product Success Contract is absent, stale, unbound to the current goal/candidate/claim ids, missing required fields, digest-mismatched, actorless, optional, prose-only, or contradicted by downstream claims.
   - Add red fixtures for product goal without Product Success Contract, product lane without Product Success Contract, stale Product Success Contract digest, wrong goal id, wrong claim id, placeholder target user, missing quality dimension, missing critical journey, missing first value event, missing claim ceiling, markdown-only Product Success Contract, and package inventory missing Product Success Contract surfaces.

67. Enforce product success binding in goal contracts, lane launch, and ExecPlan lane authority:
   - Update `templates/GOAL_CONTRACT.md`, `templates/LANE_EXECPLAN.md`, `schemas/lane-registry.schema.json`, `schemas/completion-manifest.schema.json`, `schemas/contract-amendment.schema.json`, and related validators so product-impacting text, product classification, or product claims require Product Success Contract binding before any lane can launch, become active, become ready, merge, archive, or support completion.
   - Every product-impacting lane must declare `product_success_contract_id`, `product_success_claim_ids`, exact owed Product Cohesion gates, exact owed Product Fitness gates, product evidence plan, same-surface proof requirement, forbidden substitutions, current product claim ceiling, and blocking relation to `update_goal()`.
   - Explicit non-product lanes must carry a typed non-product rationale, affected claim ids, validator-checked claim ceiling, and contradiction check against lane text, goal text, completion claims, packet claims, manifest claims, and review language. A false non-product waiver is a hard failure.
   - Product-impacting work cannot be split across lanes in a way that lets one lane emit product claims while another lane owns the Product Success Contract proof. Shared claims must have closed claim ids and all lanes must block until the product success obligations are satisfied.
   - Validator must fail product lane launch without Product Success Contract id, lane ready without Product Fitness and Product Cohesion obligations, goal contract with product claims but no contract id, completion manifest product claim not bound to lane obligations, non-product waiver contradicted by text, lane consuming undeclared product claim, and lane passing while product evidence plan is absent.
   - Add red fixtures for product goal without Product Success Contract binding, product lane without contract id, product lane ready without Product Fitness owed, product lane ready without Product Cohesion owed, explicit non-product waiver contradicted by claim text, product claim consumed outside declared goal claim ids, lane ready receipt omitting product success obligations, and product evidence split across lanes without closed claim ids.

68. Enforce product success lineage, amendments, and closed product claim ids:
   - Every product claim in completion manifest, verification backlog, review packet, review target, archive, Product Fitness receipt, Product Cohesion receipt, ready receipt, final packet, report, README, plugin manifest, installed/cache receipt, and final response must trace to exactly one current Product Success Contract claim id or an append-only contract amendment row.
   - Late product claims introduced by side-thread steering, reviewer feedback, session logs, Chronicle findings, packet language, docs, README, manifest text, or validation output must create an append-only amendment row with product success ids, required claim ids, classification, evidence obligation, claim ceiling impact, source artifact, timestamp, actor, and digest before the claim can appear as supported.
   - Product Fitness and Product Cohesion receipts are invalid unless they bind to the Product Success Contract id, exact product claim ids, target user/operator, job to be done, context of use, critical journey, quality dimensions, proof surface, and current claim ceiling.
   - Freeform product success claims, orphan Product Fitness receipts, orphan Product Cohesion receipts, umbrella product-readiness claims, duplicate claim ids, stale amendments, weakening amendments without explicit approval, and product packet claims outside the completion manifest are hard failures.
   - Validator must fail late product-readiness claim without amendment, receipt with unknown Product Success Contract id, Product Fitness receipt missing contract lineage, Product Cohesion receipt missing critical journey lineage, packet claim not mapped to product claim id, amendment that weakens product obligation without approval, amendment that adds product claim without required fields/digests, and final response product claim outside completion manifest.
   - Add red fixtures for late product-readiness claim, unknown Product Success Contract id, stale Product Success Contract id in receipt, Product Fitness orphan receipt, Product Cohesion orphan receipt, product packet claim unmapped, product claim duplicated, broad umbrella product claim hiding child obligations, weakening amendment without approval, and side-thread product requirement not amended.

69. Enforce product proof joins and substitution blocking:
   - Product Cohesion proof must bind exact `critical_journey_id`, critical journey steps, interaction boundaries, product surface, user/operator role, and first value event from the Product Success Contract. Product Fitness proof must bind exact audience, job, context, desired outcome, quality-in-use dimensions, continuance requirement, evidence ladder, and same-surface proof requirement from the Product Success Contract.
   - Live product success, daily-driver, release, marketplace, reviewer-ready, active-registry, app-registry, app-enabled, install-useful, or user-value claims require same-surface proof at the Product Success Contract evidence ladder level. Package/static/source/install/cache proof may support only static enforcement/package claims unless paired with same-surface product proof.
   - Explicitly forbidden substitutions must be validator-enforced: install success, cache sync, package publication, smoke test, unit/integration/fixture pass, first use, reviewer agreement, review-packet creation, Product Cohesion alone, Product Fitness alone, happy path only, Quality Score/taste score alone, docs completeness, app registry existence without current reviewer exposure, and dogfood outside the declared target audience/context.
   - Product proof must include failure/recovery burden, cognitive load, accessibility, trust burden, human attention cost, and continuance when those dimensions are in the Product Success Contract. A product claim that lacks required negative-path or burden proof must be mechanically withheld.
   - Validator must fail all forbidden substitutions, product receipt proof-surface mismatch, evidence ladder downgrade, source/install/cache proof used for live product success, dogfood outside declared context, Product Fitness without Product Success Contract lineage, Product Cohesion without critical journey lineage, product proof missing burden dimensions, and product claim ceiling raised above proof.
   - Add red fixtures for install success used as product success, package publication used as product success, smoke test used as Product Fitness, reviewer approval used as product proof, Product Cohesion alone used as product success, Product Fitness alone used without contract lineage, happy path used without failure/recovery proof, Quality Score used as product outcome, app-registry existence used without reviewer exposure proof, and dogfood outside declared audience/context.
   - Add green fixtures for valid package/static enforcement claim, valid live same-surface product proof, valid withheld product claim with strict ceiling, valid Product Fitness joined to Product Success Contract, and valid Product Cohesion joined to the critical journey.

70. Enforce Product Success Contract review, packet inclusion, review-team ownership, and skill routing:
   - Product Success Contract must be included in review packet, detached review target, candidate archive, package inventory, manifest/source/install/cache receipts, and final packet for every product-impacting candidate. Review packets cannot be created, archived, or labeled reviewer-ready when product success lineage, proof joins, claim ceilings, review ownership, or same-surface proof are missing.
   - A Product/Simplicity reviewer or dedicated Product Success owner must review the Product Success Contract, Product Fitness obligations, Product Cohesion obligations, proof joins, substitution blocks, and product claim ceiling. The existing four-person review team may satisfy this only if Product/Simplicity is explicitly assigned as owner and the other personas have typed adjacency duties; otherwise the packet fails.
   - `fit-repo`, `agent-first-repo-init`, `agent-first-repo-retrofit`, `ultragoal`, `execplan-lane`, `product-cohesion-gate`, Product Fitness gate/report surfaces, `proof-gate`, `standards-gardener`, and `orchestrator-reconciler` must route Product Success Contract creation, validation, lineage, and claim-ceiling enforcement during initiation/planning. Product success cannot be routed only during final review or packet repair.
   - Package inventory, plugin manifest, source/install/cache package surfaces, schema catalog, fixtures, validators, and reports must include the Product Success Contract schema, template, examples, receipts, red fixtures, green fixtures, and validation commands.
   - Validator must fail review packet missing Product Success Contract, review packet missing product reviewer disposition, review target/archive omitting product contract surfaces, packet using stale product lineage, skill routing omitting Product Success Contract at initiation, fit-repo emitting product-ready without contract, Product/Simplicity review not assigned, and package inventory missing Product Success Contract components.
   - Add red fixtures for review packet missing Product Success Contract, product reviewer disposition missing, review packet includes product proof with stale lineage, review target omits product contract, archive omits product contract receipt, skill routing omits Product Success Contract, fit-repo emits product-ready without contract, existing review team lacks typed product owner, and package inventory omits product success schema/template/fixtures.

71. Enforce product-success inspiration-source provenance and disposition:
   - Every at-mentioned plugin, skill family, session log family, Chronicle summary, deep-research-v2 artifact, foundational article, and repo source used to shape Product Success requirements must be represented in a validated `product-success-inspiration-map` artifact and receipt. The map is mandatory evidence input, not background reading.
   - The inspiration map must include source id, source type, exact path or connector/app id, timestamp or version, access status, read status, source digest when local, extracted principle, adopted requirement ids, rejected/non-applicable rationale, claim ids affected, owner, and evidence path. Missing, inaccessible, stale, unread, or unverified sources must create a capability-gap record and block any claim that depends on the source.
   - The named sources in this contract are mandatory to disposition: Product Design, Creative Production, Sales, Compound Engineering, Superpowers, Template Creator, Chronicle summaries, session logs, deep-research-v2 structure/design/eval artifacts, Product Fitness docs, Product Cohesion docs, and the foundational articles. A generic "reviewed sources" statement is a hard failure.
   - Validator must fail missing source map, missing named source, source listed without exact path/id, source listed without extracted principle, adopted principle without requirement id, rejected source without rationale, stale local digest, Chronicle/session evidence without timestamp, plugin capability inferred from prose without installed/current proof, and final packet/product contract claims not mapped to inspiration evidence.
   - Add red fixtures for Product Design mentioned but not dispositioned, Template Creator mentioned but treated as repo dependency, deep-research-v2 cited without artifact path, Chronicle cited without timestamp, Sales value evidence adopted without evidence hierarchy, generic inspiration summary accepted, inaccessible source ignored, and adopted principle with no validator gate.

72. Enforce product strategy, positioning, research, and eval artifacts before lane planning:
   - Product-impacting Ultragoal initiation must generate or update first-class product strategy and product-success brief artifacts before lane planning begins. Required surfaces are `PRODUCT_STRATEGY.md`, `PRODUCT_SUCCESS_BRIEF.md`, `PRODUCT_POSITIONING.md`, `PRODUCT_RESEARCH_NOTES.md`, `PRODUCT_EVAL_PROTOCOL.md`, schemas/receipts for each, validator/report surfaces for each, and package inventory entries for each. No combined prose section or alternate filename can satisfy this gate unless the replacement is explicitly named through an append-only contract amendment and all validators/receipts/package surfaces use that amended name.
   - Strategy fields must include target problem, approach/guiding choice, primary user/operator, job to be done, 3-5 success metrics with measurement surfaces, 2-4 investment tracks, non-goals, and claim ceiling. Weak feature lists, vague users, vanity metrics, unmeasurable metrics, missing measurement location, or product goals dressed as problems are hard failures.
   - Positioning fields must include audience, use case or occasion, business or mission outcome, believable proof, product implication, assumptions, watch-outs, avoid list, and handoff path to Product Fitness/Product Cohesion/proof gates. Generic "premium", "better UX", "AI-powered", or value-prop prose without proof requirements is a hard failure.
   - Research and eval fields must include source map, observed evidence versus inference, currentness timestamp, open gaps, eval scenarios, red paths, green paths, negative paths, first-value path, time-to-value path, adoption loop, continuance signal, and claim ceiling. Existing deep-research-v2 precedent of research/design/eval artifacts before code is mandatory for product-impacting Ultragoal planning.
   - Validator must fail lane planning, lane launch, review packet generation, completion manifest generation, or Product Success Contract support when strategy/brief/positioning/research/eval artifacts are missing, stale, placeholder-filled, unbound to claim ids, lacking measurement surfaces, lacking eval scenarios, or contradicted by lane/packet claims.
   - Add red fixtures for product lane plan without strategy artifact, feature list accepted as strategy, generic audience accepted, vanity metric accepted, positioning route without proof needed, research notes without source map, eval protocol without negative paths, first-value path missing, continuance signal missing, and deep-research-v2 precedent cited but not converted into required planning artifacts.

73. Enforce template-generation governance and Template Creator boundary:
   - The repo/plugin must include validated, schema-backed templates for every required Product Success artifact created by this contract: Product Success Contract, Product Success Brief, Product Strategy, Product Positioning, Product Research Notes, Product Eval Protocol, Product Fitness receipt, Product Cohesion receipt, Product Evidence Matrix, Product Review Disposition, and Product Template Generation Receipt.
   - Template Creator must be dispositioned as an external operator aid. The parent must either use it and receipt that use, or record a validated non-use disposition. In both cases, it must not become a bundled repo/plugin runtime dependency, installed-plugin dependency, cache dependency, marketplace claim, or substitute for repo-owned templates. If Template Creator is used, a `product-template-generation-receipt` must record tool id/version, input artifacts, generated outputs, retained references, operator, timestamp, digest, manual edits, and claim ceiling. If not used, the same receipt must state non-use and prove the repo-owned templates were still created and validated.
   - Every product template must be deterministic, parseable, schema-bound, placeholder-free, no `TBD`/`TODO`/`fill in later`, no hidden optional required fields, line-cap compliant, namespace compliant, package-included, source/install/cache aligned, and covered by red/green fixtures plus render/parse round-trip tests.
   - Validator must fail missing required template, template not in package inventory, template absent from manifest/source/install/cache surfaces, placeholder in required field, schema mismatch, generated template without provenance receipt, Template Creator output treated as canonical without repo import, personal-skill cache mutation supporting package claim, and product artifact emitted from prose instead of template/schema path.
   - Add red fixtures for missing Product Success Brief template, Template Creator used with no receipt, Template Creator personal skill treated as repo artifact, product template with `TBD`, template field not in schema, package inventory omits template, installed/cache template drift, render/parse mismatch, and product artifact generated from untemplated prose.

74. Enforce value, adoption, continuance, and business/mission evidence hierarchy:
   - Product Success Contract and Product Fitness receipts must include a value/adoption model with explicit evidence hierarchy: `Known`, `Inferred`, `Assumed`, and `Missing`. Product claims must preserve this labeling through reports, review packets, manifests, README, final packets, and final responses.
   - Required value/adoption fields are value bucket, value logic, baseline, desired outcome, leading metric, lagging metric, measurement surface, first value event, time to value, adoption loop, repeat-use/continuance signal, daily-driver claim boundary, failure/recovery burden, human attention cost, assumptions, missing inputs, confidence label, and claim ceiling.
   - Allowed value buckets are enhanced productivity, cost reduction, risk reduction, revenue acceleration, time to market, mission effectiveness, and operator trust. Custom buckets require schema extension, validator update, red fixtures, and claim-ceiling review before use.
   - Public research, analogous wins, reviewer agreement, product intuition, package install, app visibility, or fixture success cannot become customer/user/product value proof. Missing baselines, missing measurement surfaces, unmeasured adoption, unmeasured continuance, and unsupported daily-driver claims must mechanically lower the claim ceiling or fail completion when those claims are required.
   - Validator must fail unlabeled value claims, unsupported return-on-investment math, missing baseline, missing measurement surface, missing first-value event, missing time-to-value path, missing continuance signal, daily-driver claim without repeat-use evidence, public-context value claim treated as product proof, and confidence raised above evidence.
   - Add red fixtures for unlabeled value claim, invented baseline, public research used as product value proof, install success used as adoption, one-time first use used as continuance, daily-driver claim without repeat use, value bucket outside enum, missing confidence label, and claim ceiling not lowered when value evidence is missing.

75. Enforce current product discovery, audit, and quality-in-use evidence before product-facing claims:
   - Product-facing claims require current evidence from the appropriate surface: captured flow screenshots or recordings for visible product journeys, source-backed research for user/customer pain, telemetry or usage receipts for adoption/continuance, accessibility and cognitive-load checks for quality-in-use, and typed reviewer dispositions for judgment-only findings. Memory, stale packets, prior screenshots, old sessions, plugin descriptions, or docs are context only until refreshed or explicitly bound as historical evidence.
   - Product audit evidence must name the product surface, flow/task, actor, capture tool, timestamp, accepted artifacts, rejected artifacts, step list, observed strengths, observed failures, accessibility risks, limits, and exact claim ids. Screenshots alone cannot prove accessibility compliance, continuance, adoption, daily-driver status, live registry exposure, or real-user product fitness.
   - Product research evidence must separate observed evidence from inference, rank severity/frequency/confidence, name source quality, preserve weak-source caveats, and bind recommended product moves to Product Success Contract claim ids. Complaint dumps, anecdote-only claims, or public-source-only claims cannot satisfy product fitness.
   - Validator must fail product-facing claims with stale discovery evidence, no capture timestamp, no flow/task id, screenshot-only accessibility compliance, memory-only research, current-source gap hidden by packet text, product audit not mapped to claim ids, recommendation not mapped to Product Success Contract, and final packet claiming product readiness without current product discovery evidence.
   - Add red fixtures for stale screenshot accepted, memory summary used as current product proof, product audit without flow id, accessibility compliance from screenshots alone, research brief without observed/inferred split, recommendation with no claim id, old packet used as live product proof, and Product Fitness passed with no current discovery evidence.

76. Enforce product-success lifecycle transitions and no-late-afterthought integration:
   - Product success must be a lifecycle spine across initiation, strategy, planning, lane launch, implementation, validation, review, packet generation, archive, install/cache/app proof, release/readiness, and post-use measurement. It cannot appear only at final packet, review round, validator repair, or after a user complains.
   - Every Product Success obligation must be decomposed into lane-owned tasks with claim ids, artifact ids, validator checks, fixtures, receipts, evidence surfaces, owner persona, transition state, and `update_goal()` blocking status before implementation starts. Product-critical debt created early in the repo must be restructured until it satisfies this lifecycle spine.
   - Any product-success change after lane launch must create an append-only amendment, reopen affected lifecycle transitions, invalidate stale Product Fitness/Product Cohesion/product-proof receipts, require fresh source/install/cache/package validation, and block positive product claims until the lifecycle is revalidated.
   - Product-success lifecycle receipts must prove no orphan obligations, no unowned product tasks, no product-critical debt parked in docs, no stale review-round assumptions, no packet-only product claims, no "minimum/enough" completion framing, and no unresolved product gaps under a positive claim ceiling.
   - Validator must fail Product Success Contract added after lane launch without amendment/reopen, Product Fitness added only at review time, Product Cohesion added only as packet repair, product-critical debt marked non-goal while claims depend on it, product obligations without lane tasks, stale product receipts after amendment, and `update_goal()` while any product lifecycle transition is unvalidated.
   - Add red fixtures for Product Success Contract after lane launch with no reopen, product obligation not decomposed into lane task, product-critical debt left in docs only, Product Fitness review afterthought accepted, Product Cohesion packet repair accepted as lifecycle proof, stale product receipt after amendment, and final packet product claim with lifecycle gap.

77. Enforce validator-theater and miswire resistance:
   - A validator, schema, standards row, receipt, or fixture cannot count as enforcement unless it proves real non-compliant behavior fails through the same authority path used for completion, review, package, readiness, and release claims. Shape checks, row presence, fixture-name matching, audit-path existence, happy-path parsing, and reviewer agreement are not enforcement.
   - Every law-bearing validator check must have a validator-theater receipt that names the law id, authority surface, parsed input type, trusted canonical inputs, exact failure predicate, claim-ceiling effect, command, cwd, input digests, output digest, and candidate version.
   - Every law-bearing validator check must include at least one minimal valid fixture, one realistic valid fixture when the law can appear in a full candidate, one minimal red mutant, one stale/digest mutant, one wrong-surface mutant, and one miswire mutant proving the check is not satisfied by fixture name, path, broad status string, or adjacent-law failure.
   - Validator checks must fail for the intended law-specific reason. A red fixture that fails only because some earlier generic schema/provenance check rejected the packet does not prove the target law unless the fixture is explicitly classified as a schema/provenance fixture.
   - Validator must fail law claims backed only by row shape, fixture catalog membership, missing actual behavior mutation, wrong check id, wrong fixture id, adjacent check failure, swallowed error, generic `validation_failed`, stale generated report, or unchecked validator exit.
   - Add red fixtures for row-shape-only mechanization, fixture-name-only pass, wrong-check pass, adjacent-law failure misattributed as target law, validator path present but never called, stale validator receipt accepted, red fixture fails for wrong reason, swallowed validator error, and completion claim with only validator-theater evidence.

78. Enforce green-path adequacy and satisfiable strictness:
   - Every mandatory law must prove both that non-compliance fails and that compliant behavior can pass. A law that only has red fixtures, only blocks claims, or has no realistic compliant path is not a product-quality gate; it is an unusable wall and must fail completion.
   - Every mandatory law must include a minimal green fixture and, where the law applies to real candidates, a full-candidate green fixture or receipt proving the law can be satisfied in the actual package/source/install/cache/review-packet context.
   - Green fixtures must exercise typed authority, current digests, package inventory inclusion, source/install/cache alignment where applicable, claim-ceiling calculation, and report/packet projection. A green fixture that proves only parser acceptance is insufficient.
   - Validator must fail laws that have red-only coverage, parser-only green fixtures, unrealistic toy greens for production law surfaces, green fixtures missing package/review/report projection, contradictory green and red fixture expectations, or law ceilings that can never become positive for supported claims.
   - Add red fixtures for law with no green path, law with only toy green fixture, green fixture missing claim-ceiling projection, green fixture missing package inclusion, green fixture accepted despite stale digest, and full-compliance claim while any mandatory law lacks a realistic green path.

79. Enforce clean-room rebuild and author-memory independence:
   - A fresh checkout, clean installed plugin copy, and clean cache package must regenerate all law-bearing authority artifacts from documented commands without private session memory, personal absolute proof paths, stale local state, unstated environment variables, hidden caches, or author-specific files.
   - Clean-room rebuild receipts must name the root, mode, command set, tool versions, environment variables used, excluded private state, generated artifacts, input/output digests, elapsed time, exit codes, and claim ceiling. The receipt must prove the rebuilt artifacts match or intentionally supersede the current candidate authority digests.
   - Clean-room proof must cover source workspace, installed package, cache package, review target, candidate archive, schema catalog, package inventory, product success surfaces, validator receipt, red fixture report, coverage receipt, and final packet or successor packet.
   - Validator must fail any claim that depends on author memory, private local proof paths, undocumented commands, untracked generated artifacts, stale caches, non-reproducible timestamps outside allowed receipt fields, manual post-processing, or source/install/cache divergence discovered by clean-room rebuild.
   - Add red fixtures for command missing from clean checkout, private path dependency, hidden cache dependency, generated artifact not reproducible, source-only rebuild used for installed/cache proof, stale artifact surviving rebuild, and clean-room receipt omitted from final packet.

80. Enforce historical regression corpus from session logs, Chronicle, reviewers, and side-thread signals:
   - Every session-log, Chronicle, reviewer, side-thread, or validation signal that exposed a repeated failure, weak enforcement, stale proof, overclaim, miswire, namespace violation, product-fitness substitution, source/install/cache drift, or missing law must become a frozen regression corpus row or a typed non-goal that blocks every related claim.
   - Regression corpus rows must include source artifact, timestamp/session id, quoted or summarized signal, affected law id, affected package surface, observed bad behavior, required repair, fixture ids, validator ids, receipt ids, claim ids, claim ceiling impact, implementation status, and evidence digest.
   - No issue is closed by being mentioned in the packet, checklist, final response, or review summary. It closes only when the regression corpus row points to deterministic enforcement, a passing green path, intended-failing red fixtures, regenerated receipts, and claim-ceiling projection.
   - Validator must fail when a known historical signal has no corpus row, no fixture, no validator, no receipt, no claim impact, stale source reference, session evidence without timestamp, duplicate signal hidden under broad umbrella row, or final packet claim that ignores an open regression row.
   - Add red fixtures for session-log finding not converted to regression, Chronicle finding without timestamp, reviewer repeat finding closed by prose, side-thread requirement not amended, historical namespace repeat offender without fixture, stale receipt repeat offender without fixture, and final packet omitting open regression claim impact.

81. Enforce cross-artifact consistency solver and authority graph closure:
   - The package must generate a machine-readable authority graph that joins laws, foundational sources, standards rows, source obligations, schemas, templates, validators, red fixtures, green fixtures, receipts, package inventory, source/install/cache artifacts, review target, archive, packet claims, required claim ids, Product Success Contract ids, and claim ceilings.
   - The authority graph must include node type, id, version, path, digest, producer command, owner, candidate version, proof surface, inbound edges, outbound edges, and claim-ceiling effect. It must be recomputed from canonical current inputs; hand-edited graph output cannot support claims.
   - Cross-artifact consistency must fail orphan claims, orphan laws, orphan fixtures, orphan receipts, referenced-missing artifacts, unreferenced law-bearing generated artifacts, duplicate authority, umbrella law hiding child law, stale digest, source/install/cache graph divergence, final packet claim outside graph, and graph edges pointing to private local state.
   - Validator must fail completion, review packet readiness, archive generation, package readiness, and `update_goal()` while authority graph closure has any error, warning classified as material, missing node class, stale digest, or unresolved orphan.
   - Add red fixtures for orphan claim, orphan red fixture, orphan green fixture, missing receipt edge, duplicate law authority, stale graph digest, private local graph edge, packet claim outside graph, source/install/cache graph mismatch, and child law hidden under umbrella parent.

82. Enforce authority exhaustiveness, closed enums, and impossible-state elimination:
   - All law-bearing statuses, claim ceilings, proof surfaces, target modes, receipt kinds, review dispositions, product evidence levels, package surfaces, validator outcomes, fixture outcomes, and transition states must be closed typed enums or discriminated unions. Freeform strings, nullable authority fields, partial catch-all maps, broad `Record` authority, and unknown variants are hard failures.
   - Every authority enum variant must have explicit validator handling, report projection, receipt projection, red fixture coverage where invalid, green fixture coverage where valid, and claim-ceiling behavior. Exhaustiveness must be enforced by code structure, tests, schema, or generated checks, not reviewer memory.
   - Unknown variants must fail closed unless the schema version explicitly carries a migration/supersession path that blocks related claims until upgraded. Silent acceptance of unknown status, unknown proof surface, unknown claim ceiling, or unknown receipt kind is forbidden.
   - Validator must fail nullable authority fields, unhandled enum variants, stringly typed status checks, unknown proof surface accepted, catch-all claim ceiling accepted, partial map authority, report projection missing enum variant, and final packet claim derived from untyped/freeform authority.
   - Add red fixtures for unknown claim ceiling, unknown proof surface, unknown target mode, nullable law status, missing review disposition variant, catch-all authority map, unhandled schema enum after version bump, and final packet generated from freeform authority text.

83. Enforce non-E2E claim ceiling and confidence bounds:
   - Until live end-to-end validation proves the real plugin journey on the intended surface, the package must carry an explicit non-E2E ceiling. Contract, receipts, reports, review packets, archive metadata, README, plugin manifests, and final responses must not imply product success, daily-driver readiness, marketplace readiness, release readiness, adoption, sustained value, live reviewer readiness, or external user success above that ceiling.
   - Required fields are `e2e_status`, `max_confidence_without_e2e`, `current_confidence_claim`, `confidence_basis`, `blocked_claim_classes`, `allowed_claim_classes`, `required_e2e_surfaces`, `last_same_surface_evidence`, `claim_ceiling`, and `receipt_digest`.
   - Without E2E proof, positive claims may cover only static/package/enforcement/reproducibility surfaces that are actually proven. Product-success and live-use claims must be withheld or explicitly labeled unproven, even if all non-E2E gates pass.
   - Validator must fail confidence raised above the non-E2E ceiling, final packet using "ready" or "successful" for unproven product/live surfaces, review packet omitting non-E2E ceiling, same-surface evidence older than allowed window, install/cache/static proof substituted for E2E, and `update_goal()` claiming product success without E2E proof.
   - Add red fixtures for 98% confidence without E2E, product success claimed from static proof, daily-driver claim without repeated-use proof, reviewer-ready claim without current reviewer exposure, marketplace-ready claim without marketplace proof, and final response omitting non-E2E ceiling.

84. Enforce adversarial packet tampering and forged-proof rejection:
   - Final packets, review targets, archives, validator receipts, red fixture reports, coverage receipts, Product Success receipts, Product Fitness receipts, source/install/cache receipts, and active-registry receipts must be tested against tampered variants before they can support claims.
   - Tamper tests must cover swapped package digest, stale receipt, wrong candidate version, missing Product Success Contract, forged active-registry proof, wrong installed path, wrong cache path, altered claim ceiling, deleted red fixture failure, changed reviewer disposition, changed generated-at timestamp beyond freshness policy, private path insertion, and packet claim added outside required claim ids.
   - Tamper fixtures must fail for precise, law-specific reasons. A forged packet rejected only by accidental JSON parse failure does not prove tamper resistance unless the tamper case is malformed JSON by design.
   - Validator must fail if tamper fixtures are absent, stale, fail for wrong reasons, fail only through unrelated schema checks, are not included in package inventory, are omitted from review-target/archive proof, or do not block claim ceilings.
   - Add red fixtures for swapped package digest, forged validator receipt, stale active-registry proof, wrong installed path, wrong cache path, altered claim ceiling, reviewer disposition flip, Product Success Contract removed, private proof path inserted, and final packet claim injected after validation.

85. Enforce runtime feasibility, cost, and strict-gate usability:
   - Strict enforcement must remain runnable as a product. Full no-cache audit, focused audit, fixture audit, coverage, source/install/cache comparison, packet generation, and clean-room rebuild must have documented commands, expected outputs, runtime budgets, concurrency bounds, cache invalidation rules, and failure behavior.
   - Runtime feasibility receipts must record command, cwd, start/end time, duration, cache mode, cache keys, invalidation inputs, concurrency, peak relevant resource measurement when available, exit code, output digest, and claim-ceiling impact.
   - A gate that only passes by relying on hidden stale caches, unbounded runtime, manual sampling, undisclosed skip lists, flaky retries, unavailable local services, or excessive repeated model/tool calls is non-compliant. Speed cannot be bought by weakening proof, and proof cannot be bought by making routine validation unusable.
   - Validator must fail hidden cache dependency, stale cache pass, no-cache command missing, runtime budget missing, unbounded concurrency, unbounded model/tool loop, skip list without typed exception, flaky check accepted, focused check substituted for full proof, and final packet omitting feasibility impact.
   - Add red fixtures for hidden cache pass, no-cache mismatch, unbounded concurrency, runtime over budget without exception, stale cache accepted, focused audit substituted for full audit, manual sample substituted for complete proof, and guardrail too slow without claim impact.

86. Enforce schema evolution, receipt migration, and stale-version invalidation:
   - Every schema, receipt, template, fixture catalog, package inventory, manifest, and validator authority version change must declare whether older artifacts migrate, supersede, or become invalid. Silent compatibility assumptions are forbidden.
   - Schema-evolution records must include changed schema id/version, affected receipt kinds, affected fixture kinds, migration command or invalidation rule, old digest, new digest, claim ids affected, source/install/cache impact, installed/cache/package refresh requirement, and claim-ceiling impact.
   - Older receipts cannot support current claims unless a validated migration receipt proves exact compatibility. Installed/cache/source artifacts with mismatched schema versions must fail or trigger a typed migration path before positive claims resume.
   - Validator must fail schema version drift, receipt version drift, fixture catalog version drift, missing migration rule, stale receipt accepted under new schema, installed/cache package using old schema without migration, and final packet omitting schema-evolution impact.
   - Add red fixtures for stale schema id, stale receipt version, fixture catalog mismatch, old Product Fitness receipt accepted after schema change, installed package old schema accepted, cache package old schema accepted, and migration receipt missing affected claim ids.

87. Enforce failure remediation quality and agent-actionable validator output:
   - Every validator failure, schema failure, fixture failure, packet failure, package inventory failure, source/install/cache mismatch, Product Success failure, and claim-ceiling failure must emit structured remediation data useful to a fresh agent without author memory.
   - Required failure fields are law id, check id, artifact path, parsed entity id, failed invariant, observed value, expected value or predicate, repair class, required evidence, exact rerun command, claim ids affected, claim ceiling impact, source/install/cache impact, severity, and whether the failure is deterministic or judgment-only.
   - Generic messages, swallowed errors, multiline prose without typed fields, wrong law id, missing affected artifact, missing rerun command, missing claim impact, and failures that require reading validator source to understand the repair are hard failures.
   - Validator must fail validator outputs that are not agent-remediating, schema errors without path/context, red fixture failures without expected/observed reason, report summaries that hide individual failures, and final packet summaries that omit unresolved failure remediation fields.
   - Add red fixtures for generic `validation_failed`, missing law id, missing artifact path, missing expected/observed values, missing rerun command, missing claim impact, wrong repair class, swallowed IO/schema error, and failure message leaking private data.

88. Enforce review disagreement, override, and judgment-boundary governance:
   - Reviewer, custom-agent, side-thread, or human judgment may discover issues and assign semantic risk, but it cannot override deterministic failure, raise a claim ceiling, mark mandatory law compliance complete, or replace validator/schema/fixture/receipt proof.
   - Every review disagreement or override attempt must become a typed disposition: converted to deterministic validator, converted to judgment-only claim blocker, rejected with evidence, accepted as non-goal with claim removal, or escalated to contract amendment. No freeform "resolved", "agreed", "n/a", "won't fix", or "reviewer approved" disposition can close a law-bearing issue.
   - Review disposition records must include reviewer id, persona, model/tool identity when available, issue id, affected law id, affected claim ids, decision, rationale, evidence paths, deterministic sibling check when required, claim ceiling impact, and transition history.
   - Validator must fail reviewer signoff over deterministic failure, human override without typed disposition, disagreement hidden in prose, reviewer approval raising claim ceiling, judgment-only issue counted as full compliance, accepted non-goal without claim removal, and final packet omitting unresolved review disagreement.
   - Add red fixtures for reviewer signoff overriding failing validator, human approval closing mandatory law, disagreement marked resolved in prose only, non-goal without claim removal, reviewer raising confidence above evidence, judgment-only Product Fitness claim counted as proof, and final packet hiding open disagreement.
