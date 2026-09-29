> **Approved for execution, 2026-09-28.** Tree approved this spec. Execution started at 02:07:36 UTC; the five-hour target is 07:07:36 UTC (00:07:36 Pacific). Earlier document-only and unset-T0 wording below describes preparation and is superseded by this approval. The sole active ExecPlan records live execution; all D1–D13 remain required.

Template choice: ChatPRD **Product Spec for AI Agents** (id 4317).

# Spec Header

## Field Descriptions

| Field | Value |
| --- | --- |
| Product | UltraGoal Next — complete initial dogfooding with enforceable agent legibility |
| Document role | Proposed completion supplement to [PRD.md](PRD.md); the PRD, main `GOAL.md`, and `.agents/decisions.md` remain governing. This is not a replacement PRD or a second active execution plan. |
| Product owner | Terry Noblin (Tree) |
| Conductor / integration owner | Existing `/root` task; owns scope reconciliation, main checkout, global receiving, acceptance, and user reporting. |
| Implementation owner | Existing `/root/sol_owner_recovery`, `gpt-6-sol` / `medium`, in the existing `ultragoal-next-owner` worktree. Retain this owner through corrections and integration support. |
| Judgment owner | Existing `/root/astra_completion_review_now`, `gpt-6-astra` / `xhigh`; document author and focused independent reviewer. Requested settings are not proof of effective settings. |
| Status | **PROPOSED — document only.** Root reviews and presents this supplement before implementation starts. No product, configuration, installed package, or adopted requirement is changed by this document. |
| Starting candidate | `b192382363d860b4`, functionally accepted CL-USABLE-LOOP; 162 bound source entries and eight package payloads matched. `ultragoal` SHA-256 `22bdb2ea4946b9dbe680201b008698c6dd98a63c33cf4f032c7a5e7dbe0a7158`; core SHA-256 `b64f8f03c5c9092335d1d6de8ce519a9276a1ba3a5ab7f2a01db975816826ac3`. |
| Existing evidence | Main `.codex-worktree/final-b192/`; current report `audits/ultragoal-current.html`; source/legibility review board post `01a0e599-036b-71a3-be2a-0a23fb151c73`. Evidence paths identify observations, not authority. |
| Recommended completion deadline | **T0 + 5 hours**, where T0 is the recorded actual implementation execution start after release. T0 is **not yet set**. Include owner investigation, edits, tests, heavy-build waits, review, corrections, integration, installed receiving, and cleanup. |
| Estimate ranges | **Optimistic 2.5–3.5 hours; likely 3.5–5 hours; risk case 5–8 hours** of remaining elapsed agent execution. These are judgment ranges, not measured probabilities. A required general checker or widespread semantic redesign needs a new estimate rather than an invented upper bound. |
| Calendar dependencies | Waiting for Tree’s decision, idle time between messages, and an explicit user pause are not active engineering effort. Report those delays separately and move the calendar ETA transparently. Ordinary build/tool/review waits during execution remain inside the estimate. |
| Stack | Preserve Bend 2.0.27 and the recorded Rust/toolchain initially. Bend owns product decisions; Rust supplies native observation/transport. Python/shell may remain developer test/build tools. No toolchain upgrade in this milestone. |
| Authority | This authoring task permits only this file. Existing task-needed Keychain/live Jev authorization persists; no recurring credential ceremony. Implementation, supported package update, and other effects follow the actual current user grant and host controls when execution is released. No default-branch merge, public release, unrelated global changes, or EJ retirement is implied. |

**Superseded timing history:** the initial draft began at 2026-09-28 01:34 UTC and was delivered at about 01:44 UTC. It attempted to fit a requested **1–3 hour** window ending **02:34–04:34 UTC**. Tree subsequently withdrew that forced window and requested a realistic agent-execution estimate. Those timestamps remain history only; they are not current deadlines. Completed planning is retained work, not silently billed again as future implementation.

# The Problem

## Required Problem Statement

Tree and fresh coding agents need to maintain and use the complete UltraGoal product without reconstructing its current architecture from chat history. The installed `b192` product has passed dirty fit/check/verify, failure/recovery, automatic task sessions, default Jev, and deep-directory 10k-file/100 MB receiving. Its development surface still directs agents toward the legacy implementation, does not provide one executable Next verification route, and leaves important ownership and validation obligations unenforced.

The immediate risk is a plausible change that passes the wrong checks, changes a machine outcome by rewording an error, or adds an effect outside its intended owner. A prior proposed chunk normalization passed generic proof checking while changing predicate meaning; the independent review caught it and the final candidate removed it. This illustrates the need for discriminating behavior checks alongside proof and architecture rules. It does not establish a defect in the accepted final projection.

## Problem Details

| Question | Evidence-grounded answer |
| --- | --- |
| Who is affected? | Tree as operator; Astra as ordinary product user; Sol or another coding agent making a future change. |
| What does first dogfooding mean here? | Tree can use the full installed product and delegate a bounded maintenance task using repository instructions alone. The relevant ownership, meaningful verification, failure handling, and integration route are discoverable and enforced. |
| What is already done? | Functional `b192` acceptance, single installed UG route, fresh native-host fit application, default narrow Jev access, dirty preservation, and representative scale receiving. Preserve this evidence. |
| What remains? | Current routing; a maintained Next gate; material effect/dependency and output ownership with negative controls; typed consequential errors/handoffs; selective factoring of confusing ownership; truthful generated/cache inventory; a fresh-agent maintenance receiving task. |
| What is the frequency or cost of reconstruction? | **Unknown.** Source and observed review mistakes establish mechanisms; no matched study measures agent time or token savings. |
| Why now? | Tree explicitly wants agent legibility and best practices enforced before treating the product as ready for initial dogfooding. |
| What does the EJ count mean? | The retained red 565 mixes missing enforcement, unsupported analysis, data representation flags, and conventions. It is neither 565 proved bugs nor an acceptable substitute for understanding them. |
| What is outside this delta? | New product features unrelated to legibility, universal language semantics, a new authority service, whole-protocol type migration, broad stylistic refactoring, new calibration campaigns, and public-release qualification. Existing required capabilities remain in scope for preservation. |

## Evidence Inventory

Paths below are relative to the main repository unless stated otherwise. Let **E** mean `engines/ultragoal-next/`.

