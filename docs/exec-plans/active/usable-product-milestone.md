# Harness UltraGoal — Foundation Reset, Current Product Loop, and Anti-Theater ExecPlan

**Repository:** `RLTree/ultragoal`
**Intended repository path:** replace `docs/exec-plans/active/usable-product-milestone.md` atomically
**Operation:** do not keep the old plan active or create another plan, lane registry, completion manifest, research-law graph, or receipt ledger
**Research lock:** 2026-08-08, America/Los_Angeles
**Observed active candidate:** branch `codex/usable-loop-master-candidate`; committed HEAD `5640de128899e92f807d5a12f4e9ccd88ab55337`, tree `ee5159526756f29beba1963548a3892c86672267`; exact `master@30c4a19ca9ac2b5ff6b7d856ef5d3cf355e387ce` plus plan/foundation custody and the U2 guidance repair; U3 is an uncommitted integrated change and must be frozen to a new exact commit/tree before later-stage claims
**Observed repository-control gap:** GitHub default remains stale `main@eb51ba4d6ea8844197e855d42fa239ef898be47d`; the bounded candidate contains the `master` verification workflow, but it still triggers pushes only to `main`; no branch protection, required check, ruleset, or current-candidate workflow result exists
**Current claim ceiling:** research, repository inspection, and executable plan only

## Selected bounded candidate custody (2026-08-09)

- Tree decision: Terry selected the clean `master`-based bounded candidate.
- Worktree: `/Users/terrynoblin/.codex/worktrees/usable-loop-master-candidate`; branch `codex/usable-loop-master-candidate`.
- Base: exact `master@30c4a19ca9ac2b5ff6b7d856ef5d3cf355e387ce`, tree `8b08ff3251f8a472704020665a13fb1a8da384e8`.
- Integration topology: read-only cherry-pick simulation showed only the active plan conflicts; the three foundation files were clean. The bounded branch therefore applies the two plan/foundation-only commits in order, resolving the first active-plan conflict in favor of the landed plan. No product source or protected root path changed in those commits.
- Integrated plan candidate before this state update: `390a76bb0829b37914c8bd0fb184f606ad331051`, tree `6a8a48e2c1e3f52b05842f3f4d40e5a6c0023ffc`; `master` is its direct ancestor and it is two commits ahead.
- Foundation digests remain `01eae0a275d223f76bfd81ffb4d8b9fc1510675dd107dfa2a3f65667f8544b98`, `39c054831f0913a4301e25ebca58afa939facfb06c6f903a4aecbe2111a8d8c1`, and `74d0f70963d24eea519cca98f4a7e011a8062bfc3885e2f3408914fa35fc4569`.
- The dirty root checkout remains on `codex/successor-contract-v2-live-product`; its four modified distribution files and untracked sentinel remain unchanged at the protected digests recorded below.
- U3 integration is currently uncommitted in the bounded worktree. The unrelated formatter-shaped distribution duplicates were removed from this candidate only after their bytes were matched to Terry's root changes. No protected distribution path or sentinel is present in the candidate diff.

## U1 current-product baseline (2026-08-09)

