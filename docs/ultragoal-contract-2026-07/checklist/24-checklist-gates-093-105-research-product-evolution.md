## Gate 93 - Research Source Authority And Article-To-Law Integration

- [ ] Mandatory research-source registry includes the original nine observability/harness-engineering sources, the OpenAI agent-improvement loop cookbook, and the OpenAI self-improving tax-agent article with stable ids, URLs, source digests, retrieved/source-card evidence, affected canonical law ids, setup/retrofit implications, tool/package implications, privacy implications, and claim-ceiling impact.
  - Status: validated current: source-local research registry binding; full Gate 93 remains in progress.

- [ ] Article-to-law trace maps every source requirement to canonical law ids, standards rows, source obligations, foundational trace entries, schemas, typed check enums, validator checks, red fixtures, green fixtures, tamper fixtures, receipts, package inventory entries, setup/retrofit outputs, claim guards, and final-packet fields.
  - Status: validated current: source-local article-to-law trace includes tamper fixture binding; full tamper/source-audit closure remains in progress.

- [ ] Gate 93 green/tamper proof covers every mandatory research source and every requirement class. One mapped source, one mapped category, one green fixture, one source family, or one adjacent observability surface cannot satisfy any other source, category, fixture, law id, setup/retrofit implication, package surface, claim guard, final-packet field, or update_goal blocker.
  - Evidence:
  - Green fixtures:
  - Tamper fixtures:
  - Research registry:
  - Article-to-law trace:
  - Candidate digest:
  - Claim impact:
  - Status:

- [ ] Validator fails unmapped, stale, prose-only, umbrella-only, law-family-alias-only, fixture-incomplete, receipt-missing, package-omitted, setup/retrofit-omitted, or claim-guard-omitted research requirements.
  - Status: validated current: registry/trace fail-closed bindings; full tamper/source-audit closure remains in progress.

- [ ] Claim guards block completion, review readiness, package readiness, product readiness, release readiness, registry readiness, setup/retrofit completeness, active-repo rollout completeness, final packet, and `update_goal()` when mandatory research mapping is incomplete.
  - Evidence: Partial guard strings and validator bad paths exist in `docs/research-article-to-law-trace.json`, `docs/mandatory-law-surfaces.json`, `templates/agent-standards/enforcement.json`, and `validator/src/audit/research/`. This row remains unchecked because final-packet/update_goal claim-guard closure and current source-audit proof have not been regenerated.
  - Claim guards: `research_backed_claims_withheld_without_current_article_to_law_trace`; update_goal blocker `research_source_authority_incomplete`.
  - Receipt: No current same-candidate source-audit receipt yet for this Gate 93 checkpoint.
  - Candidate digest: `sha256:f67bcb8c9ede3ef968c6e372d7f1092a5cc55a9a054edc1faa0d7b8777860383`.
  - Status: in progress

## Gate 94 - Harness Improvement Loop, Trace Feedback, Eval, And Codex Handoff

- [ ] Plugin provides a first-class Harness Improvement Loop skill/surface with progressive-disclosure routing, schemas, CLI commands, setup/retrofit integration, package inventory coverage, and same-candidate receipts.
  - Evidence: Partial source-local Gate 94 surface is implemented for package digest `sha256:801597e0dd840db5da58f8efca4e8afcb9e659a280d88d71be104f4aeab5f628`. `target/debug/ultragoal --root . improvement-loop prove --receipt validation_artifacts/improvement-loop/loop-closure-receipt.json` exited `0` and minted `validation_artifacts/improvement-loop/loop-closure-receipt.json` with `status = pass`, nested observability run id `run-27e913c14a164adfc9f71b7679fd073f84263b78e0e1ad77d029e66d0455181c`, correlation id `corr-8e6c5e442518fddec2496fb000c6b5e4c6c10941b12de950eb2588a824151824`, and `claim_impact = supports_same_candidate_improvement_loop_closure_only`.
  - Skill/surface path: `skills/agent-improvement-loop/SKILL.md`
  - Schemas: `schemas/improvement-loop-registry.schema.json`; `schemas/improvement-loop-stage-evidence.schema.json`; `schemas/improvement-loop-receipt.schema.json`
  - CLI commands: `target/debug/ultragoal --root . improvement-loop prove --receipt validation_artifacts/improvement-loop/loop-closure-receipt.json`
  - Package entries: `plugin-manifest-draft.json` includes the improvement-loop skill, registry, schemas, validator source, fixtures, and receipt path.
  - Candidate digest: `sha256:801597e0dd840db5da58f8efca4e8afcb9e659a280d88d71be104f4aeab5f628`
  - Status: Partial, not checked. Source-local loop closure now passes, but setup/retrofit integration and full Gate 96/97 adapter closure remain open. Readiness, release, completion, final-packet correctness, registry/reviewer exposure, and `update_goal()` claims remain blocked.

