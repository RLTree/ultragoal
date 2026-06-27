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

## Gate 89: CLI Control Plane Authority And Non-Bypassable Harness Law Execution

This gate is additive to Gates 1-88. It does not replace, narrow, weaken, summarize, defer, or supersede any existing Harness Ultragoal law, validation requirement, update_goal() stop condition, packet requirement, Product Fitness requirement, source/install/cache/app-registry separation requirement, coverage requirement, typed-boundary requirement, line-cap requirement, standards requirement, foundational traceability requirement, or claim-ceiling requirement.

The Harness Ultragoal plugin must become a CLI-governed enforcement product. The CLI is not merely a convenience wrapper, audit runner, receipt generator, or optional validator. The CLI is the mandatory control plane and authority kernel for Harness Ultragoal law execution. Agents, reviewers, markdown files, TOML prompts, JSON fixtures, generated packets, checklists, receipts, reports, session summaries, Chronicle summaries, issue comments, and human-written status text may provide inputs to the CLI or projections from the CLI, but they may not grant authority, satisfy laws, mint evidence, raise claim ceilings, mark review readiness, or make update_goal() eligibility decisions without CLI verification.

The core rule is:

> A Harness Ultragoal claim is unsupported unless it is computed, minted, or verified by the installed Harness Ultragoal CLI from typed inputs, current source state, current package state, current proof surfaces, current law graph, current schema catalog, current standards rows, current source obligations, current fixtures, and current receipts.

Any path that allows an agent to satisfy a law by prose, row shape, checklist text, stale receipt reuse, copied evidence, generic reviewer approval, packet existence, install success, fixture count, smoke test, source-only proof, cache-only proof, package presence, or claim-ceiling apology is non-compliant.

### 89.1 Authority Model

The CLI must implement a typed authority model. Authority may not be inferred from freeform strings, file names, fixture names, markdown headings, receipt paths, reviewer names, or agent-written summaries.

The CLI must parse all external inputs into closed authority types before using them. Inputs that cannot be parsed into closed authority types must fail before validation logic runs.

The required authority types include, at minimum:

- `GateId`
- `LawId`
- `SourceObligationId`
- `FoundationalRequirementId`
- `StandardsRowId`
- `CheckId`
- `FixtureId`
- `FixtureKind`
- `ReceiptKind`
- `ReceiptSchemaVersion`
- `CandidateVersion`
- `CandidateDigest`
- `PackageDigest`
- `SourceDigest`
- `SchemaCatalogDigest`
- `LawGraphDigest`
- `StandardsDigest`
- `SourceObligationDigest`
- `FixtureCatalogDigest`
- `ProofSurface`
- `PackageSurface`
- `RuntimeSurface`
- `ClaimClass`
- `ClaimCeiling`
- `CapabilityId`
- `CapabilityAuthority`
- `CapabilitySurface`
- `ReviewRoundState`
- `ReviewerRole`
- `ProductDisposition`
- `ProductFitnessDisposition`
- `ProductCohesionDisposition`
- `ProductSuccessDisposition`
- `FailureDisposition`
- `FailurePromotionState`
- `ExecutionMode`
- `InstallMode`
- `TargetRepoMode`
- `RegistryMode`
- `PacketMode`
- `IssueLifecycleState`
- `UpdateGoalEligibilityState`

The CLI must reject:

- unknown enum values;
- unknown JSON keys;
- missing required fields;
- nullable authority fields;
- duplicate IDs;
- normalized ID collisions;
- case-only ID collisions;
- Unicode-normalization collisions;
- path traversal;
- absolute private local proof paths in package-owned inventory;
- stale schema versions;
- stale law graph digests;
- stale standards digests;
- stale source-obligation digests;
- stale fixture catalog digests;
- stale receipt schema versions;
- stale candidate versions;
- stale package digests;
- stale source digests;
- wrong proof surface;
- wrong package surface;
- wrong runtime surface;
- missing capability authority;
- unverified generated artifacts;
- hand-authored authority artifacts;
- receipts whose issuer cannot be verified;
- receipts whose input graph does not match the current repo;
- artifacts whose claims cannot be derived from current CLI-verified receipts.

No validator check may accept unparsed `serde_json::Value`, raw strings, raw paths, or untyped maps as authority after boundary parsing. Boundary modules may parse raw input, but they must return typed domain objects or typed failures. Downstream law execution must operate on typed authority objects only.

### 89.2 CLI As Sole Completion Authority

The following outcomes must be impossible to emit, satisfy, or mark complete without CLI authority:

- source audit pass;
- installed plugin audit pass;
- cache package audit pass;
- active registry proof;
- app-surface proof;
- plugin UI proof;
- marketplace proof;
- launcher runtime proof;
- reviewer exposure proof;
- review-target receipt;
- candidate archive receipt;
- final packet;
- claim ceiling;
- Product Fitness proof;
- Product Cohesion proof;
- Product Success proof;
- coverage proof;
- line-cap proof;
- typed-boundary proof;
- standards enforcement proof;
- foundational traceability proof;
- source-obligation parity proof;
- namespace/progressive-disclosure proof;
- package inventory proof;
- red fixture report;
- green fixture report;
- tamper fixture report;
- clean-room rebuild proof;
- agent-authored artifact provenance proof;
- source/install/cache digest comparison;
- package sync proof;
- issue lifecycle/update_goal eligibility proof;
- final completion decision.

Agents may run the CLI and repair failures. Agents may not reinterpret CLI failures. Agents may not manually mark a gate as complete after CLI failure. Agents may not claim that a CLI failure is "only a receipt issue," "only stale evidence," "only documentation," "not material," "reviewer acceptable," "blocked but okay," "claim-ceiling handled," or "safe to finish anyway" unless the CLI has parsed that disposition as a typed non-goal exclusion and blocked all related claims.

A reviewer may add judgment, lower claims, require repairs, or identify new failure modes. A reviewer may not override deterministic CLI failure or raise a claim ceiling above the CLI-computed ceiling.

### 89.3 Mandatory CLI Command Surface

The plugin must expose a stable CLI command surface. Command names may be refined during implementation, but equivalent authority operations must exist, be documented, be tested, be included in the installed plugin package, and be discoverable from a clean checkout.

The CLI must include commands equivalent to:

```text
ultragoal --version
ultragoal help
ultragoal doctor
ultragoal init
ultragoal retrofit
ultragoal classify-target
ultragoal law graph --strict
ultragoal law check --gate <gate-id>
ultragoal law check --all
ultragoal standards check --strict
ultragoal source-obligations check --strict
ultragoal foundational-trace check --strict
ultragoal namespace check --strict
ultragoal typed-boundaries check --strict
ultragoal line-caps check --strict
ultragoal coverage prove
ultragoal product init
ultragoal product prove-fitness
ultragoal product prove-cohesion
ultragoal product prove-success
ultragoal product check-claims
ultragoal capability discover
ultragoal capability prove
ultragoal source audit
ultragoal install audit
ultragoal cache audit
ultragoal registry probe
ultragoal app-surface probe
ultragoal target-repo audit
ultragoal package digest
ultragoal package inventory
ultragoal package verify
ultragoal review-target build
ultragoal review-target verify
ultragoal archive build
ultragoal archive verify
ultragoal review-round verify
ultragoal packet build
ultragoal packet verify
ultragoal receipts verify
ultragoal fixtures red
ultragoal fixtures green
ultragoal fixtures tamper
ultragoal fixtures all
ultragoal clean-room rebuild
ultragoal claim-ceiling compute
ultragoal explain <failure-id>
ultragoal failure capture
ultragoal failure promote
ultragoal issue check-lifecycle
ultragoal update-goal eligibility
```

Existing commands may remain only as compatibility wrappers. A compatibility wrapper must call the same typed authority kernel as the canonical command. A compatibility wrapper may not bypass parsing, law graph closure, receipt verification, fixture enforcement, claim-ceiling computation, or update_goal eligibility rules.

The CLI must expose machine-readable output for every command that affects evidence. Human-readable output may exist, but it may not be the authority format. Machine-readable output must be schema-versioned JSON, validated by the CLI before writing, and re-verifiable by the CLI after writing.

### 89.4 Canonical Law Graph

The CLI must build one canonical law graph for the candidate. This graph is the source of truth for whether Harness Ultragoal laws are connected to enforcement.

For every material law, the law graph must join:

- parent prompt gate ID;
- checklist item ID;
- source-obligation law ID;
- foundational article requirement ID;
- standards row ID;
- enforcement TSV row ID when applicable;
- enforcement JSON row ID when applicable;
- schema ID;
- schema version;
- validator check ID;
- validator module path;
- red fixture ID;
- green fixture ID when applicable;
- tamper fixture ID when applicable;
- receipt kind;
- receipt schema version;
- package inventory inclusion rule;
- claim classes blocked by failure;
- claim classes supported by success;
- proof surface required;
- same-surface restrictions;
- freshness requirements;
- candidate digest binding;
- install/cache/app/registry propagation requirements.

The CLI must fail law graph validation if any material law is:

- missing from the source-obligation matrix;
- missing from foundational traceability;
- missing from agent-standards enforcement rows;
- missing from schemas;
- missing from validator checks;
- missing from red fixtures;
- missing from green fixtures where a green path is possible;
- missing from tamper fixtures where artifact forgery is possible;
- missing from receipt requirements;
- missing from package inventory requirements;
- missing from claim-ceiling impact;
- represented only by prose;
- represented only by a row;
- represented only by a fixture name;
- represented only by a reviewer prompt;
- represented only by a checklist item;
- represented only by a claim-ceiling downgrade;
- represented only by a packet section;
- represented only by a non-actionable "blocker" note;
- represented only by historical evidence;
- represented only by a stale session summary;
- represented only by source proof when install/cache/app proof is required;
- represented only by install/cache proof when app/registry/reviewer proof is required;
- represented only by red fixtures with no valid green path;
- represented only by green fixtures with no red proof;
- represented by a check that is not actually executed by the strict audit command.

The law graph must include a digest. Every law receipt must bind to the law graph digest. If the law graph changes, prior law receipts become stale unless explicitly revalidated by the CLI.

### 89.5 Standards And Governing Repo Docs

CLI authority must be reflected in governing repo documents and package-controlled standards surfaces. The repo may not rely on informal agent memory or side-thread instructions to require CLI usage.

The following surfaces must be updated to require CLI-governed enforcement:

- `templates/agent-standards/enforcement.json`
- `templates/agent-standards/enforcement.tsv`
- `templates/agent-standards/enforcement-audit.tsv`
- `docs/source-obligation-matrix.md`
- foundational trace registry
- validator check registry
- red fixture catalog
- green fixture catalog
- tamper fixture catalog
- receipt schema catalog
- plugin manifest surfaces
- init templates
- retrofit templates
- ExecPlan templates
- review-round schemas
- review-round fixtures
- Product Fitness reviewer prompts
- Product Cohesion reviewer prompts
- Product Success reviewer prompts
- final packet schema
- review-target schema
- candidate archive schema
- update_goal eligibility schema if present
- package inventory schema if present

At least one explicit standards law must state:

> Harness Ultragoal completion, review readiness, package readiness, product readiness, release readiness, registry readiness, and update_goal eligibility must be computed or verified by the Harness Ultragoal CLI. Manual checklist updates, generated packet text, reviewer agreement, stale receipts, substituted proof, and agent-written summaries are not authority.

The standards law must fail closed. It must not be advisory, backlogged, blocked, documentation-only, reviewer-only, or claim-ceiling-only. It must have a standards row, source-obligation row, foundational trace entry, schema entry, validator check, red fixture, valid fixture, receipt requirement, and claim-ceiling guard.

### 89.6 Init And Retrofit Enforcement Hooks

The plugin must install or generate enforcement hooks during `init` and `retrofit` so downstream repos cannot treat CLI usage as optional.

The init and retrofit flows must create or verify a repo-local enforcement layout equivalent to:

```text
.harness/
  ultragoal.toml
  law-graph.lock.json
  schema-catalog.lock.json
  standards.lock.json
  source-obligations.lock.json
  fixture-catalog.lock.json
  receipts/
  reports/
  hooks/
  commands/
  product/
  coverage/
  packets/
  failures/
```

The CLI must install or generate command hooks equivalent to:

```text
.harness/commands/pre-completion
.harness/commands/pre-review-packet
.harness/commands/pre-update-goal
.harness/commands/pre-package
.harness/commands/pre-install
.harness/commands/pre-release
```

If the host ecosystem supports git hooks, task-runner hooks, CI hooks, plugin lifecycle hooks, or Codex/harness command hooks, the plugin must wire them to the CLI. If a hook surface is unavailable, the CLI must emit a typed hook-unavailable receipt and block only the claims that require that hook surface. It may not silently skip hook installation.

The required hook behavior:

- `pre-completion` runs strict law graph, strict standards, strict source obligations, strict audit, fixture reports, coverage, line caps, typed boundaries, Product Fitness/Cohesion/Success gates, and claim-ceiling computation.
- `pre-review-packet` refuses to build or verify a packet unless all packet claims derive from current CLI receipts.
- `pre-update-goal` refuses eligibility unless every mandatory stop condition has a current CLI receipt bound to the same candidate digest.
- `pre-package` refuses packaging if package inventory, private-path checks, bundled component graph, schemas, fixtures, receipts, and CLI binary inclusion fail.
- `pre-install` refuses installation if source compliance is incomplete or if installing would create source/install/cache divergence.
- `pre-release` refuses readiness or release claims unless same-surface proof exists for every released claim.

Generated hooks must be package-owned or repo-owned according to typed ownership metadata. The CLI must verify that hook files have not drifted from the installed plugin version unless the drift is represented by a typed local policy extension that cannot weaken mandatory laws.

### 89.7 Agent Standards Enforcement

The agent-standards surfaces must explicitly forbid agents from bypassing the CLI.

The standards must include fail-closed rules equivalent to:

1. Agents must use the Harness Ultragoal CLI for all Harness Ultragoal law claims.
2. Agents must not manually mark Harness Ultragoal checklist items complete without current CLI evidence.
3. Agents must not hand-author final packets, review targets, archives, claim ceilings, source/install/cache receipts, registry receipts, Product Fitness receipts, Product Cohesion receipts, Product Success receipts, coverage receipts, fixture reports, or update_goal eligibility records.
4. Agents must not treat reviewer agreement as a substitute for CLI law satisfaction.
5. Agents must not treat install success, package publication, smoke tests, first run, fixture pass counts, packet existence, source audit pass, or cache proof as substitutes for same-surface claims.
6. Agents must not downgrade a missing law to "not material" unless the CLI records a typed non-goal exclusion and blocks related claims.
7. Agents must not copy receipts across candidate versions, package digests, source digests, schema versions, law graph versions, or proof surfaces.
8. Agents must not claim current registry, app, plugin UI, marketplace, launcher, reviewer exposure, product usage, daily-driver, or release readiness without same-surface CLI proof.
9. Agents must promote newly observed material failure modes through the CLI failure-promotion path before completion.
10. Agents must treat CLI failure as authoritative until the code, schema, fixture, receipt, or proof surface is repaired and the CLI passes.

Each agent-standard rule must have a validator check and red fixture. A standards row that lacks CLI-backed enforcement must fail strict audit.

### 89.8 Receipt Authority And Anti-Fabrication

The CLI must be the receipt issuer and receipt verifier.

A receipt is valid only if it includes all required authority metadata:

- receipt kind;
- receipt schema version;
- CLI command name;
- CLI command argv;
- CLI binary digest;
- plugin version;
- source root;
- command cwd;
- execution mode;
- created-at timestamp;
- run ID;
- source digest;
- candidate version;
- candidate package digest;
- schema catalog digest;
- law graph digest;
- standards digest;
- source-obligation digest;
- fixture catalog digest;
- input artifact digests;
- output artifact digests;
- proof surface;
- package surface when applicable;
- runtime surface when applicable;
- capability authority when applicable;
- claim classes supported;
- claim classes blocked;
- stale-after rule;
- issuer identity;
- machine-readable result;
- human-readable explanation;
- failure IDs when failing.

Receipts must be rejected if:

- hand-authored;
- edited after minting;
- missing issuer data;
- missing command data;
- missing input digests;
- missing output digests;
- missing proof surface;
- missing candidate digest;
- missing law graph digest;
- missing schema catalog digest;
- stale by time;
- stale by source digest;
- stale by package digest;
- stale by law graph digest;
- stale by schema catalog digest;
- stale by standards digest;
- stale by source-obligation digest;
- stale by fixture catalog digest;
- produced by a different CLI binary without compatibility proof;
- produced by a different plugin version without compatibility proof;
- copied from source to install/cache/app surface;
- copied from install/cache to app/registry/reviewer surface;
- minted against a different target repo;
- minted against a different mode;
- minted against a different package version;
- referencing files outside allowed evidence roots;
- referencing private local paths in package-owned inventory;
- claiming support for a claim class not computed by the CLI.

A passing validator receipt is not sufficient unless the receipt itself passes receipt verification.

### 89.9 Packet Authority

The final packet must be built or verified by the CLI.

The CLI must reject a packet if:

- the packet was hand-authored without CLI packet provenance;
- any packet claim cannot be traced to a current CLI receipt;
- any packet evidence path is missing;
- any packet evidence path is stale;
- any packet evidence path points to a private local package-owned proof path;
- any packet claim exceeds the CLI-computed claim ceiling;
- any packet says reviewer-ready without same-surface reviewer exposure proof;
- any packet says registry-ready without same-surface registry proof;
- any packet says app-ready without same-surface app proof;
- any packet says release-ready without release-surface proof;
- any packet says Product Fitness passed without current Product Fitness proof;
- any packet says Product Success passed without current Product Success proof;
- any packet says source/install/cache synced without current digest comparison;
- any packet says all red fixtures pass without current red fixture report;
- any packet omits unsupported claims;
- any packet hides deterministic failures in prose;
- any packet presents blockers as acceptable completion.

The packet must include a CLI-generated claim ceiling. Agents may not hand-write the claim ceiling. Agents may add explanatory text only if the CLI verifies that the text does not raise the claim ceiling or imply unsupported claims.

### 89.10 Same-Surface Proof And Claim Separation

The CLI must encode proof surfaces as closed typed values. At minimum:

- `source`
- `package-archive`
- `installed-plugin`
- `versioned-cache`
- `app-registry`
- `plugin-ui`
- `marketplace`
- `launcher-runtime`
- `reviewer-exposure`
- `target-repo`
- `product-live-surface`
- `release-surface`

A proof from one surface must not satisfy another surface unless a typed same-surface equivalence rule exists. Equivalence rules must be explicit, schema-validated, red-fixtured, and claim-limited. No equivalence rule may allow disk install/cache proof to imply app registry, plugin UI, marketplace, launcher runtime, reviewer exposure, product live-surface, or release readiness.

The CLI must block these substitutions:

- source audit as install proof;
- source audit as cache proof;
- source audit as app-registry proof;
- source audit as reviewer exposure proof;
- installed-plugin audit as cache proof;
- cache audit as installed-plugin proof;
- install/cache proof as app-registry proof;
- install/cache proof as plugin UI proof;
- install/cache proof as marketplace proof;
- install/cache proof as launcher runtime proof;
- install/cache proof as reviewer exposure proof;
- registry proof as Product Fitness proof;
- reviewer exposure proof as Product Fitness proof;
- package publication proof as Product Success proof;
- smoke-test proof as product-readiness proof;
- fixture pass proof as user value proof.

If a live proof surface cannot be probed on the current machine, the CLI must emit a typed unsupported-surface result and block only the claims that require that surface. It may not replace unavailable live proof with adjacent proof.

### 89.11 Product Fitness, Product Cohesion, And Product Success Control Plane

The CLI must own product gates. Product claims must not be left to reviewer prose, template existence, report language, or packet text.

The product command group must enforce:

- Product Success contract creation at goal initiation or retrofit;
- Product Fitness proof tied to same candidate digest;
- Product Cohesion proof tied to same candidate digest;
- Product Success proof tied to user/job/value/release evidence;
- explicit product reviewer ownership;
- explicit Product Fitness falsifier disposition;
- explicit Product Cohesion disposition;
- explicit Product Success disposition;
- current source inspiration map;
- source-card freshness;
- product research/eval artifact disposition;
- strategy/positioning artifact disposition;
- template generation artifact disposition when relevant;
- claim class mapping for product-impacting claims.

The CLI must reject Product Fitness, Product Cohesion, or Product Success claims based on:

- documentation-only proof;
- install success;
- package publication;
- first run;
- first use;
- smoke test;
- fixture pass;
- source audit pass;
- cache audit pass;
- reviewer agreement;
- generic Product/Simplicity approval;
- Product Cohesion alone;
- Product Fitness alone when Product Success is claimed;
- happy-path demo alone;
- stale Product Fitness receipt;
- stale Product Cohesion receipt;
- stale Product Success receipt;
- proof from the wrong package digest;
- proof from the wrong source digest;
- proof from the wrong live surface;
- proof with no target user;
- proof with no job-to-be-done;
- proof with no first-value statement;
- proof with no daily-driver or release-surface evidence when those claims are made;
- proof with no falsifier disposition;
- proof with no claim-ceiling effect.

Product proofs must use typed schemas. Product proof schemas must reject unknown keys and unsupported claim classes. Product proof reports must be receipt-bound and package-included when the package makes product enforcement claims.

### 89.12 Coverage, Line Caps, Typed Boundaries, And Namespace Enforcement Through CLI

Coverage, line caps, typed boundaries, and namespace/progressive-disclosure laws must be CLI-governed.

The CLI must expose first-class commands for:

- coverage proof;
- line-cap proof;
- typed-boundary proof;
- namespace/progressive-disclosure proof.

These commands must not merely call external tools and trust output text. They must parse external output into typed receipts, bind receipts to source digest and candidate digest, and reject stale or malformed output.

Coverage proof must reject:

- test pass counts as substitute;
- smoke tests as substitute;
- fixture pass counts as substitute;
- reviewer approval as substitute;
- old coverage reports;
- coverage reports for the wrong source digest;
- coverage reports for the wrong package version;
- coverage reports with uncovered records;
- coverage reports not produced by the declared coverage command;
- coverage reports that omit repo-owned scope;
- coverage reports that exclude source files without typed exclusions.

Line-cap proof must reject:

- prose assertion that files are small enough;
- partial scans;
- missing generated-file classification;
- generated-file exceptions without typed justification;
- stale line-cap reports;
- line-cap reports not bound to source digest.

Typed-boundary proof must reject:

- ad hoc string validation at authority boundaries;
- raw JSON value use beyond boundary parser modules;
- unknown JSON keys;
- unchecked path strings;
- unchecked environment variables;
- unchecked CLI args;
- unchecked receipt fields;
- unchecked schema references;
- unchecked runtime capability names.

Namespace/progressive-disclosure proof must reject:

- generic utility buckets for law-bearing modules;
- umbrella names that hide authority boundaries;
- missing progressive-disclosure metadata for skills;
- law IDs buried only in prose;
- source-obligation rows without first-class law IDs;
- package surfaces whose namespace does not reveal authority role.

### 89.13 Red, Green, Tamper, Stale, Wrong-Surface, And Miswire Fixture Requirements

Every CLI-controlled law must include fixture coverage sufficient to prove both acceptance and rejection.

For each law, the fixture catalog must include or explicitly type why non-applicable:

- direct red fixture;
- minimal valid green fixture;
- realistic valid green fixture;
- stale receipt red fixture;
- wrong source digest red fixture;
- wrong package digest red fixture;
- wrong candidate version red fixture;
- wrong proof surface red fixture;
- wrong schema version red fixture;
- missing binding red fixture;
- substitute proof red fixture;
- hand-authored receipt red fixture;
- tampered receipt red fixture;
- validator miswire red fixture;
- orphaned law red fixture;
- row-shape-only red fixture;
- prose-only red fixture;
- reviewer-only red fixture.

A law with only red fixtures is not complete. A law with only green fixtures is not protective. A law with no tamper path for generated artifacts is gameable. A law with no miswire fixture is vulnerable to validator theater. A law with no stale fixture is vulnerable to receipt laundering.

The CLI must produce a fixture report that proves:

- every required red fixture fails for the intended reason;
- every required green fixture passes for the intended reason;
- tampered artifacts are rejected;
- stale artifacts are rejected;
- wrong-surface artifacts are rejected;
- wrong-digest artifacts are rejected;
- miswired checks are detected;
- fixture counts are current;
- fixture catalog digest matches the law graph;
- fixture reports are receipt-bound.

### 89.14 Failure Capture And Promotion

The CLI must provide a mandatory failure capture and promotion path.

Any newly observed material failure mode must become a typed failure record. Sources include:

- parent session logs;
- Chronicle summaries;
- side-thread audits;
- reviewer feedback;
- validator failures;
- stale receipt discoveries;
- stale red fixture discoveries;
- product proof substitutions;
- registry/app proof overclaims;
- source/install/cache drift;
- package inventory holes;
- packet correctness gaps;
- standards prose-only rows;
- foundational trace gaps;
- manually checked checklist items;
- attempted update_goal() before eligibility;
- agent attempts to finish after blockers;
- claims made from unsupported surfaces;
- external plugin/runtime capability mismatches;
- deep-research or source-card misses;
- target repo audit misses;
- trust-boundary abuse paths;
- malicious or accidental artifact tampering.

Each failure record must include:

- failure ID;
- source artifact path or source session ID;
- observed timestamp if available;
- law ID affected;
- gate ID affected;
- claim classes affected;
- affected proof surface;
- observed behavior;
- exploit path;
- required repair type;
- required validator check;
- required schema change if applicable;
- required red fixture;
- required green fixture if applicable;
- required tamper fixture if applicable;
- receipt impact;
- package impact;
- claim ceiling impact;
- disposition;
- promotion evidence path.

Allowed dispositions:

- `promoted_to_law`
- `promoted_to_validator`
- `promoted_to_schema`
- `promoted_to_fixture`
- `promoted_to_receipt_requirement`
- `promoted_to_claim_guard`
- `promoted_to_package_inventory_rule`
- `typed_non_goal_with_claims_blocked`

Disallowed dispositions:

- `ignored`
- `future`
- `backlog`
- `blocked_but_ok`
- `reviewer_accepted`
- `documented_only`
- `claim_ceiling_only`
- `not_material` without typed non-goal and claim blocking.

Completion must fail while any material failure record is unpromoted.

### 89.15 Update Goal Eligibility

The CLI must own update_goal() eligibility.

The parent session may not call update_goal() until `ultragoal update-goal eligibility` returns a passing receipt for the same candidate digest and source/install/cache/app-surface status required by the current claim ceiling.

The eligibility command must verify:

- all mandatory gates pass;
- all stop conditions pass;
- checklist state matches CLI receipts;
- all checked checklist items cite current evidence;
- all evidence is same-candidate;
- all evidence is same-digest where required;
- all red fixture reports pass;
- all green fixture reports pass;
- all tamper fixture reports pass;
- source audit passes;
- installed audit passes if installation occurred;
- cache audit passes if cache package exists;
- app/registry/reviewer claims have same-surface proof or are blocked;
- Product Fitness is current;
- Product Cohesion is current;
- Product Success is current where claimed;
- coverage is 100 percent for declared repo-owned scope;
- line caps pass;
- typed boundaries pass;
- standards rows are mechanized;
- foundational trace is complete;
- source obligations are first-class or typed parent/child enforced;
- package inventory has no private local proof paths;
- version bump and package sync are complete if required;
- final packet is CLI-built or CLI-verified;
- no unpromoted failure records remain;
- claim ceiling does not exceed proof.

If the eligibility command fails, parent session completion must fail. The final packet may describe the failure only as an unsupported claim state; it may not call the goal complete.

### 89.16 Clean Checkout And Installed Plugin Discovery

The CLI must be discoverable and runnable from a clean checkout and from the installed plugin package.

The plugin must prove:

- CLI binary/source is included in package inventory;
- CLI command discovery works from source checkout;
- CLI command discovery works from installed plugin;
- CLI command discovery works from versioned cache package;
- init/retrofit setup can locate the CLI;
- generated hooks call the correct CLI path or command alias;
- command paths do not depend on private local source paths;
- clean checkout bootstrap can install or build the CLI using documented commands;
- source/install/cache command versions agree;
- source/install/cache command digests agree where expected;
- version mismatch fails audit;
- missing CLI fails package readiness;
- missing CLI fails review readiness;
- missing CLI fails release readiness;
- missing CLI fails update_goal eligibility.

### 89.17 Config, Defaults, Environment, And Secret Boundaries

CLI configuration must be typed and fail closed.

The CLI must reject:

- unknown config keys;
- unsupported config versions;
- implicit defaults for authority-bearing settings;
- environment variables used as direct authority;
- secret values written into receipts;
- token values written into packets;
- private local paths written into package-owned manifests;
- host-specific paths in portable package inventory;
- unredacted command environment in evidence;
- config precedence ambiguity;
- conflicting config layers;
- config that weakens mandatory laws;
- local override files that weaken mandatory laws.

Allowed configuration layers must be explicitly ordered. The CLI must emit a config-resolution receipt showing which values came from package defaults, repo config, environment indirection, command-line flags, and generated lockfiles. Authority-bearing values must be lockfile-bound or receipt-bound.

### 89.18 Capability Discovery And Same-Surface Capability Authority

The CLI must own connector and capability discovery.

The CLI must distinguish:

- capability not installed;
- capability installed but not configured;
- capability configured but not authenticated;
- capability authenticated but not authorized for required action;
- capability authorized but not same-surface;
- capability same-surface but stale;
- capability same-surface and current.

A capability from one connector, plugin, app, registry, cache, or source package must not authorize claims for another surface.

The CLI must reject:

- source package proof as active connector capability;
- installed plugin proof as app capability;
- cache proof as registry capability;
- registry listing as reviewer exposure;
- reviewer packet existence as reviewer exposure;
- app plugin visibility as product usage;
- connector presence as same-surface proof;
- authenticated connector as authorized action proof;
- stale capability discovery as current proof.

### 89.19 Productized CLI User Experience Without Weakening Strictness

The CLI must be usable enough that strictness does not become an excuse for bypass.

Every failing command must produce:

- stable failure ID;
- law ID;
- gate ID;
- check ID;
- failing input path;
- expected authority shape;
- actual parsed result;
- claim classes blocked;
- minimal repair guidance;
- command to rerun;
- related fixture ID if applicable.

The CLI must include an `explain` command. `explain` must not weaken any law. It exists to make deterministic failure actionable.

The CLI must provide a fast focused mode for local repair and a full strict mode for completion. Focused mode may help repair but cannot satisfy final completion, review readiness, release readiness, or update_goal eligibility.

### 89.20 Required Validation Evidence For This Gate

This gate is not satisfied unless all of the following evidence exists and is current for the same candidate digest:

- CLI authority kernel tests pass.
- CLI command discovery proof exists for source.
- CLI command discovery proof exists for installed plugin if installed.
- CLI command discovery proof exists for cache package if cache package exists.
- Strict law graph receipt passes.
- Standards CLI authority row is present and mechanized.
- Source-obligation CLI authority row is present and mechanized.
- Foundational trace entry for CLI authority is present and mechanized.
- Schema catalog includes CLI authority schemas.
- Validator includes CLI authority checks.
- Red fixture report proves CLI bypass attempts fail.
- Green fixture report proves valid CLI-governed evidence passes.
- Tamper fixture report proves forged/edited/generated artifacts are rejected.
- Packet verification rejects hand-authored unsupported packets.
- Checklist verification rejects manually checked items without CLI evidence.
- Receipt verification rejects stale, copied, wrong-surface, wrong-digest, wrong-schema, and hand-authored receipts.
- Product proof commands reject all forbidden substitutes.
- Same-surface proof commands reject disk/cache proof for app/registry/reviewer claims.
- update_goal eligibility command fails when any mandatory gate lacks CLI evidence.
- update_goal eligibility command passes only when all mandatory gates and stop conditions pass.
- Final packet is CLI-built or CLI-verified.
- Claim ceiling is CLI-computed.
- No unpromoted material failure records remain.

### 89.21 CLI Self-Law Compliance And Self-Hosting

The CLI is not exempt from any Harness Ultragoal law. The CLI tool, validator source, schemas, receipts, fixtures, reports, package inventory, generated hooks, command wrappers, init/retrofit outputs, configuration, product surfaces, and documentation it ships must obey the same laws the CLI enforces against target repos and plugin packages.

The CLI may not act as a privileged root of trust that bypasses its own standards. The CLI may be the authority kernel only after it proves that the authority kernel itself is law-governed, typed, covered, line-capped, product-bound, receipt-bound, source/install/cache honest, package-included, clean-room reproducible, tamper-resistant, same-surface disciplined, and claim-ceiling constrained.

The CLI must include a self-law compliance path equivalent to:

```text
ultragoal self audit --strict
ultragoal self law-graph --strict
ultragoal self fixtures red
ultragoal self fixtures green
ultragoal self fixtures tamper
ultragoal self update-goal eligibility
```

Command names may be refined during implementation, but equivalent self-law authority operations must exist. A generic source audit is insufficient unless it explicitly covers CLI self-law scope and emits a typed self-law receipt.

Self-law scope must include, at minimum:

- CLI command parser and command dispatch;
- CLI boundary parsers for JSON, TOML, Markdown, schemas, receipts, fixture catalogs, environment variables, paths, command output, package inventory, and runtime capability data;
- validator modules;
- schema catalog;
- law graph builder;
- standards/source-obligation/foundational-trace join logic;
- receipt issuer;
- receipt verifier;
- packet builder;
- packet verifier;
- archive builder;
- archive verifier;
- review-target builder;
- review-target verifier;
- Product Fitness, Product Cohesion, and Product Success commands;
- source/install/cache/app/registry/reviewer proof commands;
- coverage command integration;
- line-cap command integration;
- namespace/progressive-disclosure command integration;
- typed-boundary command integration;
- init/retrofit command outputs;
- generated hooks;
- config resolution;
- secret redaction;
- capability discovery;
- failure capture;
- failure promotion;
- update_goal eligibility computation;
- focused repair mode;
- strict completion mode.

The CLI must prove its own compliance with every law it enforces, including:

- 100 percent coverage for declared repo-owned CLI/validator scope, with uncovered records empty;
- typed parsing at every CLI boundary;
- line-cap adherence for CLI/validator source;
- namespace/progressive-disclosure law for CLI/validator modules, commands, and packaged skills;
- Product Fitness, Product Cohesion, and Product Success law for the CLI as a product surface;
- source/install/cache/app-registry separation for CLI package claims;
- standards fail-closed behavior for CLI authority;
- foundational traceability for CLI authority;
- source-obligation parity for CLI authority;
- red, green, stale, wrong-surface, wrong-digest, tamper, substitute-proof, and miswire fixtures for CLI authority;
- generated/proof artifact provenance for CLI-generated artifacts;
- clean-room rebuild for CLI artifacts and receipts;
- config precedence, unknown-key rejection, environment indirection, and redaction for CLI config;
- trust-boundary abuse-path and failure-path coverage for CLI inputs, outputs, runtime calls, filesystem access, registry/app probes, secrets, and destructive operations;
- runtime feasibility, cost, strict-gate usability, and agent-actionable remediation output for CLI commands;
- schema evolution and stale-version invalidation for CLI schemas and receipts.

Bootstrap enforcement may exist only as a typed transitional state. A bootstrap validator, pre-self-hosted CLI, or compatibility wrapper may generate `bootstrap_untrusted` or `transition_only` receipts, but those receipts cannot support completion, package readiness, review readiness, product readiness, release readiness, registry readiness, app readiness, reviewer exposure, or update_goal eligibility. Final compliance requires a self-hosted CLI self-law receipt generated by the same candidate CLI and bound to the same candidate digest.

The CLI must reject every self-exemption path:

- CLI command claims outside the law graph;
- CLI source excluded from coverage without typed exception and claim blocking;
- CLI source excluded from line-cap checks without typed generated/mechanical exception;
- CLI boundary parser accepting raw authority strings after parse;
- CLI receipts accepted without self-law issuer verification;
- CLI-generated packets accepted without packet verifier self-law proof;
- CLI hooks accepted without hook self-law proof;
- CLI config accepted with unknown authority keys;
- CLI package inventory omitting CLI law-bearing artifacts;
- CLI fixtures proving target-repo behavior but not CLI self-behavior;
- CLI source audit used as substitute for installed/cache CLI proof;
- pre-self-hosted validator receipts used for final completion;
- final packet claiming CLI authority without current CLI self-law proof.

The CLI self-law gate must have governing documentation and enforcement, not only this prompt text. It must be represented in agent-standards enforcement rows, source-obligation rows, foundational trace entries, schema catalog, validator checks, red fixtures, green fixtures, tamper fixtures, receipt requirements, package inventory, claim-ceiling guards, validation evidence, final packet, and update_goal eligibility.

Add red fixtures for CLI self-exemption, CLI source excluded from coverage, CLI parser raw-string authority escape, CLI line-cap omission, CLI namespace omission, CLI receipt accepted without self-law proof, CLI package inventory missing CLI law artifact, bootstrap receipt used for completion, CLI source proof substituted for installed/cache CLI proof, CLI generated packet accepted without self-law proof, and update_goal eligibility passing without CLI self-law receipt.

### 89.22 CLI Performance, Latency, Speed, And Iteration Fitness

The CLI must be fast enough to be used constantly. A dictatorship-level control plane that is too slow for routine agent iteration is not a real control plane; it becomes a burden that agents will avoid, defer, summarize around, or replace with stale proof. Performance, latency, cache honesty, concurrency, and speed are therefore Harness Ultragoal laws, not polish.

The foundational trace for this law must explicitly include the "AI Is Forcing Us To Write Good Code" requirements for fast, ephemeral, concurrent dev environments, fast automated guardrails, short change-check-fix loops, cheap repeated test/check execution, high-concurrency isolated runs, cache-backed third-party calls with no-cache verification, one-command setup, and conflict-free concurrent environments. It must also bind those article requirements to standards rows, source-obligation rows, validator checks, schemas, red fixtures, green fixtures, receipts, package inventory, claim-ceiling guards, and final packet evidence.

The CLI must define hard performance budgets for every command class. Budgets may be made stricter by repo policy, but they may not be absent, advisory, prose-only, or hidden in documentation. A command without a typed budget cannot support completion, review readiness, package readiness, product readiness, release readiness, registry readiness, app readiness, or update_goal eligibility.

Required default budget classes:

- `instant`: `ultragoal --version`, `ultragoal help`, command discovery, schema version lookup, and static command metadata. Cold p95 must be <= 2 seconds. Warm p95 must be <= 500 milliseconds.
- `interactive`: `ultragoal explain`, single failure lookup, single receipt verification, single law status lookup, single claim-ceiling query, and single checklist evidence lookup. Cold p95 must be <= 5 seconds. Warm p95 must be <= 1 second.
- `focused`: one gate, one law, one fixture, one source-obligation row, one standards row, one product proof join, one typed-boundary class, or one package-inventory check. Cold p95 must be <= 15 seconds. Warm p95 must be <= 5 seconds.
- `repair_loop`: commands expected after a small code change, including focused audit, focused fixture run, focused receipt verification, focused law graph closure, focused package inventory check, and focused claim-ceiling recomputation. Cold p95 must be <= 30 seconds. Warm p95 must be <= 10 seconds.
- `strict_local`: full local strict audit excluding coverage, live external probes, and clean-room rebuild. p95 must be <= 60 seconds on the declared baseline machine and declared repository size class.
- `strict_fixtures`: full red, green, stale, wrong-surface, wrong-digest, tamper, substitute-proof, and miswire fixture suite excluding live external probes. p95 must be <= 120 seconds on the declared baseline machine and declared fixture count class.
- `strict_coverage`: full coverage proof. p95 must be budgeted, measured, and justified against repo-owned source size. It must not be unbounded, and it must not rely on coverage substitutes. If the repo cannot meet its declared strict coverage budget, product/readiness/update_goal claims are blocked until source structure, test structure, concurrency, or instrumentation is repaired.
- `strict_final`: full final proof, including source audit, red/green/tamper fixtures, coverage, package inventory, source/install/cache comparison when applicable, product proofs, packet verification, and claim-ceiling computation. p95 must be <= 10 minutes on the declared baseline machine unless a stricter repo policy applies. Any exception requires typed performance debt, claim blocking, and a standards-gardener repair path; it cannot support product readiness, routine usability, release readiness, or update_goal eligibility.
- `external_live`: registry, app, marketplace, launcher, reviewer-exposure, connector, network, or third-party live probes. Each live probe must have a typed timeout, retry/backoff policy, same-surface capability authority, offline fallback behavior, claim-blocking behavior, and redacted telemetry. External slowness may block only dependent live-surface claims, but it may not excuse local CLI slowness.

The CLI must emit performance receipts for every evidence-affecting command. Performance receipts must include:

- command name;
- command argv;
- command class;
- budget version;
- budget threshold;
- source digest;
- candidate digest;
- CLI binary digest;
- schema catalog digest;
- law graph digest;
- standards digest;
- fixture catalog digest;
- input size metrics;
- output size metrics;
- file count scanned;
- fixture count executed;
- receipt count read;
- receipt count written;
- cache mode;
- cache key;
- cache hits;
- cache misses;
- no-cache mode result when required;
- concurrency level;
- worker count;
- queue depth when applicable;
- start timestamp;
- end timestamp;
- wall-clock duration;
- CPU duration when available;
- peak memory when available;
- relevant IO bytes when available;
- external call count;
- external wait duration;
- timeout count;
- retry count;
- exit code;
- claim classes supported;
- claim classes blocked by performance failure.

The CLI must support fast iterative use and honest final proof at the same time:

- Focused and repair-loop commands may use verified caches only when cache keys include source digest, candidate digest, CLI binary digest, schema catalog digest, law graph digest, standards digest, source-obligation digest, fixture catalog digest, config digest, and command arguments.
- Final strict proof must include no-cache execution or cache-validation execution sufficient to prove no hidden stale cache dependency.
- A focused command may help repair a law but cannot satisfy final completion, review readiness, package readiness, product readiness, release readiness, registry readiness, app readiness, or update_goal eligibility.
- A cache hit may reduce runtime only when the CLI proves the cache entry is same-candidate, same-digest, same-schema, same-law-graph, same-fixture-catalog, same-config, and same-command.
- A cache miss may not silently downgrade proof; it must either compute fresh proof within budget or fail with a typed performance/availability claim impact.
- A live external timeout may not be replaced by stale local proof or source/install/cache proof.

