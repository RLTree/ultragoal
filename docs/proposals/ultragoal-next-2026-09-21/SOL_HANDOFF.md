# UltraGoal implementation handoff — GPT-6-Sol

Prepared 2026-09-25 using the installed handoff brief and the existing ChatPRD Product Spec for AI Agents (4317). This is the complete implementation outcome, not a new feature subset or a new plan. The sole active ExecPlan remains the progress owner.

## 1. The real job (Goal and Success)

Complete the full UltraGoal product so Tree's agents can fit requirements, inspect, check, verify, index, select, advise, explain and recover during real repository work with less repeated investigation and user effort. Preserve incremental reuse, Bend decision ownership, internal Jev, native/plugin integration and all existing migration obligations. Scope stays as requested. The user should not manage routine session handles, receipt chains or signing steps.

Read the current main-checkout `GOAL.md` and `.agents/decisions.md` first. They contain Tree's adopted September 25 recommendations and outrank old targets and ownership notes.

- **Said/adopted:** target scale 10,000 files / 100 MB, real adopted contracts, target-sized deep trees; full Babel and 10× are stress observations.
- **Said/adopted:** initial p95 gates are ≤1 s warm unchanged, ≤2 s affected one-file feedback and ≤10 s cold complete index. Keep 200 ms / 500 ms / 5 s as optimization targets. Include capture/revalidation, output, rotation and normal recovery; no cache-only stopwatch.
- **Said/adopted:** material improvement in task success, time or correction effort, without material regression in the others. Large percentage gains do not all have to pass simultaneously.
- **Adopted; operational detail inferred:** routine overhead target ≤5% on comparable-outcome tasks; use matched total elapsed time initially, and freeze that choice before confirmation. Account for billed cost and token categories separately.
- **Must not get worse:** preserve user work, required coverage, real permissions, credential exclusion, correct freshness and incremental/full equivalence. Bend continues to own rule decisions. Semantic blocking retains ≥95% precision, its 95% lower bound ≥90%, and ≥85% recall. Unqualified families remain advisory.

Done is the full installed journey at target scale, useful ordinary agent behavior, relevant clean laws/tests and the practical metrics above. No successful test, source hash or receipt alone establishes that outcome. Keep historical evaluations frozen and clearly labeled.

## 2. Tree's decisions and ownership (Context and Constraints)

Canonical project: `/Users/terrynoblin/Projects/harness-ultragoal-plugin-proposal`.

Implementation worktree: `/Users/terrynoblin/.codex/worktrees/ultragoal-next-owner/harness-ultragoal-plugin-proposal`.

Engine: `<implementation-worktree>/engines/ultragoal-next`.

**Tree explicitly confirmed on September 25: “Yes, Claude is stopped; Sol can take over.”** GPT-6-Sol is the next sole implementation owner; no new implementation worker was launched while preparing this handoff. Reuse this worktree and preserve all dirty/untracked work. Do not checkout/reset to b32 or substitute the older main-checkout engine. A still-open Claude process does not invalidate Tree's explicit stopped-writer confirmation.

Requested model/effort: **gpt-6-sol / medium**, confirmed available in the local model catalog. The actual receiving turn must be set to these values through the host; this text cannot switch the model. Current runtime settings are unverified until that turn starts. Novel architecture/proof/trust judgments use Astra under Tree's standing model rules; return ordinary implementation to the same Sol owner. Evaluation runners use Luna xhigh; independent authors/graders must differ from the system and baseline. Preserve the exact model within comparison arms.

This is currently a solo prepared handoff; no message-board discussion has been created and no launch/callback is claimed. If the conductor dispatches a same-tree subagent, set up the native board first, pass its real channel/discussion IDs and this ownership, and use lifecycle activation/final delivery. A separate chat is outside that board and requires the supported authorized callback route. Do not create user-visible chats just to satisfy coordination machinery. Use Conduct Agent Cadence if delegating; its shared concurrency setting prevents competing writers and coordination load, not a new product limit.

You own the complete coupled engine, adapter, plugin, test and documentation changes in the implementation worktree, plus needed integration into the main checkout after verification. Main `GOAL.md`, decisions, PRD, EVALUATION and the sole active ExecPlan are shared project records; keep their copies aligned. Unrelated work and historical worktrees remain outside your write scope. Existing legacy `validator/` work is not an invitation to redesign it; touch it only for a demonstrated dependency of this product's real receiving path.

Make ordinary decisions yourself and record their reason briefly in the existing plan. No one will answer routine questions during the run. Continue independent work if a genuine effect/permission boundary blocks one transition, and report the exact blocker rather than silently weakening the goal.

## 3. Current state and next implementation outcomes

### What already works