- [ ] Improvement-loop registry binds traces, feedback, feedback clusters, eval ids, promptfoo suite ids, HALO ranking ids, Codex handoff ids, implementation change ids, validation receipt ids, before/after telemetry comparison ids, and promotion ids.
  - Evidence: `docs/improvement-loop-registry.json` records a source-local bootstrap loop with trace, feedback, cluster, eval, promptfoo suite, HALO ranking, Codex handoff, implementation change, validation receipt, before/after telemetry, and promotion ids. The registry now dereferences `docs/improvement-loop/stage-evidence/source-local-gate-94.json` by schema `harness-ultragoal.improvement-loop-stage-evidence.v1` and digest `sha256:50347177af0f2b59247cb4c15cbb3c6ea5a537df1b16bdf88fd1eaedddae51f7`; the CLI rejects missing evidence, digest mismatch, missing stages, bad paths, and incomplete closure.
  - Registry path: `docs/improvement-loop-registry.json`
  - Receipt: `validation_artifacts/improvement-loop/loop-closure-receipt.json`
  - Candidate digest: `sha256:801597e0dd840db5da58f8efca4e8afcb9e659a280d88d71be104f4aeab5f628`
  - Status: unchecked after current source-local edits; prior registry-binding evidence targets `sha256:801597e0dd840db5da58f8efca4e8afcb9e659a280d88d71be104f4aeab5f628`, while the current live package digest is `sha256:e2e422756f4404c5e0d3f363d1172c4ee5aba891cd1aff2a02d6c8fb4a9abd41`. This does not check full Gate 94 stop condition 106, setup/retrofit integration, live HALO ranking, full promptfoo eval closure, final-packet correctness, readiness, release, completion, registry/reviewer exposure, or `update_goal()`.

- [ ] CLI proves the loop from current same-candidate traces to typed feedback, clustering, eval generation, promptfoo execution, HALO ranking, Codex handoff, implementation linkage, narrow validation, before/after telemetry comparison, and promotion into laws/fixtures/schemas/standards.
  - Evidence: CLI authority exists and now passes a source-local loop closure from dereferenced registry/stage-evidence/receipt evidence. Focused tests passed: `cargo test --offline improvement_loop --lib --quiet` (`4/4`), `cargo test --offline red_identity --lib --quiet` (`3/3`), `cargo test --offline red_fixture_runtime_binding --lib --quiet` (`1/1`), `cargo test --offline hu_family --lib --quiet` (`5/5`), `cargo test --offline schema_catalog --lib --quiet` (`3/3`), and `cargo test --offline --lib --quiet` (`506/506`) before the stage-evidence follow-up. The production command `target/debug/ultragoal --root . improvement-loop prove --receipt validation_artifacts/improvement-loop/loop-closure-receipt.json` exited `0` for the current digest.
  - Commands: `target/debug/ultragoal --root . improvement-loop prove --receipt validation_artifacts/improvement-loop/loop-closure-receipt.json`
  - Receipts: `validation_artifacts/improvement-loop/loop-closure-receipt.json`
  - Before/after telemetry: Source-local stage evidence records the fail-closed-to-green receipt transition; full Gate 92 live query proof across every law-bearing path remains separately unchecked.
  - Candidate digest: `sha256:801597e0dd840db5da58f8efca4e8afcb9e659a280d88d71be104f4aeab5f628`
  - Status: Partial, not checked. Source-local CLI loop closure is green, but this row remains unchecked until promptfoo eval execution, HALO ranked-change authority, setup/retrofit linkage, tamper fixtures, and full same-candidate source-audit/red-report evidence are complete.

- [ ] Red/green/tamper fixtures prove raw traces, raw feedback, raw model output, raw promptfoo output, raw HALO output, reviewer agreement, checklist prose, stale telemetry, and hand-authored receipts cannot close an improvement loop.
  - Evidence: Initial red/green/tamper-style fixtures and tests are registered and focused tests passed, but this is not full Gate 94 fixture closure. The validator rejects hand-authored green receipts in `improvement_loop_receipt_rejects_hand_authored_green`, rejects missing stage evidence in `improvement_loop_receipt_rejects_missing_stage_evidence`, and has a possible green path in `improvement_loop_receipt_has_possible_green_path`.
  - Red fixtures: `fixtures/red/harness-improvement-loop-trace-without-feedback-red.json`; `fixtures/red/harness-improvement-loop-raw-halo-authority-red.json`; `fixtures/red/harness-improvement-loop-validation-skipped-red.json`; `fixtures/red/harness-improvement-loop-hand-authored-receipt-red.json`
  - Green fixtures: `fixtures/mandatory-law-surfaces/valid/harness-improvement-loop-trace-feedback-eval-codex-handoff.json`
  - Tamper fixtures: Partial only through focused Rust tamper test; dedicated tamper fixture catalog entries remain open.
  - Candidate digest: `sha256:801597e0dd840db5da58f8efca4e8afcb9e659a280d88d71be104f4aeab5f628`
  - Status: Partial, not checked. Raw HALO authority and hand-authored receipt substitutions fail in the focused slice; complete raw trace/raw feedback/raw model/raw promptfoo/reviewer/checklist/stale-telemetry/tamper fixture closure remains open.

## Gate 95 - OpenAI API, Key Authority, Model Identity, Cost, And External AI Boundary

