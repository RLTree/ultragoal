# Harness UltraGoal — Complete One Installed Usable Loop

This ExecPlan is a living document maintained under `PLANS.md`. Its only
purpose is to decide `CL-USABLE-LOOP` on one exact installed candidate. Git
history owns prior chronology. This file owns only current truth, remaining
work, effect boundaries, recovery, and the terminal product decision.

## Purpose and observable outcome

An authorized repository operator must be able to use one exact installed
Harness UltraGoal candidate to fit a representative dirty repository, run
useful affected work, understand a deterministic failure, take one legal next
action, recover or refuse safely, and repeat unchanged useful work without
losing unrelated tracked or untracked state.

The external evaluator decides the milestone only after observing that complete
journey through the installed product surface. Source checks, package bytes,
install success, receipts, or reviewer agreement cannot substitute for it.

## Authority and starting custody

`GOAL_CONTRACT.md` defines the product goal and protected invariants.
`PRODUCT_SUCCESS_CONTRACT.md` and `PRODUCT_FITNESS.md` define acceptance.
`ARCHITECTURE.md` owns code and durable-state boundaries. `PLANS.md` defines
planning, evidence, recovery, and stopping rules.

P0 starts from:

- repository `/Users/terrynoblin/Projects/harness-ultragoal-plugin-proposal`;
- branch `codex/successor-contract-v2-live-product`;
- source baseline commit `00d769058739a6ade2277785e5f8b1b1cbc88dce`;
- source baseline tree `49afc87c1acf643759f6012364932c4dd0dbd741`;
- installed Harness UltraGoal `0.0.41+codex.20260824093100`; and
- no project `.codex/config.toml`; the global user configuration is the
  active default.

This plan is a packaged source resource. Rewriting it changes the successor
candidate identity even though it causes no install, runtime, HostState, or
target effect. Freeze, build, and package work must therefore occur after P0.

The following user-owned working-tree state is protected and outside this
plan's write scope:

- `validator/src/distribution/host_effect/executor/tests/mod.rs`;
- `validator/src/distribution/host_effect/mod.rs`;
- `validator/src/distribution/host_effect/selected_codex_executable/selected_tests.rs`;
- `validator/src/distribution/package/manifest_bind.rs`; and
- `.ultragoal-e2e-unrelated-state.txt`.

The combined tracked protected diff at P0 entry is SHA-256
`7d1451497a7992fbc21bb3f7c3474c7e2ec35a937e4d2bf1594f623a6999ebc9`.
Every checkpoint must re-resolve HEAD, tree, status, and this digest. A mismatch
stops integration before staging or effect.

## Integration ownership

The root conductor in the canonical checkout is the sole integration owner for
P1 through P4. That owner retains shared public grammar, version and package
bindings, effect-authority fan-in, accepted-commit ordering, reviewer
correction, and terminal `CL-USABLE-LOOP` acceptance. A bounded work owner may
implement one coherent P1 or P3 change but cannot independently change shared
surfaces, integrate itself, cross a P2 authority gate, or decide P4.

## Current product truth

### Established at the installed 0.0.41 surface

- Marketplace source, cache, and installed runtime were observed byte-identical.
- Repository fit completed and verified 73 of 73 declared conditions
  idempotently while preserving unrelated tracked and untracked state.
- The installed routine journey stopped before effect on two real product
  defects: an overbroad migration observer and write-capable opening of an
  authority lock during a read-only operation.
- `CL-USABLE-LOOP` is therefore `blocked_by_product` for installed 0.0.41.

### Established on the current source candidate

- Migration observation is confined to the relevant owner and reserved
  namespace, terminal-event evidence is classified causally, and read-only
  authority authentication uses read-only access.
- Default-root repository-fit planning resolves the representative nested
  target without an explicit `--root` workaround and without a hidden write.
- Ordinary migration remains planless for incomplete or conflicting legacy
  history.
- The separate read-only command
  `ultragoal --json migrate abandon-plan --target <repository>` emits
  `RoutineStateAbandonmentPlan-v1` with a consequence-bound
  `RoutineStateQuarantinePlan-v6`.
