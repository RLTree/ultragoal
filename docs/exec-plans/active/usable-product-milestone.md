# Harness UltraGoal — Foundation Reset, Current Product Loop, and Anti-Theater ExecPlan

**Repository:** `RLTree/ultragoal`
**Intended repository path:** replace `docs/exec-plans/active/usable-product-milestone.md` atomically
**Operation:** do not keep the old plan active or create another plan, lane registry, completion manifest, research-law graph, or receipt ledger
**Research lock:** 2026-08-08, America/Los_Angeles
**Observed active candidate:** `codex/successor-contract-v2-live-product@cbdc0bdd275e5988a2dbf3e219823c11ccab3796`; HEAD tree `ae3e210bc97ebde058628ad54a054fd7806688f1`; re-resolve after every integration or commit
**Observed repository-control gap:** GitHub default remains stale `main@eb51ba4d6ea8844197e855d42fa239ef898be47d`; `master@30c4a19ca9ac2b5ff6b7d856ef5d3cf355e387ce` is 641 commits ahead and contains the only verification workflow; the current branch diverges from `master` at `5dae7dd24490b1a37bacb48b40694a840067fbe7`, with 2 current-branch commits and 12 `master` commits to reconcile; no branch protection, required check, ruleset, or current-branch workflow result exists
**Current claim ceiling:** research, repository inspection, and executable plan only

## Current root-conductor rebaseline (2026-08-09)

- Durable goal: `active` for “Complete the sole active Harness UltraGoal ExecPlan through CL-USABLE-LOOP on one exact integrated candidate, preserving unrelated state and obeying every Tree approval boundary”; no token budget is set.
- Candidate custody: branch `codex/successor-contract-v2-live-product`, HEAD `cbdc0bdd275e5988a2dbf3e219823c11ccab3796`, HEAD tree `ae3e210bc97ebde058628ad54a054fd7806688f1`, exactly aligned with `origin/codex/successor-contract-v2-live-product` at observation time.
- Repository-control truth: authenticated GitHub and `git ls-remote` both report default `main@eb51ba4d6ea8844197e855d42fa239ef898be47d` and `master@30c4a19ca9ac2b5ff6b7d856ef5d3cf355e387ce`; `main` is an ancestor of `master` and is 641 commits behind.
- Integration truth: `master...HEAD` has merge base `5dae7dd24490b1a37bacb48b40694a840067fbe7`; `master` has 12 unique commits and HEAD has 2 unique commits. A read-only `git merge-tree` reports 12 conflicts, all in root/standards/plan documents; the four protected distribution paths do not overlap the `master` delta.
- CI truth: `.github/workflows/verify.yml` exists on `master` only. It runs for pull requests and for pushes to `main`, not `master`. GitHub retains one historical active workflow registration and one successful PR run on `codex/plugin-eval-repair-integration@666145252`, but the current candidate has no workflow run, check run, or commit status.
- Protection truth: `main`, `master`, and the current branch are unprotected; no required status checks or repository/inherited rulesets apply.
- Protected unrelated state: four modified `validator/src/distribution/**` files plus `.ultragoal-e2e-unrelated-state.txt` remain outside plan ownership. Their combined tracked binary-diff digest is `ef30d9dbe465af5ea3df8084c6aef5db9c63310ac5d92c78d80e759f380cc0ef`; the untracked file digest is `0777519bda010fe1d51a89ae095d9da5e61d2482a51732125a3ffd141189076d`.
- Foundation custody: the three landed foundation files are byte-identical to their named handoff sources; no current-source card, generated projection, or historical research artifact was refreshed.
- Authority hold: changing the GitHub default branch, branch protection, required checks, rulesets, or any other remote setting requires Terry’s explicit approval. No remote mutation has occurred.
- CLI hold: this root conductor has not invoked the UltraGoal CLI. Prior handoff command observations below bind the preceding candidate only and must be rerun after the exact product candidate is integrated.

## Prior handoff rebaseline (superseded candidate observation)

