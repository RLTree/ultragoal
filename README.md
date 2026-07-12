# Harness Ultragoal

Harness Ultragoal is a Codex plugin and typed Rust CLI candidate for fitting
repositories, running conservative routine checks, diagnosing failures,
orchestrating durable goals, proving named claims, reviewing product journeys,
and improving or retiring harness behavior.

The source tree is under active successor-contract integration. Source,
package, marketplace, install, cache, host discovery, runtime behavior, product
journeys, readiness, and release are separate truth surfaces. The presence of
these files proves none of the higher surfaces by itself.

## Start here

Use `$harness-ultragoal:harness-ultragoal` as the only first-entry skill. It
selects one of seven specialized workflows after disclosing the effect,
required authority, candidate boundary, and unavailable capabilities:

1. `$harness-ultragoal:repository-fit` for fresh setup or retrofit.
2. `$harness-ultragoal:routine-work` for dirty-tree-safe affected checks.
3. `$harness-ultragoal:diagnose-and-observe` for causal diagnosis and local
   semantic queries.
4. `$harness-ultragoal:goal-run` for durable dependency-closed orchestration.
5. `$harness-ultragoal:prove` for one named strict claim boundary.
6. `$harness-ultragoal:improve-and-maintain` for evaluation, research,
   migration, compatibility, or retirement.
7. `$harness-ultragoal:product-journey-review` for independent quality-in-use,
   security, recovery, or fresh-operator review.

If more than one route appears relevant, select the skill that owns the
operator's immediate outcome and list later workflows as follow-ons. Use
`goal-run` only when the immediate outcome is coordinated multi-scope
execution. If the target, authority, or requested outcome is ambiguous, stop
selection and ask one focused question.

## Product resources

- [Plugin resource map](docs/plugin-resource-map.md): deterministic routing,
  representative journeys, and the six project-scoped read-only agents.
- [Install and visibility](docs/install-and-visibility.md): source, package,
  repository marketplace, personal install, discovery, and runtime boundaries.
- `.codex-plugin/plugin.json`: root-owned Codex plugin metadata.
- `plugin-manifest-draft.json`: root-owned package inventory input.
- `.codex/agents/`: the six current project-scoped read-only agent roles.
- `skills/`: canonical skills plus compatibility sources awaiting root-owned
  migration and retirement decisions.
- `validator/`: the Rust enforcement and acceleration kernel.

## Command boundary

Probe the actual binary before relying on it:

```text
ultragoal --json --help
ultragoal --json inspect capabilities
ultragoal --json inspect context
```

The source candidate is frozen with every direct semantic-test input, including
the typed successor command catalog. Any member-byte or protected root-metadata
change invalidates the proposal before its tests can be reused. Read, help, and
query probes are exercised against an isolated dirty repository and must
preserve recursive content, object type, mode, modification time, and Git
status.

Help, parse, inspect, query, diagnose, and next-action selection are read-only
and must produce zero hidden writes. Mutating commands disclose structural
effects: `WorkspaceWrite`, `ExternalWrite`, or `Destructive`. A prompt or source
file never proves that a command, plugin, model, permission, agent, or host
surface is currently available.

## Proof boundary

Use live evidence from the same surface as the claim. Unit tests, fixtures,
schemas, docs, receipts, generated rows, telemetry, signatures, and package
bytes can support narrower checks but cannot substitute for installed bytes,
host discovery, representative runtime behavior, preserved repository state,
quality in use, release, or completion.
