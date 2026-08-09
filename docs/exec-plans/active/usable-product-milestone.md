# Harness UltraGoal — Foundation Reset, Current Product Loop, and Anti-Theater ExecPlan

**Repository:** `RLTree/ultragoal`
**Intended repository path:** replace `docs/exec-plans/active/usable-product-milestone.md` atomically
**Operation:** do not keep the old plan active or create another plan, lane registry, completion manifest, research-law graph, or receipt ledger
**Research lock:** 2026-08-08, America/Los_Angeles
**Observed active candidate:** `codex/successor-contract-v2-live-product@7e46169749611b3fa335700bfaf2105ae471bc48`; HEAD tree `ecf1ad39424b67fb4eed557176020aecb3b6946c`; re-resolve before any implementation
**Observed repository-control gap:** local `main` is absent; `master@30c4a19ca9ac2b5ff6b7d856ef5d3cf355e387ce` exists, `origin/HEAD` points to `origin/main@eb51ba4d6`, and this checkout has no `.github/workflows/verify.yml`; GitHub settings and current workflow authority remain unverified
**Current claim ceiling:** research, repository inspection, and executable plan only

## Local rebaseline recorded before handoff installation (2026-08-09)

- Candidate: branch `codex/successor-contract-v2-live-product`, HEAD `7e46169749611b3fa335700bfaf2105ae471bc48`, HEAD tree `ecf1ad39424b67fb4eed557176020aecb3b6946c`.
- Working tree: dirty before this handoff; four modified `validator/src/distribution/**` files and untracked `.ultragoal-e2e-unrelated-state.txt` are pre-existing and excluded from plan ownership.
- Foundation inputs installed from `/Users/terrynoblin/Downloads/research-foundations-third-pass`: `current-2026-08-08.md` (sha256 `01eae0a275d223f76bfd81ffb4d8b9fc1510675dd107dfa2a3f65667f8544b98`), `register.csv` (sha256 `39c054831f0913a4301e25ebca58afa939facfb06c6f903a4aecbe2111a8d8c1`), and header-only `decision-log.csv` (sha256 `74d0f70963d24eea519cca98f4a7e011a8062bfc3885e2f3408914fa35fc4569`).
- `cargo fmt --all -- --check`: blocked by existing formatting drift across unrelated Rust files; no formatter run was authorized.
- `cargo check -p ultragoal --lib --locked`: passed.
- `cargo build -p ultragoal --bin ultragoal --locked`: passed.
- `cargo test -p ultragoal --tests --locked -- --list`: blocked during test compilation by existing fixture/API mismatches (including `observability_fixture_scratch`, `public_api_witness`, and routine host checkpoint symbols).
- `target/debug/ultragoal --json --help`, `inspect capabilities`, and `inspect context`: passed read-only; the context reports this dirty candidate and `inspect capabilities` reports host discovery unavailable without an authorized package root.
- No GitHub branch/default/protection/workflow state was changed or verified; no external target repository, install, package, remote, or release effect was performed.

These observations are planning evidence only. They do not raise the claim ceiling above research, repository inspection, and executable plan.

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

- [ ] U0 restore branch/CI truth and establish the current foundation owner (foundation owner installed; branch/CI authority still pending).
- [ ] U1 run the current installed operator loop before governance redesign.
- [ ] U2 repair only observed blocking product boundaries.
- [ ] U3 replace the research-authority model and simplify active instructions.
- [ ] U4 separate product, governance and release checks; add only needed runtime state.
- [ ] U5 evaluate current versus reduced instructions on byte-identical source.
- [ ] U6 verify exact Agentic coexistence and rerun the external loop.
- [ ] U7 decide `CL-USABLE-LOOP`, retire bounded obsolete surfaces, and stop.

---

# U0 — Repository and foundation authority

## U0.1 Rebaseline repository control

```bash
git status --short --branch
git branch --show-current
git rev-parse HEAD^{commit} HEAD^{tree}
git rev-list --left-right --count main...master
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