The CLI must be scalable and bounded:

- Every law-bearing scan must declare its input size model and expected complexity class.
- Full strict commands must reject unbounded recursion, unbounded globbing, unbounded network calls, unbounded subprocess fan-out, unbounded model/tool loops, global locks that serialize independent work, and hidden shared mutable cache state.
- Concurrent execution must allocate ports, temp directories, cache namespaces, database names, log paths, receipt paths, and worker IDs without cross-talk.
- Performance baselines must be measured on a declared baseline machine and declared repository size class. Claims about speed must not be made without this baseline.
- Performance regressions must be detected against committed baselines. A regression beyond the typed tolerance blocks routine-usability, product-readiness, release-readiness, and update_goal claims.

The CLI must be product-fit as a tool:

- `ultragoal init` and `ultragoal retrofit` must provide one-command setup paths.
- Check-only init/retrofit must complete within the `interactive` or `focused` budget class.
- Local no-network init/retrofit must complete within the declared `repair_loop` budget unless package installation or compilation is explicitly required and separately budgeted.
- Manual multi-step setup, undocumented environment tinkering, hidden local state, and slow setup that causes agents to avoid fresh environments are product failures, not acceptable inconvenience.

The CLI must reject performance theater:

- no performance budget;
- budget declared only in prose;
- performance receipt missing;
- stale performance receipt;
- performance receipt from wrong candidate digest;
- performance receipt from wrong CLI binary digest;
- performance receipt from wrong schema/law/fixture digest;
- command over budget without claim blocking;
- focused check substituted for strict final proof;
- cached proof substituted for no-cache proof where no-cache is required;
- hidden stale cache pass;
- unbounded concurrency;
- serial global lock blocking independent work;
- network call in local-only command;
- missing timeout;
- missing retry/backoff policy for live probe;
- missing external-call telemetry;
- missing input-size telemetry;
- missing fixture-count telemetry;
- missing cache-key telemetry;
- missing performance regression baseline;
- final packet omitting CLI performance status.

The CLI performance law must have governing documentation and enforcement, not only this prompt text. It must be represented in agent-standards enforcement rows, source-obligation rows, foundational trace entries, schema catalog, validator checks, red fixtures, green fixtures, tamper/stale-cache fixtures where applicable, receipt requirements, package inventory, claim-ceiling guards, validation evidence, final packet, and update_goal eligibility.

Add red fixtures for missing command budget, prose-only budget, over-budget focused command accepted, over-budget strict command accepted, stale performance receipt accepted, wrong-candidate performance receipt accepted, wrong-CLI-digest performance receipt accepted, focused check substituted for final proof, cached proof substituted for required no-cache proof, hidden network call in local command, unbounded worker fan-out, global lock serialization, missing timeout on live probe, cache key missing law graph digest, performance regression ignored, init requiring manual setup, and final packet claiming routine usability without current CLI performance receipt.

### 90. Validator Source Namespace Topology And Semantic Repo-Law Enforcement

This gate is additive to Gates 1-89.22. It does not replace, narrow, weaken, summarize, defer, or supersede namespace/progressive-disclosure law, semantic domain-type naming law, line caps, typed parsing, coverage, CLI self-law compliance, package inventory closure, standards fail-closed behavior, foundational traceability, source-obligation parity, validator-theater resistance, green-path adequacy, source/install/cache/app-surface separation, review packet correctness, version bump, or any update_goal() stop condition.

The validator source tree is itself an agent-facing product surface. It is not exempt from the laws it enforces. A validator that accepts broad top-level `internal_*` and `internal_coverage_*` file clusters while claiming namespace and semantic repo-law enforcement is demonstrating validator theater: the law exists, but the law is not binding on the repo's own most important enforcement surface.

Current side-thread evidence that must be verified live before repair:

- `validator/src` currently has 155 top-level Rust files.
- `validator/src` currently has 105 top-level `internal_*.rs` files.
- `validator/src` currently has 16 top-level `internal_coverage*.rs` files.
- `validator/src/internal_test_modules.rs` currently routes many top-level internal test modules from one flat namespace.
- `plugin-manifest-draft.json` currently lists the top-level `validator/src/internal_*.rs` files as package resources.
- `docs/namespace-law-exceptions.json` currently contains `repeated-prefix-validator-src-internal`, with `directory = "validator/src"`, `prefix = "internal"`, `kind = "external_contract"`, and `applies_to = ["validator/src/internal*"]`.
- The broad exception reason says the flat package contract exposes related public surfaces by stable names and the plugin manifest is the routing owner. That is not acceptable for repo-owned validator source/test topology. A package manifest may route shipped public artifacts, but it cannot justify a broad hand-authored source namespace escape hatch.

Foundational article and repo-law basis:

- `docs/source-article-synthesis.md` records the foundational requirements that repository knowledge is the system of record, invariants should be mechanically enforced, strong tests, clear docs, scoped modules, static types, reproducible dev environments, and small well-scoped files are essential for agents, and the filesystem is an interface for agents.
- `templates/agent-standards/01-namespace-and-progressive-disclosure.md` states that directory structure and filenames must explain domain responsibility before a file is opened, repeated prefixes across more than two files usually mean a missing subdirectory with the prefix removed, and compatibility exceptions must name the external contract that makes a less-ideal name worth keeping.
- `docs/source-obligation-matrix.json`, `docs/source-obligation-matrix.md`, `docs/foundational-law-traceability.json`, `templates/agent-standards/enforcement.json`, `templates/agent-standards/enforcement.tsv`, and `templates/RED_FIXTURES.json` must all represent this as enforceable same-law authority. A row, trace entry, fixture catalog entry, or claim-ceiling sentence is not enough unless non-compliant validator source topology fails through the same authority path used for completion, review, package, readiness, release, and update_goal eligibility.

Required source topology repair:

- Move validator self-tests and internal test surfaces out of flat `validator/src/internal_*.rs` names and into semantically routed directories with the repeated prefix removed. Acceptable shape includes paths such as:
  - `validator/src/self_tests/coverage/...`
  - `validator/src/self_tests/claim/...`
  - `validator/src/self_tests/cli/...`
  - `validator/src/self_tests/review/...`
  - `validator/src/self_tests/schema/...`
  - `validator/src/self_tests/target_repo/...`
  - `validator/src/self_tests/package/...`
  - `validator/src/self_tests/audit/...`
  - `validator/src/self_tests/product/...`
  - `validator/src/self_tests/boundaries/...`
- Remove `internal_` from final validator test filenames where the directory already communicates test/internal scope.
- Remove coverage-wave history from final filenames where it is only evidence of how a file was created. Names such as `coverage_wave82c_tests.rs` or `internal_coverage_waveNN_tests.rs` must be replaced with domain-behavior names such as `receipt_authority.rs`, `schema_dispatch.rs`, `target_fixture_boundaries.rs`, or `semantic_receipt_boundaries.rs`.
- Split large test clusters by domain responsibility, not by chronological wave, coverage chase, or implementation history.
- Preserve line caps while moving modules. The repair must not create a new over-cap file or hide over-cap behavior behind generated/mechanical exceptions.
- Update Rust module routing (`mod.rs`, `#[path]`, or equivalent) so tests remain discoverable by domain. A single mega-router that merely reintroduces an opaque flat list is not enough.
- Update `plugin-manifest-draft.json`, `.codex-plugin/plugin.json` if applicable, package inventory surfaces, component graph, schema catalog, review target/archive surfaces, and source/install/cache package surfaces so moved files are listed exactly once and stale top-level paths disappear.
- Preserve or improve 100 percent coverage after the topology repair. A moved test file cannot become an excuse for coverage regression, fixture drift, or package inventory drift.

Required namespace-law enforcement changes:

- Delete the broad `repeated-prefix-validator-src-internal` exception from `docs/namespace-law-exceptions.json`.
- Forbid broad repo-owned source exceptions for `validator/src/internal*`, `validator/src/*_wave*`, `validator/src/*coverage*` where the prefix is standing in for a directory, and typo variants such as `iinternal_*`.
- Tighten namespace parsing so exceptions are typed authority, not raw string loopholes. Exception records must parse into closed kinds such as:
  - generated fixture/catalog exception;
  - public distribution surface exception;
  - external compatibility surface exception;
  - narrow source-layout exception.
- Narrow source-layout exceptions are allowed only when they name a specific file or narrow file family, name the external contract or generator that requires the layout, prove no semantic directory can express the responsibility better, and lower or preserve claim ceilings. They cannot apply to broad hand-authored source globs.
- Namespace validation must inspect actual repo-owned source files as well as package manifest resources. A missing manifest entry must not let source topology escape the law, and a manifest entry must not bless non-compliant source topology.
- Repeated-prefix validation must distinguish generated fixture catalogs from hand-authored source. Generated red fixtures may be catalog-routed when `templates/RED_FIXTURES.json` or another generator is the real route. Hand-authored validator source/test modules must route through semantic source directories.
- Namespace validation must fail when a directory has more than two hand-authored files sharing a non-semantic prefix and no typed narrow exception. Prefixes such as `internal`, `coverage`, `claim`, `review`, `schema`, `package`, `plugin`, `target`, `audit`, and `semantic` are allowed only when the directory is the corresponding semantic domain or when the repeated prefix is unavoidable and narrowly excepted.
- Namespace validation must reject path names whose first useful token is implementation history rather than domain responsibility: `internal`, `wave`, `coverage_wave`, `tmp`, `old`, `misc`, `helpers`, `utils`, `common`, `shared`, `lib`, `services`, and typo variants.
- Namespace validation must produce agent-remediating failures that include directory, offending prefix, count, representative paths, why the prefix indicates a missing directory, required repair class, affected claim classes, and whether a typed exception could ever be valid.

Required semantic domain-type naming changes:

- Treat file names, module names, test module names, schema file names, receipt names, fixture ids, check ids, and authority object names as semantic authority surfaces.
- Fail generic source/module names that encode storage status or implementation history instead of domain responsibility. Examples that must fail when used as authority surfaces include `internal_coverage_waveNN_tests`, `internal_claim_tests` when it should be `claim/identity.rs` or `claim/evidence.rs`, `internal_review_tests` when it should be `review/anchors.rs` or `review/materiality.rs`, and typo variants such as `iinternal_*`.
- Require semantic source modules to name the law, domain, authority, boundary, receipt, fixture, product surface, or workflow they govern. Domain directories must carry the repeated concept; filenames inside those directories must drop redundant prefixes.
- Typed exceptions for generated code, local generic algorithms, or external compatibility contracts must be narrow, parsed, package-included, claim-limited, and independently red-fixtured.

Required red, green, and tamper fixtures:

- Add red fixtures proving a top-level `validator/src/internal_coverage_wave99_tests.rs` style file fails namespace law.
- Add red fixtures proving top-level `validator/src/internal_claim_tests.rs`, `validator/src/internal_review_tests.rs`, and `validator/src/internal_schema_tests.rs` style clusters fail when more than two files share the prefix.
- Add red fixtures proving typo variants such as `validator/src/iinternal_coverage_tests.rs` fail.
- Add red fixtures proving a broad source exception for `validator/src/internal*` fails even when it names `plugin-manifest-draft.json` as a contract.
- Add red fixtures proving source exceptions with `applies_to = ["validator/src/*"]`, `["validator/src/internal*"]`, or equivalent broad globs fail.
- Add red fixtures proving a generated/catalog exception cannot be used for hand-authored validator source.
- Add red fixtures proving a package manifest listing cannot satisfy namespace compliance for a non-compliant source path.
- Add red fixtures proving a renamed file that keeps coverage-wave history but moves directories still fails semantic repo-law when the name remains non-semantic.
- Add red fixtures proving namespace errors cannot be hidden by capped reporting, row presence, foundational trace presence, source-obligation presence, coverage pass, line-cap pass, package inventory pass, Product Fitness pass, reviewer approval, or lowered claim ceiling.
- Add green fixtures proving semantically routed validator test directories pass with names such as `validator/src/self_tests/coverage/receipt_authority.rs` and `validator/src/self_tests/claim/evidence_boundaries.rs`.
- Add green fixtures proving generated fixture catalogs can still use repeated prefixes only when a generator/catalog route owns the family and the exception is narrow, typed, and claim-limited.
- Add tamper fixtures proving that widening a narrow namespace exception after receipt generation invalidates the receipt and blocks completion/review/package/readiness/release claims.

Required standards, trace, source-obligation, and claim-ceiling integration:

- Add or tighten agent-standards rows for validator source namespace topology and semantic repo-law enforcement. These may point to Gate 8 and Gate 24, but they must name this concrete source-topology law explicitly and must not rely on generic `namespace-progressive-disclosure` or `semantic-domain-type-naming` row presence alone.
- Add foundational trace entries mapping filesystem-as-agent-interface and scoped-module article requirements to validator source topology enforcement, the new red fixtures, valid fixtures, receipt requirements, package inventory, and claim ceilings.
- Add source-obligation parity entries so this law is not bundled under broad namespace, documentation freshness, architecture, line-cap, coverage, or semantic-domain rows without independent child-law failure proof.
- Add claim-ceiling guards so completion, review, package, readiness, release, product-readiness, CLI self-law, source audit, final packet, and update_goal eligibility all fail while validator source topology violates namespace or semantic repo-law.
- Add feedback-to-rule and historical regression corpus rows for the side-thread signal that found the `internal_*`/`internal_coverage_*` sprawl and the broad exception loophole. Include source artifact, timestamp/session id if available, observed failure, affected surfaces, implemented repair, red fixtures, valid fixtures, validator ids, receipt ids, claim ids, and claim impact.

Required validation:

- Before editing, verify current state live with commands equivalent to:
  - count top-level `validator/src/*.rs`;
  - count top-level `validator/src/internal_*.rs`;
  - count top-level `validator/src/internal_coverage*.rs`;
  - search for `validator/src/iinternal_*.rs`;
  - inspect `docs/namespace-law-exceptions.json` for broad validator source exceptions;
  - inspect `plugin-manifest-draft.json` for stale top-level validator internal paths.
- After repair, run and capture:
  - `cargo fmt --check`;
  - `cargo test --offline`;
  - namespace law proof through the CLI;
  - semantic domain-type naming proof through the CLI;
  - package inventory closure and exactly-once proof;
  - line-cap proof;
  - coverage proof with 100 percent and `uncovered_records = []`;
  - source audit with red fixture report;
  - red fixtures for the new namespace/semantic topology failures;
  - green fixtures for the routed source topology;
  - tamper fixture for widened namespace exception;
  - source/install/cache package evidence after source passes and only after source passes.
- Validation must prove that no top-level `validator/src/internal_*.rs`, `validator/src/internal_coverage*.rs`, `validator/src/iinternal_*.rs`, or equivalent prefix-as-directory source cluster remains accepted by the law.
- Validation must prove that moved files are package-included exactly once, no stale manifest entries remain, review target/archive/package inventory references are current, and source/install/cache digests align after final sync.

Required confidence calculation:

- The final packet and final response must include an explicit confidence value for this repair, but the value must be calculated from evidence rather than asserted. Use a scored model at minimum:
  - root cause observed directly: 25 points;
  - foundational/source-law alignment: 20 points;
  - direct enforcement path implemented: 20 points;
  - Rust/source topology refactor feasibility validated: 15 points;
  - red/green/tamper proof completeness: 15 points;
  - residual integration risk bounded: 5 points.
- The expected target confidence is at least 99 percent only if the broad exception is removed, source topology is physically repaired, recurrence fails mechanically, red/green/tamper fixtures pass, package inventory is current, coverage remains 100 percent, and source audit passes on the same candidate.
- A validator-only patch, a standards-row-only patch, a claim-ceiling-only patch, a packet-only patch, or a broad exception rewrite without physical source topology repair cannot claim 99 percent confidence. It must report lower confidence and block completion.
- The rationale must explain why the chosen solution was selected over alternatives:
  - Chosen solution: physical source topology repair plus typed exception tightening plus red/green/tamper enforcement.
  - Rejected alternative: keep flat files and add more prose or rows, because that preserves the agent-facing smell.
  - Rejected alternative: keep broad exceptions and rely on package manifest routing, because package inventory cannot justify hand-authored source namespace drift.
  - Rejected alternative: validator-only prefix check without moving files, because the repo would still fail its own product/namespace laws.
  - Rejected alternative: coverage-only or line-cap-only repair, because passing those gates does not make filesystem topology semantic.

### 91. Rust Developer Experience, Runtime Resource Discipline, And Workspace Garbage Collection

This gate is additive to Gates 1-90. It does not replace, narrow, weaken, summarize, defer, or supersede CLI authority, CLI self-law compliance, CLI performance, namespace law, typed-boundary law, coverage law, line-cap law, source/install/cache/app-surface separation, clean-checkout command discovery, one-command bootstrap, package inventory, Product Fitness, Product Cohesion, Product Success, source-obligation parity, foundational traceability, red/green/tamper fixture proof, version bump, final packet correctness, or any update_goal() stop condition.

Harness Ultragoal's Rust developer experience is a governed product surface. It is not "developer preference." A slow, ad hoc, locally magical Rust loop is a product failure and a Harness Ultragoal law failure. The Rust toolchain, Cargo, nextest, llvm-cov, Clippy, rustfmt, dependency/security tools, profiling tools, caches, watchers, linkers, test fixtures, runtime resource management, and cleanup flows may produce observations. They are not claim authority.

The governing doctrine is mandatory:

- Raw tools may produce observations.
- Only the `ultragoal` CLI may convert observations into typed, digest-bound, same-surface receipts and claim ceilings.
- Cargo is the canonical Rust substrate, but Cargo is not claim authority.
- Accelerators are governed infrastructure, not optional suggestions. If a tool materially improves speed, correctness, repeatability, supply-chain safety, memory/resource discipline, cleanup safety, or professional Rust workflow quality, Harness Ultragoal must either adopt it in a declared class or explicitly reject it with evidence.
- Hidden local state, warm caches, watcher state, editor diagnostics, global Cargo configuration, local aliases, untracked scripts, stale target dirs, unbounded logs, abandoned worktrees, partial receipts, and cleanup done outside the CLI cannot support claims.
- Rust has no built-in tracing garbage collector. Harness Ultragoal must use Rust-native ownership, RAII, bounded resources, explicit cleanup, leak detection, and governed cleanup receipts rather than treating "Rust" as automatic memory/resource correctness.

Adopt this Rust governance boundary:

- `rust-toolchain.toml`, `Cargo.lock`, Cargo workspace metadata, `cargo check`, `cargo build`, `cargo fmt`, `cargo clippy`, `cargo test` for doctests/compatibility, `cargo nextest` for standard/release normal test execution, `cargo llvm-cov`, Cargo profiles, feature strategy, and MSRV policy are required Rust substrate surfaces.
- `cargo metadata --format-version=1 --locked`, `cargo tree --workspace --duplicates`, `rustup show active-toolchain`, `rustc --version --verbose`, `cargo --version --verbose`, and installed component inventory are required toolchain/workspace observation inputs.
- `cargo nextest` is required for standard and release test execution after bootstrap. If missing, the CLI must emit a missing-tool failure with bootstrap repair instructions, not silently downgrade to weaker proof. Doctests still require Cargo.
- `cargo llvm-cov` is the canonical Rust coverage proof path for this repo's declared Rust scope. Exact 100 percent coverage with `uncovered_records = []`, candidate digest binding, source digest binding, toolchain binding, and anti-gaming checks remain mandatory.
- `rustfmt` and `clippy` are required. Clippy failures must include lint code, span, suggestion when machine-applicable, affected claim classes, and rerun command.
- `cargo-deny` and `cargo-audit` are required supply-chain/security gates for standard/release proof. `cargo-vet` is required for release/supply-chain maturity when release readiness or external distribution is claimed; it is not required for every fast local repair loop.
- CycloneDX SBOM generation, checksums, signing, and release provenance are required for official release/distribution claims. `cargo-dist` is required only when Harness Ultragoal claims binary release/distribution readiness; it is not source compliance, review readiness, or product success proof by itself.
- Secret scanning with a governed primary scanner such as Gitleaks is required for release/package readiness. Secondary deep scanners such as TruffleHog are governed adapters.
- `serde`, `schemars`, `jsonschema`, `serde_path_to_error`, `clap` derive/value enums, `camino`, `thiserror`, `miette`, and `tracing` are required typed-boundary/diagnostic/observability substrates for the Rust control plane unless a narrower same-law replacement is explicitly adopted and proven.
- `anyhow` is allowed only at binary/application edges. Law-core APIs must expose typed error enums and parse results, not opaque catch-all errors.
- `proptest`, `cargo-fuzz`/libFuzzer, `trybuild`, `insta`, `snapbox` or equivalent CLI snapshot harness, `assert_cmd`, and `tempfile` are required for parser, path, fixture, CLI, compile-fail, diagnostic, and temp-resource proof where the corresponding code surface exists.
- Criterion and Hyperfine are required for performance proof surfaces: Criterion for critical library paths and Hyperfine for CLI command budget proof. Flamegraph, Samply, Instruments, Heaptrack, Valgrind, sanitizers, Miri, Divan, and iai-callgrind are governed adapters for diagnosis or scheduled scoped proof, not universal claim authority.
- `sccache`, Cargo incremental compilation, linker acceleration, Cargo target dirs, nextest recordings, rust-analyzer target dirs, Cargo registry/git caches, Docker/CI caches, plugin caches, install caches, and remote caches are legal only when declared in cache/no-cache receipts. Cache use is legal. Cache concealment is illegal. Warm-cache proof may never support cold/no-cache claims.
- `sccache` is default-on acceleration when available and declared; absence must be reported as acceleration unavailable, not as correctness failure. Remote cache is forbidden unless explicitly declared with endpoint identity redaction, cache key, policy, and claim limitation.
- `lld` is default-on where platform policy permits. `mold` is a governed adapter because it is valuable but platform-dependent. Linker acceleration supports timing/iteration claims only, never correctness/product/release claims by itself.
- `cargo-binstall` is default-on for bootstrap speed only when version/digest policy is satisfied; fallback must be `cargo install --locked`. Tool installation receipts support tool/bootstrap claims only.
- `watchexec` is the default low-level watch substrate for `ultragoal rust watch`; Bacon is default-on or governed adapter for high-quality human/agent Rust feedback if bootstrap can install it safely. Watchers emit feedback events, not completion receipts. `cargo-watch` must not be standardized; if present, it is local-only or rejected for claim support.
- `rust-analyzer` is default-on for editor diagnostics and navigation, but editor state, test lenses, and check-on-save are optional local observations only. They cannot support claims.
- `cargo xtask` is required for bootstrap/control tasks before installed `ultragoal` exists. `just` may exist only as a governed alias layer that delegates to `ultragoal` or `xtask`; it is not authority. Makefiles and unwrapped shell scripts are rejected as authority.
- Nix, mise, devcontainers, cross, cargo-zigbuild, cargo-release, cargo-public-api, cargo-semver-checks, cargo-msrv, cargo-udeps, cargo-geiger, cargo-machete, cargo-outdated, cargo-chef, grcov, and alternate profilers are governed adapters unless this repo declares a stricter same-law requirement. They may not support claims without declared config, version/tool identity, output digests, and claim limits.
- Raw `cargo`/tool output as final proof is rejected. Raw tool commands may be used for investigation only. Final claims must route through `ultragoal` wrappers and receipts.

Required Rust command classes:

- FAST loop: `ultragoal rust fast`.
  - Goal: fastest meaningful local feedback for frequent agent iteration.
  - Required internal checks: quick toolchain verification, changed-scope workspace topology, changed-scope namespace law, changed-scope line-cap law, changed-scope typed-boundary law, `cargo fmt --all -- --check`, `cargo check --workspace --all-targets --locked --message-format=json`, and focused `cargo nextest` profile when nextest is installed/bootstrap-proven.
  - Allowed acceleration: declared sccache, incremental compilation, platform-safe linker acceleration, and watch-mode routing.
  - Claim support: fast feedback and changed-source structural observations only.
  - Claim exclusions: review ready, release ready, Product Fitness, Product Cohesion, Product Success, package ready, install verified, cache coherent, final packet, and update_goal eligibility.
- STANDARD loop: `ultragoal rust standard`.
  - Goal: serious local proof before claiming a repair.
  - Required internal checks: toolchain receipt, cargo metadata receipt, workspace topology receipt, namespace receipt, line-cap receipt, typed-boundary receipt, `cargo fmt`, `cargo clippy --workspace --all-targets --all-features --locked --message-format=json -- -D warnings`, `cargo check --workspace --all-targets --all-features --locked --message-format=json`, `cargo nextest list`, `cargo nextest run --profile standard`, doctests, required red/green/tamper fixtures for touched validators, current receipt verification, and claim ceiling computation.
  - Claim support: source compliance, typed-boundary compliance, standard test verification, fixture triad verification, and repair-claim candidate only.
  - Claim exclusions: coverage complete, package ready, install verified, runtime product success, release ready, final packet, and update_goal eligibility unless other gates independently pass.
- RELEASE loop: `ultragoal rust release`.
  - Goal: full law proof for the claim requested.
  - Required internal checks: standard loop, exact coverage proof, dependency/security proof, supply-chain proof, feature-matrix proof, performance budget proof, memory/resource proof, package inventory proof, clean install proof, cache separation proof, Product Fitness/Cohesion/Success proofs only when the requested claim requires those product surfaces, GC dry-run proof, current receipt verification, claim ceiling computation for the requested claim, and CLI-built packet when packet claim is requested.
  - Claim support: release, product, final packet, and update_goal claims only when every applicable same-surface receipt is current and same-candidate.
  - Product Success is not a default substitute for every release loop; it is mandatory when a product success, daily-driver, user outcome, product readiness, external-user, marketplace, or release claim depends on product outcome.
- COLD/CLEAN loop: `ultragoal rust clean-proof`.
  - Goal: prove clean-checkout and no hidden local cache dependency.
  - Required behavior: fresh checkout or clean worktree snapshot, isolated `CARGO_HOME`, isolated `CARGO_TARGET_DIR`, `RUSTC_WRAPPER` unset unless explicitly testing an accelerator, `CARGO_INCREMENTAL=0`, no editor/watcher state, no global Cargo config unless inventoried, locked fetch or offline cache verification, standard loop, and optional release subset.
  - Claim support: clean-checkout onboarding, no-hidden-local-magic, and no-cache proof.
- WATCH loop: `ultragoal rust watch`.
  - Goal: continuous feedback.
  - Required behavior: invoke the declared watcher and route to `ultragoal rust fast`.
  - Claim support: watch observations only. Completion/review/release/update_goal claims require subsequent receipt verification and claim ceiling computation.
- MEMORY/RESOURCE loop: `ultragoal rust memory prove`.
  - Goal: bounded memory, bounded queues, bounded file descriptors, cleanup on cancellation/error/panic, no leaked temp resources, child processes reaped, and long-running stability.
  - Required behavior: static resource audit, async-task audit, resource-lifecycle tests, memory-budget performance proof, selected leak checks, long-running scenario with telemetry where applicable, and receipt verification.
- GARBAGE-COLLECTION/CLEANUP loop: `ultragoal gc plan`, `ultragoal gc dry-run`, `ultragoal gc apply`, `ultragoal gc verify`.
  - Goal: identify and remove stale workspace artifacts safely with protected-artifact rules, deletion receipts, and active-claim preservation.
  - Required behavior: classify every artifact, compute protected set, compute stale set, produce deletion plan, dry-run plan, apply only with plan digest, emit deletion receipt, verify protected artifacts remain, verify active claims still have supporting receipts, and verify workspace locks/pids/ports/tempdirs are clean.
  - Deletion without a receipt is artifact destruction, not cleanup.

Required Rust receipt models:

- Every Rust loop receipt must include schema version, receipt id, law ids, command argv, working directory digest, toolchain identity, `rust-toolchain.toml` digest, `Cargo.lock` digest, `Cargo.toml`/workspace metadata digest, source/candidate digest, law graph digest, schema catalog digest, fixture catalog digest when applicable, environment class, OS/arch, env allowlist digest, Cargo home class, target dir class, cache mode, tool observations with tool versions and output digests, proof surface, subject digest, issued/expiry timestamps, result, claim support, claim exclusions, and staleness policy.
- A Rust receipt is stale if any bound source digest, Cargo lock/manifest digest, toolchain file, law registry, schema registry, validator binary, fixture suite, tool version, tool config, feature matrix, proof surface, package digest, install tree digest, runtime config digest, environment equivalence class, or command identity changes outside the allowed policy.
- Cache receipt fields must include cache mode, Cargo incremental state, `RUSTC_WRAPPER`, target dir, Cargo home, sccache stats before/after when used, remote cache endpoint hash or none, cache policy digest, cache hit/miss where available, and timing class.
- Timing classes must distinguish cold no-cache timing, cold declared-cache timing, warm local-cache timing, warm remote-cache timing, editor-warm timing, and CI-cache timing. A warm-cache result may never support a no-cache claim.

Required runtime memory/resource discipline:

- Rust ownership, borrowing, RAII/drop, owned values, local references, `Box<T>` for large/recursive/trait-object ownership, `Arc<T>` for shared concurrent immutable/config state, and `Weak<T>` for back edges are required memory-discipline tools where applicable.
- `Rc<T>` is allowed only in single-threaded internal graphs; cycles require `Weak`.
- `RefCell`, `Mutex`, `RwLock`, `DashMap`, `arc-swap`, crossbeam epoch tools, arenas, object pools, memory-mapped files, and alternate allocators are governed adapters. They require reason, scope, budget, metrics, drop/reset policy, tests, and receipts.
- Tracing garbage-collection crates are rejected for the correctness-critical core. They may not become the memory model for Harness Ultragoal's CLI/control plane.
- Bounded caches are required wherever caching exists. Allowed cache shapes include LRU, TTL, size-bound, entry-count-bound, and digest-addressed immutable caches. Global unbounded maps, static lazy caches without max size, non-canonical path string keys, caches without invalidation, and caches used as proof without digest binding are forbidden.
- Streaming parsers are required for large artifacts unless a size-bound whole-file read is justified and receipt-bound. Whole-file loading of package inventories, coverage reports, logs, session transcripts, source cards, generated packets, or audit outputs must have explicit size bounds or streaming behavior.
- Every long-running task must have owner, cancellation token or shutdown channel, bounded queue, tracked join handle, shutdown timeout, drop cleanup path, panic/error reporting, and memory/queue telemetry.
- Spawn-and-forget tasks, unbounded channels, unbounded join sets, unmanaged background watchers, leaked tempdirs on cancellation, child processes without kill/reap policy, and long-running loops without shutdown proof are forbidden.
- Memory/resource receipts must include scenario, binary digest, runtime config digest, duration, max RSS budget and observed RSS, max open file descriptors, max queue depth, max child processes, max temp bytes, created/removed tempdirs, spawned/reaped child processes, shutdown signal, cancellation delivery, all-tasks-joined status, shutdown duration, leak-check adapters and results, and claim impact.
- Leak detection must exist as scheduled/scoped proof: selected Miri checks, long-running RSS stability scenarios, resource lifecycle tests, tempdir/child-process/file-descriptor cleanup tests, and governed adapters for Valgrind, Heaptrack, sanitizers, and platform profilers where applicable.

Required workspace/artifact/cache garbage collection:

- Harness Ultragoal must implement both runtime resource cleanup and workspace/artifact/cache cleanup. Rust has no built-in workspace garbage collector, and Cargo cache/target cleanup does not satisfy Harness proof hygiene by itself.
- Every non-source workspace artifact must be classified before cleanup. Artifact classes must include Cargo target artifacts, Cargo registry cache, Cargo git cache, sccache entries, nextest recordings, coverage artifacts, validation artifacts, current receipts, stale receipts, final packets, review packets, generated source, generated non-source, package artifacts, install copies, plugin cache copies, worktree lanes, temp dirs, lock files, pid files, port reservations, trace logs, heap profiles, flamegraphs, and performance baselines.
- Unclassified artifacts cannot be deleted by automated cleanup and cannot support claims.
- Protected artifacts include current receipts supporting active claims, current final packets, current failure receipts needed for repair, release SBOM/provenance/signature/checksum artifacts, declared performance baselines, package/install/cache inventory receipts, Product Fitness/Cohesion/Success evidence, law/schema/source-obligation/standards registries, fixtures, source files, `Cargo.lock`, `rust-toolchain.toml`, and active review target/archive evidence.
- Protected artifacts may not be deleted unless replacement proof exists or the active goal is explicitly retired with typed disposition.
- GC commands must be plan/dry-run/apply/verify. `apply` must reference the plan digest. Post-delete verification must prove active claims remain supported, protected artifacts are present, locks/pids/ports are clean, and workspace artifact state matches the deletion receipt.
- Cleanup after failed or interrupted agents must inspect locks, pids, ports, child process records, temp dirs, partial receipts, partial package/install/cache copies, watcher state, and abandoned worktree lanes. Blind `rm -rf` cleanup is forbidden.

Required Rust workspace and topology direction:

- The repo must move toward a workspace/module shape that mirrors authority boundaries. Acceptable crate families include CLI, core/domain types, law registry/execution, namespace, receipt, claim ceiling, fixture execution, package inventory, install verification, cache verification, product proof, diagnostics, observability, plugin integration, and xtask/bootstrap.
- The CLI crate may parse and dispatch but must not hide law logic. Law-core crates must expose typed APIs and typed errors.
- The Rust module tree must remain maximally factored with no residual prefix encoding, no `utils`/`common`/`misc`/`shared` buckets, no coverage-wave/history names, and no unclassified generated/test/cache paths.
- Suggested line caps are governed defaults, not excuses: production Rust source file 300 non-comment LOC, test source file 400, fixture manifest 200, CLI command module 250, validator module 300, schema file 300 unless generated with receipt. The active repo line-cap law may be stricter and prevails.

Required migration path for the current repo:

- Phase 0: inventory with Cargo metadata, dependency tree, workspace topology report, namespace report, module-tree violations, generic bucket names, line-cap violations, toolchain gaps, dependency risks, test classes, and artifact directories.
- Phase 1: toolchain/bootstrap with `rust-toolchain.toml`, `.cargo/config.toml`, Cargo lock policy, xtask bootstrap, `.ultragoal/tool-inventory.toml`, and governed aliases only.
- Phase 2: command routing through `ultragoal rust fast`, `standard`, `coverage prove`, and `dependency audit`. Raw command output remains observation only.
- Phase 3: namespace and line-cap repair into maximally factored module tree.
- Phase 4: typed-boundary repair from `serde_json::Value` authority, stringly ids, open enum strings, and `anyhow` in law-core APIs into newtypes, enums, schemas, parsers, and typed errors.
- Phase 5: fixture triads for every validator.
- Phase 6: exact coverage and anti-gaming with declared source classes and no unclassified exclusions.
- Phase 7: package/install/cache separation.
- Phase 8: Product Fitness, Product Cohesion, and Product Success proof where applicable.
- Phase 9: release and GC with `ultragoal rust release`, `ultragoal gc plan`, `dry-run`, `apply`, `verify`, and CLI-built final packet.

Required standards, trace, source-obligation, and claim-ceiling integration:

- Add or tighten agent-standards rows for Rust Developer Experience, Rust toolchain/substrate authority, Rust command loops, Rust cache/no-cache honesty, Rust memory/resource discipline, and workspace/artifact garbage collection.
- Add foundational trace entries mapping AI Is Forcing Us To Write Good Code requirements for fast guardrails, fast feedback loops, clean checkout, concurrent isolated environments, and filesystem-as-interface; Parse, Don't Validate requirements for typed tool observations and receipt authority; Harness Engineering requirements for repo-local system of record, mechanical invariants, observability, and entropy cleanup; Symphony requirements for isolated workspaces and long-running orchestration; and ExecPlan requirements for restartable documented proof.
- Add source-obligation parity entries so Rust DevX, memory/resource discipline, and GC cleanup are not bundled under generic CLI performance, runtime feasibility, cleanup, or namespace rows without independent child-law failure proof.
- Add schema catalog entries, validator check ids, red fixtures, green fixtures or valid receipts, tamper/stale/wrong-surface fixtures, package inventory entries, receipt schemas, claim-ceiling guards, and final packet fields for each new law surface.
- Claim-ceiling guards must block completion, review, package, readiness, release, Product Fitness/Cohesion/Success, CLI self-law, source audit, final packet, and update_goal eligibility when applicable Rust DevX, Rust cache/no-cache, Rust memory/resource, or GC cleanup proof is missing, stale, wrong-surface, hidden-cache-dependent, unbounded, destructive, or non-CLI-built.

Required red, green, and tamper fixtures:

- Red fixtures proving raw Cargo/nextest/llvm-cov/deny/audit output cannot satisfy claims without `ultragoal` receipt conversion.
- Red fixtures proving `cargo test` pass cannot satisfy coverage, namespace, package, install, runtime, product, release, or update_goal claims.
- Red fixtures proving hidden global `RUSTC_WRAPPER`, hidden `CARGO_TARGET_DIR`, hidden Cargo config, editor-only green status, watcher-only pass, and warm-cache timing substituted for no-cache proof all fail.
- Red fixtures proving missing `rust-toolchain.toml`, missing Cargo lock policy, missing tool inventory, missing machine-readable output, missing cache mode, stale toolchain receipt, stale Cargo metadata, stale feature matrix, and changed tool version invalidate Rust loop receipts.
- Red fixtures proving `cargo-nextest` absence is a missing-tool/bootstrap failure for standard/release proof rather than silent fallback.
- Red fixtures proving `cargo-watch`, Makefile authority, unwrapped shell script authority, and raw local aliases cannot support claim pathways.
- Red fixtures proving `anyhow`/freeform error authority in law-core APIs, unparsed `serde_json::Value` authority, stringly law ids, open proof surfaces, and nullable/catch-all authority fail typed-boundary law.
- Red fixtures proving unbounded caches, unbounded queues, spawn-and-forget tasks, child process without kill/reap policy, temp resource without owner/drop policy, reference cycle without `Weak`, large unbounded whole-file load, and long-running loop without shutdown proof all fail memory/resource discipline.
- Red fixtures proving tracing GC crate adoption as core memory model fails.
- Red fixtures proving deletion without plan, deletion without dry-run, apply with wrong plan digest, protected artifact deletion, active-claim receipt deletion, unclassified artifact deletion, blind `rm -rf`, interrupted-agent cleanup without lock/pid/port/tempdir inspection, and deletion receipt without post-verify all fail GC law.
- Green fixtures/valid receipts proving canonical fast, standard, release, clean-proof, watch observation, memory/resource, and GC dry-run/apply/verify flows can pass on a compliant minimal fixture.
- Tamper fixtures proving changed tool version, changed tool config, changed cache mode, changed source digest, changed Cargo lock digest, changed law graph digest, changed plan digest, removed protected artifact, or altered deletion receipt invalidates proof and blocks claims.

Required validation:

- Run `ultragoal rust toolchain verify --emit-receipt` or equivalent once implemented.
- Run `ultragoal rust fast`, `ultragoal rust standard`, `ultragoal rust coverage prove --exact`, `ultragoal rust dependency audit`, `ultragoal rust performance prove`, `ultragoal rust memory prove`, `ultragoal rust clean-proof`, `ultragoal rust workspace topology check`, and `ultragoal gc plan/dry-run/apply/verify` as applicable to the candidate.
- Run focused red fixtures for raw-tool substitution, cache/no-cache dishonesty, missing toolchain/tool inventory, hidden local state, memory/resource leaks, and unsafe cleanup.
- Run source audit and red fixture report after implementing these surfaces.
- Do not refresh install/cache or launch reviewers until source-level Rust DevX/memory/GC enforcement passes on the same candidate.

Required confidence calculation:

- The final packet and final response must include calculated confidence for Gate 91 using this 100-point model:
  - doctrine/root cause alignment observed directly: 15 points;
  - Rust ecosystem maturity and adoption for required substrate: 15 points;
  - direct CLI receipt enforcement path implemented: 20 points;
  - cache/no-cache and clean-checkout honesty proven: 15 points;
  - memory/resource and long-running cleanup proof implemented: 15 points;
  - workspace/artifact GC protected-deletion proof implemented: 10 points;
  - red/green/tamper fixture completeness: 10 points.
- The expected target confidence is at least 96 percent only if the command loops, cache/no-cache receipts, memory/resource receipts, GC plan/dry-run/apply/verify receipts, standards/source-obligation/foundational trace entries, red/green/tamper fixtures, package inventory, source audit, and coverage proof all pass on the same candidate.
- A raw-tool-only patch, Cargo-only patch, performance-prose patch, watcher-only patch, cache-only speed claim, cleanup script without deletion receipt, memory-prose-only patch, or claim-ceiling-only patch cannot claim high confidence and must block completion.

Validation requirements:

Run and capture exact commands/output for:

- `cargo fmt --check`
- `cargo test --offline`
- full source audit with receipt and red fixture report
- full installed plugin audit
- full cache package audit
- coverage command proving 100%
- Rust toolchain/substrate receipt
- Rust fast loop receipt
- Rust standard loop receipt
- Rust release loop receipt for requested release/package/product claims
- Rust clean-proof/no-hidden-local-magic receipt
- Rust cache/no-cache honesty receipt
- Rust dependency/security/supply-chain receipt
- Rust performance budget receipt
- Rust memory/resource discipline receipt
- workspace/artifact/cache GC plan, dry-run, apply, verify receipts where cleanup is performed
- Rust DevX red, green, and tamper fixtures
- namespace law proof
- namespace red fixtures
- validator source namespace topology proof
- semantic repo-law source topology proof
- validator source namespace red, green, and tamper fixtures
- line-cap command
- runtime-tool identity proof and red fixtures
- product live-surface receipt proof and red fixtures
- transcript-quality receipt proof and red fixtures
- clean-checkout command-discovery proof and red fixtures
- restartable ExecPlan validator proof and red fixtures
- source-card freshness proof
- memory/wiki/Chronicle context-only proof and red fixtures
- Product Fitness proof
- Product Fitness review-team ownership proof
- Product Fitness review-round red fixtures
- standards enforcement proof
- foundational-law trace proof
- architecture dependency topology proof and red fixtures
- Quality Score/taste gate proof and red fixtures
- feedback-to-rule promotion receipt and red fixtures
- autonomy-loop proof receipts and red fixtures
- orchestrator state-machine proof and red fixtures
- scheduler/runner/tracker-boundary proof and red fixtures
- subagent/custom-agent sandbox and approval-inheritance proof and red fixtures
- skill progressive-disclosure metadata proof and red fixtures
- plugin install-surface metadata/cache/enable-state proof and red fixtures
- ExecPlan no-handback/prototype promotion-discard proof and red fixtures
- semantic domain-type naming proof and red fixtures
- agent-remediating validator failure-message proof and red fixtures
- third-party dependency legibility/typed-adapter proof and red fixtures
- repo knowledge index/core-beliefs proof and red fixtures
- workflow template parsing/rendering/reload proof and red fixtures
- workspace command confinement/lifecycle cleanup proof and red fixtures
- plugin bundled component graph and hook/app/MCP safety proof and red fixtures
- instruction precedence/nested AGENTS routing proof and red fixtures
- ExecPlan plain-language/expected-output/interface-completeness proof and red fixtures
- guardrail speed/isolation/cache-honesty proof and red fixtures
- secret/token boundary proof and red fixtures
- generated/proof artifact provenance and anti-fabrication proof and red fixtures
- review feedback disposition and same-round satisfaction proof and red fixtures
- behavior-example coverage and coverage anti-gaming proof and red fixtures
- one-command fresh environment bootstrap/concurrency proof and red fixtures
- agent-queryable observability proof and red fixtures
- subagent orchestration explicitness/token-model-cost/reconciliation proof and red fixtures
- skill catalog context-budget/omission-warning proof and red fixtures
- distribution and sharing-surface claim-separation proof and red fixtures
- total authority types and impossible-state elimination proof and red fixtures
- CLI self-law compliance/self-hosting proof and red fixtures
- CLI performance/latency/speed/iteration-fitness proof and red fixtures
- agent-authored source/tooling/docs provenance proof and red fixtures
- stable identifier/normalization/collision proof and red fixtures
- agent session telemetry/token/rate-limit proof and red fixtures
- config precedence/default/env-indirection proof and red fixtures
- fresh-init versus retrofit mode proof and red fixtures
- issue/tracker lifecycle/eligibility/terminal-state proof and red fixtures
- targeted refactor/debt-removal/standards-gardener cadence proof and red fixtures
- plugin flow graph/package dependency closure/plugin product journey proof and red fixtures
- portable non-prescriptive adapter proof and red fixtures
- derived authority recomputation/named-authority fallback proof and red fixtures
- offline schema catalog/resolver portability proof and red fixtures
- batch fan-out/custom-agent job discipline proof and red fixtures
- raw-private artifact handling/category-only evidence proof and red fixtures
- active setup-to-idle orchestration/thread-bound heartbeat proof and red fixtures
- connector capability discovery/same-surface capability proof and red fixtures
- target-repo audit capability/target-scope support proof and red fixtures
- trust-boundary abuse-path/failure-path coverage proof and red fixtures
- source-obligation parity/anti-bundling proof and red fixtures
- human-audit disposition decomposition/judgment-only claim-blocking proof and red fixtures
- capability-gap extraction/harness-capability promotion proof and red fixtures
- goal-contract amendment authority/closed-required-claim-id proof and red fixtures
- forward-only state transition/silent-reopen prevention proof and red fixtures
- initiation-time Product Success Contract authority proof and red fixtures
- product success goal/lane/ExecPlan binding proof and red fixtures
- product success lineage/amendment/closed-claim-id proof and red fixtures
- product proof joins/substitution-blocking proof and red/green fixtures
- Product Success Contract packet/review-team/skill-routing proof and red fixtures
- product-success inspiration-source provenance/disposition proof and red fixtures
- product strategy/positioning/research/eval pre-lane proof and red fixtures
- template-generation governance/Template Creator boundary proof and red fixtures
- value/adoption/continuance evidence hierarchy proof and red fixtures
- current product discovery/audit/quality-in-use evidence proof and red fixtures
- product-success lifecycle transition/no-afterthought proof and red fixtures
- validator-theater/miswire resistance proof and red/green/stale/wrong-surface fixtures
- green-path adequacy and satisfiable strictness proof
- clean-room rebuild/author-memory independence proof
- historical regression corpus proof from session logs, Chronicle, reviewers, and side-thread signals
- cross-artifact consistency solver/authority graph closure proof
- authority exhaustiveness/closed-enum/impossible-state elimination proof
- non-E2E claim ceiling and confidence-bound proof
- adversarial packet tampering/forged-proof rejection proof
- runtime feasibility/cost/strict-gate usability proof
- schema evolution/receipt migration/stale-version invalidation proof
- failure remediation quality/agent-actionable validator output proof
- review disagreement/override/judgment-boundary governance proof
- source/install/cache digest comparison
- review-target receipt regeneration
- candidate archive receipt regeneration
- final packet/successor packet validation

Do not call `update_goal()` until all are true:

1. Current source audit passes.
2. Current red fixture report passes with all fixtures failing for intended reasons.
3. No standards law remains optional/unmechanized for material claims.
4. Foundational article law trace is complete and validator-enforced, with no `partial`, `backlog`, `blocked`, `reviewer`, `reviewer_and_backlog`, `future`, stale-source, row-shape-only, or prose-only enforcement escape hatch for any foundational law.
5. Plugin self-coverage is 100% with typed receipt and no uncovered records.
6. Parsing/typed-boundary checks are enforced and tested.
7. Namespace and progressive-disclosure law is first-class and fail-closed: dedicated standards row, foundational trace entry, validator check, red fixtures, valid fixtures, receipt evidence, and claim-ceiling guard all exist and pass.
8. Line-cap adherence is enforced.
9. Runtime/tool identity, product live-surface, transcript-quality, clean-checkout command discovery, restartable ExecPlan, source-card freshness, and memory/wiki/Chronicle context-only laws are all deterministically enforced with schemas, validators, red fixtures, receipts, and claim-ceiling guards.
10. Product Fitness receipt is current and substitution failures are enforced.
11. Product Fitness review-team ownership is first-class and fail-closed in agent prompts, custom-agent TOML, review-round schema/fixtures, validator checks, and red fixtures; product-impacting review rounds fail without typed owner, disposition, current receipt binding, claim binding, and substitution review.
12. Source/install/cache are same candidate and same digest.
13. App-registry/reviewer exposure claims are either freshly proven on the same surface or impossible to emit.
14. Package inventory contains no private local proof paths.
15. Plugin version is bumped and all installed/cache/package metadata agrees.
16. Architecture dependency topology is first-class and fail-closed, with registry, validator, red fixtures, valid fixtures/receipts, and claim-ceiling guard all passing.
17. Quality Score/taste gates are typed, current, evidence-bound, and unable to pass while any underlying law fails.
18. Repeated feedback/session-log/reviewer findings are all promoted to deterministic enforcement or claim-blocking typed non-goals; none remain backlog/future/reviewer-only/documentation-only.
19. Autonomy-loop receipts prove before/after behavior for every behavior-changing repair that supports a claim.
20. Orchestrator state-machine invariants pass with bounded concurrency, authoritative state, stop-on-state-change, retry/backoff, deterministic workspaces, observability, and restart recovery.
21. Scheduler/runner/tracker mutation boundaries are enforced; handoff is not misrepresented as Done and external mutations require typed authority.
22. Subagent/custom-agent sandbox and approval inheritance is enforced and proven for reviewer/custom-agent claims.
23. Skill progressive-disclosure metadata and load routing are enforced for all packaged skills.
24. Plugin install-surface metadata, cache semantics, personal marketplace/install copy, enable-state, and installed-load proof agree and are enforced separately from source audit proof.
25. ExecPlan no-handback, stopping-point update, and prototype promotion/discard laws are enforced.
26. Semantic domain-type naming is enforced on law-bearing authority surfaces with only narrow typed exceptions.
27. Validator failure messages are agent-remediating, typed, law-bound, and claim-impacting.
28. Third-party dependency use is legible through typed adapters and no untyped/direct upstream authority remains.
29. Repo knowledge index and core-beliefs routing prove every law-bearing doc/proof surface is discoverable, fresh, owned, and validator-bound.
30. Workflow template parsing, strict rendering, source digests, path safety, and dynamic reload are enforced.
31. Workspace command confinement and lifecycle cleanup are proven for lane/workspace/runtime claims.
32. Plugin bundled component graph is closed and hook/app/MCP/component safety is enforced.
33. Instruction precedence and nested `AGENTS.md` routing are enforced, with conflicts resolved to the strictest applicable law or claim-blocking non-goal.
34. ExecPlans are plain-language, expected-output complete, interface/dependency complete, and executable by a fresh agent without author memory.
35. Guardrails meet runtime/isolation/cache-honesty requirements and final proof does not rely on hidden stale caches or fast-check substitutes.
36. Secret/token boundaries for subagents, dynamic tools, hooks, receipts, packets, and package inventory are enforced.
37. Generated/proof artifacts are deterministic, provenance-bound, reproducible, and anti-fabrication guarded.
38. Review feedback disposition is complete for human comments, agent findings, side-thread corrections, reviewer issues, and session-log review signals.
39. Coverage proves behavior with meaningful executable examples and cannot pass through hit-count theater, dead code, or coverage gaming.
40. Fresh environment bootstrap is one-command, fast enough for routine use, deterministic, and safe for concurrent workspaces.
41. Observability surfaces are agent-queryable, typed, bounded, redacted, correlated, and claim-bound.
42. Subagent orchestration is explicit, budgeted, reconciled, synthesized by the parent, and never treated as proof without live verification.
43. Skill catalog context budget, truncation/omission warning, and discoverability claim ceilings are enforced.
44. Local/personal/repo marketplace/install/cache/app/workspace/public distribution and sharing claims are separated and same-surface proven.
45. Authority types eliminate impossible states after parsing and reject partial/nullable/catch-all authority shapes.
46. Agent-authored source/tooling/docs provenance is enforced for every law-bearing change and no out-of-band/manual edit supports a compliance claim without current agent-visible proof.
47. Stable identifiers, normalization, and collision checks are enforced across candidates, packages, sessions, workspaces, receipts, archives, review rounds, cache keys, install ids, and registry/app surfaces.
48. Agent session telemetry, token accounting, model/reasoning identity, liveness, retry/backoff, and rate-limit impacts are recorded or explicitly unavailable for every law-bearing agent/reviewer run.
49. Config precedence, defaults, environment indirection, unknown-key rejection, redaction, and source/install/cache/app config separation are enforced.
50. Fresh-init, retrofit, source-only, installed-audit, cache-audit, registry/app proof, and review-packet modes are typed and cannot substitute for each other.
51. Issue/tracker lifecycle, eligibility, terminal-state, non-goal, blocked-by-external-authority, and `update_goal()` gates are typed and evidence-bound.
52. Targeted refactor/debt-removal/standards-gardener cadence proves repeated deviations and early debt are eliminated, mechanized, or claim-blocking.
53. Plugin flow graph, package dependency closure, and plugin product journey receipt are enforced and prove the complete plugin use journey with exact claim ceilings.
54. Portable non-prescriptive adapter boundaries prevent generic claims from depending on one tracker, UI, language stack, workflow engine, VCS, observability backend, or deployment surface.
55. Derived authority is recomputed from canonical current inputs, and named-authority fallback is refused or claim-limited with explicit receipts.
56. Offline schema catalog and resolver portability prove package validation does not depend on network fetches or undocumented local resolver state.
57. Batch fan-out/custom-agent job discipline proves stable item ids, output schemas, one result per worker, bounded concurrency/runtime, and parent synthesis.
58. Raw-private artifact handling proves durable outputs are redacted/category-only and no raw private material enters package artifacts, fixtures, receipts, packets, or child prompts.
59. Active setup-to-idle orchestration proves transition receipt, first-wave launch, thread-bound heartbeat, evidence cursor inspection, and typed wakeup outcomes.
60. Connector capability discovery proves same-surface capability authority and withholds every claim that depends on unavailable, stale, fallback, disabled, unauthenticated, or wrong-boundary capabilities.
61. Target-repo audit capability proves the exact target class, mode, surfaces, adapters, missing commands, unsupported ceilings, and prevents generic target-repo support claims without current target evidence.
62. Trust-boundary abuse-path and failure-path coverage is enforced for every security, permission, resolver, package, registry/app, model/tool authority, external input, dependency, destructive action, secret, and sensitive-data boundary.
63. Source-obligation parity proves every source-obligation law is either first-class same-law enforcement or a typed parent/child law with independent failure proof, receipts, red fixtures, valid fixtures, and claim-ceiling guard.
64. Human-audit dispositions are decomposed so mandatory law compliance is deterministic, judgment-only review is typed and claim-blocking, and no hybrid human-audit label can close full compliance.
65. Capability gaps are extracted, owned, promoted, and claim-blocked until repaired for every missing tool, context, runtime legibility, validator, schema, fixture, receipt, adapter, command, target surface, reviewer route, permission, doc index, install/cache/app surface, or underspecified environment affecting a claimed surface.
66. Goal-contract amendment authority and closed required-claim-id mapping prove every side-thread addition, checklist gate, validation obligation, final-packet claim, law-surface change, and scope mutation is append-only amended, digest-bound, claim-id mapped, and claim-blocked when unsupported.
67. Forward-only state transition integrity proves no approved, signed-off, verified, fixed, terminal, or claim-green surface can silently re-enter active/review/support states without a typed reopen/regression/supersession transition, fresh validation obligations, and claim blocking until revalidated.
68. Initiation-time Product Success Contract authority is enforced for every product-impacting goal, lane, packet, manifest, source/install/cache receipt, and claim ceiling, with schema/template/package surfaces, required typed fields, digests, actors, red fixtures, and package inventory coverage.
69. Product success binding is enforced in goal contracts, lane registries, ExecPlan lane authority, completion manifests, amendments, product evidence plans, non-product waivers, and lane launch/ready/merge/archive gates.
70. Product success lineage, append-only amendments, closed product claim ids, Product Fitness receipt lineage, Product Cohesion receipt lineage, packet claim mapping, and final-response product claims are enforced with no orphan or freeform product claims.
71. Product proof joins and substitution blocking are enforced so install/cache/package/publication/smoke/test/fixture/reviewer/packet/cohesion/fitness/quality-score/dogfood substitutes cannot support product success unless same-surface Product Success Contract evidence is present.
72. Product Success Contract review, review-packet inclusion, review-team ownership, detached target/archive inclusion, package inventory coverage, and initiation-time skill routing are enforced and fail closed.
73. Product-success inspiration-source provenance and disposition is enforced for every at-mentioned plugin, skill family, session log family, Chronicle summary, deep-research-v2 artifact, foundational article, and repo source used to shape requirements.
74. Product strategy, Product Success Brief, positioning, research notes, eval protocol, success metrics, first-value path, adoption loop, and continuance signal are generated and validated before product-impacting lane planning begins.
75. Template-generation governance and Template Creator boundary are enforced so required repo-owned product templates exist, are schema/fixture/round-trip validated, and no personal template/plugin-cache/tool output substitutes for package-owned authority.
76. Value, adoption, continuance, daily-driver, business/mission outcome, and confidence claims preserve `Known`/`Inferred`/`Assumed`/`Missing` evidence hierarchy and fail when unsupported.
77. Current product discovery, product audit, source-backed research, quality-in-use, accessibility/cognitive-load, and claim-id mapping evidence is required before any product-facing claim.
78. Product-success lifecycle transitions prove no late afterthought integration, no packet-only product claims, no stale product receipts after amendments, and no unowned product-critical debt under a positive claim ceiling.
79. Validator-theater and miswire resistance proves every law-bearing validator check rejects real non-compliant behavior through the same authority path used for completion/review/package/readiness/release claims, with minimal valid, realistic valid, red mutant, stale/digest mutant, wrong-surface mutant, and miswire mutant coverage.
80. Green-path adequacy proves every mandatory law has satisfiable strictness, with minimal and realistic compliant fixtures/receipts where applicable, claim-ceiling projection, package/review/report projection, and no red-only or impossible compliance law.
81. Clean-room rebuild and author-memory independence prove source, installed plugin, cache package, review target, candidate archive, schema catalog, package inventory, Product Success surfaces, validator receipt, red fixture report, coverage receipt, and final packet can be regenerated from documented commands without private paths, hidden caches, stale local state, or author memory.
82. Historical regression corpus proves every session-log, Chronicle, reviewer, side-thread, and validation repeat signal is frozen into deterministic enforcement or claim-blocking typed non-goal with source artifact, timestamp/session id, fixture ids, validator ids, receipt ids, claim ids, and claim impact.
83. Cross-artifact consistency solver and authority graph closure prove laws, sources, standards rows, source obligations, schemas, templates, validators, fixtures, receipts, package inventory, source/install/cache artifacts, review target, archive, packet claims, required claim ids, Product Success Contract ids, and claim ceilings have no orphan, stale, duplicate, hidden, private, or umbrella-only authority.
84. Authority exhaustiveness, closed enums, and impossible-state elimination prove law-bearing statuses, claim ceilings, proof surfaces, target modes, receipt kinds, review dispositions, product evidence levels, package surfaces, validator outcomes, fixture outcomes, and transition states reject freeform, nullable, unknown, partial, or catch-all authority.
85. Non-E2E claim ceiling and confidence bounds prove no product-success, daily-driver, marketplace, release, adoption, sustained-value, live reviewer readiness, or external-user-success claim exceeds the explicit pre-E2E ceiling, no matter how many non-E2E gates pass.
86. Adversarial packet tampering and forged-proof rejection prove final packets, review targets, archives, receipts, red fixture reports, coverage, Product Success/Fitness, source/install/cache, and active-registry evidence reject swapped digests, stale receipts, wrong candidate versions, forged registry proof, wrong paths, altered claim ceilings, disposition flips, private paths, and injected claims for precise reasons.
87. Runtime feasibility, cost, and strict-gate usability prove strict enforcement remains runnable with documented commands, expected outputs, runtime budgets, concurrency bounds, no-cache/full-proof modes, cache invalidation rules, and failure behavior, without hidden stale caches, unbounded loops, flaky checks, or focused-check substitution.
88. Schema evolution, receipt migration, and stale-version invalidation prove every schema/receipt/template/fixture catalog/package inventory/manifest/validator version change either migrates, supersedes, or invalidates old artifacts with claim blocking and source/install/cache refresh obligations.
89. Failure remediation quality proves every validator/schema/fixture/packet/package/source-install-cache/product/claim-ceiling failure emits typed agent-actionable repair data with law id, artifact, failed invariant, observed/expected values, repair class, rerun command, affected claims, severity, and claim impact.
90. Review disagreement, override, and judgment-boundary governance proves reviewer/human/custom-agent judgment cannot override deterministic failure or raise claim ceilings, and every disagreement/override attempt has typed disposition, deterministic sibling or claim blocker, evidence, transition history, and claim impact.
91. Final packet states implemented repairs, exact evidence, strict supported claims, explicit non-E2E ceiling, and no unresolved blockers.