- [ ] Repo declares governed OpenAI API key destination and typed config policy; `OPENAI_API_KEY` is loaded only from an untracked local env surface or secure OpenAI Platform setup flow and is never committed, logged, traced, metric-labeled, packeted, or passed to child agents without typed authorization.
  - Evidence: Partial source-local config, live-provider call-boundary, provider-budget, command-inventory fitting, and typed model-output boundary governance exists for package digest `sha256:72da24c6dcf737cdcd2db83c57a90ed59a8564839aa7a0eaa245a83464c5b85c`. `target/debug/ultragoal --root . openai config prove --receipt validation_artifacts/openai/config-receipt.json` exited `0`; `target/debug/ultragoal --root . openai call prove --receipt validation_artifacts/openai/call-receipt.json --mode openai_live --budget-class source_live_low --model gpt-5.4-nano --endpoint responses --purpose openai-live-boundary-proof` exited `0`; `target/debug/ultragoal --root . openai output prove --receipt validation_artifacts/openai/model-output-authority.json --parsed-output-digest sha256:9e0ddb4f57c273027581b6c13422339eeed4865aa288863d6502a443d59e4b38` exited `0`; `cargo test --offline openai --lib --quiet` passed `11/11`; `cargo build --offline` passed; `cargo fmt --check` passed; `find validator/src -name '*.rs' -exec wc -l {} + | awk '$2 != "total" && $1 > 250 { print }'` emitted no rows.
  - Config path: `docs/openai-key-policy.json`; local key destination `.codex-worktree/env.sh` remains gitignored and untracked.
  - Redaction proof: `validation_artifacts/openai/config-receipt.json` has `status = pass`, `redaction_status = pass`, `secret_material_serialized = false`, supports only `openai_config_redacted_resolution`, and blocks `completion`, `readiness`, `release`, `reviewer_exposure`, `app_registry_exposure`, `final_packet_correctness`, `update_goal_eligibility`, `product_success`, `model_output_authority`, and `live_model_claim`; `validation_artifacts/openai/call-receipt.json` has `provider_mode = openai_live`, `redaction_status = pass`, `model_output_authority = observation_only_until_cli_schema_validated`, supports only `openai_call_receipt_boundary`, records request id `83c9b714-b6e8-4c96-aa2a-d6102f08d02a`, token counts `24/9/33`, latency `982 ms`, `rate_limit_observed = true`, and blocks the same completion/readiness/release/update_goal/model-output/live-model claims.
  - Secret-scan proof: `rg -n "sk-proj-[A-Za-z0-9_-]{20,}|sk-[A-Za-z0-9]{20,}" validation_artifacts/openai docs/openai-key-policy.json docs/openai-provider-policy.json .codex-plugin plugin-manifest-draft.json schemas/openai-*.json` returned no matches; `.codex-worktree/env.sh` remains gitignored and untracked.
  - Command inventory: `docs/generated/observability/command-inventory.json` records `openai call prove` live provider invocation and live token/cost/rate-limit receipt capture as fitted, while same-candidate live log/metric/trace query proof and promptfoo/improvement-loop adapter binding remain missing.
  - Candidate digest: `sha256:72da24c6dcf737cdcd2db83c57a90ed59a8564839aa7a0eaa245a83464c5b85c`; rerunning `target/debug/ultragoal --root . package digest` after live receipt generation returned the same digest.
  - Status: Partial, not checked. Config resolution, provider-budget policy, live timeout/retry/cache/cost/rate-limit receipt fields, typed model-output observation receipt, and redaction blockers are governed, but full Gate 95 remains open until offline fixture execution, schema-specific model-output parser adapters, promptfoo/HALO/OpenAI integration calls, complete provider-mode red/green/tamper proof, and end-to-end claim guards are implemented and current.

- [ ] Every OpenAI call receipt records model id, endpoint/API family, purpose, prompt/input digest, schema id, output digest, request id when available, token counts when available, cost estimate or cost-unavailable reason, latency, retry/backoff, rate-limit observations, redaction status, candidate digest, run id, correlation id, and claim impact.
  - Evidence: `schemas/openai-call-receipt.schema.json`, `schemas/openai-provider-policy.schema.json`, and `schemas/openai-cost-rate-limit-receipt.schema.json` define the required call, budget, timeout/retry/cache, and cost/rate-limit fields; `validation_artifacts/openai/call-receipt.json` is a current live-provider boundary receipt for package digest `sha256:72da24c6dcf737cdcd2db83c57a90ed59a8564839aa7a0eaa245a83464c5b85c`.
  - Schema: `schemas/openai-call-receipt.schema.json`; `schemas/openai-provider-policy.schema.json`; `schemas/openai-cost-rate-limit-receipt.schema.json`
  - Receipt: `validation_artifacts/openai/call-receipt.json`; supports live call-boundary observation only, not model output authority, product success, readiness, release, or update_goal.
  - Validator check: `openai-api-key-model-cost-external-ai-boundary` is registered in standards, source obligations, foundational traceability, mandatory-law surfaces, schema enums, and `validator/src/audit/openai/mod.rs`.
  - Candidate digest: `sha256:72da24c6dcf737cdcd2db83c57a90ed59a8564839aa7a0eaa245a83464c5b85c`
  - Status: Partial, not checked. Live call receipt plus typed budget/cache/retry/cost/rate-limit policy is implemented; offline fixture execution, complete provider-mode red/green/tamper proof, and schema-specific output authority guards remain open.