| Evidence | Observation / strength | Ceiling |
| --- | --- | --- |
| `GOAL.md`; `.agents/decisions.md`; canonical PRD | Adopted scope and revised completion/performance standards. | Current user steering controls; old status rows are not current proof. |
| `.codex-worktree/final-b192/runtime/RESULT.json` and raw operations | Fresh installed Astra: failure/explain/repair, six requirements, 13 project tests, default Jev, off control, sessions/recovery, preservation; subsequent nonempty fit/apply/check passed. | Functional receiving on a reused diagnostic fixture, not independent improvement measurement. |
| `.codex-worktree/final-b192/deep-summary.json` and raw reports | 10,000 files, 100,332,000 bytes, depth 34; check 5/5, index 10,000 fact sets, select 16,000 candidates; unchanged tree. | Single loaded-host observations: check/index/select about 8.16/5.95/7.40 s. No p95 claim or universal scale guarantee. |
| Root `ARCHITECTURE.md:3–9`, `Cargo.toml:2`, `scripts/check-product:38–49` | Required navigation and aggregate product checks still name the legacy `validator` product. | Specific source finding; does not invalidate separately recorded Next receiving. |
| E `README.md:3,52`; `IMPLEMENTATION_STATUS.md:93,99,103`; `SEMANTIC_CONTRACT.md:4,125` | Actively routed material includes obsolete installation, credential, implementation, and calibration wording. | Repair current guidance; preserve frozen historical results. |
| E `src/main.rs:464–483`; `src/segmented.rs:222` | Machine error classification depends on substrings; “changed since listing” misses “changed during”/“stale.” | Concrete diagnosis defect inferred from exact code; not false verification. |
| E `QueryPlan.bend`, `QueryCompute.bend`, `Predicate.bend`, `QueryValue.bend`; `tests/query_revisions.py` | Projection meaning and cache equality have different proof obligations. Current correction has direct-versus-retained negative controls. | Preserve the independent oracle; generic proof success alone is insufficient. |
| E `docs/legibility/` and owner `.codex-worktree/ultragoal-next-owner-b192/ej-check.json` | Four registered boundaries; incomplete broader effect/dependency ownership; no output registry. Raw EJ red 565 retained. | Review sampled important flows and all finding families, not every diagnostic individually. |
| E `build.py:14`, `package.py:6–11`, `docs/legibility/sources.json` | Build inputs include Ruff caches and source-root `ug-core`; build/package enumeration differs on absolute versus relative `target` path components. | Requires a shared inventory and generated-artifact ownership, not deletion by pattern. |

## Problem Quality Bar

- The user, maintenance journey, concrete failure mechanisms, and accepted starting product are identified.
- Every proposed MUST below names an observable result and a failure that must be caught.
- Unknown agent productivity, full architectural coverage, and deadline feasibility remain explicit.
- No requirement is removed to meet the clock. A missed MUST produces a named HOLD and a continuation estimate.

## Agent Prompt: Strengthen The Problem

“Use the current goal and repository to locate one way a future agent could make a plausible wrong change. Name its owner, the missing or misleading check, and the existing evidence that should distinguish the wrong result. Do not reopen an accepted functional surface without an affected dependency or contradictory observation.”

# The Bet

## Required Bet Format

If we align UltraGoal’s development entrypoints with its installed implementation, enforce the actual ownership/validation boundaries, and make consequential handoffs explicit, then a fresh agent can safely make a representative change and use the full installed product without chat lore, demonstrated by discriminating negative controls and an actual maintenance-and-receiving task while preserving all accepted functional requirements.

## Bet Details

| Dimension | Proposed decision |
| --- | --- |
| What ships? | One source-bound successor package plus current repository guidance, an executable Next gate, adopted legibility checks/fixtures, and an acceptance result. |
| Product scope | All current `fit`, `inspect`, `check`, `verify`, `index`, `select`, `advise`, `explain/recovery`, reuse, native/plugin integration and migration-support behavior. |
| New user behavior | A fresh agent discovers the right owners and checks; Tree does not manage session handles, credential flags, or per-command receipts. |
| Enforcement scope | This repository’s actual production authority/dependency/identity boundaries and truthful output handling. This milestone does not promise a universal architectural analyzer for arbitrary Bend/Python projects. Unsupported mandatory analysis remains explicit and cannot earn a pass. |
| Policy ownership | Bend retains product rule decisions. Native static/compiler tools can check repository structure and supply observations. The developer gate is verification of this implementation, not a second runtime policy engine or a source of effect authority. |
| Key assumption | Existing EJ observations, compiler checks, targeted source checks, and runtime fixtures can enforce the adopted dogfood boundary set without building a new general semantic checker. Validate this assumption early. |
| Main time risk | Completing meaningful ownership coverage may reveal a boundary that requires more than a small typed adapter or factoring change. Do not mask it with a permissive registry entry. |
| Improvement claim | “Initial dogfooding requirements passed on the named candidate.” No quantified productivity, cost, or success-rate gain. |

## Example Bet

A fresh agent asked to improve pressure diagnostics follows the root router to Next, finds the typed error and source-custody owners, runs the advertised check, and sees a deliberately misclassified changed-source result fail. It makes the bounded correction, keeps cancellation and unknown outcomes intact, and verifies installed recovery. It never edits the legacy `validator` or declares all raw EJ findings harmless.

## Good Bet Checklist

- [x] Preserves the complete adopted product and the Bend/Jev split.
- [x] Names the new maintenance capability and a concrete wrong implementation.
- [x] Uses actual user/system paths and independent task evidence.
- [x] Separates enforcement coverage from generic checker counts.
- [x] Allows an honest deadline miss without changing success criteria.

## Agent Prompt: Make The Bet Falsifiable

“Try the advertised repository workflow with no chat history. If it leads to the wrong code, accepts an unowned effect, accepts stale evidence, misses the seeded regression, or needs undocumented assistance to choose the check, the dogfooding bet has not passed. Preserve the failure and repair its cause.”

# Success Criteria

## Required Success Criteria

All rows marked **MUST** are part of initial dogfooding. Retained evidence is acceptable only where the owner and reviewer identify unchanged dependencies. “Later” never removes an adopted requirement.

