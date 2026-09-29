# Harness Ultragoal Plugin Resource Map

This is the source-level product map. It defines one preferred entry and seven
specialized workflows so an operator does not need to remember an internal
sequence of lanes, gates, receipts, or helper scripts.

## Canonical skill topology

`$harness-ultragoal:harness-ultragoal` is the only front door. It selects
ordinary native execution or one specialized primary route when needed:

| Immediate outcome | Canonical skill | First capability probe | Effect ceiling |
| --- | --- | --- | --- |
| Complete a self-contained implementation, analysis or documentation task | `harness-ultragoal` directly | Relevant task inputs; no runtime startup sequence | Existing authorized scope only |
| Set up a fresh repository or retrofit an existing one | `repository-fit` | `ultragoal --json fit inspect --target <relative-path>` | Read until an accepted apply plan |
| Run affected checks while preserving a dirty tree | `routine-work` | `ultragoal --json check routine --target <relative-path>` | Declared local `WorkspaceWrite` |
| Explain a failure, query local events, or choose the next action | `diagnose-and-observe` | `ultragoal --json inspect findings` | Read; export is separately approved `ExternalWrite` |
| Coordinate multi-owner dependencies or recover in-flight work | `goal-run` | Current acceptance and ownership; typed context only when needed | Existing authorized scope only |
| Prove one named claim at its exact truth surface | `prove` | `ultragoal --json inspect claims` | Declared proof output `WorkspaceWrite` |
| Evaluate, research, migrate, preserve compatibility, or retire | `improve-and-maintain` | `ultragoal --json eval audit --spec <relative-path>` or `migrate plan` | Read until an accepted bounded operation |
| Independently falsify a real operator journey | `product-journey-review` | Probe every required command and host surface | Read-only reviewer |

The old skill names may remain as non-authoritative compatibility sources until
the root-owned migration registry retires them. They are not preferred product
routes and must never be selected as a fallback when a canonical capability is
missing.

## Check routing

| Need | Command | Ceiling |
| --- | --- | --- |
| Source-local UltraGoal product behavior | `scripts/check-product` | Compile and focused product-contract evidence only. |
| Current standards and generated compatibility inputs | `scripts/check-governance` | Governance projection only. |
| Existing callers that require both | `scripts/check` | Product plus governance; not a release gate. |
| Release-surface verification | `scripts/check-release` | Holds at one clean committed candidate until Terry separately authorizes an exact release scope; it does not infer approval from an environment variable. |

## Supported personal-install boundary

UltraGoal owns package construction, one exact zero-write supported-Codex
handoff, and fresh-task read-only install verification. Supported Codex and the
user own every personal install, update, removal, retry, cancellation after
invocation, and recovery effect. `package install-plan` returns a typed
no-effect HOLD unless it can bind the exact monotonic candidate, materialized
workspace-local marketplace source, selected Codex binary and supported action,
configured marketplace, canonical personal scope, predecessor, target, and
protected state. `package install-verify --handoff <exact-owner-only-record>`
classifies exact target, exact prior/no effect, partial or stale, or ambiguity;
it never repairs state. Discovery, installed runtime invocation, Product
Fitness, and release are later independent proof surfaces.

## Deterministic selection

Complete ordinary self-contained work directly with native tools. Do not load another
skill or inspect runtime state just because the task includes code, analysis or
documentation. For a specialized capability, select its owning skill:

1. A request to independently review or falsify an existing journey selects
   `product-journey-review`.
2. A request to prove a named claim selects `prove`.
3. A symptom, finding, failure, query, repair plan, or next-action request
   selects `diagnose-and-observe`.
4. Fresh setup, retrofit, partial fitting, or an ownership conflict selects
   `repository-fit`.
5. Affected tests, changed-impact validation, or safe reuse selects
   `routine-work`.
6. Substantial multi-owner execution or recovery of in-flight dependencies selects
   `goal-run`; sequential dependencies remain with the existing owner.
7. Evaluation, research refresh, compatibility, migration, or retirement
   selects `improve-and-maintain`.

