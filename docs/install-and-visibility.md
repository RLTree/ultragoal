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
`./`. The reserved 0.0.12 root integration uses the package materialization
path `./plugins/harness-ultragoal`; catalog bytes are invalid evidence until
that relative target exists and its package identity is independently
reconciled.

The packaged runtime entry remains `runtime/runtime-probe-bin`. Once the
package is materialized at the catalog source, the supported host execution
path is `plugins/harness-ultragoal/runtime/runtime-probe-bin`; running a
separate copy outside that resolved source cannot support the installed
journey.

A repository marketplace is non-default host configuration. After the root has
accepted the catalog and materialized the exact package, an authorized operator
may plan these host effects:

```text
codex plugin marketplace add <repository-root>
codex plugin add harness-ultragoal@<repository-marketplace-name>
```

Do not execute those commands during source validation. They mutate host state
and require the selected repository marketplace, package bytes, and authority
to be current.

The source-local `HostCommandPlan` binds those exact argv rows to the package
identity and carries an empty scrubbed environment, a 30-second timeout, and
one attempt. The frozen source joins that plan to sealed lifecycle admission
and typed fail-closed outcome records in its focused authority tests, but it
does not yet provide the root-owned production caller or Darwin process
backend that would consume and enforce those fields. Therefore this source
contract is not evidence of command execution, host mutation, installation,
or fresh-task discovery; those claims remain outside this candidate's ceiling.

## Personal marketplace

The default personal marketplace file is
`~/.agents/plugins/marketplace.json`. It is discovered implicitly; do not add
it through `codex plugin marketplace add`. A personal entry must point at the
actual authorized local plugin source and include installation,
authentication, and category policy.

For an already configured local marketplace, first confirm through the host's
supported marketplace-listing surface which marketplace currently surfaces the
plugin. Reinstall only after that observation:

```text
codex plugin add harness-ultragoal@<confirmed-local-marketplace>
```

Use a new Codex task after an authorized reinstall so discovery can be observed
without stale task context. Never hand-edit host marketplace or cache state as
a substitute for the supported install path.

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
3. **Marketplace:** validate the selected catalog path and relative source,
   while proving nothing about installation or host registration.
4. **Install:** materialize the authorized package and compare installed bytes
   with the package identity.
5. **Cache:** observe the selected cache separately and reconcile it to the
   installed package; do not infer hidden cache state.
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

The source-local lifecycle coordinator covers eight operations with typed
plan, apply, verify, and recovery semantics:

| Operation | Required invariant |
| --- | --- |
| Fresh install | The observed state is absent and a host write is explicitly authorized. |
| Monotonic update | The target version is strictly newer and the expected installed digest still matches. |
| Failed-update recovery | The exact captured prior installed and cache authority is restored before reuse. |
| Authorized rollback | The target is older and a separate downgrade authorization is present. |
| Idempotent reinstall | Matching installed and cache bytes are verified without replacement. |
| Uninstall and teardown | Installed and cache authority are removed and absence is verified. |
| Stale-cache recovery | Installed authority is preserved while cache identity is reconciled. |
| Repeat use | Installed bytes, cache identity, and runtime behavior are re-observed without writes. |

Planning and verification are read-only. Applying a host mutation remains
behind an explicit adapter and authorization; source tests do not authorize or
perform installation. Every effect rechecks the observed prior state. A failed
effect restores that prior authority or returns a recovery-required result.

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