| ID / priority | Required outcome and trace | Acceptance signal | Anti-signal / response |
| --- | --- | --- | --- |
| D1 MUST | Preserve full capability set and all original requirements. `GOAL.md`, PRD. | Requirement map names each capability, actual owner, current evidence and any affected receiving action; none disappears from reports or guidance. | A feature is omitted to finish sooner: HOLD; obtain an explicit scope decision before any sacrifice. |
| D2 MUST | Current navigation and operation routes. Review finding 1. | Root architecture/README identify Next; legacy routes are labeled; current engine status/credential/calibration wording agrees with code. A fresh agent reaches the right source and checks without board/history. | Agent edits/tests `validator` for a Next change, or repeats consumed calibration due to stale guidance. |
| D3 MUST | One maintained executable Next gate connected to root `scripts/check`. | Offline quick/full modes bind the candidate, run the relevant real tests, expose exact failures, and reject stale binaries/probes or a seeded Next defect. Live/provider tests are explicit, not hidden in routine checks. | Green root check while a required Next guard is missing or a Next regression is deliberately present. |
| D4 MUST | Material production effect/dependency ownership and negative enforcement. Review finding 2. | Every discovered real production effect route in the declared Next inventory is assigned to an actual owner with permitted callers, validation and a checked outcome. High-consequence native/process/socket/provider/ledger/custody paths have executable red controls. Unreviewed real effects or unprofiled production dependencies cannot disappear behind a count baseline. | Bulk registration, an empty validation record masquerading as proof, a new direct effect outside its adapter, or a material site left unreviewed: HOLD. |
| D5 MUST | Typed consequential outcomes and trustworthy parsing at the handoffs being maintained. Review finding 3. | Fix the changed-source classification defect; carry cancellation, source change, pressure and credential refusal as distinguishable outcomes through their handling boundary. Separate public error category from free-text context. Parsed session/native/provider records used for decisions have named validated owners. | Rewording a message changes machine disposition; arbitrary imported JSON becomes a current native observation; a wrapper supplies no real validation. |
| D6 MUST | Selective modularity with visible state ownership. Review finding 4. | Factor the smallest mixed responsibility seam that otherwise prevents D4/D5 enforcement; use explicit imports on touched seams. Document actual local/remote/core/lane/ledger lifetimes and allowed calls in the existing ownership map. An agent can identify all affected callers without reconstructing an unrelated subsystem. | Splitting solely at 250 lines; duplicating policy; global state gains another writer; renamed files retain the same hidden coupling. |
| D7 MUST | Truthful authored/generated/cache inventory. Review finding 5. | One shared root-relative input enumeration for build/package/legibility; generated outputs have generators; true caches do not change source identity; authored semantic/test changes do. Source relocation does not change inclusion rules. | Ruff output or obsolete binary treated as authored interface; excluded executable/test content used to manufacture a pass. |
| D8 MUST | Correctness, freshness and user preservation. Existing goal. | Zero observed lost user work, stale admission, hidden obligations, credential disclosure or differential mismatches in required checks; source/contract changes and unavailable observations stay explicit. | Any prohibited outcome: reject candidate and repair before promotion. |
| D9 MUST | Representative scale and pragmatic performance. Existing goal. | Deep-directory 10k files/100 MB remains usable, with coverage/progress/cancellation/recovery and named omissions. No arbitrary total file/size ceiling; RAM policy remains adaptive. Record elapsed/load/RSS/output and investigate an obvious regression on a matched affected path. | Whole-input refusal at the test shape, fixed small product cap, silent omissions, hung cancellation, or repeated avoidable stalls. Old p95/overhead percentages are not gates. |
| D10 MUST | Default installed Jev and semantic qualification preserved. Existing goal. | Existing narrow credential route works without recurring flags; truthful disclosure and `--local` remain. Qualified recovery-state blocking retains its recorded precision/lower-bound/recall evidence; other families stay advisory. | A new unqualified blocking family, altered thresholds without affected requalification, credential exposure, or silent default provider disablement. |
| D11 MUST | Automatic ordinary use, failure, recovery and fit through installed path. Existing goal. | Exact final package discovery; dirty check/failure/explain/repair/repeat; task-owned session expiry/stale/restart/cleanup; nonempty fit/native apply if its paths changed; relevant native syntax/project tests; one active UG route. | Source-only success substituted for installed behavior, user manages handles, stale fit overwrites later work, duplicate active UG routes. |
| D12 MUST | Fresh-agent maintenance receiving. | One fresh Astra task uses repository instructions to perform a bounded real maintenance exercise in a disposable copy, locates the correct owners/checks for the three scenarios below, and rejects a seeded wrong alternative with the real gate. Root grades against task evidence. | Passing only because the task packet supplied the solution/file map, or success claimed from self-report without checked diff/test output. |
| D13 MUST | Safe integration and a concrete handoff. | Source identity, package, installed bytes and receiving evidence agree; conflict-rejecting main integration preserves unrelated edits; rollback candidate/evidence remain available; one concise report states remaining limits. | Blind overwrite, old binaries with new source, default-branch merge/release without authority, or hidden cleanup. |

**Explicitly proposed later work:** deep factoring of otherwise coherent algorithms/proofs; replacing every internal `String`/`Value`; optional `foo.rs` versus `foo/mod.rs` layout; `native_*` renaming without a real ownership gain; general Bend/Python semantic adapters; byte-level persistent resume; exhaustive retrieval/recall claims; new calibration or matched benefit campaigns; larger-scale stress; public release; unrelated EJ consumer retirement. These are not excuses to defer a D1–D13 obligation. If the owner finds one is necessary to satisfy a MUST, it becomes critical-path work and the deadline estimate changes.

## User Behavior Criteria

Tree receives a ready-to-use global plugin and a repository that a new agent can safely maintain. Ordinary use retains automatic session setup/reuse/recovery, compact explanations and the local/off switch. The next action for unknown/failed results is intelligible. An agent can run the right verification command without locating old worktree logs. Meaningful failures remain visible and recoverable; historical evidence is available through a compact pointer rather than copied into every prompt.

## AI Product Criteria

- Bend owns rule applicability, semantic admission, retry decisions, grouping/ranking and predicate meaning. Host adapters observe and transport; none mint permission.
- Existing `recovery-state` qualification is reused only with unchanged relevant model/rubric/policy inputs. No new campaign is needed for first dogfooding when those inputs are unchanged.
- Existing optional excerpt/shortlist omissions remain explicit and recoverable. Long lines excluded from bounded excerpts do not imply missing mandatory file coverage. No exhaustive-retrieval claim follows.
- “Complete” means the required dogfood criteria passed on the exact candidate, not zero EJ diagnostics or universal architectural certification.
- All attempts and corrections count. Billed cost and cached/uncached/input/output tokens are separate; unavailable values are **Unknown**, never zero.

## Example Success Criteria