- Exact source candidate: `937815a3364fc5fe76d94a038f9f3cd0fdec1f5c`, tree `4cafe73b51cba0a6f9c3800f7da23ad82bd1f348`. The built debug executable is `target/debug/ultragoal`, SHA-256 `634d6f497ad3610660fc56b97dc92c2a006222ed9fbfe9951570b44aa27b2972`, mode `0755`.
- Source gates: `cargo check -p ultragoal --lib --locked` and `cargo build -p ultragoal --bin ultragoal --locked` pass. `cargo fmt --all -- --check` reports broad pre-existing formatting drift; no formatter was run. `cargo test -p ultragoal --tests --locked -- --list` fails while compiling several integration-test crates because their crate/module imports no longer match current source. This is a source-check gap, not evidence that the external product loop ran or failed.
- Read-only public routes: `--json --help`, `inspect capabilities`, and `inspect context` pass on the exact executable. Capabilities truthfully withholds package-root, host-discovery, and runtime-exposure claims. Candidate bytes and Git status were unchanged by those probes.
- Disposable target: `/private/tmp/ultragoal-u1-fixture.cjIQ2R/repo@71fea8e7756dc0d53f3b4ef64cc71328fe2c5809`, tree `39947538d50f7533bb97302bc469dc81c42d6293`, with one intentionally modified tracked Rust file and one untracked sentinel. Evaluator snapshots before fit, after `fit inspect`, and after `fit plan` are byte-identical, SHA-256 `70fdf8683f584a5c11d550d8978ce1fcd0b81d9600e1d11b16ebc2f419f9949e`.
- The emitted `RepositoryFitPlan-v1` has accepted identity `sha256:7dc37c88d57c1a0f92372f631f7f033fe2e639127bbe11ca20ed8d8473c97fb1`, zero conflicts, 73 declared target mutations, and one declared `.gitignore` update. The evaluator-owned plan is outside the target and no apply was attempted.
- Authority classification: public fit apply and routine derive authority only from the process `HOME` and persist beneath `$HOME/.codex/state`; no supported CLI state-root selector exists. Writing Terry's real host state is not authorized, and repurposing `HOME`/`CODEX_HOME` is prohibited by the conductor environment. Accepted apply, routine execution/reuse, failure, diagnosis, and recovery are therefore `blocked_by_environment_or_authority`, not silently substituted with a test-only adapter.
- First material product partial: the shipped repository-fit skill tells an operator to pass a relative plan path, while the parser requires an absolute host path. This is `partial` and enters U2; it is independent of the host-effect authorization hold.

## U2/U3 current implementation state (2026-08-09)

- U2 repair is committed at `5640de128899e92f807d5a12f4e9ccd88ab55337`: shipped repository-fit guidance and its current reader now require the exact unchanged plan projection in a current-user-owned absolute regular file and use `plan.plan_sha256` as the acceptance identity. Focused repository-fit and source-contract oracles pass.
- An experimental explicit `--state-root` route was independently security-falsified and then fully removed before integration. Alternate roots could fork target effect/recovery authority; ancestor replacement and extended ACLs were not bounded; and generated rerun/next instructions could drop the selected authority binding. No state-root files or grammar changes remain. A future implementation requires one target-derived durable domain binding and lock plus descriptor-chain and ACL validation, or a deliberate retirement of ambient effectful compatibility.
- U3 is in progress. Root routers/contracts/modules/templates, research-to-law current fan-in, retained compatibility classification, and current inception/state ownership are integrated locally. Public state, fit plan/apply, routine, observe, and diagnosis contexts no longer bind the adopted-handoff identity; explicit package, inventory, and migration routes retain named compatibility contexts. Current Product Success semantics no longer execute AMEND/v2 history prose. Minimal current-only fixtures withhold historical/generated/evidence authority inputs, and retained predecessor bytes cannot alter current authority. Authority-separation acceptance now awaits final independent falsification and exact candidate freeze.
- Ordinary routine operation still reads and writes `validation_artifacts/observability/spool` as behavior state. This is not a historical authority input, but it conflicts with `ARCHITECTURE.md`'s evidence/behavior separation and keeps literal U3 evidence-root acceptance conditional until U4 moves local events under the existing host routine-state owner.

## D0 input root rebaseline (2026-08-09)

- Durable goal: `active` for “Complete the sole active Harness UltraGoal ExecPlan through CL-USABLE-LOOP on one exact integrated candidate, preserving unrelated state and obeying every Tree approval boundary”; no token budget is set.
- Candidate custody at D0 input: branch `codex/successor-contract-v2-live-product`, HEAD `cbdc0bdd275e5988a2dbf3e219823c11ccab3796`, HEAD tree `ae3e210bc97ebde058628ad54a054fd7806688f1`, exactly aligned with `origin/codex/successor-contract-v2-live-product` at observation time.
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