- [ ] Model outputs used for feedback clustering, eval generation, grading, summarization, ranking, or handoff are parsed into typed schemas before they affect any law, fixture, receipt, claim ceiling, or final packet.
  - Evidence: Model-output authority is blocked by `validation_artifacts/openai/config-receipt.json` and `validation_artifacts/openai/call-receipt.json`; `validation_artifacts/openai/model-output-authority.json` now dereferences the current live call receipt and records `authority_state = typed_observation_not_claim_authority`, `parser_schema_id = harness-ultragoal.openai.typed-output.v1`, raw output digest `sha256:9e0ddb4f57c273027581b6c13422339eeed4865aa288863d6502a443d59e4b38`, and parsed-output digest `sha256:9e0ddb4f57c273027581b6c13422339eeed4865aa288863d6502a443d59e4b38` for package digest `sha256:72da24c6dcf737cdcd2db83c57a90ed59a8564839aa7a0eaa245a83464c5b85c`.
  - Parser/schema: `schemas/openai-model-output-authority.schema.json`; schema-specific feedback/eval/grader/ranking parser adapters remain pending.
  - Focused tests: `cargo test --offline openai --lib --quiet` passed `11/11`, including `openai_output_receipt_dereferences_current_call_and_blocks_claims` and `openai_package_audit_rejects_output_authority_overclaim`.
  - Red fixtures: Mandatory-law red fixtures added for `secret_serialized`, `model_output_authority_claim`, `wrong_candidate_digest`, and `unbounded_cost`, but model-output parser fixtures remain pending.
  - Candidate digest: `sha256:72da24c6dcf737cdcd2db83c57a90ed59a8564839aa7a0eaa245a83464c5b85c`
  - Status: Partial, not checked. Live model output is observation-only and overclaim is rejected, but schema-specific parser adapters and promptfoo/HALO/OpenAI integration use remain open.

- [ ] OpenAI live calls have budget classes, max retries, timeout, backoff, cache policy, no-cache verification when required, offline fixture mode, cost/rate-limit receipts, and fail-closed behavior when the key or provider is unavailable.
  - Evidence: Provider modes are declared in `docs/openai-key-policy.json`; budget classes are declared in `docs/openai-provider-policy.json`; `openai call prove --mode openai_live --budget-class source_live_low` mints a live call-boundary receipt that binds `budget_class`, `provider_policy_digest`, `timeout_retry_backoff`, `cache_policy`, request id, token counts, latency, rate-limit observation, and a nested cost/rate-limit receipt; `validator/src/audit/openai/mod.rs` rejects wrong/missing provider-policy digest, missing budget class, invalid cost/rate-limit receipt, invalid timeout/retry/backoff, and missing cache policy.
  - Budget policy: Implemented for source-local no-network/offline/local-mock/live-low policy shape; live-low is now exercised for this source digest but still claim-limited to observation.
  - Offline fixture proof: Pending beyond provider-mode shape.
  - Live proof if configured: `validation_artifacts/openai/call-receipt.json` (`provider_mode = openai_live`, `model_identity = gpt-5.4-nano`, endpoint `responses`, request id `83c9b714-b6e8-4c96-aa2a-d6102f08d02a`).
  - Candidate digest: `sha256:72da24c6dcf737cdcd2db83c57a90ed59a8564839aa7a0eaa245a83464c5b85c`
  - Status: Partial, not checked. Live call proof is current, but offline fixture execution and complete provider-mode red/green/tamper proof remain open.

## Gate 96 - promptfoo Eval, Red-Team, Regression, And Provider Separation

- [ ] promptfoo is installed, pinned, package-inventoried, provider-separated, and CLI-governed; raw promptfoo output is observation only.
  - Evidence: `package.json` pins `promptfoo` to `0.121.17` with `packageManager: pnpm@11.1.2`; `pnpm-lock.yaml` binds the resolved package; `pnpm-workspace.yaml` explicitly sets the generated native/browser build approvals to `false`; `target/debug/ultragoal --root . promptfoo prove --receipt validation_artifacts/promptfoo/adapter-receipt.json` minted a CLI receipt with `raw_promptfoo_authority = observation_only_until_cli_parsed_receipt`, provider modes, registry digests, and completion/readiness/release/final-packet/update_goal/product-success blockers.
  - Install receipt: `validation_artifacts/promptfoo/adapter-receipt.json` (`status: pass`, source-local adapter boundary only; `install_lifecycle_status: build_scripts_not_approved_no_eval_execution_claimed`).
  - Config path: `docs/promptfoo-provider-registry.json`; `docs/promptfoo-suite-registry.json`; `schemas/promptfoo-adapter-receipt.schema.json`.
  - Package entries: `plugin-manifest-draft.json` lists `package.json`, `pnpm-lock.yaml`, `pnpm-workspace.yaml`, promptfoo registries, promptfoo schema, `validation_artifacts/promptfoo/adapter-receipt.json`, and the promptfoo CLI/audit source files.
  - Candidate digest: `sha256:0e96ed517c2025f1fa31d11e47566b1f2d7767178abeb6a87da24d6e2cd83dd4`.
  - Status: Partial; adapter setup/provider-separation boundary is current, but full promptfoo eval/red-team/regression coverage and live/offline parsed result authority remain open.