- Candidate at that observation: branch `codex/successor-contract-v2-live-product`, HEAD `7e46169749611b3fa335700bfaf2105ae471bc48`, HEAD tree `ecf1ad39424b67fb4eed557176020aecb3b6946c`.
- Working tree: dirty before this handoff; four modified `validator/src/distribution/**` files and untracked `.ultragoal-e2e-unrelated-state.txt` are pre-existing and excluded from plan ownership.
- Foundation inputs installed from `/Users/terrynoblin/Downloads/research-foundations-third-pass`: `current-2026-08-08.md` (sha256 `01eae0a275d223f76bfd81ffb4d8b9fc1510675dd107dfa2a3f65667f8544b98`), `register.csv` (sha256 `39c054831f0913a4301e25ebca58afa939facfb06c6f903a4aecbe2111a8d8c1`), and header-only `decision-log.csv` (sha256 `74d0f70963d24eea519cca98f4a7e011a8062bfc3885e2f3408914fa35fc4569`).
- `cargo fmt --all -- --check`: blocked by existing formatting drift across unrelated Rust files; no formatter run was authorized.
- `cargo check -p ultragoal --lib --locked`: passed.
- `cargo build -p ultragoal --bin ultragoal --locked`: passed.
- `cargo test -p ultragoal --tests --locked -- --list`: blocked during test compilation by existing fixture/API mismatches (including `observability_fixture_scratch`, `public_api_witness`, and routine host checkpoint symbols).
- `target/debug/ultragoal --json --help`, `inspect capabilities`, and `inspect context`: passed read-only; the context reports this dirty candidate and `inspect capabilities` reports host discovery unavailable without an authorized package root.
- No GitHub branch/default/protection/workflow state was changed or verified; no external target repository, install, package, remote, or release effect was performed.

These prior observations are provenance only after the candidate changed. They do not raise the current claim ceiling above research, repository inspection, and executable plan.

## 1. Purpose and observable user outcome

Deliver one exact UltraGoal candidate that allows an authorized operator to:

1. build the current plugin and Rust CLI;
2. observe the exact candidate through supported installation/discovery;
3. inspect and fit one representative repository without changing unrelated state;
4. run useful dirty-tree affected work;
5. encounter a representative failure;
6. receive a causal diagnosis and one legal next action;
7. recover or refuse safely;
8. repeat useful work without stale reuse or hidden writes; and
9. understand what ran, what did not, why, and what remains.

The repository must reach this result without requiring:

- the full research/source-law graph;
- recursive root manuals;
- per-command or per-lane receipts;
- source-card refresh;
- broad reviewer panels;
- several state projections; or
- a governance cleanup program before the first current-source loop.

The first value event is:

> On the current exact candidate, an external evaluator observes useful repository work, preservation, diagnosis, recovery and repeat use—or identifies the first product blocker—without treating UltraGoal’s own receipts or prose as the final oracle.

## 2. Stage architecture

```text
U0 repository and foundation authority
  ↓
U1 current-source installed-loop baseline
  ↓ if product-blocked
U2 one coupled blocker repair
  ↓ loop passes
U3 replace research authority and reduce active instruction path
  ↓
U4 split product/governance/release checks and add only needed runtime state
  ↓
U5 byte-identical instruction/context A/B
  ↓
U6 exact Agentic coexistence + final external loop
  ↓
U7 decide CL-USABLE-LOOP, bounded retirement, stop
```

No later prose reduction, check split, or coexistence pass can substitute for a failed U1/U2 product loop.

## 3. Program invariants

