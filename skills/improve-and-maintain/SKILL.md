---
name: improve-and-maintain
description: "Evaluate and maintain Harness Ultragoal without creating duplicate authority. Use for task and scorer audits, paired behavior evaluation, failure harvesting, current primary-source research, improvement candidates, compatibility routing, migration verification, or approved retirement."
---

# Improve And Maintain

Turn observed failures and current research into bounded candidates, then
migrate or retire old surfaces through measured compatibility. Evaluations,
research, adapters, and generated artifacts cannot promote product law or
claims by themselves.

## Inputs

Require current candidate identity, behavior or surface under review,
evaluation specification and dataset, baseline, contamination and negative
controls, current source requirements, migration registry, compatibility
observations, active readers and writers, ownership, rollback plan, and explicit
external or destructive authority.

## Evaluate and research

Audit before running:

```text
ultragoal --json eval audit --spec <relative-spec-path>
ultragoal --json eval run --spec <relative-spec-path> --output <relative-output-path>
ultragoal --json eval harvest --input <relative-input-path> --output <relative-output-path>
ultragoal --json eval promote --candidate <candidate-id> --output <relative-output-path>
```

`eval audit` is Read. Check task validity, scorer incentives,
representativeness, provenance, contamination, split leakage, reproducibility,
and negative controls. `run`, `harvest`, and `promote` write bounded local
candidates only; they do not adopt behavior. An external adapter is
`ExternalWrite` and requires an explicitly named provider and authority:

```text
ultragoal --json eval adapter --spec <relative-spec-path> --provider <provider-id>
```

For research refresh, verify current primary sources and separate external
fact, binding requirement, advisory practice, hypothesis, and rejected
recommendation. Submit law or registry changes to the root.

## Decide whether machinery earns its keep

Before promoting a universal check, agent, workflow, prompt rule, artifact, or
adapter, answer:

1. Which current representative failure or protected boundary requires it?
2. Is the need recurrent, cross-repository, or unconditionally security,
   privacy, destructive-effect, or authority critical?
3. Does an existing maintained component already own the behavior?
4. Can the mechanism be precise, inexpensive, actionable, and verified on the
   real surface?
5. Does it remove more operator attention, coordination, maintenance, and
   artifact cost than it adds without lowering quality?

Choose `keep`, `conditional`, `simplify`, `repository_fit_specific`, or
`retire`. Use manual-first handling for real but infrequent work. Do not
universalize a one-off preference, add an enforcement layer because a schema
can represent it, or preserve machinery whose output is routinely regenerated
or ignored by the next owner.

## Migrate, verify, and retire

```text
ultragoal --json migrate plan --registry <relative-registry-path>
ultragoal --json migrate verify --registry <relative-registry-path>
ultragoal --json migrate apply --plan <relative-plan-path> --accept-plan <plan-id>
ultragoal --json migrate retire --plan <relative-plan-path> --approve-retirement
```

Plan and verify are Read effects with zero hidden writes. Require exact
replacement behavior, complete active reader/writer inventory, compatibility
boundary, usage evidence, rollback, unknown-route rejection, and proof that no
duplicate authority remains. Apply only an accepted current plan as
`WorkspaceWrite`. Retire only after the named destructive decision; otherwise
preserve the blocked source and keep it non-authoritative through root-owned
routing.

An unavailable evaluation or migration capability blocks only that operation.
Do not substitute a compatibility helper, scorecard, research summary, receipt,
or renamed source for real behavior and retirement proof.

## Maintain the plugin lifecycle

Treat fresh install, monotonic update, failed-update recovery, authorized
rollback, idempotent reinstall, uninstall and teardown, stale-cache recovery,
and repeat use as distinct operations. Inspect and bind the exact installed and
cache authority before planning. Any host write requires explicit authority;
rollback additionally requires downgrade authority. A failed effect preserves
or restores the prior authority before another operation can proceed.

If the current host does not expose the reviewed lifecycle adapter, report that
surface as unsupported. Do not hand-edit plugin, marketplace, cache, app
registry, or Plugins UI state. Verify source, package, marketplace, install,
cache, app registry, Plugins UI, discovery, runtime, and journey layers
separately after an authorized operation.

## Output

Report audited validity, paired behavioral observations, failures,
non-regression, source classifications, proposed improvement or migration,
effects, active readers and writers, compatibility state, rollback, retirement
blockers, maintenance disposition, attention-cost evidence, unsupported
capabilities, and highest candidate-only ceiling.

Metric gains, scorecards, research prose, receipts, tests, telemetry, or
generated registries do not prove improvement, migration, retirement,
readiness, release, or completion.