- Applying that record requires the immutable record, its exact
  `routine-quarantine-sha256` plan identity, current source/target/owner
  evidence, and `--approve-retirement`.
- The isolated actual-binary journey proves zero-write planning, refusal
  without approval, whole-owner quarantine, fresh `routine-host-state-v8`
  bootstrap without legacy authority import, replay idempotence, dynamic
  `RoutineNext-v1`, fresh routine execution then reuse, and preservation of
  unrelated target bytes.
- The current release binary previously derived a real zero-write plan against
  the preserved representative target. That observation is comparison evidence
  only; its plan identity must be rederived and must not be reused after source,
  install, target, or HostState drift.
- P1a repair commit `0d8feda22bf5a7c2f9bb488ef4b404cf6566ded7`
  closes exact-home/predecessor admission, parent-death lease custody,
  production cancellation, exact-prior restart custody, causal diagnostics,
  and plan truth at source and isolated-process surfaces. Independent rereview
  accepted those six findings and kept one material blocker open: macOS
  `sandbox-exec` grants descendant writes by pathname, not by the retained
  `.codex` descriptor. A suspended child can therefore write into a different
  ordinary directory renamed into the accepted pathname after spawn.
- The deterministic red oracle
  `spawn_boundary_real_directory_substitution_cannot_mutate_either_tree` is
  retained as an explicitly ignored test in
  `validator/src/distribution/host_effect/selected_codex_executable/execution/darwin_custody_tests.rs`.
  Running it with `--ignored --exact` fails against `0d8feda22` because the
  substituted directory is mutated; its corrected symlink companion still
  passes. P1a-S must remove the ignore only after making this exact oracle
  green. `/dev/fd/<directory-fd>` is not traversable as a child namespace on
  macOS, and initializing the sandbox before pathname substitution does not
  bind the rule to the original vnode.
- Current upstream Codex installation performs two coupled effects: plugin
  cache replacement and semantic enablement in `CODEX_HOME/config.toml`.
  Therefore a safe successor cannot patch the direct pathname-authorized child
  route. It requires an independently approved staged-CODEX_HOME transaction
  with a retained-descriptor commit and cross-process recovery. P1a is
  `blocked_by_product` at this architecture gate; P1b remains closed. No live
  Codex command or personal-host effect was exercised.

### Still unknown

- Whether the staged-CODEX_HOME plus retained-descriptor transaction can be
  bounded without copying arbitrary personal state, relying on undocumented
  Codex storage, or weakening the same-user pathname-substitution threat model.
- Whether architecture/security review approves exact cache/config preimages,
  postimages, commit ordering, durable recovery, cancellation, and cleanup.
  Only approved architecture followed by an approved immutable implementation
  rereview may open P1b version freeze and packaging.
- Whether the live HostState still produces an acceptable abandonment plan.
- Whether authorized live abandonment completes without ambiguous effect.
- Whether the installed successor completes useful routine work, deterministic
  failure, causal next action, recovery or refusal, and unchanged reuse.
- Whether the journey is understandable and helpful enough to satisfy Product
  Fitness on this tested envelope.

## Non-goals and deferred questions

This plan does not schedule public marketplace publication, release, GitHub
default-branch or ruleset changes, universal repository support, repeated
adoption, daily-driver fitness, a UI, a telemetry platform, privileged tracing,
the U5 instruction/context A/B campaign, or broader Agentic coexistence work.

The prior U5 campaign, prior version checkpoints, superseded migration and
quarantine generations, and the prior 0.0.41 terminal decision remain in Git
history. They may return only if this installed canary proves one is a causal
prerequisite or Terry authorizes a separate later product goal.

## Program invariants

1. Preserve unrelated user work and exact source, target, and HostState custody.
2. Keep read, inspect, plan, diagnose, next, and verify routes free of hidden
   writes.
3. Bind each effect to the exact candidate, target, scope, current observation,
   plan identity, and applicable authority.
4. Fail closed on path escape, ambiguous ownership, stale identity, duplicate
   effect, incomplete recovery, or post-effect ambiguity.
5. Keep source, test, build, package, install, discovery, runtime, journey,
   Product Fitness, and release claims separate.
6. Package and version work are internal gates of the installed canary, not
   standalone milestones or recurring approval cycles.