1. **Product before governance.** The first source edit after U1 addresses the first observed operator-loop blocker.
2. **One current foundation owner.** `docs/foundations/current-2026-08-08.md` and a compact register own current external source status. The old source-to-law machinery becomes historical compatibility input.
3. **Research does not mint authority.** A source changes one named decision or changes nothing. It does not automatically create laws, schemas, fixtures, receipts, setup outputs, package obligations, or blockers.
4. **One current state owner per state class.** Program state lives in the active plan; operation state lives in runtime; Git owns reproducible history. Do not mirror state in registries, manifests, receipts and backlogs.
5. **One implementation path by default.** Parallel lanes require proven semantic/write independence, no peer-unmerged dependency, local oracles and reserved root integration capacity.
6. **Evaluator owns product proof.** UltraGoal may emit typed facts but cannot self-certify `CL-USABLE-LOOP`.
7. **Reads and routine work are zero-authority-write.** Read, inspect, plan, diagnose, next-action, routine checks, drafts and no-ops create no tracked receipt or governance files.
8. **Transition-only retention.** Persist only cross-process recovery, an irreproducible external observation, an authorization transition, exact distribution artifacts, or the compact milestone result.
9. **Vocabulary is not deletion evidence.** Keep a receipt-/law-/proof-named test if its actual oracle detects forgery, path escape, stale identity, custody loss, interruption or unsafe cleanup.
10. **Final state before process.** Required/prohibited user outcomes and target preservation outrank receipt count, reviewer agreement, stage trace and source coverage.
11. **Two similar failures require a changed hypothesis or stop.** More policy, more agents, more reviews, or more artifacts is not a repair by itself.
12. **Stop after the bounded milestone.** A second target, broader autonomy, UI, release, crate split, telemetry platform or general scheduler requires a new owner decision.

## 4. Allowed durable artifacts

Commit only:

- current product source, tests and canonical configuration;
- `docs/foundations/current-2026-08-08.md`;
- `docs/foundations/register.csv`;
- a foundation decision log row when a material decision must survive sessions;
- this single active plan;
- one small held-out product/context oracle needed for regression; and
- one compact final milestone decision if another process consumes it.

Do not commit:

- per-command or per-lane receipts;
- ready/validator/reviewer packets;
- copied transcripts;
- source snapshots refreshed for currentness alone;
- article-to-law projections;
- context inventories or prose-dedup reports;
- routine coverage reports;
- a new reader/writer registry;
- one proof file per claim; or
- an archive of removed policy.

## 5. Progress

- [ ] U0 restore branch/CI truth and establish the current foundation owner (foundation owner and fresh control observations complete; Terry selected bounded `master`-based candidate integration; candidate construction plus Tree-authorized default/protection repair pending).
- [ ] U1 run the current installed operator loop before governance redesign.
- [ ] U2 repair only observed blocking product boundaries.
- [ ] U3 replace the research-authority model and simplify active instructions.
- [ ] U4 separate product, governance and release checks; add only needed runtime state.
- [ ] U5 evaluate current versus reduced instructions on byte-identical source.
- [ ] U6 verify exact Agentic coexistence and rerun the external loop.
- [ ] U7 decide `CL-USABLE-LOOP`, retire bounded obsolete surfaces, and stop.

## Surprises and discoveries

- The verification workflow is not missing repository-wide: it exists on `master`, but not on stale default `main` or the current branch. GitHub’s retained workflow registration therefore cannot substitute for current-branch CI.
- The plan branch is not the current product line by ancestry. It contains the landed plan/foundation commits while `master` contains 12 product/security/CI commits; one root integration is required before U1 can bind an exact current candidate.
- A read-only merge analysis confines textual conflicts to 12 root, standards, product-contract, and active-plan files. The protected dirty distribution paths are disjoint from the incoming `master` delta.

## Decision log

- The committed `docs/exec-plans/active/usable-product-milestone.md` is the sole executable ExecPlan. The downloaded `ultragoal-EXECPLAN-v4.md` remains handoff/provenance only; its stale candidate and workflow assumptions will not overwrite the landed adaptations.
- Preserve the five unrelated working-tree changes by digest and path. Do not format, stage, commit, overwrite, or use them as candidate inputs.
- Hold UltraGoal CLI invocation until this fresh U0 custody/control refinement is durable. Afterward, invoke only the exact current grammar on the integrated candidate.
- D0 was a no-safe-default integration choice. Terry selected Option B, the bounded `master`-based candidate. Keep the dirty root checkout fixed; select the bounded candidate’s merge/rebase/cherry-pick/patch topology only after exact ancestry/conflict simulation.
- Keep the GitHub default/protection mutation at HOLD until Terry explicitly authorizes the exact remote effects.