1. A changed source emits a typed `input_changed` outcome even when the human explanation changes; cancellation remains cancellation and never becomes retryable success.
2. A new direct `Command`/filesystem effect outside its declared owner fails the legibility gate, while an authorized pure report formatter passes without a cosmetic wrapper.
3. A cache-file update does not alter the build source set; changing `QueryCompute.bend` does, and the gate requires the applicable direct-versus-retained tests.

## Success Criteria Quality Bar

Every MUST has a recipient-visible result, discriminating failure and owning proof surface. Enforcement scope is explicit. Unsupported analysis stays unavailable; it cannot close a required architectural claim. A source pass does not imply installed acceptance. A functional PASS does not retroactively certify every inherited style or checker obligation.

## Agent Prompt: Generate Success Criteria

“For each D1–D13 row, name the command or observation that could falsify it, the exact input/candidate and one plausible wrong result. Reuse unaffected b192 evidence. If a proposed check only counts files, declarations or green rows, replace it with an outcome or negative control.”

# The Evaluation

## Required Evaluation Summary

This is bounded engineering qualification and first dogfooding, not a comparative model campaign. Start with the accepted b192 evidence, add the new enforcement fixtures and affected regression checks, then perform one fresh-agent maintenance task and exact installed receiving. Root decides acceptance from actual diffs, source/build binding and raw results. Independent Astra reviews only the material changed boundaries.

The decision rule is: **all D1–D13 pass; zero observed prohibited outcomes; explicit limitations retained**. No fixed p95, overhead percentage, improvement percentage, universal line cap or 10× campaign is introduced.

## Measurement Plan

| Surface | Baseline and method | Required record |
| --- | --- | --- |
| Functional behavior | Reuse exact b192 receiving where dependencies are unchanged; rerun affected final installed paths. | Candidate, source/binary hashes, actual commands/results, preserved failure, cleanup. |
| Legibility enforcement | Inventory actual effects/dependencies; valid controls and seeded direct-bypass/missing-validation/stale-input controls. | Complete declared scope, reviewed owner/caller mapping, raw checker output, failing and passing controls. |
| Navigation | Fresh task starts with the repository, its instructions and user request, not this review’s answer map. | Which owner/check it selected, wrong routes, assistance/corrections, actual outcome. |
| Semantic coupling | Direct uncached versus retained/paged evaluation; malformed partials, source/contract changes and exact/prefix/wildcard scopes. | Zero mismatches; preserve observed failures and seed. |
| Performance | Existing deep34 shape, and a matched b192 affected path only if new behavior or an observed stall warrants it. | Wall/CPU as available, load, RSS, output, progress, cancellation/recovery. Shared load limits timing interpretation. |
| Costs | Available agent and Jev counters for this bounded work. | Input, cached input, uncached input, output, billed cost, attempts; mark missing categories Unknown. |
| Artifact integrity | Shared input enumeration; before/after manifest; clean-source build/package checks. | Authored/generated/cache classification, exclusions, changes and rejection reason. |

Do not repeatedly rebuild unchanged Bend or rerun a broad historical suite to age-refresh evidence. A source, dependency, toolchain, rule, host or package change determines what must be refreshed. The gate must still reject genuinely stale consumed artifacts.

## Kill / Scale / Graduate Thresholds

| Outcome | Action |
| --- | --- |
| Lost user work, credential disclosure, stale acceptance, weakened obligation or authority bypass | Reject candidate; preserve failure; same owner repairs. Keep/recover the accepted b192 installation through supported tools if the candidate was installed. |
| Missing owner/validation for a material real production effect; required analysis unsupported | HOLD the affected dogfood claim. Implement the missing boundary/check and update the remaining-work estimate. Never convert it into an advisory warning solely for schedule. |
| Checker flags a pure formatter, legal module layout, blanket line cap or unsupported all-language analysis | Classify against adopted semantics; preserve raw result. Add the applicable project disposition/positive control. Do not announce strict EJ green. |
| All D1–D13 pass on the exact integrated/installed candidate | Graduate to initial dogfooding, with the documented supported-host and evidence ceilings. |
| T0 + 5-hour deadline is no longer defensible without dropping a MUST | Preserve concrete work and accepted installation; report unmet IDs, the cause, and a revised completion ETA. Continue authorized work with the same owner. A reduced milestone requires Tree’s explicit consent; missing an estimate does not reduce scope or automatically stop work. |
| Additional polish after required checks pass | Stop this milestone. Record only useful revisit conditions in the existing plan; no automatic campaign or recurring monitor. |

## AI Evaluation Plan

Use three scenarios, all tied to real owners and deterministic oracles:

| Scenario | Fresh-agent task and proof | Wrong alternative that must be rejected |
| --- | --- | --- |
| S1 Predicate/projection | Locate the rule/argument/projection owner and its direct-versus-retained checks; inspect a small injected selector/projection regression in a disposable copy and make the minimal repair. | Ignore a required partial or accept unchanged cached admission for changed relevant inputs. |
| S2 Provider outcome | Locate the credential/dispatch/attempt/ledger boundaries; demonstrate a synthetic outcome/control without live keys. | Direct dispatch outside the owner, retry of an ambiguous billed attempt, or imported output treated as current evidence. |
| S3 Pressure/diagnosis | Locate typed source/pressure/cancellation handling and the installed explanation path; run the changed-source and partial-recovery control. | Text rewording changes the public error category, prior completed results vanish, or the failed source inherits old success. |

One fresh Astra task may execute one substantive repair and demonstrate the other two seeded controls. It must discover the relevant owners/checks for all three. It must not receive chat history, the prior reviewer’s file/line answers, or a solution patch. Sol prepares fixtures; root retains the independent expected outcomes. Score correctness, preservation, choice of verification and need for assistance; time/tokens are observations.

This receiving exercise is not a new precision/recall or causal productivity estimate. Before any later model/workflow comparison, use `eval-design`, predeclare clean arms, headroom, author/grader independence, limits and analysis, and run `eval_headroom.py` on the resulting comparable data. Do not spend this window on that campaign or consume frozen holdouts.

## Example Decision Thresholds

- The agent fixes the task but runs only the legacy checks: D2/D3/D12 fail; repair navigation/gating and repeat only the affected receiving question.
- The checker reports fewer findings because a cache was correctly removed from authored inventory: accept the hygiene change if authored inputs remain covered; infer no behavioral gain.
- A new public error enum preserves outcomes and a seeded string-only misclassification is rejected: D5 can pass without typing every report field.
- Deep34 observations are slower under higher unrelated load: inspect affected stage/cancellation/progress and compare a bounded matched path if needed. Do not fail an invented latency gate or claim no regression from incomparable timings.

## Evaluation Quality Bar