- [ ] U0 restore branch/CI truth and establish the current foundation owner (clean `master`-based candidate, foundation owner, and fresh control observations complete; Tree-authorized default/protection repair pending).
- [x] U1 classify the current operator loop: read-only fit passes; effectful journey is honestly authority-blocked; one operator-facing plan-path partial enters U2.
- [x] U2 repair only observed blocking product boundaries.
- [ ] U3 replace the research-authority model and simplify active instructions (authority separation integrated locally; independent recheck and clean freeze pending; evidence-root runtime location closes in U4).
- [ ] U4 separate product, governance and release checks; add only needed runtime state.
- [ ] U5 evaluate current versus reduced instructions on byte-identical source.
- [ ] U6 verify exact Agentic coexistence and rerun the external loop.
- [ ] U7 decide `CL-USABLE-LOOP`, retire bounded obsolete surfaces, and stop.

## Surprises and discoveries

- The verification workflow is not missing repository-wide: it exists on `master`, but not on stale default `main` or the current branch. GitHub’s retained workflow registration therefore cannot substitute for current-branch CI.
- The plan branch is not the current product line by ancestry. It contains the landed plan/foundation commits while `master` contains 12 product/security/CI commits; one root integration is required before U1 can bind an exact current candidate.
- A read-only merge analysis confines textual conflicts to 12 root, standards, product-contract, and active-plan files. The protected dirty distribution paths are disjoint from the incoming `master` delta.
- The selected bounded topology reduces that broad merge to two plan/foundation-only commits on exact `master`; its simulation exposed only one active-plan conflict and no product-source conflict.
- The current binary exposes no explicit disposable authority-root argument. Fit and routine both consume ambient `HOME`; crossing from read-only planning to effect would write canonical host state even when the target itself is disposable.
- The external fit planner is deterministic and zero-write on the representative dirty target, but the shipped skill's relative plan-path example cannot satisfy the parser's absolute-host-path contract.
- The broad `--tests -- --list` gate currently fails at integration-test crate wiring before a complete test inventory can be listed; focused product checks must be repaired or selected without treating source presence as proof.
- A first explicit-state-root implementation would have created parallel effect/recovery domains for one target and did not close ancestor/ACL or authority-preserving rerun boundaries. Removing it was the smallest safe repair; the host-state effect remains an explicit authority hold rather than an implementation claim.
- A U3 formatting pass touched four protected distribution paths inside the bounded worktree. Their exact diff matched Terry's still-preserved root changes; all bounded-worktree copies were then restored to the candidate base and are absent from the current diff.
- The first broad `cargo test -p ultragoal --lib --locked` run completed with 1,567 passed, 219 failed, and 1 ignored. Serial classification separated current-route fixture drift from frozen compatibility and shared-fixture failures: current state/inception/fit/routine/observe/diagnosis routes now pass focused tests; a stale generated-surface v2 test fixture was repaired to the current v3 registry projection; explicit migration verification honestly reports three retained compatibility errors; the remaining broad-suite cleanup belongs to U4 check classification rather than U3 authority semantics.

## Decision log