7. Local source, package, and disposable corrections do not consume the
   product-defect repair budget.
8. After the first installed canary, permit at most one material product-defect
   repair followed by one exact installed rerun. A repeated mechanism failure
   or second independent material product defect ends this plan at a product
   decision; it never triggers another automatic version cycle.
9. Add no receipt, schema, evaluator ledger, or governance surface unless a
   demonstrated cross-process recovery or irreproducible observation requires
   it.
10. Full-auto operation covers in-scope local implementation, directly coupled
    checks, packaging, disposable verification, ordinary repair, and plan
    maintenance. It does not silently grant personal-install, live HostState,
    representative-target, publication, release, credential, or external
    service authority.

## Remaining-work graph

```text
P0 replace the sole active plan with current truth and validate its bindings
  |
P1a-A remap and independently approve a staged-CODEX_HOME plus
      retained-descriptor cache/config commit transaction
  |
P1a-S implement and prove that transaction, including the preserved red
      pathname-rebinding oracle and cross-process recovery
  |
P1b freeze one monotonic successor and complete every local pre-host gate
  |
P2 cross separately authorized install and live-product gates, then run the
   complete installed canary under a minimal independent evaluator
  |
P3 if and only if P2 finds one material product defect, repair it once and
   rerun the exact installed canary
  |
P4 decide CL-USABLE-LOOP and close this plan
```

P1 and P2 are one product outcome, separated only because P2 contains distinct
consequential authority boundaries. A green P1 proceeds without another
planning cycle. A missing P2 authority stops only that transition and preserves
the exact candidate and read-only plan evidence.

## Progress

- [x] P0 replace this file in place, validate current authority/package
  bindings, preserve protected state, and commit only the plan. The first
  aggregate run exposed a missing canonical `The external evaluator` literal;
  restoring that binding made the focused current-authority regression and
  complete `scripts/check .` aggregate pass on 2026-08-27. Two bounded revision
  rounds closed five plan/source-binding findings; independent immutable review
  approved exact P0 commit `3e75f650fc4552fd0e85fb0dab6c016a54a7ed0b`.
- [x] P1a-D falsify the direct pathname-authorized child design. Candidate
  `0d8feda22bf5a7c2f9bb488ef4b404cf6566ded7` closes six of seven review
  findings, but a real-directory suspended-spawn oracle proves that the child
  can mutate a same-path substitute. Direct repair is stopped.
- [ ] P1a-A specify and independently approve the staged-CODEX_HOME plus
  retained-descriptor cache/config transaction before assigning implementation.
- [ ] P1a-S implement the approved transaction, make the ordinary-directory
  oracle green without weakening it, pass focused/process/aggregate gates, and
  obtain a new immutable rereview.
- [ ] P1b freeze a monotonic successor, pass exact-source checks, build the
  release CLI and deterministic package, verify it, complete disposable
  installation, and derive a read-only personal-install plan.
- [ ] P2 obtain the exact internal effect authorities, install through the
  supported lifecycle route, observe same-byte runtime identity, derive a fresh
  live abandonment plan, apply it only under its exact authority, and complete
  the installed useful-work/failure/recovery/reuse journey.
- [ ] P3 conditional: repair at most one material installed product defect and
  rerun the same P2 journey on one new exact monotonic candidate.
- [ ] P4 record one compact same-surface product decision, clean disposable
  evidence after proving no unique recovery state remains, and move this plan
  to completion.

## Milestone P0 — Current-truth reset

Replace the chronological U0-U7 ledger with this self-contained plan. Retain
only current candidate/install/HostState truth, protected state, claim ceiling,
effect gates, recovery, and deferred dependencies. Do not copy prior chronology
to another active registry or plan.

Validate from the repository root:

```sh
git diff --check -- docs/exec-plans/active/usable-product-milestone.md
scripts/check .
git status --short
```

The repository check is required because this plan participates in package
inventory and candidate identity. Stage and commit only this file after the
checks pass and the protected digest remains exact.

P0 acceptance is a committed, restartable plan whose Progress, current truth,
authority gates, repair budget, and terminal outcomes agree. P0 raises no
runtime or product claim.