When one prompt contains multiple outcomes, choose the earliest required dependency
and continue through the remaining authorized outcome. Implementation that includes
diagnosis, checks or documentation does not by itself require another skill.
Multiple outcomes do not mandate workers. Resolve a target from current workspace evidence when
unambiguous; ask only for a missing decision that materially changes correctness
or authority. A missing typed capability holds its dependent operation. Independent
native work retains its own authorization and actual execution proof surface.

## Calibrated next action

Keep one outcome, constraints and acceptance contract across Astra, Sol, Terra
and Luna. Model and effort remain configurable; record effective values only
when the runtime exposes them. Select context and probes for the next dependent
action, revalidating identity and authority before typed effects. The existing
plan and actual operation owner retain authority; historical lane registries do
not schedule ordinary work.

Delegate substantial independent work when its benefit earns the extra cost.
One root owns integration; sequential dependencies do not require a team. Use
bounded independent review for material trust-boundary or uncertain acceptance
decisions. Run required and relevant checks, expanding only for changed state,
failures or unresolved risk. Keep reproducible output ephemeral unless a current
claim, cross-process handoff or recovery consumes it. Report useful outcomes
and material caveats rather than internal routing fields.

## Automatic Agentic advisory selection

The Harness front door automatically makes the advisory calibration after it
has selected one operational route. It selects no lens for an already-specified
or no-change task; otherwise it selects one available explicit Agentic skill
only when current evidence identifies a material ambiguity. This is a second
calibration inside the existing route, not a second route or lifecycle. It
reads current candidate, lifecycle, risk, authority, evidence, active
truth-loop, and failure context and chooses the smallest sufficient primary
lens. Supporting lenses appear only when one material decision crosses layers.
Missing or stale co-install exposure is an explicit unavailable-advice result,
not permission to infer a lens from source or cache bytes.

The existing eight-skill `ultragoal` co-install view is retained for its
bounded current scope. The complete Agentic Engineering 4.0 four-pack skill
set is reachable through three additional stage-scoped views:

- core advisory for task framing, context, architecture, construction,
  orchestration, verification, security, and learning;
- product/lifecycle advisory for discovery, feasibility, authentic use,
  requirements, Product Fitness, experimentation, release, readiness,
  maintenance, and retirement; and
- Rust systems advisory for architecture, runtime, durability, protocols,
  verification, and observability.

The three stage views cover all thirty Agentic skills. Each selected skill is
fully qualified by its owning package. Every view binds the external
`harness-ultragoal` gateway, stage-profile digest, exact four-package aggregate,
all four per-package manifest identities, current Harness candidate, current
configuration, all four independently recomputed package-source inventory
digests, and closed skill set. A missing companion package, mutable same-version
source substitution, legacy monolithic `3.0.1` profile, duplicate package,
wrong digest, or second implicit gateway is unavailable rather than a fallback.
The exact accepted package set is Agentic Engineering `4.0.0` from
commit `3ebedbbf0967386057724ee166043ce5c39d6acf`, with aggregate
`sha256:f1a4d45fe88e9c9b572609ff79630c9750fa04a552d8904db3858353b482caf7`.
That source/cache identity is structural evidence only; actual host
configuration and fresh-session discovery remain separate proof surfaces.
Agentic's gateway is explicit-only when co-installed. A full thirty-skill view
is not loaded by default and must first demonstrate a safe combined context
margin and no routing degradation.

Advice has no lease, permission, effect, evidence, review, or claim authority.
`AgenticCoInstallProfile-v2` and `AdvisorySelectionRequest-v2` carry the changed
wire contract; `EngineeringAdvisorySelector-v2` emits
`EngineeringAdvisorySelection-v2`. The source selector consumes one exact
candidate-bound profile and read-only, no-claim catalog projection, verifies
their complete rendered skill set, and produces either a plain-language,
proposal-only selection or a visible no-selection/fail-closed result. It does
not persist an activation tracker: unchanged input suppresses reactivation,
while changed candidate, context, configuration, lifecycle, evidence,
assumption, risk, failure mechanism, package bytes, profile, or active
transition invalidates prior advice. A root-issued
`EngineeringAdvisoryAdoption-v2` can then record reuse, extension, mapping as a
projection, or rejection through an existing UltraGoal owner. It validates the
exact pack-set, profile, selector, candidate provenance, fully qualified routes,
and selection identity; it cannot create advisory effect, evidence, or claim
authority. Public CLI routing, package, installation, discovery, runtime, and
journey proof remain separate pending surfaces.