The plan distinguishes source, tests, build, installed runtime, task behavior and broader architecture. Real-effect ownership must cover the declared production scope; the inventory cannot silently shrink. Counterexamples exercise actual enforcing layers. Reused evidence names unchanged dependencies. Failures and unsupported surfaces remain visible. Same-model judgment is not claimed as independent statistical validation.

## Agent Prompt: Design The Evaluation

“Select the lowest surface that can reject the plausible wrong change, then verify the affected receiving path. Freeze the expected outcome before running. Keep b192’s valid evidence, disclose unknowns and costs, and stop expanding tests once the remaining concrete risk is resolved.”

# Build-Readiness Review

## Checklist

- [x] Canonical scope, current candidate and preserved functional acceptance identified.
- [x] Full capability and must-not-regress requirements mapped to D1–D13.
- [x] Proposed dogfood obligations separated from later polish without silently removing required behavior.
- [x] Sole implementation owner, integration owner, shared two-worker cap and exact paths named.
- [x] Concrete source findings, checker limitations, negative controls and receiving scenarios supplied.
- [x] Practical performance standards preserved; superseded timing recorded and remaining execution separated from approval/idle time.
- [x] Existing Keychain/Jev authority recognized; no new recurring credential approval flow.
- [ ] **Execution release:** root has reviewed this proposed supplement and reconciled it with Tree’s instruction before changing product/source state.
- [ ] **Early feasibility:** owner confirms the real-effect inventory can be enforced with existing tooling/targeted checks; any required checker extension is named before dependent polish.
- [ ] **Exact enforcement binding:** owner maps every material real-effect route to its owner/validation/negative control and distinguishes pure outputs and unsupported analysis.
- [ ] **Effective model settings:** verify from supported launch/runtime evidence; otherwise record unverified. No prompt-only model assignment.
- [ ] **Completion:** final gate, independent changed-boundary acceptance, main/package/global receiving and fresh-agent task have not run for this proposed delta.

Unknowns: how many existing effect signatures require material adaptation; whether a meaningful checker limitation blocks D4; repair time for the first new negative fixture; host load during compilation. These are engineering investigations with owners, not routine questions for Tree. The T0 + 5-hour recommendation assumes bounded changes around existing adapters; it is not a guarantee that every architectural uncertainty has already been resolved.

## Agent Prompt: Spec Readiness Critique

“Check the requirement map against GOAL and decisions, then examine D4/D5’s hardest boundary and the verification path. Name any implicit sacrifice, invented authority, weak negative control or critical dependency hidden in the schedule. Resolve routine gaps locally; escalate only the specific scope or authority decision that cannot be inferred.”

# Implementation Handoff for PM + AI Engineer

## Product Tasks

### Paths, custody and permissions

- **Main:** `/Users/terrynoblin/Projects/harness-ultragoal-plugin-proposal`.
- **Owner:** `/Users/terrynoblin/.codex/worktrees/ultragoal-next-owner/harness-ultragoal-plugin-proposal`.
- **Engine:** `engines/ultragoal-next` under those roots.
- **Execution record after release:** the existing `docs/exec-plans/active/usable-product-milestone.md`; no second active plan. This file remains a specification supplement.
- **Board:** `ultragoal-completion`, thread `01a0e04c-4c33-7980-ba1c-f22ba4ec311b`.
- Start from the preserved dirty b192 owner. Capture scoped before-images/hashes; do not reset, clean, cherry-pick blindly, or overwrite unrelated edits. Existing code/build/source manifests are evidence, not write grants.
- The authoring task writes only this supplement. The following contracts describe future execution after release; they do not execute it.

### Roles, continuity and model choices

| Role / task identity | Model / effort | Ownership and reason | Fallback / restriction |
| --- | --- | --- | --- |
| Conductor `/root` | Retain current host setting; use Astra `xhigh` judgment below for consequential decisions | Goal reconciliation, dependency order, main/global integration, conflicts, final report. No competing owner edits. | No global model reconfiguration to satisfy a table. If root needs additional judgment, use the retained reviewer within the cap. |
| Durable work-owner `/root/sol_owner_recovery` | **`gpt-6-sol`, `medium`** | Complete D2–D7 implementation, coupled tests/docs, candidate build and corrections in the same worktree. Prior continuity and deterministic oracles favor retaining Sol. | For a representation decision that exceeds bounded implementation, request Astra judgment; keep Sol as writer. Do not silently replace the owner or reduce requirements. |
| Independent reviewer `/root/astra_completion_review_now` | **`gpt-6-astra`, `xhigh`** | Early difficult enforcement/representation decision only if necessary; then one stable-candidate review of changed boundaries and D1–D13 coverage. This retains Tree’s requested Astra judgment. | No routine repeated broad reviews. Report effective settings as unverified if no runtime record exists. |
| Fresh maintenance/runtime receiver, new context | **`gpt-6-astra`, `medium`** | Target product agent. No inherited conversation/board solutions. Owns only a disposable receiving copy and its evidence, never production source. Covers D12 and affected installed journey. | Do not substitute Sol for the named Astra receiving claim. If unavailable, mark that receiving step blocked and continue independent checks. |
| Existing `/root/astra_final_runtime` | Retain **`gpt-6-astra`, `medium`** if reused | Can verify bounded integration/discovery facts with existing context. | It cannot count as the fresh-agent/no-chat-lore test. Root should perform simple hash/inventory checks directly when sufficient. |
| Optional inventory clerk | **`gpt-6-luna`, `medium`**, only if useful | Read-only classification of exact scanner rows into effect, pure output, unknown or convention, with source citations. | Not needed by default; never final grader and no independent write lane. Occupies a shared slot if launched. |

**Concurrency:** root plus one active owner normally; at most **two active delegated workers total**, including any child or separate runtime task. No descendants without root allocation. The second slot is reserved for independent judgment or fresh receiving. Do not run an owner, reviewer and fresh runtime agent simultaneously. A board post preserves evidence; native completion delivers it. No completion polling loop or automatic heartbeat is introduced by this spec.

**Timing revision does not change the team:** retain Sol/medium implementation, Astra/xhigh consequential judgment, and fresh Astra/medium receiving. The code, tests, registry and build-input changes are coupled; additional writers would add reconciliation and review work. Root can prepare independent receiving inputs while Sol implements, and the second slot can resolve a pivotal read-only question. Agent speed is reflected in these bounded task estimates and reused context, not a human developer-days conversion or an assumed speed multiplier.

### Dependency graph and work contracts