## Milestone P1 — Live-install capability, exact successor, and pre-host gates

P1a-D established that the unavailable personal-install binding cannot be
repaired safely by running the selected Codex child directly against the live
personal `CODEX_HOME`. The retained directory descriptors authenticate the
preimage, but the macOS child sandbox authorizes writes by pathname. Pre/post
revalidation detects a same-path real-directory substitution only after an
escaped write; advisory lease custody does not constrain a noncooperating
same-user process. Do not resume or weaken that design.

P1a-A is an architecture gate owned by the root conductor and reviewed at both
product-simplicity and security/trust-boundary surfaces. Its candidate must
define one bounded transaction with all of these properties:

1. Run the selected Codex child only against a disposable private staged
   `CODEX_HOME`; the child never receives the live personal HOME or `.codex`
   pathname as a writable scope.
2. Admit the exact retained personal-home authority, exact `config.toml`
   preimage or absence, exact prior plugin-cache tree, source/package identity,
   selected executable, and immutable install plan before staging.
3. Copy only the minimum reviewed non-secret inputs needed by current Codex.
   Refuse if successful staging would require auth, transcripts, sessions,
   arbitrary personal state, an undocumented store, or an unbounded tree.
4. Require a closed staged-tree manifest. The only accepted semantic postimage
   is the exact target cache tree plus the one expected plugin-enabled config
   change; every other staged mutation refuses before personal effect.
5. Commit through retained descriptor-rooted confinement, not personal
   pathnames. Revalidate exact cache/config preimages at each boundary and use
   compare/exchange semantics for both surfaces.
6. Define commit ordering, a durable owner-only recovery record, exact rollback
   snapshots, parent-death/interruption/cancellation reconciliation, replay,
   and ambiguity. No success is legal until cache, config, registry, source,
   runtime, and lease all reobserve exact target.
7. Preserve unrelated config formatting and fields, unrelated plugin cache,
   global user configuration, and protected repository state. Cleanup removes
   only exact disposable staging/recovery paths after proving no unique recovery
   authority remains.

The architecture review must bind these requirements to current upstream Codex
source: `install_resolved_plugin` installs the cache and then calls
`set_user_plugin_enabled`; the store atomically replaces
`plugins/cache/<marketplace>/<plugin>`; the config editor reads or creates and
atomically rewrites `CODEX_HOME/config.toml`. Review must reject a cache-only
shim, raw TOML string patch, full personal-home clone, pathname-only commit, or
revalidation-only boundary.

Only an approved P1a-A contract may open P1a-S implementation. At the lowest
falsifying surfaces, P1a-S must retain the six accepted findings and prove the
ordinary-directory race, exact-prior apply and target replay, staged-manifest
refusal, config/cache partial commit in both orders, interruption after every
possible effect, result loss, exact rollback/replay, cancellation, lease
contention, post-effect observation, and cleanup. A required durable recovery
record is permitted here because the newly established two-surface commit is a
real cross-process recovery boundary. If this requires editing a protected
path, stop for an ownership decision rather than overwrite user work. This
pre-host architecture and implementation remain inside P1 and do not consume
P3.

Only after that adapter and its failure matrix pass, advance every canonical
`0.0.41+codex.20260824093100` product-version binding to one monotonic
`0.0.42+codex.<UTC timestamp>` candidate, including the local unpublished
policy references required by current package validation. Do not publish those
policies or package bytes. Exclude every protected path.

Set `/Users/terrynoblin/Projects/harness-ultragoal-plugin-proposal/.codex-worktree/tmp/runtime-tmp`
as `TMPDIR` for execution-capable checks and isolated temporary roots. Keep the
existing package contract: reproducible outputs live beneath
`target/ultragoal`, product-check build output lives beneath
`target/ultragoal-product-check`, and the exact release CLI is
`target/ultragoal/release/ultragoal`. Do not create work under `/tmp` or
`/private/tmp`.

On the exact post-version source candidate:

1. run the focused version, manifest, package-identity, and lifecycle tests;
2. run `cargo check --locked --offline -p ultragoal`;
3. build `target/ultragoal/release/ultragoal`;
4. run `scripts/check .` on a capable execution surface;
5. write package inventory and package output beneath `target/ultragoal`;
6. build twice and require byte-identical archives;
7. verify the archive against the exact release CLI and current source;
8. run `package install-test` without retaining an isolated root; and
9. run the read-only `package install-plan` against the unchanged personal
   installation.

The public command grammar and canonical P1 paths are:

```text
target/ultragoal/release/ultragoal --json package inventory --output target/ultragoal/inventory.json
target/ultragoal/release/ultragoal --json package build --output target/ultragoal/package-a.hugpkg --cli target/ultragoal/release/ultragoal
target/ultragoal/release/ultragoal --json package build --output target/ultragoal/package-b.hugpkg --cli target/ultragoal/release/ultragoal
target/ultragoal/release/ultragoal --json package verify --input target/ultragoal/package-a.hugpkg --cli target/ultragoal/release/ultragoal
target/ultragoal/release/ultragoal --json package install-test --input target/ultragoal/package-a.hugpkg --output target/ultragoal/install-test.json --cli target/ultragoal/release/ultragoal
target/ultragoal/release/ultragoal --json package install-plan --input target/ultragoal/package-a.hugpkg --cli target/ultragoal/release/ultragoal
```

Before P1 completion, record exact source commit/tree/status, plan digest,
release-CLI digest, both archive digests, disposable-install result and cleanup,
install-plan identity, installed predecessor observation, and protected-state
digest. Correct ordinary pre-host package or custody defects in this same
milestone; they do not consume P3.

P1 stops before `package install-apply`, live HostState mutation, or target
effect. Its ceiling is exact source/build/archive/disposable-install/read-only
install-plan evidence.

## Milestone P2 — Installed successor canary

P2 is one outcome lane with separate internal authority gates. Do not infer one
gate from another.

### Gate A: personal installation

Require explicit authority for the exact P1 install plan and personal
marketplace/cache/config scope. Revalidate candidate, archive, predecessor,
install-plan identity, the recorded digest and identity of the exact P1 release
CLI at `target/ultragoal/release/ultragoal`, global Harness config projection,
and protected state immediately before effect. Apply only through that exact P1
release CLI:

```text
target/ultragoal/release/ultragoal --json package install-apply --plan <exact-plan-file> --accept-plan <exact-plan-id>
```

After effect, reconcile marketplace source, cache, supported Codex
registration, config postimage, and release runtime bytes to the exact
candidate. Failure or ambiguity invokes the approved P1a-S staged-transaction
recovery contract and cannot be reported as success.

### Gate B: live abandonment and target effects

Through the exact installed runtime, run read-only fit verification, `diagnose`,
`next`, and:

```text
ultragoal --json migrate abandon-plan --target <representative-repository>
```

Capture fresh evaluator-owned before snapshots. Compare the new plan with the
last observation, but accept only the current immutable bytes and identity.
Present or otherwise bind the exact target, HostState owner, counts, causal
relation, plan identity, and these consequences:

- legacy continuity is abandoned;
- no legacy authority is imported;
- previous reuse and recovery are unavailable; and
- declared target-local routine outputs may re-execute.

Require explicit authority for that exact live abandonment plan and the exact
representative-target effect scope. Refusal or drift remains a no-effect HOLD.
Apply only through:

```text
ultragoal --json migrate apply --target <representative-repository> --plan <exact-record> --accept-plan <exact-plan-id> --approve-retirement
```

Reconcile the whole-owner quarantine, fresh v8 bootstrap, absence of imported
legacy authority, unrelated target preservation, and exact replay
`already_applied_no_effect` before routine work.

### Gate C: complete product journey

Using the same installed bytes and representative target:

1. verify fit and current dynamic `next`;
2. run useful dirty-tree affected work;
3. repeat unchanged work and observe lawful reuse;
4. introduce one deterministic representative failure within the authorized
   target scope;
5. run `diagnose` and `next` and require a causal explanation plus one exact
   legal action derived from current evidence;
6. recover or refuse safely with custody understandable;
7. rerun useful work and verify the repaired or safely refused state; and
8. compare evaluator-owned after snapshots for target preservation, hidden
   writes, HostState, installed bytes, config projection, and retained artifacts.

