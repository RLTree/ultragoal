# Install And Visibility

This document separates source layout from package, marketplace, installed
bytes, host discovery, and runtime behavior. It is source guidance only; it is
not evidence that any live Codex host has installed or loaded this candidate.

## Supported source shape

The current product shape is:

```text
.codex-plugin/plugin.json
.agents/plugins/marketplace.json       # root-owned repository catalog
.codex/agents/*.toml                   # six project-scoped read-only roles
skills/harness-ultragoal/SKILL.md      # single front door
skills/<seven-specialized-routes>/SKILL.md
```

The canonical specialized routes are `repository-fit`, `routine-work`,
`diagnose-and-observe`, `goal-run`, `prove`, `improve-and-maintain`, and
`product-journey-review`. Root-owned package inventory must include those eight
skills and the six `.codex/agents` files, and must exclude compatibility
sources from active packaged discovery unless the migration registry explicitly
adopts a bounded compatibility route.

## Repository marketplace

The supported repository catalog path is `.agents/plugins/marketplace.json`.
Its local plugin source is relative to the marketplace root and begins with
`./`. The current 0.0.15 root integration uses the package materialization
path `./plugins/harness-ultragoal`; catalog bytes are invalid evidence until
that relative target exists and its package identity is independently
reconciled.

An installed candidate carries exactly one compiled CLI runtime entry,
`runtime/ultragoal`. The public package commands require the confined explicit
input `target/ultragoal/release/ultragoal`; they never run Cargo or accept a
source-owned fallback while packaging. Once materialized at the catalog source,
the supported host execution path is
`plugins/harness-ultragoal/runtime/ultragoal`; running a separate copy outside
that resolved source cannot support the installed journey. The package binds
the explicitly selected native payload to the candidate label and archive
identity; it does not establish source-to-binary compilation provenance.
Compilation provenance and actual runtime behavior remain separate proof
surfaces.

A repository marketplace catalog identifies only a package source. P1 may
materialize an accepted package at `plugins/harness-ultragoal` only when the
current catalog is descriptor-read, binds exactly that workspace-local path,
and the complete postimage reconciles to the package tree. UltraGoal does not
register that catalog with a personal host and does not treat repository bytes
as proof that a personal marketplace alias exists.

The retained source-local host-command and lifecycle types are private,
non-personal fixtures for confined disposable testing. They cannot authorize,
dispatch, retry, roll back, or recover a personal Codex mutation. Public package
authority ends at an exact read-only handoff and read-only verification.

## Personal marketplace

Personal installation, update, removal, retry, and recovery belong to supported
Codex and the user. `package install-plan` may emit
`HarnessPersonalMarketplaceInstallHandoff-v1` only after descriptor-bound reads
bind the exact candidate, durable workspace-local source, selected Codex binary
and supported action surface, confirmed configured marketplace identity,
canonical `HOME` and `CODEX_HOME`, config/profile context, working directory,
predecessor, target, and protected state. If any input is absent, personal, or
ambiguous, it returns a typed no-effect HOLD.

The handoff names one current supported action for the user to consider. It is
not execution authority. UltraGoal never invokes that action, edits the
personal catalog, cache, or config, or claims Codex rollback behavior. After an
explicit user-owned supported action is terminal, start a fresh Codex task and
run `package install-verify --handoff <exact-owner-only-record>`. Non-target
results name one observed supported Codex/user recovery action and stop; they do
not repair or retry.

## Project-scoped agents

The current agent sources are:

```text
.codex/agents/claim-falsifier.toml
.codex/agents/orchestration-recovery-reviewer.toml
.codex/agents/product-journey-reviewer.toml
.codex/agents/repo-recon.toml
.codex/agents/research-verifier.toml
.codex/agents/security-reviewer.toml
```

They remain project-scoped. Do not direct operators to copy them into a global
agent directory. Source presence proves neither package membership nor current
host discovery. A supported new task must independently observe the exact agent
names and read-only effect boundary before a discovery claim can rise.

## Verification ladder

Verify each layer independently against the same candidate:

1. **Source:** parse the root manifest, validate exactly eight canonical skill
   identities and six project agent manifests, and reject unknown or duplicate
   active routes. Bind every direct semantic-test input, including the typed
   successor catalog, into the same invalidation closure.
2. **Package:** build two HUGPKG artifacts from the root-owned inventory and
   compare their bytes, inventory, version, and provenance inputs.
3. **Marketplace source:** validate the selected catalog path and materialized
   workspace-local source tree, while proving nothing about personal host
   registration.
4. **Supported handoff:** bind the exact current package, source, host context,
   predecessor, selected Codex identity, supported action, and expected target
   without executing the action.
5. **Install verification:** in a fresh task, read marketplace source, cache,
   config enablement, registry identity, and runtime bytes and classify only
   exact target, exact prior/no effect, partial or stale, or ambiguous.
6. **App registry and Plugins UI:** use only host-exposed observations in a
   current task. Record unsupported surfaces explicitly.
7. **Discovery:** start a fresh task and observe the front-door skill and all
   six agent identities.
8. **Runtime:** invoke representative canonical routes and inspect their real
   effects, failures, recovery, and zero-write read behavior.
9. **Product journey:** independently review fresh setup, retrofit, routine
   repeat use, diagnosis, interrupted recovery, proof, and migration.

Every executable source-level read, help, or query probe is run against an
isolated dirty repository. The before/after oracle recursively compares bytes,
object type, Unix mode, modification time, and Git porcelain status; a delta or
an operator canary echoed by a failure blocks reuse of the result.

Package, marketplace, install, cache, app registry, Plugins UI, discovery, and
runtime are not synonyms. A successful lower layer does not raise a higher claim.

The source-local public observation route is:

```text
ultragoal --root <project-root> --json inspect capabilities --package-root <package-root>
```

`--root` and `--package-root` must be distinct absolute host paths, and `HOME`
must be an absolute host path before installed, cache, or global authority is
read. Missing roots report `unavailable`; aliased or unsafe authority reports
`blocked`. A verified six-role projection is still only a local source/package/
project observation: it deliberately reports host discovery and runtime
exposure as unavailable and cannot raise a claim.

## Lifecycle coordinator boundary

The generic eight-case lifecycle model is retained only as a private,
non-personal, non-public verifier fixture. It can test transition mathematics
and interrupted local adapters, but it cannot mint or route personal install,
update, removal, rollback, restore, or recovery authority. No active skill or
package command exposes its apply or recovery adapters.

For personal Codex state, UltraGoal owns only the exact zero-write handoff and
the fresh-task zero-write verifier. Supported Codex and the user own the
mutation, cancellation after invocation, completion, and recovery. Never
retain a durable marketplace source under a disposable build or temporary path;
materialize only the catalog-confirmed workspace-local source, increment the
version on every changed installable candidate, and keep discovery and runtime
as later independent observations. Cache or config presence alone does not
prove fresh app recognition.

Product Fitness is independently withheld until accessibility, cognitive
load, recovery burden, continuance, and real-use evidence are all bound to the
same current candidate and independently reviewed. A passing source fixture or
lifecycle simulation cannot raise that ceiling.

## Safe failure

Stop the dependent layer when the required package, marketplace, host command,
agent discovery, or runtime route is unavailable or mismatched. Preserve source
and host state, name the exact unsupported surface, and give the exact next
authorized action. Do not fall back to global agent copying, a stale cache,
source inspection, or documentation as proof of live behavior.