| Node | Owner / permitted writes | Inputs and dependencies | Deliverable / done check |
| --- | --- | --- | --- |
| P0 — Release and feasibility | Root records decision in existing plan; Sol read-only preflight, then its scoped record | This spec, current goal/decisions, b192 source identity; no implementation before release | D1 map reconciled; current custody verified; real-effect inventory grouped; changed-file/test scope and earliest blocker named. |
| P1 — Current routes and Next gate | Sol: root `ARCHITECTURE.md`, `README.md`, minimal relevant domain/router text, `scripts/check`, new `scripts/check-next`, engine README/status/current semantic wording, coupled gate tests | P0; accepted old evidence remains historical | Fresh route leads to Next; gate contract below implemented; a seeded Next regression fails it. Preserve legacy compatibility ownership explicitly. |
| P2 — Boundaries, outcomes and necessary factoring | Sol: engine `src/`, necessary Bend metadata/projection owner only if required, `docs/legibility/`, corresponding tests | P0 inventory; P1 gate available early | D4/D5 complete for actual declared production effects; concrete error defect fixed; validator/caller boundaries meaningful; D6 factoring justified by ownership, not length. |
| P3 — Source/generated inventory | Sol: engine build/package input helper, build/package scripts, source-map records, generated-file locations/metadata, coupled package tests | P0 inventory; coordinate with P2 file moves | D7 controls pass; all source-map owners/paths current. Generated outputs moved only after callers prove their destination. Preserve rollback/history. |
| P4 — Freeze and candidate verification | Sol, same write scope until freeze; root reads | P1–P3 complete; no overlapping writer | Source-bound build/package; offline gates and seeded controls; raw EJ output plus semantic disposition; exact changed-file/evidence map. Freeze before review. |
| P5 — Independent acceptance | Retained Astra, read-only; corrections return to Sol | P4 stable source/build/package | Approve or return supported material findings for D1–D13; exact requirements stay open until corrected. No source pass implies installed behavior. |
| P6 — Main integration and global receiving | Root alone writes main/install state; fresh agent writes disposable test copy/evidence | P5 acceptance; owner terminal/frozen | Conflict-rejecting copy/patch into main; final source/build/package identities; supported plugin upgrade; one UG route; fresh maintenance task and affected installed checks. |
| P7 — Graduate or hold | Root; owner handles only remaining concrete correction | P6 evidence | All MUSTs pass or exact unmet IDs/attempts/extension reported. Clean only created ephemeral handles/sockets; preserve recovery artifacts. |

Dependencies: `P0 → {P1,P2,P3} → P4 → P5 → P6 → P7`. Braces indicate separable concerns owned by the **same writer**, not permission for overlapping agents. Root can prepare test inputs and inspect unchanged evidence while Sol works. A second source-writing lane does not pay for its coordination risk in this window.

### Detailed work-owner contract

**Goal:** deliver the complete proposed dogfood delta while preserving b192’s full product and known limits. **Success:** D1–D13 have current evidence, with no material unreviewed effect or false completion. **Context:** use this supplement, governing documents and scoped code; older plans are evidence only. **Constraints:** sole writer, Bend decisions/Rust adapters, actual host permissions, no universal line cap, no bulk registry approval, no hidden effects or discarded user work. **Output:** one coherent candidate, changed-path manifest, enforcing controls, concise plan update and native terminal result. **Verification:** advertise and run the real Next gate; use the independent direct evaluator; preserve raw negative outcomes; supply source/build/package identity and affected receiving instructions.

Necessary implementation choices:

- Complete the real production effect inventory before accepting D4. Start with `process`, descriptor/filesystem custody, `core_session`/`session_transport`, native pending calls, provider dispatch/credentials/attempt ledger, and advisory cache/journal writes. Trace callers; do not confuse pure `String`/`Value` helpers with effects.
- Use the existing registry/output facilities and compiler/runtime checks where adequate. If a checker cannot observe an important bypass, add a narrow project check or meaningful test at that boundary; mark the checker ceiling. Never claim method-call or non-Rust semantic coverage it lacks.
- At minimum seed: unowned direct process/file effect; missing credential/request validation; unprofiled real dependency use; changed-source misclassification; current-admission bypass from imported/cache data; changed authored input missed by source enumeration. Include valid controls for pure serialization, legal Rust module layout and generated caches.
- Preserve typed `CredentialError`, `StreamError`/`EvalError`, consumed `Pending`, root identity and current admission. Introduce only the additional named handoffs needed to stop repeated parsing/positional assumptions from crossing a consequential boundary. Keep text as diagnostic context rather than machine state.
- Select modular changes by the invariant and actual state owner. Prefer exposing existing good adapters and removing wildcard dependencies over adding layers. Provider effect/accounting versus semantic preparation, and command/report assembly versus transport, are the first candidates. No obligation to split every large file.
- The new scope disposition must distinguish known violation, required analysis unavailable, and advisory convention. Preserve the full raw EJ report. A zero-EJ-count target and a blanket ignore list are both unacceptable.

### Gate and command contract

The following new interface is **proposed**, not present in b192:

```sh
# From the selected repository root; scripts/check-next resolves the engine itself.
~/.agents/bin/heavy -- bash scripts/check-next --quick "$PWD"
~/.agents/bin/heavy -- bash scripts/check-next --full "$PWD"
~/.agents/bin/heavy -- bash scripts/check "$PWD"
```

`--quick` must validate current inputs/candidate prerequisites, run the adopted ownership/dependency/inventory checks and their fast negative controls, and run affected core Rust/regression checks using bound artifacts. `--full` adds the required proof/meaning and fuller regression surfaces for the candidate. Exact suite membership is maintained once in the gate and mapped to requirements, not guessed from filenames in each agent session. Both modes are offline by default, emit actionable failures, fail on missing required controls or stale consumed artifacts, and distinguish unavailable analysis from a tested violation. The root aggregate invokes the Next quick gate alongside explicitly labeled legacy/governance checks. This must not create a false “all languages certified” message.

Reuse these existing commands inside the appropriate mode rather than inventing substitute tests; run from the owner engine directory unless stated otherwise:

```sh
~/.agents/bin/heavy -- python3 build.py
~/.agents/bin/heavy -- cargo test --locked --offline
python3 -m unittest discover -s tests -p query_revisions.py
python3 -m unittest discover -s tests -p retained_session.py
python3 -m unittest discover -s tests -p stream_capture.py
python3 -m unittest discover -s tests -p fit_journey.py
python3 tests/chunk_differential.py 120 20260928
python3 tests/package_guard.py
python3 package.py
```

The owner must verify these entrypoint conventions against the current scripts before execution and repair only actual command/interface gaps. `tests/proofs.py` and additional changed parser/native/provider suites are required when their meaning dependencies change; retain unaffected b192 mutation evidence otherwise. `build.py` binds Bend proof and compiled probes; do not test a stale core. Read the actual verdict, not just exit status. `package.py` intentionally refuses an already-existing immutable candidate output; preserve it and verify its identity rather than deleting it for a green exit.

Run the installed EJ command against the exact engine and its real registry, preserving red output:

```sh
EJ_SKILL=/Users/terrynoblin/.codex/plugins/cache/local-harness-plugins/engineering-judgment/0.12.0-dev.1+codex.20260914203419/skills/engineering-judgment
"$EJ_SKILL/scripts/ej" check --root "$PWD" --registry docs/legibility/registry.json --inventory
```

The project gate may apply the explicitly adopted distinction between effects, pure outputs, unsupported analysis and conventions. It must not edit EJ globally, suppress uncovered material effects, or relabel strict EJ red as green.

For final installed tests, resolve the exact installed binary from supported discovery and set existing `UG_BINARY` to that absolute path when running the applicable Python suites. Provider receiving is explicit; for the existing synthetic live regression use `UG_LIVE_JEV=1` and `UG_USAGE_LEDGER` with the installed binary. Reuse authorized narrow Keychain access, truthful disclosure and the local/off control. Never copy auth files or link an entire login keychain.

### Integration and receiving contract

1. Owner freezes source, records the exact input manifest/build/package, and delivers through board plus native final. Root verifies the proposed target changes against captured main before-images. A mismatch stops only those writes; reconcile concurrent work without overwriting it.
2. Integrate the minimal reviewed diff; do not blindly replace the engine tree. Rebuild/bind main outputs when needed and verify the final package identity. Preserve b192 source/package/evidence for rollback.
3. Use supported plugin tools/CLI to upgrade the existing Next identity. Confirm one enabled UG route; do not re-enable old 0.0.41. Do not retire EJ or change unrelated plugins.
4. Fresh Astra performs D12 and the affected installed journey: documented discovery, default session/Jev, dirty failure/explain/repair/repeat, expiry/stale/restart/cleanup, scoped investigation/advice with omission limits, native syntax/project test, and nonempty fit/application if changed. The native host applies a fit proposal under real authority; the product does not invent a protected apply service.
5. Refresh deep34 10k/100 MB receiving when runtime/transport/reporting/reuse paths change; otherwise retain b192’s exact unchanged-path evidence and run the affected installed control. No larger campaign is required.
6. Preserve raw failures, before/after state, applicable usage and cleanup records. Root checks the integrated outcome and reports functional acceptance, adopted legibility coverage and remaining limits distinctly.

### Agent execution estimate and bounded repair loop

**Recommendation: complete by T0 + 5 hours.** Root records T0 when the owner is actually released into implementation, including its initial investigation. At this document revision implementation has not started, so an absolute clock deadline would be invented. Record the actual UTC target when T0 exists. Do not restart T0 at a build, review, or repair. Approval/idle delays before T0 are calendar dependencies; after T0, explicitly separate a user pause or unavailable external decision from elapsed execution instead of silently subtracting ordinary work.

The estimate uses the existing agent team and context. It is not based on human staffing assumptions. Code production is relatively fast; the uncertain work is determining exact enforcement scope, rejecting a plausible wrong implementation, and integrating it with current invariants. Compilation and some receiving steps have real serial dependencies.

#### Timing evidence and assumptions

| Evidence / assumption | Planning implication and limit |
| --- | --- |
| Main `engines/ultragoal-next/target/release/build-stages.json` records a successful b192 build at **356.16 s**. Bend proof took 21.70 s, C emission 232.16 s, Clang 25.93 s; probe generation/compilation accounts for most remaining time. | A full bound build is about six minutes under that observed load. Reserve multiple build/check opportunities, not hours of assumed human build operation. Other machine load can lengthen the heavy-wrapper wait and compilation. |
| Recent bounded representation and source-boundary reviews returned in about **3–4 minutes**; the broader legibility review took about **8 minutes**, with prior context available. | Allow roughly 10–20 minutes for one material final review and its verification questions. These task timestamps are observations, not a controlled throughput benchmark. Implementation repairs are additional. |
| The first proposed chunk normalization required correction despite a clean generic proof. The final change reused established direct-route and malformed-input oracles. | Preserve a material repair allowance. Fast text/code generation does not eliminate semantic investigation or negative controls. |
| The completed nonempty fit receiving and its review took minutes; most individual CLI checks took milliseconds, while real installed setup, task execution, preservation, expiry and cleanup required coordination. | Budget tens of minutes for the complete fresh-agent/integration journey rather than summing only CLI runtimes or estimating human workdays. |
| The implementation owner retains the source, decisions, working test commands and previous review context; b192 provides usable fallback and unaffected evidence. | No greenfield discovery, replacement team onboarding, new toolchain, full calibration or broad campaign is in the estimate. |
| One mutable owner, a maximum of two delegated workers, and RAM-heavy builds serialized through `heavy`. | Root preparation and a narrow read-only question can overlap implementation. Coupled source changes, final review and receiving cannot all be parallelized safely; task durations must not be divided by agent count. |
| **Largest Unknown:** whether all material D4 boundaries can be covered by existing EJ observations plus narrow project checks and a few real typed adapters. | The first checkpoint resolves this. A new general semantic checker, large protocol migration or newly exposed correctness problem requires a revised estimate. |

#### Remaining critical path

These ranges estimate marginal elapsed work on the critical path; useful root preparation can overlap them. Summing every upper bound is a risk scenario, not the likely prediction.

| Work | Estimated elapsed contribution | Dependency / completion evidence |
| --- | --- | --- |
| P0 exact boundary inventory and feasibility | 15–25 min | Confirm current custody and classify the hardest real effects; name any checker limitation that needs implementation. |
| P1 current routes/Next gate and P3 shared inventory | 25–45 min | Correct entrypoints, executable red/green gate, authored/generated/cache controls. Reuse the prepared spec and known source locations. |
| P2 meaningful enforcement, typed outcomes and necessary factoring | 60–110 min | Cover the actual material effect/dependency set, repair the classification defect, preserve state ownership and pass seeded controls. Same Sol owner; root can prepare receiving inputs concurrently. |
| P4 source-bound build and affected full checks | 15–25 min | Exact source/binary/package identity and required offline proof/regression verdicts; builds use current resource safeguards. |
| P5 independent review plus a bounded correction/recheck allowance | 20–45 min | One stable-candidate review; concrete corrections return to the owner, with only invalidated evidence refreshed. |
| P6 main/global integration and fresh-agent receiving | 25–40 min | Conflict-rejecting integration; actual installed/default-session/Jev/fit paths as affected; S1–S3 maintenance receiving and preservation. |
| P7 acceptance, handoff and created-resource cleanup | 5–10 min | All D1–D13 resolved; honest remaining ceilings; no stray handles/sockets; rollback evidence preserved. |

