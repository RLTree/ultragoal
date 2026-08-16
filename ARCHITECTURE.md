# ARCHITECTURE

Harness Ultragoal is one product with two delivery surfaces:

- a Codex plugin package rooted at `.codex-plugin/plugin.json`, with skills,
  custom agents, installation metadata, and repository templates; and
- the `ultragoal` Rust binary from `validator/src/bin/ultragoal.rs`, which
  supplies typed inspection, diagnosis, enforcement, orchestration, package,
  evaluation, migration, and proof behavior.

Source, package, installed cache, host discovery, runtime activation, journey,
and release are separate proof surfaces. A valid source tree proves only the
source surface.

## Dependency Direction

The Rust implementation follows this dependency order:

1. `argument_parser/` and `cli/successor/` define the closed public grammar,
   typed values, help, and diagnostics.
2. `cli/successor_public/` maps accepted invocations to product operations. It
   is the public adapter, not a second domain model.
3. Product domains such as `inventory/`, `repository_fit/`, `routine_work/`,
   `distribution/`, `orchestration/`, `evaluation/`, and `migration/` own
   behavior and domain records.
4. `context/`, `state/`, `observability/`, `claims/`, and `capture/` bind
   candidate identity, durable state, observations, evidence, and decisions.
5. `audit/`, `generated_authority/`, and `schema_catalog/` enforce repository
   laws and reconstruct generated authority independently.

Boundary adapters may depend inward on domain records. Domain code must not
parse raw command arguments, mint root authority, or write public artifacts
through an untyped convenience path. Tests exercise production entry points;
test-only constructors cannot become a live authority route.

Routine effect custody is one private production leaf. The existing private
routine `HostState` adapter alone owns the durable store, non-clone attempt
state, process lease and process-group identity, staged outputs, cleanup
observation, descriptor-bound semantic-event append/read, terminal precommit,
recovery, rollback, and atomic terminal publication. Every routine event is
bound to the exact candidate, target, context, and source, and terminal
checkpoints are authenticated. Joined checkpoint projections are required;
the pending-append crash window may be absent or present. Reads are strictly
observational and never bootstrap missing state. The owner pins an exact host
state format before interpreting descendants. Diagnosis reads derive and
verify only the selected canonical compartment plus common structure;
mutations and explicit audits retain whole-state validation, so stale sibling
history cannot veto diagnosis while inconsistent shared state cannot authorize
a write. A behavior child may return only the closed refusal envelope; the
mediator maps its fixed reason vocabulary to public failure codes and treats
unknown, malformed, or signaled failures generically. Mediators and
output/process adapters supply typed requests or observations only; no sibling or descendant
may construct, clone, settle, release, recover, roll back, register, reopen,
or access raw event storage for routine authority. Source-shape checks are
secondary regression controls, not semantic authority proof.

Legacy HostState inspection may construct a read-only
`RoutineStateQuarantinePlan-v1` only while holding the legacy adapter lock and
only from an authenticated single, equivalent, or strictly ordered history.
The plan binds the complete descriptor inventory and specifies whole-owner
quarantine followed by a fresh v8 bootstrap; it never rewrites selected legacy
records in place. Conflicting, unsafe, unauthenticated, or changing history
cannot mint a plan. Planning authorizes no rename, quarantine, bootstrap,
migration, deletion, recovery, or claim. The only apply route is the separate
exact-record boundary described below.

Repository-fit authority device drift uses the existing `fit apply` effect
boundary rather than a sibling migration writer. A complete closed diagnosis
record and its exact accepted quarantine-plan identity are rederived under an
exclusive advisory lock on the owner parent; every ordinary repository-fit
apply or recovery holds the shared form of that same lock for its complete
HostState lifetime. The transaction binds the parent identity and complete
owner inventory, atomically moves the entire owner to an exclusive quarantine
name, fsyncs and revalidates the parent and quarantined inventory, then creates
a fresh authority without importing legacy bytes. Any failure before fresh
bootstrap restores and fsyncs the exact owner; after bootstrap begins, failure
retains both states and returns ambiguity for diagnosis. Quarantine persists
until a same-surface installed journey passes. This source route does not
authorize manual state edits, mutate the repository target, or prove an
installed recovery journey.

Complete-repository fit does not implicitly reconcile a
`validation_artifacts/` ignore rule. Candidate evidence is never ordinary
repository behavior. If the explicit `fit plan --local-state` compatibility
route remains supported, it is a narrow one-file policy only: it may reconcile
the named `.gitignore` rule while preserving unrelated bytes, newline shape,
mode, and dirt, but it cannot create or update a template-managed path.
Inspection, planning, and verification remain zero-write; an authorized apply
carries that prepared mutation through the existing confined compare-exchange,
rollback, recovery, and revalidation transaction. Its production precondition
is the descriptor-bound repository root and exact `.gitignore` target, not
unrelated repository or Git-internal contents that the one-file effect never
reads or follows.

Routine-configuration fit uses the same narrow-scope rule for its two canonical
configuration targets. Their descriptor chains, root binding, scope, and exact
path set form the protected boundary; unrelated repository and Git-internal
objects remain outside that effect. Complete-repository fit retains the full
recursive protected-tree capture.

Routine dirty-state capture treats Git-visible tracked and untracked paths as
repository authority and excludes paths that the same bound Git observation
classifies as ignored. This permits ordinary build caches such as `target/`
without weakening visible symlink, hard-link, special-file, substitution, or
concurrent-mutation refusal. Selected routine inputs and declared output scopes
retain their separate exact filesystem revalidation.

Local agent authority is one private production transaction under
`plugin_product/agent_discovery/`. The public read route
`inspect capabilities --package-root <host-path>` supplies one typed
home/package/project root set, while the transaction independently
captures source, package, installed, cache, empty-or-unrelated global, and
project authority under one candidate and session binding. Registry rows are a
four-role projection of the canonical six-role observation. This local
observation never proves host discovery, new-session comprehension, runtime
activation, route eligibility, exposure, or a claim effect.

## Agentic Engineering Advisory Layer

Harness Ultragoal has one implicit front door and one operational authority
chain:

```text
user intent
  -> Harness Ultragoal front door
  -> current candidate, state, lifecycle, risk, evidence, and truth loop
  -> smallest sufficient Agentic Engineering advisory selection
  -> proposal-only, no-claim advice
  -> root reuse / extend / map / reject decision
  -> existing plan, lease, effect, recovery, evidence, and claim owners
  -> operator-facing result
```

`engineering_advisory` owns the typed, read-only selection and adoption
projections. `plugin_product::skill_catalog` owns exact plugin, profile,
gateway, skill-set, candidate, and discovery-budget validation. Neither owns a
lifecycle, scheduler, activation store, effect, receipt, or claim. The
selection projection has no durable status: it is recomputed from current
authority and binds one primary lens plus only the supporting lenses required
by a genuine cross-layer decision.

All thirty Agentic Engineering skills are mapped to existing Harness lifecycle
owners. The existing eight-skill `ultragoal` co-install view and the core,
product/lifecycle, and Rust-system stage views are candidate-bound projections
under the external Harness gateway; the latter three cover the full skill set.
A full profile is optional and must earn a safe combined discovery and context
margin. Agentic's own gateway is explicit-only when co-installed.

Advice becomes stale when its candidate, context, lifecycle, evidence,
assumption, risk, failure mechanism, profile, or active truth-loop binding
changes. That change causes a new selection. An unchanged fingerprint returns
the current disposition and cannot recursively reactivate. Explicit expert
requests use the same selector and adoption boundary; they never bypass
authorization, effects, recovery, migration, or claim controls.

The default product response describes the result, important tradeoff, and next
action without requiring Agentic terminology. Intermediate and advanced
read-only inspection expose the selected lenses, rationale, exact inputs,
alternatives, adoption decision, unsupported surfaces, and claim ceiling.

## Public Surfaces

The only successor command groups are `inspect`, `next`, `fit`, `check`,
`diagnose`, `prove`, `observe`, `package`, `eval`, and `migrate`. Their catalog
and parser live under `validator/src/cli/successor/`; dispatch lives under
`validator/src/cli/successor_public/`. Replaced commands may remain only behind
an explicit compatibility route recorded in `migration/authority-routes.json`.

Plugin discovery surfaces are `.codex-plugin/`, `skills/`, `agents/`, and
`install/`. `plugin-manifest-draft.json` is the source package manifest.
`templates/` contains repository material installed or projected by the
product. Canonical generated-surface definitions live in
`migration/generated-surface-authority/`; their aggregate registry is
`migration/generated-surface-authority.json`.

## Codemap

```text
.codex-plugin/                 supported plugin identity
skills/ and agents/            plugin discovery content
install/                       supported host installation metadata
validator/src/cli/             public and compatibility CLI adapters
validator/src/<domain>/        typed product behavior by semantic domain
validator/tests/               cross-boundary and live-journey contracts
scripts/                       narrow repository checks and projectors
agent-standards/               canonical operating-law modules and indexes
migration/                     authority routes, retirements, projections
schemas/                       public artifact contracts
templates/                     installed repository projections
validation_artifacts/          candidate-bound evidence, never behavior
```

## Invariants

- No external string, path, JSON value, environment value, or host observation
  reaches product behavior before typed parsing and authorization.
- No read, help, parse, inspect, query, or verification route writes hidden
  state or claim artifacts.
- No worker, test helper, receipt, or generated row mints root authority or
  raises a claim ceiling.
- No generated output is authoritative without a complete canonical source
  set, deterministic generator, independent reconstruction, and same-session
  revalidation.
- No active command, reader, writer, state store, schema, or alias duplicates
  the authority named by the migration registry.

## Change Routing

- Public grammar or diagnostics: `validator/src/cli/successor/`, then its
  parser and zero-write tests.
- Product behavior: the owning semantic domain, then the single public adapter
  in `validator/src/cli/successor_public/`.
- Plugin packaging or discovery: `.codex-plugin/`, `plugin-manifest-draft.json`,
  `skills/`, `agents/`, `install/`, and package/install journey tests.
- Generated authority: canonical shards and projector first; root integrates
  the aggregate registry and verifies independent reconstruction.
- Replaced behavior: `migration/authority-routes.json` plus reader, writer,
  compatibility, and retirement tests.

`scripts/check-product` is the source/product-boundary gate and
`scripts/check-governance` checks governance projections only;
`scripts/check` is their compatibility aggregate. `scripts/check-release`
always stops at the authority boundary: it cannot perform an external,
package, installation, host, publication, deployment, or release action
without Tree approval. The public `check strict` route has separate,
recursively read-only adapters for `cli-self-law-compliance` and the
source-local `namespace-progressive-disclosure` law. A pass supports only
the requested adapter's exact claim. No command proves installation, runtime
activation, journey fitness, release, or completion by itself.