## Minimal independent evaluator

The evaluator is independent of the deferred U5 context campaign. It owns:

- before/after target, Git-status, HostState, installed-runtime, marketplace,
  cache, and configuration projections;
- deterministic failure injection and expected recovery boundary;
- unrelated tracked/untracked preservation and hidden-write oracles;
- first useful-value event and time to verified value;
- operator interventions and comprehension of target, effect, consequence,
  refusal, correction, and recovery;
- effectiveness, cognitive/recovery/trust burden, retained-artifact cost, and
  any false pass or false rejection; and
- one exhaustive result: `pass`; `material_product_defect` with one exact causal
  mechanism; `blocked_by_product` after the repair budget is exhausted;
  `blocked_by_environment_or_authority`; or `inconclusive`.

UltraGoal's output, tests, and receipts are evidence inputs, not the evaluator's
authority or final decision. A first `material_product_defect` enters P3 only
while its repair budget remains. The P3 rerun must map to `pass`,
`blocked_by_product`, `blocked_by_environment_or_authority`, or `inconclusive`;
no generic failure value or unmapped result may exit the plan.

## Milestone P3 — One conditional product repair

P3 exists only if the first installed P2 canary exposes one material product
defect after same-byte runtime observation. Identify one causal mechanism and
the lowest surface that can falsify it. Repair that mechanism and its directly
coupled failure path, freeze one new monotonic candidate, repeat P1, and rerun
the full P2 journey.

A pre-host package, disposable-custody, test-harness, or documentation
correction remains inside P1 and does not consume P3. A repeated mechanism
failure, renamed equivalent failure, or second independent material product
defect ends P3 and proceeds to P4 as `blocked_by_product`. Do not append P4,
create another automatic repair tranche, or bump another version.

## Milestone P4 — Product decision and stop

Record one compact candidate-bound outcome:

- `pass`: the exact installed candidate completes every journey step with no
  prohibited outcome; claim `CL-USABLE-LOOP` only for this tested envelope and
  close the plan;
- `blocked_by_product`: the repair budget is exhausted or a second material
  product defect appears; close the plan and return continue, narrow, redesign,
  or retire as a new owner decision;
- `blocked_by_environment_or_authority`: a required capability or explicit
  effect authority is unavailable; preserve the candidate and exact resumable
  boundary without treating it as product failure;
- `inconclusive`: evidence cannot distinguish success from failure; preserve
  custody, state the missing oracle, and stop.

Do not use P4 to authorize publication, release, a second representative
journey, broader autonomy, U5 execution, or destructive cleanup outside the
exact disposable roots proven safe to remove.

## Surprises and discoveries

- The prior active plan became a 1,211-line chronological ledger. It marked U7
  complete and stopped while U5 remained open and many post-decision repairs
  continued. Its later current-state section also lagged the newer explicit
  abandonment evidence.
- Packaging and installation repeatedly became standalone slices even though
  they are dependencies of one installed-product outcome. This delayed the
  first-truth loop and obscured the actual product unknown.
- The plan itself participates in package identity. Plan reset must precede
  successor freezing rather than being treated as effect-neutral housekeeping.
- Approval of abandonment behavior and safeguards authorized source behavior
  and isolated tests, not application to the preserved live HostState.
- The current-authority reader deliberately treats the literal sentence prefix
  `The external evaluator` as a contract-binding marker. Rephrasing the same
  idea as “an evaluator” made `inspect context` return
  `successor_runtime_state_unavailable`; restoring the canonical marker repaired
  the state projection without weakening validation.
- The first immutable P0 review found four restartability defects: the known
  unavailable live-install adapter was mislabeled unknown and omitted from P1,
  artifact paths contradicted current package grammar, evaluator outcomes were
  non-exhaustive, and no integration owner was named. These are plan defects;
  correcting them neither starts P1 nor raises the proof ceiling.
- The direct personal-install child design remained unsafe even after six of
  seven P1a findings closed. A real ordinary-directory substitution after
  suspended spawn bypassed the pathname sandbox and mutated non-retained
  authority. macOS exposed no supported descendant-write capability rooted in
  the inherited directory descriptor for an unmodified Codex child.