Last runnable package: b32 `e95e4f5564597cd0`, at `<implementation-worktree>/.codex-worktree/opus-work/b32/ultragoal-next/target/packages/e95e4f5564597cd0/harness-ultragoal`.

`bin/ultragoal` SHA-256: `5ab0a44ed10327ec94fb5a53c9a2f8fc72a227950bc07c217c7548fdafe6d00d`; core: `849fbd7da929d0d8cae6676d81cf32829f6bbbb7a5bd148a7e1885e0f6b5872b`. The packaged `inspect` returned 0 during this review. Source Git base is `f447a7a83e16`; the implementation is substantially uncommitted. Build identity alone is not a source checkout instruction.

Prior evidence supports chunked size handling through 1.17 GB, append-only usage logging, per-request deadlines, reused semantic answers, automatic core rotation and an installed b28 journey. The new selection shortlist recalls 98.3% on its held-out cases; the final Jev-filtered result recalls 94.2% while removing unrelated files. Brief output shrank from 22,892 to 1,471 bytes. These are different measurements; do not merge them into a whole-product pass.

### Worktree reconciliation completed after the initial handoff draft

Read main `audits/ultragoal-2026-09-26/worktree-reconciliation.md` and `evidence/worktree-reconciliation.json`. All five registered local worktrees were inspected; only this owner tree contains UltraGoal Next. b32 remains the latest recorded Next build. Streaming, selection and index improvements and brief output are already integrated; reuse their source and diagnostics.

Preserve the older `1ebc` worktree and commits `87a4cfdf5`, `e7a2c51f5`, `ae4273972` (legacy cross-domain guidance, approval drift, package wiring/tests). Its six dirty host-adapter files are already byte-identical to main, but its committed changes are not merged. Review relevant behavior when doing the existing migration obligation; do not blindly cherry-pick older signing/authority/process requirements over Tree's current UX direction. `e7ee` and `usable-loop-master-candidate` are historical legacy trees with no Next engine. Do not delete or use them as the starting source.

### What remains, in the recommended order

1. **Repair the repeated real-repository work.** Follow the b32 fitness diagnosis: directory-handle reuse and efficient stat/list/revalidation; retain rule results and chunk partials by valid inputs in Bend. Current `src/fs_adapter/capture.rs:44–116` repeats lineage walks; the retained path still reevaluates text rules. Preserve concurrent-change/path protections and differential correctness. Real warm medians were ~1.34 s on Larkspur and ~20.2 s on Babel; old 153 ms p95 was a different, easier b29 workload.
2. **Make normal use automatic.** Existing plugin `plugin/skills/harness-ultragoal/SKILL.md:22` restricts sessions to explicit requests. Have the current task host manage start, reuse, expiry recovery and cleanup, with a narrow off switch and compact output. Accept a valid existing empty session directory or give a direct recovery action. Reuse existing components; no new global daemon or signing service is implied.
3. **Preserve meaningful requirements.** Fix the exact-import-line contract that falsely rejects adding `quoteattr` to `XMLGenerator`, and generated-cache scope pollution. Keep the intended requirement and negative control strict; do not erase historic grades or weaken tests to make results green.
4. **Finish the promised investigation path.** Profile/parallelize serial selection planning (about 45 of 68 seconds at 111 MB in the review), and implement/use reuse where it serves repeated work. Preserve exact evidence recovery and both shortlist/final coverage. Chunk remaining unjustified whole-input limits instead of merely raising them; advice >256 candidates and the combined file-ranking frame are named current cases.
5. **Integrate and exercise every requested surface.** Run the installed dirty-tree journey, a real failure and recovery, session expiry and the next normal use after the final affected edit. Retain valid existing proof; refresh affected binaries and receiving checks. Update one current status summary; do not create per-operation approval or receipt rituals.
6. **Validate value with the actual default experience.** The 72-cell b29 scan was Luna xhigh, no Jev, and no index/select use: 15/24 vs native 16/24, +56% input tokens. It neither qualifies the full product nor proves it can never help. Run the small diagnostic scan after task/oracle validation; schedule quiet timings separately. The 216-cell run is conditional on stable source and an informative design.

The native-only six cases are possible regressions, not disposable inconvenient data. Ten replacement tasks still leave six of 24 (25%) without headroom under the existing rule. Keep them as regression evidence, add/harden realistic efficacy counterparts prospectively, and do not combine b29 and b32 token observations into one current-candidate claim. Predeclare the outcome, practical difference and harm margins before confirmation, then preserve unfavorable results.

## 4. Limits and gates