- The committed `docs/exec-plans/active/usable-product-milestone.md` is the sole executable ExecPlan. The downloaded `ultragoal-EXECPLAN-v4.md` remains handoff/provenance only; its stale candidate and workflow assumptions will not overwrite the landed adaptations.
- Preserve the five unrelated working-tree changes by digest and path. Do not format, stage, commit, overwrite, or use them as candidate inputs.
- Hold UltraGoal CLI invocation until this fresh U0 custody/control refinement is durable. Afterward, invoke only the exact current grammar on the integrated candidate.
- D0 was a no-safe-default integration choice. Terry selected Option B, the bounded `master`-based candidate. Keep the dirty root checkout fixed; select the bounded candidate’s merge/rebase/cherry-pick/patch topology only after exact ancestry/conflict simulation.
- After simulation, the selected topology is the two plan/foundation-only commits on exact `master`; the first conflict is resolved to the landed plan, and the second U0 observation update applies directly. This choice is confined to the bounded branch and does not promote it or mutate remote authority.
- Keep the GitHub default/protection mutation at HOLD until Terry explicitly authorizes the exact remote effects.
- Treat U1 apply/routine as `blocked_by_environment_or_authority` until an exact real-host state effect is approved or a supported explicit disposable authority-root product contract is implemented and same-surface tested. Do not repurpose `HOME` or call a test-only adapter as an equivalence.
- Use the absolute-plan guidance contradiction as U2's first product repair. It is the earliest observed operator-facing product partial that can be fixed without broadening host authority.
- Withhold the experimental explicit-state-root grammar. It is not required by U4, and source-level convenience does not justify a second unresolved effect/recovery authority domain. Preserve the real-host effect approval boundary until either Terry authorizes that exact scope or a separately approved, descriptor-bound single-domain design is implemented and same-surface falsified.
- Treat the four distribution deltas in the bounded worktree as unrelated formatter contamination, not product input. Exclude them from every commit and validation candidate; remove only the bounded-worktree copies after exact comparison confirms the canonical dirty-root bytes remain unchanged.
- Treat `validation_artifacts/observability/spool` as a U4 operation-state ownership defect, not as current authority. Do not route ordinary observe/diagnose commands back through the retained context to preserve it. Move current local events under the existing routine host-state owner with read-only no-bootstrap semantics, and keep any legacy target spool frozen unless a reader-specific migration is proven.

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
- U0 disposition: `HOLD`. Foundation authority, current control truth, and one clean local `master`-based candidate pass; remote default/CI authority does not yet pass.
- U1 disposition: `PARTIAL / AUTHORITY HOLD`. Current-source build and read-only fit inspect/plan pass with evaluator-observed zero target delta. Broad integration-test listing is source-blocked; the plan-path guidance is product-partial; apply/routine and downstream recovery remain authority-blocked.
- U2 disposition: `PASS` at the shipped guidance/source-contract surface on committed candidate `5640de128899e92f807d5a12f4e9ccd88ab55337`; no higher product surface is implied.
- U3 disposition: `HOLD` pending final independent source-reader recheck and exact candidate freeze. Current authority separation is locally implemented; literal evidence-root-location acceptance remains conditional on the U4 runtime-state repair.
- Current maximum statement: exact source build, read-only CLI/disposable-target fit-plan behavior, and focused U2/U3 source tests. No package, install, discovery, effectful runtime, recovery, complete product-journey, or release claim is supported yet.

## Idempotence, recovery, and cleanup

- All U0 probes were read-only except this active-plan update. Repeating them is safe and should change plan state only when an observed fact changes.
- The authorized bounded integration completed without touching the dirty root. No in-place root integration is authorized. If a later promotion threatens a protected path or creates an unexpected conflict, stop before resolving or staging and rebaseline custody.
- No temporary repository artifact, receipt, lane registry, completion manifest, or research projection was created. Current enforcement/generated-authority projections were regenerated only as a consequence of changing their canonical current U3 inputs. Read-only merge analysis may have written an unreachable Git tree object only; normal Git garbage collection owns it.
- U1 retained only evaluator-owned files under the mode-`0700` disposable `/private/tmp/ultragoal-u1-fixture.cjIQ2R` container. The target fixture remains unchanged from its intentional dirty/untracked baseline. Preserve it through U2 same-surface rerun; remove it at final cleanup.

## Outcome state

- `CL-USABLE-LOOP`: undecided.
- Current transition: complete the final U3 falsification, freeze the clean authority-separation commit/tree, then split product/governance/release checks and move target evidence behavior under the current host-state owner in U4. Keep the real host-state effect, GitHub default/protection mutation, real install, and representative non-disposable target write on explicit authority HOLD.

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