- Current upstream Codex installs a plugin across both cache and
  `config.toml`. This converts the next truthful step from another spawn repair
  into an architecture-reviewed staged transaction with multi-surface recovery.
  The red oracle is preserved so the remap cannot erase the discovered defect.

## Decision log

- Decision: retain the `CL-USABLE-LOOP` goal and replace the active plan in
  place rather than append or prematurely retire the product.
  Rationale: the goal remains narrow and user-visible; the stale execution map,
  not the goal, caused the loop.
  Date: 2026-08-27.
- Decision: use Git history for chronology and keep this plan limited to
  current executable truth.
  Rationale: explicit current state is cheaper to resume and less likely to
  schedule obsolete work.
  Date: 2026-08-27.
- Decision: defer U5 context A/B, remote controls, broad Agentic coexistence,
  publication, release, and governance retirement.
  Rationale: none is a prerequisite for the local installed journey in
  `GOAL_CONTRACT.md`.
  Date: 2026-08-27.
- Decision: allow one material installed-product repair and exact rerun.
  Rationale: this preserves first-truth learning while making the stop rule
  operational.
  Date: 2026-08-27.
- Decision: keep personal installation and live abandonment/target authority as
  distinct internal gates inside one outcome lane.
  Rationale: outcome-sized work does not collapse independent consequential
  authorities.
  Date: 2026-08-27.
- Decision: make the live personal-marketplace adapter and its failure matrix a
  known P1 prerequisite, then retain canonical `target/ultragoal` package paths.
  Rationale: current source already proves the capability is absent and current
  public grammar already owns the output namespace.
  Date: 2026-08-27.
- Decision: name the root conductor as sole integration owner and exhaustively
  map evaluator results into P3 or the four legal terminal outcomes.
  Rationale: shared authority surfaces and chat-independent resumption require
  one owner and no outcome may leave the plan without a legal next state.
  Date: 2026-08-27.
- Decision: stop the direct pathname-authorized personal Codex child design and
  insert P1a-A architecture/security review before any further implementation.
  Rationale: the ordinary-directory race proves the child can write outside
  retained inode authority, and upstream Codex couples cache replacement with a
  config edit. Repeated spawn hardening cannot provide the missing descriptor
  write primitive or truthful two-surface recovery.
  Date: 2026-08-28.

## Idempotence, recovery, and cleanup

P0 and P1 local checks are repeatable. Canonical reproducible package outputs
live only under `target/ultragoal`; product-check build output lives under
`target/ultragoal-product-check`. Temporary runtime and isolated-install roots
live beneath the ignored project-local `.codex-worktree/tmp/runtime-tmp`. Do not
retain isolated install roots unless a failure requires causal inspection;
after resolution, prove no unique recovery state remains and remove only the
exact product-owned path.

Before P2 effects, capture exact source, package, installed predecessor,
marketplace/cache/config, HostState, target, plan, and protected-state custody.
The independently approved P1a-S staged transaction must own install recovery;
until it exists, no personal install effect is legal. The existing whole-owner
quarantine transaction owns abandonment recovery. Any ambiguous effect stops
before routine work and is reconciled through those interfaces rather than
manual rewriting or deletion.

Never clean the user-owned protected paths, the representative target, live
HostState, marketplace/cache, global Codex configuration, or deferred U5
artifacts as incidental P0-P4 cleanup.

## Outcomes and retrospective

P0 is independently approved at exact commit
`3e75f650fc4552fd0e85fb0dab6c016a54a7ed0b`. Direct P1a candidate
`0d8feda22bf5a7c2f9bb488ef4b404cf6566ded7` closes six reviewed boundaries but
is `HOLD` on same-path ordinary-directory child-write escape. P1a-A
architecture review is the only open transition; P1a-S, P1b, and every P2
effect remain closed. The protected diff remains exact, no project
`.codex/config.toml` exists, and no install, runtime, HostState, target,
publication, or release effect occurred. `CL-USABLE-LOOP` remains
`blocked_by_product` for installed 0.0.41. At P4, replace this paragraph with
the terminal candidate-bound decision and move the completed plan according to
repository convention.