### D0 candidate-integration options

| Option | What it does | Benefit | Cost and risk | Reversibility |
|---|---|---|---|---|
| A — characterize current HEAD first | Keep the root at `cbdc0bdd2` and run only safe source/read-only U1 characterization before deciding integration. | Zero branch/history movement and immediate evidence about the landed plan candidate. | The product bytes are 12 `master` commits behind; product results may be invalidated by later integration and cannot represent the current product line. | High; discard ephemeral output and leave the branch unchanged. |
| B — bounded `master`-based candidate | In a separately authorized bounded path based on `master@30c4a19c`, integrate the landed active plan and foundations, then verify one exact candidate while the dirty root remains untouched. Merge, rebase, cherry-pick, or patch topology remains unselected until that bounded candidate’s ancestry/conflict analysis is reviewed. | Starts from the current product/CI line, isolates protected dirt, and limits reconciliation primarily to the active plan/foundation surface. | Requires explicit authority to create the path and later select a history topology; root fan-in and final branch custody must be stated before promotion. | High until promotion; delete the bounded path after preserving an accepted commit. |
| C — integrate `master` into this dirty root | Merge `master` into the current branch and resolve the 12 predicted document conflicts in place. | Preserves this branch as the candidate and imports all product/CI commits in one ancestry join. | Highest custody and conflict risk; it operates beside protected user dirt and creates a broad root-doc reconciliation before U1. | Medium before commit via merge abort, but the dirty-root recovery burden is materially higher. |

Recommendation: **Option B**. It best preserves unrelated state and binds U1 to the current product line. Option A remains legal for source characterization that does not claim current-product integration; Option C should be reserved for an explicit preference to keep this root branch as the integration site.

Tree decision: **Option B selected**. This authorizes constructing the clean bounded candidate from exact `master` while leaving the root and its protected dirt untouched. It does not authorize GitHub/default/protection mutation, real host installation, target-repository writes, publication, release, or promotion of the bounded branch.

## Validation and acceptance state

- Fresh U0 read-only commands: branch/commit/tree/status, remote heads, ancestry counts, merge base, workflow contents/triggers, workflow/check/status queries, branch protection, rulesets, and read-only merge conflict analysis.
- U0 disposition: `HOLD`. Foundation authority and current control truth pass; one integrated local candidate and remote default/CI authority do not yet pass.
- Current maximum statement: exact local/GitHub control observation and executable plan refinement only. No source, package, install, discovery, runtime, recovery, product-journey, or release claim is supported yet.

## Idempotence, recovery, and cleanup

- All U0 probes were read-only except this active-plan update. Repeating them is safe and should change plan state only when an observed fact changes.
- No integration command is authorized yet. If Terry later authorizes an in-place integration and it threatens a protected dirty path or creates an unexpected conflict outside the 12 predicted documents, abort before resolving or staging and rebaseline custody.
- No temporary repository artifact, receipt, lane registry, completion manifest, or research projection was created. Read-only merge analysis may have written an unreachable Git tree object only; normal Git garbage collection owns it.

## Outcome state

- `CL-USABLE-LOOP`: undecided.
- Current transition: construct and verify D0 Option B from exact `master`, with topology chosen only from bounded ancestry/conflict evidence. GitHub default/protection changes remain a separate authority HOLD. Safe source characterization may continue, but it cannot raise the integrated-product claim ceiling until the bounded candidate is frozen.

---

# U0 — Repository and foundation authority

## U0.1 Rebaseline repository control

```bash
git status --short --branch
git branch --show-current
git rev-parse HEAD^{commit} HEAD^{tree}
git rev-list --left-right --count refs/remotes/origin/main...refs/remotes/origin/master
```

Through GitHub, re-read:

- default branch;
- `main` and `master` heads;
- workflow push/PR triggers;
- required checks;
- rulesets/branch protection; and
- latest current-branch workflow results.

Do not trust the observed 641-commit relation if it changed.