## Additional update_goal() Stop Conditions For Gate 89

These stop conditions are additive. Existing stop conditions remain fully mandatory.

92. CLI authority graph passes strict validation, and no Harness Ultragoal law authority exists outside the CLI.

93. Every checked checklist item has a current CLI law receipt bound to the same candidate digest, source digest, schema catalog digest, law graph digest, standards digest, source-obligation digest, and fixture catalog digest required by that item.

94. Final packet, review target, candidate archive, claim ceiling, source audit, install audit, cache audit, Product Fitness proof, Product Cohesion proof, Product Success proof, coverage proof, line-cap proof, typed-boundary proof, standards proof, foundational trace proof, source-obligation proof, package proof, and update_goal eligibility are CLI-built or CLI-verified.

95. No hand-authored, edited, stale, copied, wrong-surface, wrong-digest, wrong-schema, reviewer-only, prose-only, checklist-only, packet-only, row-shape-only, fixture-name-only, install-substituted, cache-substituted, source-substituted, or claim-ceiling-only artifact can satisfy completion.

96. CLI init/retrofit hook installation or hook-unavailable claim blocking is implemented, validated, receipt-bound, package-included, and same-candidate.

97. Agent-standards enforcement explicitly requires CLI-governed Harness Ultragoal law execution, and red fixtures prove agents cannot satisfy claims by bypassing the CLI.

98. Every newly observed material failure mode has been captured and promoted into deterministic CLI enforcement, schema tightening, fixture coverage, receipt requirement, claim guard, package inventory rule, or typed non-goal exclusion that blocks related claims.

99. update_goal eligibility is computed by the CLI and fails unless every mandatory gate, checklist item, stop condition, receipt, fixture report, product proof, package proof, proof surface, and claim ceiling is current and same-candidate.

100. CLI self-law compliance is proven by the same candidate CLI, with self-hosted receipts proving the CLI/validator/tooling/package artifacts obey every law they enforce; bootstrap, transition-only, source-only, stale, or target-only CLI proof cannot support completion, package readiness, review readiness, release readiness, or update_goal eligibility.

101. CLI performance, latency, speed, and iteration fitness are proven with typed budgets, current performance receipts, no-cache/cache-honesty proof, concurrency/isolation proof, performance regression proof, foundational traceability to fast guardrails and fast ephemeral concurrent environments, and claim blocking for every over-budget, stale, hidden-cache, unbounded, or focused-substituted proof path.

102. Validator source namespace topology and semantic repo-law enforcement are proven by physical source-tree repair, removal of broad `validator/src/internal*` exceptions, typed narrow exception parsing, actual repo-owned source inspection, red/green/tamper fixtures, package inventory exactly-once closure, 100 percent coverage preservation, source audit pass, and a calculated confidence score of at least 99 percent supported by evidence. No completion, review, package, readiness, release, CLI self-law, final packet, or update_goal claim may pass while top-level `validator/src/internal_*.rs`, `validator/src/internal_coverage*.rs`, `validator/src/iinternal_*.rs`, or equivalent prefix-as-directory source clusters remain accepted by the law.

103. Rust Developer Experience, runtime memory/resource discipline, and workspace/artifact/cache garbage collection are proven by CLI-routed Rust command loops, current toolchain/substrate receipt, fast/standard/release/clean-proof/watch observation command surfaces, exact coverage proof, dependency/security/supply-chain proof where applicable, cache/no-cache honesty receipt, performance budget receipt, memory/resource receipt, GC plan/dry-run/apply/verify receipts where cleanup is performed, standards/source-obligation/foundational-trace bindings, red/green/tamper fixtures, source audit pass, and calculated confidence of at least 96 percent supported by evidence. No completion, review, package, readiness, release, Product Fitness, Product Cohesion, Product Success, CLI self-law, final packet, or update_goal claim may pass from raw Cargo/tool output, hidden cache state, watcher/editor state, unbounded Rust runtime resources, unmanaged long-running tasks, blind cleanup, deletion without receipt, or stale Rust DevX/memory/GC proof.

Final response must include:
- exact files changed
- exact commands run and results
- evidence paths
- version bump details
- source/install/cache digests
- coverage percentage and receipt path
- red fixture counts
- standards law trace status
- CLI self-law compliance/self-hosting status
- CLI performance/latency/speed/iteration-fitness status
- Rust Developer Experience command-loop status
- Rust toolchain/substrate receipt status
- Rust cache/no-cache honesty status
- Rust dependency/security/supply-chain status
- Rust memory/resource discipline status
- workspace/artifact/cache garbage-collection status
- namespace law enforcement status
- validator source namespace topology and semantic repo-law enforcement status
- namespace/semantic repo-law confidence calculation and rationale
- Rust DevX/memory/GC confidence calculation and rationale
- runtime/tool identity, product live-surface, transcript-quality, clean-checkout, ExecPlan, source-card freshness, and memory-context-only enforcement status
- architecture dependency topology enforcement status
- Quality Score/taste gate enforcement status
- feedback-to-rule promotion status
- autonomy-loop receipt status
- orchestrator state-machine enforcement status
- scheduler/runner/tracker-boundary enforcement status
- subagent/custom-agent sandbox and approval-inheritance status
- skill progressive-disclosure metadata status
- plugin install-surface metadata/cache/enable-state status
- ExecPlan no-handback and prototype promotion/discard status
- semantic domain-type naming status
- agent-remediating validator failure-message status
- third-party dependency legibility and typed-adapter status
- repo knowledge index/core-beliefs status
- workflow template parsing/rendering/reload status
- workspace command confinement/lifecycle cleanup status
- plugin bundled component graph and hook/app/MCP safety status
- instruction precedence/nested AGENTS routing status
- ExecPlan plain-language/expected-output/interface-completeness status
- guardrail speed/isolation/cache-honesty status
- secret/token boundary status
- generated/proof artifact provenance and anti-fabrication status
- review feedback disposition and same-round satisfaction status
- behavior-example coverage and coverage anti-gaming status
- one-command fresh environment bootstrap/concurrency status
- agent-queryable observability status
- subagent orchestration explicitness/token-model-cost/reconciliation status
- skill catalog context-budget/omission-warning status
- distribution and sharing-surface claim-separation status
- total authority types and impossible-state elimination status
- agent-authored source/tooling/docs provenance status
- stable identifier/normalization/collision status
- agent session telemetry/token/rate-limit status
- config precedence/default/env-indirection status
- fresh-init versus retrofit mode status
- issue/tracker lifecycle/eligibility/terminal-state status
- targeted refactor/debt-removal/standards-gardener cadence status
- plugin flow graph/package dependency closure/plugin product journey status
- portable non-prescriptive adapter status
- derived authority recomputation/named-authority fallback status
- offline schema catalog/resolver portability status
- batch fan-out/custom-agent job discipline status
- raw-private artifact handling/category-only evidence status
- active setup-to-idle orchestration/thread-bound heartbeat status
- connector capability discovery/same-surface capability authority status
- target-repo audit capability/target-scope support status
- trust-boundary abuse-path/failure-path coverage status
- source-obligation parity/anti-bundling status
- human-audit disposition decomposition/judgment-only claim-blocking status
- capability-gap extraction/harness-capability promotion status
- goal-contract amendment authority/closed-required-claim-id status
- forward-only state transition/silent-reopen prevention status
- initiation-time Product Success Contract authority status
- product success goal/lane/ExecPlan binding status
- product success lineage/amendment/closed-claim-id status
- product proof joins/substitution-blocking status
- Product Success Contract packet/review-team/skill-routing status
- product-success inspiration-source provenance/disposition status
- product strategy/positioning/research/eval pre-lane status
- template-generation governance/Template Creator boundary status
- value/adoption/continuance evidence hierarchy status
- current product discovery/audit/quality-in-use evidence status
- product-success lifecycle transition/no-afterthought status
- validator-theater/miswire resistance status
- green-path adequacy/satisfiable strictness status
- clean-room rebuild/author-memory independence status
- historical regression corpus status
- cross-artifact consistency solver/authority graph closure status
- authority exhaustiveness/closed-enum/impossible-state elimination status
- non-E2E claim ceiling/confidence-bound status
- adversarial packet tampering/forged-proof rejection status
- runtime feasibility/cost/strict-gate usability status
- schema evolution/receipt migration/stale-version invalidation status
- failure remediation quality/agent-actionable output status
- review disagreement/override/judgment-boundary governance status
- strict claim ceiling

Do not finish by saying blockers remain. Do not finish because coverage is fixed. Do not finish because the source audit passes. Do not finish because a packet exists. Do not finish because non-E2E hardening is specified in prose but not enforced. Keep working until the repo/plugin itself is fully compliant with every Harness Ultragoal law in this contract, including the laws that require restructuring early codebase debt, deleting theater checks, replacing brittle validators, and refusing any claim that outruns live proof.
```