The ordinary response remains plain language. `inspect` may expose the
activated disciplines and rationale, while advanced inspection may expose
exact skill paths, evidence, alternatives, adoption decisions, invalidation
conditions, and claim ceiling. An explicit expert request uses the same
authority boundary and cannot disable mandatory guards.

## Representative journeys

### Fresh repository

1. Enter through `harness-ultragoal` and select `repository-fit`.
2. Run `fit inspect` and `fit plan` without writes.
3. Save the unchanged plan projection to a current-user-owned absolute regular
   file under a canonical, non-symlinked parent, and retain its
   `plan.plan_sha256`.
4. Present every mutation, preservation rule, conflict, rollback, and effect.
5. Run `fit apply` with that absolute plan path and accepted `plan_sha256` only
   after acceptance of the unchanged current plan.
6. Run `fit verify`, then route actual routine work to `routine-work`.

### Partial retrofit or conflicting authority

Use `repository-fit`. Classify existing owners and preserve all unrelated
modified, staged, untracked, and worktree state. Conflicts remain findings;
they are never resolved by preference or hidden behind generated output.
When the first blocked journey transition is only routine activation,
`fit plan --routine-config` may plan the two owned routine configuration files.
That explicit scope excludes every conflicting template and `.gitignore`; the
accepted apply still revalidates the same scoped candidate and cannot widen it.
When those files are current but the routine artifact store is not ignored,
`fit plan --local-state` may plan only the existing `.gitignore` policy. It
carries no template mutation and uses the same accepted apply, rollback, and
revalidation path.

### Routine repeat use

Use `routine-work`. Recompute changed impact, verify every reuse key against the
current candidate and environment, run only the dependency-closed affected set,
and compare the full declared workspace boundary before and after. A strict
claim boundary follows through `prove`; routine work does not become release
ceremony.

For a public routine interruption, `check routine` accepts only the typed
`--interrupt-after reservation` control and returns an opaque continuation.
Resuming requires that exact continuation and the host-authenticated binding.
An exact completed binding returns its authenticated prior result without a new
effect; stale, cross-binding, forged, or ambiguous records remain refusals.
Use `diagnose-and-observe` to read the resulting finding and event evidence;
those read routes do not reconcile or mutate routine state.

### Failure and diagnosis

Use `diagnose-and-observe`. Inspect the current candidate and findings, query
bounded local events when available, derive one causal explanation, and return
one legal next action. The selected repair executes only through the workflow
that owns its effect.

### Interrupted orchestration

Use `goal-run`. Refresh candidate, target, authority and preserved state for the
next action. Reconcile worker leases/results only when workers exist; preserve
accepted outputs and unique partial effects. Resume the legal remainder after
stale or ambiguous state is resolved. Parallel activity does not prove recovery.

### Strict proof

Use `prove` for one claim. Bind prerequisites, positive behavior checks,
false-pass controls, outputs, and an independent reconciler to the current
candidate. Root authority alone decides claim promotion.

### Improvement and migration

Use `improve-and-maintain`. Audit tasks and scorers before evaluation. Treat
research and metric gains as candidates. Inventory active readers and writers,
verify replacement behavior, preserve explicitly adopted compatibility, and
require destructive approval before retirement.

## Current project-scoped agents

The six current read-only roles live under `.codex/agents/`. Their presence in
source does not prove package inclusion or host discovery.