## U0.2 Lowest-churn branch repair

1. Set current `master` as GitHub default.
2. Locate the current verification workflow before changing its push trigger; this checkout currently has no `.github/workflows/verify.yml`, so do not invent or edit a replacement during U0.2.
3. Require the current verification check on `master` where controls permit.
4. Update current automation/docs that assume `main`.
5. Leave stale `main` untouched until U7.
6. Treat rename/delete as a later separate operation.

Do not combine branch repair with history rewrite, crate work, licensing, release or governance cleanup.

## U0.3 Install current foundation authority

Create:

```text
docs/foundations/current-2026-08-08.md
docs/foundations/register.csv
docs/foundations/decision-log.csv
```

Use the supplied UltraGoal foundation file and the shared register filtered to `ultragoal` and `both`.

The file must state:

- current source statuses and exact claims;
- version-sensitive guidance;
- claim ceilings;
- obsolete/historical material disposition;
- review triggers; and
- source-to-decision-delta policy.

## U0.4 Freeze the old research authority graph

Mark the following as frozen historical/compatibility inputs, not current authority:

- `docs/research-source-registry.json`;
- `docs/research-source-cards.json`;
- `docs/research-article-to-law-trace.json`;
- `docs/source-obligation-matrix.md`;
- historical source snapshots;
- contract-specific research registries/reviews;
- research-derived generated enforcement projections.

Do not delete them yet. Do not refresh them. Exclude them from default context and current product claims. Reader migration occurs in U3/U7.

## U0 acceptance

- default browsing/clone/CI authority points at current code;
- current foundation files exist;
- old research machinery cannot schedule work or block U1;
- no source-card or generated projection was refreshed;
- no branch backup or foundation receipt was created; and
- current candidate commit/tree is recorded directly in this plan.

---

# U1 — Current-source installed-loop baseline

## U1 purpose

Determine what already works and identify the first real product blocker. Do not rewrite root instructions, standards, lane templates, generated authority, source-law machinery, or observability architecture before this baseline unless they literally prevent execution.

## U1 build and focused source checks

```bash
cargo fmt --all -- --check
cargo check -p ultragoal --lib --locked
cargo build -p ultragoal --bin ultragoal --locked
cargo test -p ultragoal --tests --locked -- --list
```

Select the smallest current tests covering:

- public CLI grammar;
- repository fit;
- routine dirty-tree work;
- findings/diagnosis/next action;
- distribution identity; and
- interruption/recovery.

Do not run the complete governance/release suite merely because it exists.

## U1 external target

Create one disposable local Git repository outside UltraGoal with:

- one tracked source file;
- one intentionally modified tracked file;
- one untracked file;
- one passing affected check;
- one deterministic failing check or source condition;
- one recoverable repair; and
- no credentials, network or external service.

The external evaluator captures before/after:

```text
recursive paths and hashes
file types and modes
Git status
branch and HEAD
symlink/root identity
mtime only where behavior depends on it
```

## U1 actual command grammar

Use the built binary’s current help and capability output:

```bash
target/debug/ultragoal --json --help
target/debug/ultragoal --json inspect capabilities
target/debug/ultragoal --json inspect context
```

Exercise only current exposed routes for:

1. fit inspect;
2. fit plan;
3. accepted apply in the disposable target;
4. fit verify;
5. routine affected work;
6. deterministic failure;
7. findings and diagnosis;
8. one legal next action;
9. repair or safe refusal; and
10. unchanged repeat use.

If the grammar differs, update this plan. Do not invent compatibility commands or an equivalence receipt.

## U1 external oracle

Pass only when the evaluator observes:

- zero unrelated tracked/untracked loss;
- read-only routes make zero hidden writes;
- mutations remain inside accepted target/effect scope;
- useful work executes or returns an honest typed blocker;
- executed and reused work are distinct;
- diagnosis identifies a causal boundary and exact rerun/next action;
- interruption/failure leaves understandable recoverable state;
- unchanged repeat use avoids needless re-execution; and
- no tracked routine receipt/governance artifact appears.