- [ ] Repo-owned promptfoo suites cover Harness improvement loops, validator remediation, claim ceilings, Product Fitness, Product Cohesion, Product Success, review-packet language, setup/retrofit, active-repo rollout, and model/provider comparisons.
  - Evidence: Initial suite registry exists for claim-ceiling/provider-separation smoke coverage only; it explicitly does not close the complete Gate 96 suite scope.
  - Suite registry: `docs/promptfoo-suite-registry.json`.
  - Eval ids: `claim-ceiling-provider-separation-smoke` only.
  - Provider ids: `no_network_deterministic`, `offline_fixture`, `local_mock`, `openai_live`.
  - Candidate digest: `sha256:0e96ed517c2025f1fa31d11e47566b1f2d7767178abeb6a87da24d6e2cd83dd4`.
  - Status: Open; broad suite coverage for improvement loops, product gates, setup/retrofit, rollout, and model/provider comparisons is not complete.

- [ ] promptfoo suites contain law ids, claim ids, source trace/feedback binding, red cases, green cases, tamper cases where applicable, expected failure reasons, provider boundaries, prompt/input digests, output digests, current candidate digests, and promotion paths.
  - Evidence: `docs/promptfoo-suite-registry.json` now includes law ids, claim ids, provider ids, case counts, expected failure reasons, a source-trace binding marker, promotion path, and adapter-only claim ceiling for the initial smoke suite.
  - Validator check: `validator/src/audit/promptfoo/mod.rs` plus `validator/src/cli/promptfoo/*` reject stale/wrong digest receipts, overbroad raw promptfoo authority, missing blockers, wrong schema/status, and invalid observability binding.
  - Red fixtures: Pending beyond focused unit bad-path tests.
  - Green fixtures: Focused source-local green path in `validator/src/cli/promptfoo/tests.rs`.
  - Tamper fixtures: Pending.
  - Candidate digest: `sha256:0e96ed517c2025f1fa31d11e47566b1f2d7767178abeb6a87da24d6e2cd83dd4`.
  - Status: Partial; registry shape and receipt enforcement exist, but full red/green/tamper fixture coverage remains open.

- [ ] Validator rejects promptfoo pass accepted without CLI receipt, wrong-candidate results, live-provider proof without key receipt, offline-provider proof used as live proof, missing red cases, missing rubric, altered result JSON, and promptfoo output accepted despite failed cases.
  - Evidence: Production package audit now requires `validation_artifacts/promptfoo/adapter-receipt.json` and validates schema/status/current candidate digest/file digests/raw-authority blockers/observability binding through `validator/src/audit/promptfoo/mod.rs`.
  - Focused tests: `cargo test --offline promptfoo --lib --quiet` passed `2/2`, including source-local green receipt and wrong-candidate bad path.
  - Red fixtures: Pending; focused tests are not a substitute for the complete Gate 96 red/tamper catalog.
  - Candidate digest: `sha256:0e96ed517c2025f1fa31d11e47566b1f2d7767178abeb6a87da24d6e2cd83dd4`.
  - Status: Partial; wrong-candidate and raw-authority boundary are enforced, but the full forbidden-substitution matrix remains open.

## Gate 97 - HALO Ranked Harness Change Optimization

- [ ] HALO desktop app or HALO CLI/API availability is detected and capability-receipted with invocation mode, version/build identity when available, privacy boundary, authority class, allowed claims, and required receipts.
  - Evidence: `/Applications/HALO.app` observed by `target/debug/ultragoal --root . halo capability prove --receipt validation_artifacts/halo/capability-receipt.json`; bundle id `net.inference.halo`, bundle version `0.1.17`, `cli_available = false`, `api_available = false`. `docs/halo-adapter-registry.json` declares `desktop_manual`, `cli`, `api`, `fixture`, and `unavailable` modes plus forbidden substitutions. `validator/src/cli/halo/*` and `validator/src/audit/halo/mod.rs` enforce same-candidate receipt status, adapter registry digest, observability binding, manual-only authority, and readiness/update_goal/product-success blockers.
  - Capability receipt: `validation_artifacts/halo/capability-receipt.json` (`status = pass`, capability detection only).
  - Invocation mode: `desktop_manual`; authority class `manual_observation_only`; supported claim `halo_desktop_manual_capability_observed`.
  - Candidate digest: `sha256:483a45c9cc9fcf5fa643591b10e79034b1fb9b1543b56d94265bbedf08ef105b`; digest stayed stable after receipt write.
  - Status: Partial, not checked. Capability detection and manual-only claim blocking are current, but stable CLI/API ranking, typed ranked-change records, Codex handoff linkage, validation closure, before/after telemetry, and improvement-loop promotion remain open.