| Agent | Use |
| --- | --- |
| `repo-recon` | Recompute repository, worktree, command, component, and candidate truth. |
| `research-verifier` | Recheck mutable primary-source and capability facts. |
| `product-journey-reviewer` | Falsify acquisition-through-completion and quality-in-use journeys. |
| `claim-falsifier` | Attack prerequisites, wrong surfaces, stale evidence, guards, and ceilings. |
| `security-reviewer` | Attack confinement, effects, secrets, permissions, and supply chain. |
| `orchestration-recovery-reviewer` | Attack leases, dependency closure, interruption, reconciliation, and recovery. |

Do not copy these files into a global agent directory as part of source setup.
Use project-scoped discovery when the current host exposes it. If the host does
not expose the named role in the current task, record discovery as unsupported
and lower the dependent review ceiling.

## Human attention policy

The plugin handles deterministic routing, capability probes, read-only
inspection, causal diagnosis, safe retry planning, and independent-work
advancement. Interrupt the operator only for a missing choice that changes the
product outcome or authority, an external write, an unavailable required
access, a secret boundary, or a destructive decision. The interruption names
the exhausted safe routes, the exact blocker, preserved state, and the exact
next action.

## Calibrated assurance

Use the smallest proof loop that can honestly support the current claim.
Implementation uses required and relevant checks. A material trust-boundary
change or ambiguous acceptance decision gets a bounded independent review of
the named invariant. Add reviewers only for distinct unresolved risks or when
the specific milestone contract requires them; ordinary completion does not
automatically require a team. Source review supports only source acceptance.

Re-review requires changed authority-bearing bytes, a changed consumed
dependency, a newly eligible claim surface, or observed behavior that
contradicts the prior decision. A clean exhaustive pass closes the loop;
speculative hardening becomes bounded backlog rather than another review round.
Routine output stays ephemeral. Persist only the smallest artifact consumed by
an active claim, cross-process handoff, irreproducible observation, audit, or
recovery need, and delete it when that need ends.

Mandatory security, privacy, destructive-effect, authority, identity, recovery,
and proof-separation boundaries never become optional. Other universal checks
must earn their cost through recurrence or strong cross-repository evidence and
must remain precise, inexpensive, actionable, and easier to maintain than the
failures they prevent.

## Truth surfaces

Keep these surfaces separate and candidate-bound:

1. source coherence;
2. deterministic package bytes and package inventory;
3. repository or personal marketplace catalog observation;
4. installed bytes;
5. cache identity;
6. app registry and Plugins UI observation;
7. new-task discovery;
8. representative runtime behavior;
9. repository and product journeys;
10. release and completion.

A lower surface never proves a higher one. Help, parse, inspect, query,
diagnose, and next-action selection must also prove zero hidden writes at the
same recursive tree and Git-status scope as the command.

## Lifecycle and Product Fitness source modules

`validator/src/plugin_product/` is the source candidate for lifecycle
coordination, transitive build closure, and Product Fitness disposition. Root
integration must wire it into the crate and public command catalog only after
independent review.

- `lifecycle` owns fresh install, monotonic update, failed-update recovery,
  authorized rollback, idempotent reinstall, uninstall and teardown,
  stale-cache recovery, and repeat-use state transitions.
- `source_closure` binds repo-relative Cargo manifests, lock state, dep-info,
  Rust sources, runtime authority, verifier inputs, and dynamic inputs into one
  `BuildClosure-v1` aggregate. It rejects missing, unknown, duplicate, alias,
  outside-root, symlink, hard-link, special-file, stale, and final-session-drift
  inputs.
- `product_fitness` binds accessibility, cognitive load, recovery burden,
  continuance, and real-use evidence. The reviewer is falsification-only and
  cannot raise a claim ceiling.

The truth ladder is source, package, marketplace, install, cache, app registry,
Plugins UI, discovery, runtime, and journey. Each layer retains its own ceiling;
an unsupported or unobserved layer is explicitly withheld.

The source-candidate freeze includes these docs, the eight canonical skills,
the route and journey fixtures, their semantic tests, the plugin descriptor,
the package manifest, and `validator/src/cli/successor/catalog.rs`. Root-only
agent descriptors and marketplace absence enter only through root-rederived
metadata. Either kind of input invalidates the full extension when it changes;
the issued lease candidate identity and the worker artifact aggregate remain
separate fields.