Classify each step:

```text
passes_currently
partial
blocked_by_product
blocked_by_environment_or_authority
```

Record the baseline in this plan and the external run index, not a repository receipt.

If the evaluator cannot discriminate, repair the evaluator before source code.

---

# U2 — One coupled blocker repair

**Conditional:** execute only for a material `partial` or `blocked_by_product` U1 result.

## U2 topology

Use one branch/worktree and implementation owner by default. A second lane is legal only when current source proves:

- disjoint semantic authority;
- disjoint write paths;
- no peer-unmerged dependency;
- local independent oracles; and
- reserved root fan-in capacity.

Worktree isolation alone is not independence.

## U2 repair loop

For each blocker:

1. preserve the failing external fixture;
2. state one causal hypothesis;
3. demonstrate a red test, mutation, reversal, or failing external oracle;
4. make the smallest root-cause change;
5. run the narrow source check;
6. rerun the affected external endpoint; and
7. inspect unrelated target state.

Attempt two must change the hypothesis, variable or oracle. Otherwise stop and report `blocked`.

Do not edit root governance documents unless the fix changes a real public contract.

## U2 acceptance

- the original U1 blocker is retired by same-surface evidence;
- unrelated state remains preserved;
- no historical receipt is refreshed;
- no second state owner is created; and
- the exact candidate is frozen for U3/U5.

---

# U3 — Replace research authority and reduce active instruction path

**Begin only after the current product loop passes or is honestly environment/authority blocked.**

## U3.1 Rewrite research standards

Replace `agent-standards/11-research-improvement-and-quality-gates.md` with:

```text
new source/observation
→ classify authority and freshness
→ identify exact active decision
→ identify one canonical owner
→ choose no_change/update/replace/retire
→ prefer code/test/tool/schema for deterministic behavior
→ add prose only for noninferable semantics
→ record one compact decision delta if it must survive sessions
```

Remove requirements that every research source map through:

- law IDs;
- standards rows;
- obligation matrices;
- schemas;
- red/green fixtures;
- receipts;
- package/setup outputs;
- final-packet blockers; or
- update-goal blockers.

Regenerate/remove its template copy only after confirming the template reader.

## U3.2 Root instruction ownership

### `AGENTS.md`

Keep only:

- instruction precedence and untrusted-content handling;
- current goal/active-plan pointers;
- preserve unrelated work;
- safe local implementation versus approval boundary;
- simple/direct versus planned/complex work;
- load one relevant module/domain doc;
- risk-proportionate checks and final-state verification;
- stop/replan rule; and
- concise final outcome/check/risk fields.

Remove `validation_artifacts/` from default reading.

### `AGENT_STANDARDS.md`

Keep one task-to-module index and instruction to load only relevant modules. Remove its second-manual rule set, reviewer policy and completion schema.

### `GOAL_CONTRACT.md`

Keep:

- authority and superseded history;
- user, job, product/mission outcome;
- `CL-USABLE-LOOP` journey;
- non-goals;
- protected invariants;
- human decisions; and
- current claim ceiling.

Move current status to the active plan, architecture ownership to `ARCHITECTURE.md`, planning/proof/evidence/stopping to `PLANS.md`, and model routing to versioned measured configuration.

### `PLANS.md`

Keep one milestone/plan, ownership, proportional validation, evidence economy, budgets, recovery and stopping. Replace model names with:

> Use the least costly supported configuration that passes the representative task-class evaluation; parallelize only genuinely independent work with reserved integration capacity.

## U3.3 Domain modules

For each module touched by a representative U5 route:

1. remove rules already owned elsewhere or enforced mechanically;
2. retain domain-specific noninferable semantics;
3. remove version/session history;
4. replace long examples with one discriminative example only when measured;
5. link to source/config instead of copying evolving enumerations; and
6. remove default durable-artifact requirements.

Specific mandatory repairs:

- Module 01: remove universal 100–200/250-line rules and repeated naming taxonomies.
- Module 06: replace standards promotion ladder with repeated/material failure → earliest controllable layer → no-change/update/replace/retire.
- Module 07: retain proof-surface separation but remove routine receipt requirements and copied thresholds.
- Module 11: source-to-decision lifecycle only.