- [ ] HALO adapter consumes only CLI-generated typed inputs from failure clusters, eval results, trace summaries, product findings, cost/performance data, and claim impacts; raw private logs, secrets, unrestricted repo dumps, and unredacted local paths are forbidden.
  - Evidence: `docs/halo-adapter-registry.json` privacy boundaries forbid raw private logs and secrets; `validation_artifacts/halo/capability-receipt.json` blocks HALO ranked-change authority until a CLI adapter exists.
  - Input schema: Open; no typed HALO ranking input schema is complete yet.
  - Redaction proof: Open beyond the capability receipt secret-shape guard in `validator/src/cli/halo/proof.rs`.
  - Validator check: `halo-ranked-harness-change-optimization` currently enforces capability receipt and forbidden-substitution policy only.
  - Candidate digest: `sha256:483a45c9cc9fcf5fa643591b10e79034b1fb9b1543b56d94265bbedf08ef105b`.
  - Status: Partial, not checked. Input generation from typed failure clusters/evals/traces/product/cost data remains unimplemented.

- [ ] HALO objectives are typed, explicit, and receipt-bound; HALO output is parsed into ranked-change records with rank, hypothesis, expected effect, evidence ids, affected laws/files/surfaces, cost/risk estimate, validation plan, forbidden shortcuts, and claim impact.
  - Evidence: Capability receipt forbids `halo_ranked_change_authority` and `halo_recommendation_readiness` claims until typed ranking and validation exist.
  - Objective schema: Open.
  - Ranking receipt: Open.
  - Parser tests: Open; current focused tests cover capability green path and overbroad-authority bad path only via `cargo test --offline halo --lib --quiet`.
  - Candidate digest: `sha256:483a45c9cc9fcf5fa643591b10e79034b1fb9b1543b56d94265bbedf08ef105b`.
  - Status: Partial, not checked. No HALO ranked-output parser or objective-bound ranking receipt exists.

- [ ] HALO recommendations are linked to Codex handoffs, implementation changes, validation receipts, before/after telemetry, and standards/fixture/schema promotion before any improvement claim can pass.
  - Evidence: `docs/halo-adapter-registry.json` and `validation_artifacts/halo/capability-receipt.json` block HALO recommendation, readiness, update_goal, Product Success, final-packet, registry/reviewer, and improvement-loop closure claims.
  - Handoff: Open.
  - Implementation link: Open.
  - Validation receipt: Open.
  - Candidate digest: `sha256:483a45c9cc9fcf5fa643591b10e79034b1fb9b1543b56d94265bbedf08ef105b`.
  - Status: Partial, not checked. HALO recommendations cannot support improvement claims until handoff, implementation, validation, before/after telemetry, and standards/fixture/schema promotion are deterministic and current.

## Gate 98 - Self-Improving Domain-Agent Pattern And Tax-Agent Generalization

- [ ] Plugin provides a domain-agent improvement pattern generalized from the tax-agent article for domain workflows, product workflows, support workflows, review workflows, compliance workflows, analysis workflows, and expert-evaluable outputs.
  - Evidence:
  - Pattern docs/templates:
  - Schemas:
  - Package entries:
  - Candidate digest:
  - Status:

- [ ] Domain packs declare task type, expert role, evidence level, allowed data, forbidden data, rubric, eval cases, failure taxonomy, product claim impact, retention/redaction policy, and setup/retrofit install path.
  - Evidence:
  - Domain pack schema:
  - Fixture pack:
  - Validator check:
  - Candidate digest:
  - Status:

- [ ] Domain improvement loop binds realistic task traces, expert feedback, expected answers/rubrics, error taxonomy, eval generation, regression cases, ranked repair, before/after validation, and claim guards.
  - Evidence:
  - Receipt:
  - Eval cases:
  - Before/after proof:
  - Candidate digest:
  - Status:

- [ ] Validator rejects toy-only evals, feedback without task trace, model feedback mislabeled as human expert feedback, missing rubrics, private data leaks, eval-only product success, and domain improvement without before/after proof.
  - Evidence:
  - Red fixtures:
  - Green fixtures:
  - Tamper fixtures:
  - Candidate digest:
  - Status:

## Gate 99 - Setup And Retrofit Skill Deep Integration

- [ ] `agent-first-repo-init` installs or fail-closes observability, command inventory, improvement-loop registry, research registry, OpenAI key policy, promptfoo config, HALO adapter policy, telemetry schemas, eval schemas, domain pack templates, feedback schemas, Codex handoff templates, Rust DevX where applicable, TypeScript DevX where applicable, and active-repo rollout templates.
  - Evidence:
  - Skill/template paths:
  - Setup receipt:
  - Candidate digest:
  - Status:

- [ ] `agent-first-repo-init` detects repo type through typed evidence: Rust backend, TypeScript frontend/UI, mixed Rust/TypeScript, CLI-only, plugin-only, app/service, docs-only, product-facing, review-only, or target-repo audit fixture.
  - Evidence:
  - Detection schema:
  - Red fixtures:
  - Green fixtures:
  - Candidate digest:
  - Status:

- [ ] `agent-first-repo-retrofit` audits observability coverage, improvement-loop coverage, command inventory, law-bearing commands, opaque failures, receipt telemetry binding, promptfoo suites, HALO status, OpenAI key policy, eval-to-law mapping, domain pack needs, Rust/TypeScript DevX gaps, package inventory gaps, and claim guard gaps.
  - Evidence:
  - Retrofit receipt:
  - Fitting inventory:
  - Validator check:
  - Candidate digest:
  - Status:

- [ ] Setup/retrofit fitting inventory uses `fitted`, `partially_fitted`, `unfitted`, and `not_applicable_with_typed_reason`; row-shape-only, docs-only, prose-only, missing receipts, and unsupported positive claims fail.
  - Evidence:
  - Inventory path:
  - Red fixtures:
  - Candidate digest:
  - Status:

## Gate 100 - Cross-Repo Harness Rollout, Active Repo Inventory, And Propagation

- [ ] Active-repo registry tracks every plugin-activated repo with category-safe identity, activation surface, plugin version, source/install/cache/app status, language/runtime classes, product surfaces, observability fitting, improvement-loop fitting, OpenAI key policy, promptfoo status, HALO status, Rust/TypeScript DevX status, setup/retrofit receipts, last audit digest, claim ceiling, and next required repair.
  - Evidence:
  - Registry path:
  - Validator check:
  - Candidate digest:
  - Status:

- [ ] Rollout modes are typed and non-substitutable: source self-compliance, fresh-init target, retrofit target, installed plugin target, cache package target, app-registry target, and review-packet target.
  - Evidence:
  - Schema:
  - Red fixtures:
  - Green fixtures:
  - Candidate digest:
  - Status:

- [ ] Cross-repo rollout receipts prove which repos were evaluated, fitted, fail-closed, claim-blocked, and next-repair assigned without leaking private paths into public/package artifacts.
  - Evidence:
  - Receipt:
  - Privacy proof:
  - Candidate digest:
  - Status:

- [ ] Validator rejects source proof reused for another repo, private repo paths in public packets, installed proof used as app proof, promptfoo/HALO/OpenAI proof from one repo used for all repos, and active repo marked complete with partial fitting.
  - Evidence:
  - Red fixtures:
  - Green fixtures:
  - Candidate digest:
  - Status:

## Gate 101 - Rust And TypeScript Developer Experience Integration

- [ ] Rust DevX setup/retrofit keeps pinned toolchain, Cargo substrate, rustfmt, clippy, metadata, nextest, llvm-cov, deny, audit, vet, SBOM, sccache where available, locked installs, proptest, fuzzing, snapshots, performance tools, tracing/OpenTelemetry, serde/schemars/path diagnostics, and release tooling under CLI authority.
  - Evidence:
  - Rust tool inventory:
  - Setup/retrofit receipts:
  - Candidate digest:
  - Status:

- [ ] TypeScript/UI setup/retrofit requires pinned Node Active LTS, pnpm, lockfile, strict TypeScript, Vite, ESLint v9 flat config, typed typescript-eslint, Prettier, Vitest, Playwright, accessibility checks, Testing Library, MSW, V8 coverage, runtime parsers, dependency/bundle/performance/security tooling, memory/resource receipts, and GC receipts where applicable.
  - Evidence:
  - TypeScript tool inventory:
  - Setup/retrofit receipts:
  - Candidate digest:
  - Status:

- [ ] TypeScript external values from JSON, network, DOM, storage, URL params, postMessage, env vars, generated files, browser APIs, plugin messages, and third-party packages enter as `unknown` and are parsed into typed authority.
  - Evidence:
  - Parser schemas:
  - Red fixtures:
  - Green fixtures:
  - Candidate digest:
  - Status:

- [ ] Validator rejects raw Cargo/pnpm/tsc/eslint/vitest/Playwright/Vite/Storybook/Lighthouse/package-manager output as Harness claim authority and rejects TypeScript `any` authority leaks, unparsed JSON, missing strict config, cache dishonesty, and UI proof substitution.
  - Evidence:
  - Red fixtures:
  - Focused tests:
  - Candidate digest:
  - Status:

## Gate 102 - Feedback, Eval, Telemetry, Privacy, Retention, And Data Minimization

- [ ] Data-class registry covers public package artifacts, private local receipts, raw private traces, redacted traces, model prompts, model outputs, feedback comments, domain examples, eval cases, screenshots/videos, logs, metrics, spans, HALO inputs/outputs, promptfoo results, Codex handoffs, and final packets.
  - Evidence:
  - Registry path:
  - Validator check:
  - Candidate digest:
  - Status:

- [ ] Every data class declares retention, redaction, package-inclusion eligibility, child-agent eligibility, model-call eligibility, query eligibility, digest strategy, and claim support.
  - Evidence:
  - Schema:
  - Red fixtures:
  - Candidate digest:
  - Status:

- [ ] Raw private session logs, user prompts, secrets, local paths, sensitive screenshots, private traces, and raw model prompts do not become durable package artifacts; only category-only, redacted, digest-bound evidence may support durable claims.
  - Evidence:
  - Redaction proof:
  - Package scan:
  - Candidate digest:
  - Status:

- [ ] Validator inspects logs, metrics, traces, receipts, query output, promptfoo results, HALO inputs, OpenAI call receipts, Codex handoffs, eval cases, screenshots, videos, and final packets for leaks, retention violations, and redaction-status tampering.
  - Evidence:
  - Red fixtures:
  - Green fixtures:
  - Tamper fixtures:
  - Candidate digest:
  - Status:

## Gate 103 - Improvement Surface Separation And Non-Substitution

- [ ] Proof surfaces are separately typed for source, installed plugin, cache, app registry, Plugins UI, marketplace, install button, launcher runtime, reviewer exposure, final packet, target repo, active repo, promptfoo eval, HALO ranking, OpenAI model output, Codex handoff, product journey, domain task, and improvement-loop closure.
  - Evidence:
  - Surface schema:
  - Validator check:
  - Candidate digest:
  - Status:

- [ ] Every receipt names one primary surface and dereferences lower-level surfaces by path, digest, schema, status, currentness, surface id, and claim class.
  - Evidence:
  - Receipt schema:
  - Focused tests:
  - Candidate digest:
  - Status:

- [ ] Validator rejects observability proof as improvement closure, promptfoo proof as Product Success, HALO ranking as readiness, OpenAI output as deterministic law, source proof as installed/cache/app proof, and installed proof as reviewer exposure.
  - Evidence:
  - Red fixtures:
  - Green fixtures:
  - Tamper fixtures:
  - Candidate digest:
  - Status:

- [ ] Final-packet proof dereferences lower-level evidence without creating circular dependencies with source audit, self-law, update-goal eligibility, or improvement-loop closure.
  - Evidence:
  - Final-packet proof:
  - Cycle checks:
  - Candidate digest:
  - Status:

## Gate 104 - Research-To-Standards, Source Obligations, Traceability, Fixtures, And Package Closure

- [ ] Gates 93-103 have canonical law ids, explanatory law-family aliases only where useful, standards rows, source obligations, foundational trace entries, mandatory-law surface entries, schema enums/check ids, validator checks, red fixtures, green fixtures, tamper fixtures, valid fixtures or receipt requirements, package inventory entries, plugin cohesion manifest entries, setup/retrofit templates, command inventory rows, improvement-loop inventory rows, claim guards, final-packet fields, and update_goal blockers.
  - Evidence:
  - Law-surface paths:
  - Package inventory:
  - Candidate digest:
  - Status:

- [ ] Gates 93-103 include schemas for research registry, improvement-loop registry, trace-feedback receipt, feedback-cluster receipt, eval-generation receipt, promptfoo receipt, HALO receipt, OpenAI call receipt, Codex handoff receipt, setup/retrofit fitting receipt, active-repo rollout receipt, TypeScript DevX receipt, and surface-separation proof.
  - Evidence:
  - Schema paths:
  - Validator checks:
  - Candidate digest:
  - Status:

- [ ] Source audit, red fixture report, CLI self-law, update_goal eligibility, final packet, setup/retrofit, package inventory, and active-repo rollout all include Gates 93-103.
  - Evidence:
  - Source audit:
  - Red report:
  - Self-law:
  - Update-goal eligibility:
  - Candidate digest:
  - Status:

- [ ] Validator rejects prompt-only, checklist-only, standards-row-only, source-obligation-only, trace-row-only, package-entry-only, fixture-name-only, red-only, green-only, receipt-only, claim-ceiling-only, reviewer-only, or setup-template-only Gate 93-103 compliance.
  - Evidence:
  - Red fixtures:
  - Green fixtures:
  - Tamper fixtures:
  - Candidate digest:
  - Status:

## Gate 105 - Measured Improvement, Regression Prevention, And Harness Evolution

- [ ] Improvement metrics are defined for time-to-diagnosis, time-to-repair, rerun count, stale-receipt recurrence, wrong-digest recurrence, opaque-failure recurrence, claim-theater escape count, source-audit failure recurrence, red-fixture drift recurrence, Product Fitness substitution recurrence, active-repo rollout fitting percentage, eval trend, command latency, and manual-spelunking burden.
  - Evidence:
  - Metric registry:
  - Telemetry source:
  - Candidate digest:
  - Status:

- [ ] Every improvement metric has schema, baseline, current value, collection command, telemetry source, receipt path, candidate digest, confidence, and claim impact.
  - Evidence:
  - Baseline receipt:
  - Current receipt:
  - Candidate digest:
  - Status:

- [ ] Improvement claims compare before/after values using same-surface telemetry, explain regressions, and are protected by evals, fixtures, source-audit checks, setup/retrofit checks, package inventory checks, active-repo rollout checks, and standards-gardener promotion.
  - Evidence:
  - Before/after proof:
  - Regression suite:
  - Standards-gardener proof:
  - Candidate digest:
  - Status:

- [ ] Validator rejects fabricated baselines, stale baselines, wrong-surface comparisons, one-run improvement claims, no-regression-suite claims, ignored regressions, metric-without-telemetry claims, active-repo omissions, cherry-picked metrics, and anecdotal improvement claims.
  - Evidence:
  - Red fixtures:
  - Green fixtures:
  - Tamper fixtures:
  - Candidate digest:
  - Status:
