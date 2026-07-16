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

Routine effect custody is one private production leaf. That leaf alone owns
the durable store, non-clone attempt state, process lease and process-group
identity, staged outputs, cleanup observation, terminal precommit, recovery,
rollback, and atomic terminal publication. Mediators and output/process
adapters supply typed requests or observations only; no sibling or descendant
may construct, clone, settle, release, recover, roll back, register, or reopen
routine authority. Source-shape checks are secondary regression controls, not
semantic authority proof.

Local agent authority is one private production transaction under
`plugin_product/agent_discovery/`. The registry control plane supplies one
explicit home/package/project root set, while the transaction independently
captures source, package, installed, cache, empty-or-unrelated global, and
project authority under one candidate and session binding. Registry rows are a
four-role projection of the canonical six-role observation. This local
observation never proves host discovery, new-session comprehension, runtime
activation, route eligibility, exposure, or a claim effect.

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

`scripts/check` is the repository-wide source/standards/coverage gate. The
public `check strict --claim cli-self-law-compliance` route is the product
self-law surface; neither command proves installation, runtime activation,
journey fitness, release, or completion by itself.
