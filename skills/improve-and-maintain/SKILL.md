---
name: improve-and-maintain
description: "Evaluate and maintain Harness Ultragoal without creating duplicate authority. Use when task or scorer audits, paired behavior evaluation, failure harvesting, primary-source research, improvement candidates, compatibility routing, migration verification, or approved retirement are needed."
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

UltraGoal does not install, update, remove, retry, roll back, restore, or recover
personal Codex plugin state. First build and verify one exact monotonic package
candidate, then use `package install-plan` to request a zero-write
`HarnessPersonalMarketplaceInstallHandoff-v1`. If exact candidate, durable
workspace-local marketplace source, selected Codex, configured marketplace,
canonical personal scope, predecessor, target, and protected state cannot be
descriptor-observed, report the typed no-effect HOLD and stop.

Present the single candidate-bound supported Codex action named by that handoff
for an explicit user decision. Do not invoke it or substitute a private adapter.
After the user-owned supported action is terminal, start a fresh Codex task and
run `package install-verify --handoff <exact-owner-only-record>`. Exact target,
exact prior/no effect, partial or stale, and ambiguous are distinct results.
For any non-target result, name only the current supported Codex/user recovery
action bound by the handoff and stop; never execute, repair, or retry it.

Verify source, package, durable marketplace source, personal cache/config,
registry, fresh-task discovery, installed runtime, and journey behavior as
separate layers. Do not hand-edit plugin, marketplace, cache, app registry, or
Plugins UI state, and do not infer upstream atomicity or recovery from command
output.

## Output

Report audited validity, paired behavioral observations, failures,
non-regression, source classifications, proposed improvement or migration,
effects, active readers and writers, compatibility state, rollback, retirement
blockers, maintenance disposition, attention-cost evidence, unsupported
capabilities, and highest candidate-only ceiling.

Metric gains, scorecards, research prose, receipts, tests, telemetry, or
generated registries do not prove improvement, migration, retirement,
readiness, release, or completion.