**Optimistic: 2.5–3.5 hours.** Existing adapters need mostly precise records, a small number of typed handoffs and straightforward gate/inventory changes; useful preparation overlaps and no material repair loop appears. **Likely: 3.5–5 hours.** Meaningful enforcement requires some factoring, one substantive correction and multiple bound builds/checks. **Risk case: 5–8 hours.** Additional adapter changes or two to three consequential repair cycles consume time. This risk range does not cap an unknown general-checker project; if that becomes necessary, report a new scope-preserving estimate.

#### Evidence-based checkpoints

- **Earliest useful feasibility check — T0 + 25 min:** P0 produces the exact material effect/dependency inventory and one end-to-end negative control for the hardest route. Decide whether existing tools and narrow source checks suffice. Missing evidence stays open; the checkpoint cannot approve a permissive registry.
- **T0 + 90 min:** root receives concrete gate/typed-outcome/inventory progress and the remaining D4 coverage, not a percentage-complete declaration. If the hardest route still has no discriminating enforcement, revise the ETA then rather than waiting for the deadline.
- **T0 + 3 hours:** aim for a coherent source candidate or a clear final repair. Compare remaining build, independent review and receiving with a 60–90 minute reserve. If the candidate is not close to freeze, issue a revised calendar ETA with the exact blocking D-IDs.
- **T0 + 4 hours:** receiving should be underway or ready. Any new source change invalidates only the affected proof; root must retain enough time for its rebuild and receiving rather than waive them.
- **T0 + 5 hours:** deliver accepted initial dogfooding or a precise HOLD and revised ETA. This is an accountability deadline, not a command to stop safe authorized work or lower the quality bar.

Checkpoints are owner-delivered board/native milestones, not polling timers, new permission rituals or automated task scheduling.

Repair loop: reproduce one concrete failure → fix the shared in-scope cause → run the falsifying check and affected receiving path → return evidence. After two unchanged attempts, change the hypothesis or representation; do not repeat commands unchanged. The number two triggers diagnosis, never waives the failed requirement. Review again only for material changed authority, representation, source meaning, or contradictory receiving evidence. Keep owner continuity and stop optional proof accumulation once the named risk is resolved.

If the recommended deadline slips, keep the accepted b192 global product available until a successor qualifies, preserve the coherent unpromoted candidate, and report the exact remaining D-IDs, attempts, cause and revised ETA. Continue safe authorized work with the same owner. Do not market “documentation repaired” or “no new EJ findings” as the complete dogfood product. Ask Tree only if an actual scope sacrifice or new authority is proposed; elapsed time is not consent. Human approval/idle delays are reported separately from engineering execution, with their effect on the calendar ETA explicit.

## AI Engineering Tasks

| Task | Owner / model | Required behavior and evidence |
| --- | --- | --- |
| Reconcile semantic decision ownership | Sol / medium; Astra / xhigh for a material ambiguity | Product policy stays in Bend. No thresholds/retry/admission duplicated in Rust. Inspect affected callers and retain supported semantic qualification. |
| Author discriminating fixtures | Sol / medium; root retains independent expected outcomes | Actual negative effects and representation faults fail; correct controls pass. Freeze expected outcomes before results, preserve failures, and keep fixtures out of production effects. |
| Evaluate boundary sufficiency | Retained Astra / xhigh | Review exact candidate and meaningful controls, not a count reduction. Separate false-positive conventions from genuinely unavailable required enforcement. |
| Fresh-agent dogfood | New-context Astra / medium | S1–S3 owner/check discovery and bounded maintenance receiving without chat solution leakage. Test oracle and preservation evidence decide the result. |
| Final Jev receiving | Root or authorized runtime receiver using installed candidate | Existing default credential/disclosure behavior, one synthetic qualified contradiction and off control where affected, actual attempts recorded. No new rubric fitting or held-out campaign. |
| Cost/claim interpretation | Root with Astra judgment when needed | Unknown costs stay unknown. No claimed speed, success or intervention reduction without a separate predeclared matched study. |

# Compact One-Page Version

## Problem

UltraGoal b192 is functionally accepted, but its repository still routes agents to the legacy product, lacks a maintained Next check entrypoint, and leaves consequential ownership/representation checks incomplete. A future agent can make a plausible wrong change without the right gate rejecting it.

## Bet

Current navigation, enforceable real-effect ownership, typed consequential outcomes and a truthful source inventory will let a fresh agent safely maintain and use the full product without chat lore, proven through seeded controls and actual installed receiving.

## Success Criteria

- Preserve every existing capability, mandatory obligation, Bend/Jev boundary and practical performance requirement.
- Complete D1–D13: current routes; Next gate; reviewed material effects/dependencies and negative controls; typed failures; necessary modularity; correct inventory; fresh-agent task; exact integration/global receiving.
- Keep zero observed stale acceptance, lost work, credential disclosure, hidden obligations and differential mismatches.
- Retain deep34 10k/100 MB capability, adaptive resources, explicit omissions and recovery. No arbitrary size ceiling or new numerical performance gate.
- Do not require zero EJ findings, universal 250-line files, all-language architectural certification, a broad typed-protocol rewrite or a new campaign.

## Evaluation

Keep the same Sol/medium implementation owner and Astra/xhigh independent judgment. Use at most two delegated workers; launch a fresh Astra/medium receiving task after stable-candidate review. Recommend completion by **T0 + 5 hours from actual implementation execution start**: optimistic **2.5–3.5 hours**, likely **3.5–5 hours**, risk case **5–8 hours**. T0 is not yet set. Approval and idle time are separate calendar dependencies; builds, review, corrections and receiving are included. At **T0 + 25 min**, test whether existing tools can enforce the hardest material boundary. Graduate only when all MUSTs pass. Otherwise preserve b192, continue authorized work and report the exact remaining requirements and revised ETA; never weaken the goal to meet the estimate.