| Limit or gate | Purpose / disposition |
| --- | --- |
| Provider request/state/response envelopes | Provider-defined; retain, chunk and name uninspected material |
| Memory-sensitive compilation | Use `~/.agents/bin/heavy`; its machine-wide protection prevents concurrent builds exhausting RAM |
| Source/chunk/core boundaries | Keep only for a demonstrated provider, machine, real-data or irreversible-effect risk; make useful work resumable |
| Core/session lifetime | Forgotten-process and hung-request risk; retain runtime controls while normal recovery is automatic |
| 1× p95 latency gates | Protect ordinary UX at the specified shape/scale; 10× measures resilience rather than applying the same clock target |
| Semantic qualification | Protect users from false blocking; do not lower thresholds to make a milestone pass |
| Paid usage | Existing authorized Jev work has no spend cap; record calls; provider and RAM controls still apply |
| Frozen evals | Preserve unbiased evidence; new development does not rewrite consumed results |

Do not resurrect the old 420-second build guard or an inherited source-line/candidate count as a universal product requirement. The current build records stage times; read its owning implementation before changing resource controls.

## 5. Required tools, documents and verification

Use the installed `fitness-review`, `engineering-judgment`, `bend2` and `eval-design` skills at their relevant boundaries. The current PRD is the complete ChatPRD Product Spec for AI Agents; use its existing sections rather than authoring a competing plan. For substantive Bend changes, read the selected compiler's guide, the skill FOUNDATION and the relevant proof/compilation reference. Preserve qualified toolchain and law meanings; measured/compiler evidence determines any upgrade.

Read first:
- Main `GOAL.md`, `.agents/decisions.md`, then the latest section of `docs/exec-plans/active/usable-product-milestone.md`.
- Main `docs/proposals/ultragoal-next-2026-09-21/{PRD,EVALUATION,ARCHITECTURE,MIGRATION}.md` as needed. Current goal supersedes historical numerical gates and manual-session ceremony; scope remains.
- Main `audits/ultragoal-2026-09-26/report.html`, its source excerpts and `evidence/handoff-preflight.json`.
- Owner `.codex-worktree/opus-work/DELIVERY.md`, `journey-b28/JOURNEY.md`, `campaign/EVAL_PLAN.md`, latest `campaign/DECISIONS.md`, and the relevant `diag-warm/` / `integration/` findings. The DELIVERY headline overstates b32 by reusing b28/b29 observations; correct it without discarding valid evidence.

Commands from the engine root, after relevant changes:
```sh
~/.agents/bin/heavy -- python3 build.py
~/.agents/bin/heavy -- cargo test --locked --offline
UG_BINARY="$PWD/target/release/ultragoal" python3 tests/retained_session.py
UG_BINARY="$PWD/target/release/ultragoal" python3 tests/chunk_differential.py
```
Use the configured remaining law/mutation/differential and changed-path suites; the commands above are a starting route, not the whole acceptance set. Use project-owned scratch under the worktree, not system `/tmp`. After any model evaluation, run `~/.agents/checks/eval_headroom.py <results> --baseline <actual-label>` and score process as well as outcome. The saved scan uses `native`, not `N`, as the file's baseline label.

Repository entrypoint from the root:
```sh
~/.agents/bin/heavy -- bash scripts/check "$PWD"
```
It includes legacy product/governance surfaces. Record an exact projection/legacy gap instead of launching an unrelated governance rewrite. Final runtime acceptance still requires the real installed UltraGoal Next path. Keep campaign outputs outside repository snapshots being tested, or schedule the check during a quiet window.

## 6. Authority

Pre-approved: ordinary in-scope implementation, local dependencies, builds, tests, fixtures, reversible repairs and source integration; local feature-branch commits/pushes/PRs on Tree's repositories under his standing instructions. TypeSafe Jev development/evaluation/production spending already authorized; record attempts and use the existing authorized credential route. No recurring micro-approvals. Installation/use in the project-owned disposable receiving environment is ordinary verification; broader active-host changes retain their actual user scope.

Ask only for a genuinely ungranted consequential effect, spending outside existing authority, publishing as Tree, release/default-branch merge, destructive retirement or a named user gate. Never expand credential access merely to simplify isolation. Specifically, do not link the whole login keychain into disposable homes. This does not remove Jev from the product; use an already supported narrow route or name the exact untested arm.

## 7. First action, output and handoff back

Reconcile the source once against `handoff-preflight.json` and the current goal; do a focused fitness review of the above failure boundaries before choosing the repair. Keep one owner and existing worktree. Use the existing active ExecPlan for decisions, progress, checks and remaining work; do not restart the old research or authority program.

Deliver the integrated source and exact runnable candidate, the real installed journey, practical latency results on real contracts, useful outcome/process observations, limits, rollback/recovery behavior and cleanup status. Explain any unmet completion criterion directly. Final review belongs to the conductor/independent reviewer; fix its supported findings in this same owner lane. Do not declare completion merely because the code builds or the old benchmark passes.