## U3.4 Minimal lane template

Replace `templates/LANE_EXECPLAN.md` with:

```markdown
# Lane: <outcome>

- base commit/tree:
- outcome and acceptance:
- owned paths:
- forbidden/shared paths:
- consumed dependencies/interfaces:
- local oracle:
- effect ceiling:
- repair budget and stop:

## Work

## Result

- head commit/tree:
- changed paths:
- commands and outcomes:
- blocker or requested root change:
- worktree state:
```

Remove ready/validator receipts, registry rows, universal browser/cache/port fields, four-persona review, refreshed anchors, goal-binding receipt paths, completion-manifest JSON and durable paths for reproducible checks.

## U3 acceptance

- each stable rule has one owner;
- root routers route rather than restate manuals;
- current product route excludes historical/generated/evidence roots by default;
- no source automatically creates an enforcement graph;
- routine lane work requires no receipts; and
- no task-success claim is made until U5.

---

# U4 — Product/governance/release checks and explicit runtime state

## U4.1 Split check ownership

Create or expose three clear entry points:

```text
check-product
    compile
    CLI grammar
    fit preservation
    routine work/reuse
    diagnosis/next action
    distribution identity
    essential path/custody/fault cases

check-governance
    current standards/generated compatibility
    only when their owning inputs change

check-release
    product checks plus authorized package/install/release evidence
```

Keep `scripts/check` as a compatibility wrapper if current callers require it, but do not make governance projection a prerequisite for focused product work.

## U4.2 Test classification

Retain tests whose actual oracle detects:

- wrong root or target;
- path/symlink escape;
- stale candidate or dependency identity;
- hidden writes;
- forged/pass-shaped evidence;
- unsafe cleanup;
- interruption/cancellation/custody loss;
- duplicate or ambiguous effects;
- privacy/secret leakage; or
- failure to recover.

Demote or remove tests whose only assertion is that a correctly shaped receipt/projection exists and no current product/compatibility reader consumes it.

## U4.3 Runtime state

Use Ledger’s principle only where current behavior shows stale-state/repetition defects. A compact runtime view may track:

```text
current candidate/target
observed inputs and their validity
modifications/effects
attempted commands and still-valid outcomes
in-flight/ambiguous effects
next legal action
```

Do not create another repository ledger unless cross-process recovery requires it. Affected observations become stale after state changes; unrelated observations remain usable.

## U4 acceptance

- focused product work does not regenerate governance artifacts;
- check names and claim ceilings are explicit;
- product checks are sufficient to run U5/U6;
- current operation state has one owner; and
- ordinary checks leave no tracked authority files.

---

# U5 — Byte-identical instruction/context A/B

## U5 conditions

On the same exact source candidate:

1. direct source/tests with no repository instruction route;
2. current transitive UltraGoal route;
3. reduced U3 route; and
4. reduced route plus exact relevant specialist.

Use held-out tasks covering:

- focused Rust defect;
- CLI grammar change;
- dirty-tree preservation;
- causal diagnosis;
- package/distribution identity;
- symlink/path security;
- interrupted recovery;
- stale observation after an edit;
- docs-only correction;
- no-change investigation;
- gateway selection; and
- frozen-governance compatibility conflict.

## U5 controls

Hold constant:

- source commit/tree;
- task prompt and fixtures;
- model/reasoning/tools/sandbox/approval policy;
- time/token budget;
- evaluator and hidden tests.

## U5 endpoints

Primary:

```text
all required final-state outcomes
AND no prohibited outcomes
AND correct candidate/target
```

Secondary:

- authority/preservation adherence;
- false completion;
- files/context loaded;
- unused reads;
- time to first relevant edit;
- tool calls;
- repeated unchanged actions;
- human interventions;
- wall time and no-cache/billed cost separately.

## U5 decision

Adopt the reduced route only if:

- strict success is noninferior within a predeclared margin;
- no known security, authority, preservation or recovery regression occurs;
- median active context decreases;
- unused traversal decreases; and
- the reduction does not hide the same text in automatically loaded material.

A failed task triggers mechanism triage. Do not restore broad prose from one anecdote without a future holdout.

---

# U6 — Exact Agentic coexistence and final external loop

## U6 Agentic boundary

Bind UltraGoal to one exact compatible Agentic package set:

```text
package-set version
package names/versions
manifest and aggregate digests
enabled skill identities
one required base package
optional companions
implicit gateway = UltraGoal
proposal_only = true
claim_effect = none
```

Verify:

- exactly one implicit gateway;
- explicit fully qualified Agentic selection;
- stale/wrong digest rejection;
- typed unavailable result for missing optional adviser;
- no inference from source/cache/history;
- no Agentic effect, approval, lifecycle or claim authority; and
- clean uninstall without stale rediscovery.

## U6 final product loop

On exact package/install/runtime identities, rerun the U1 journey with:

- preserved dirty target state;
- deterministic failure and recovery;
- evaluator-owned snapshots/capture;
- one optional Agentic adviser available; and
- one missing-adviser condition.

The product must remain legal and useful when advice is unavailable.

## U6 mutation suite

The evaluator must reject:

- internal tests pass but installed path absent;
- source/package/install mismatch;
- stale reuse after an affected change;
- read route writes state;
- pass-shaped receipt without operation;
- many receipts instead of required outcome;
- unrelated work loss;
- wrong-root cleanup;
- recovery without custody;
- adviser substitution;
- Agentic advice raises an UltraGoal claim; and
- a second implicit gateway.

---

# U7 — Milestone decision, retirement, and stop

## U7 decision states

```text
pass         exact candidate passes required journey/invariants
fail         external evaluator demonstrates product defect
blocked      required authority/environment unavailable
inconclusive evaluator cannot discriminate claim
```

A pass supports `CL-USABLE-LOOP` only in the tested envelope. It does not establish release, universal repository support, repeated adoption, daily-driver status, or field Product Fitness.

## U7 bounded retirement

Use current-reader analysis to remove only superseded surfaces with no current consumer:

- old source registry/cards/article-to-law trace/obligation matrix;
- obsolete generated research projections;
- duplicate standards template;
- receipt-only tests/writers with no real invariant;
- frozen contract research graphs with no compatibility reader;
- duplicate root policy; and
- temporary context/dedup analysis.

Use Git history rather than an in-tree archive.

Do not remove:

- path/identity/effect/recovery security tests;
- migration fixtures with current readers;
- exact package/protocol compatibility inputs; or
- product-loop regression fixtures.

## U7 completion

Complete when:

- one current branch and CI authority exists;
- one current foundation owner exists;
- research sources no longer auto-generate law/receipt graphs;
- current product loop passes or is honestly classified;
- root context reduction is measured, not assumed;
- product/governance/release checks have distinct owners;
- routine work creates no authoritative files;
- Agentic coexistence remains explicit and claim-neutral;
- the external evaluator decides `CL-USABLE-LOOP`; and
- the repository stops rather than beginning another governance cycle.

## Owner decisions with no safe default

Ask only for:

- branch setting/ruleset authorization;
- representative target and exact install/write scope;
- credentials/external host use;
- publication/release;
- licensing resolution;
- a second representative journey; or
- destructive deletion with ambiguous readers.

## Confidence

| Proposition | Confidence |
|---|---:|
| Branch/CI authority must be repaired first | 100% |
| Current source must be run before broad policy cleanup | 99% |
| The source-to-law-to-receipt graph should cease being current authority | 100% |
| Real path/identity/effect/recovery controls must remain | 100% |
| The root instruction path is over-composed | 100% |
| The lane template creates routine receipt theater | 100% |
| Product, governance and release checks should be separated | 98% |
| Explicit runtime state is preferable to repeated status prose where needed | 97% |
| Exact optimal context/runtime topology can be known without U5/U6 | below 10% |
| This is the strongest defensible next implementation sequence | 98% |
