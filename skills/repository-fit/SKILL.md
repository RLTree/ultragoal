---
name: repository-fit
description: "Inspect, plan, apply, and verify Harness Ultragoal setup for fresh or existing repositories. Use for first-time fitting, retrofits, partial installations, ownership conflicts, idempotency checks, rollback planning, or preservation of local repository authority."
---

# Repository Fit

Fit one canonical repository through inspect, plan, accepted apply, and
read-only verification. Preserve user-owned content and classify the target as
fresh, partial, compatible, conflicting, or already fitted.

## Inputs

Require a canonical target path, current candidate identity, Git and worktree
state, desired outcome, existing ownership markers, exposed capabilities,
permissions, and explicit apply authority. Reject missing, ambiguous, escaped,
symlink-substituted, or stale targets before planning.

## Inspect and plan without mutation

Probe the exact grammar before relying on it:

```text
ultragoal --json fit inspect --target <relative-path>
ultragoal --json fit plan --target <relative-path>
ultragoal --json fit plan --routine-config --target <relative-path>
```

`fit inspect` records existing components, owners, modified/staged/untracked
state, worktrees, conflicts, unsupported capabilities, and the current claim
ceiling. `fit plan` must bind the same live candidate and name every proposed
mutation, preservation rule, conflict, authority need, rollback action, and
postcondition.

`--routine-config` is the one bounded retrofit scope for the installed routine
loop. It plans only `config/routine-public.json` and `config/routines.json`;
it never resolves unrelated ownership conflicts or adds the general local-state
policy. The resulting plan records that scope, and `fit apply` recomputes the
same scoped candidate-bound plan before any effect. Use it only when those two
files are the missing transition; otherwise use the complete plan.

If the target is missing, classification is ambiguous, or ownership conflicts
cannot be resolved safely, return a typed blocker. Do not silently choose a
repository, treat an existing repo as fresh, or convert a partial fit into a
clean install.

## Apply only an accepted current plan

Present the exact plan and effect boundary. Apply only after explicit
acceptance of the unchanged plan and immediate live-context revalidation:

```text
ultragoal --json fit apply \
  --target <relative-path> \
  --plan <relative-plan-path> \
  --accept-plan <plan-id>
```

Treat apply as `WorkspaceWrite` confined to declared plan mutations. Preserve
unrelated modified, staged, untracked, ignored, and worktree state. Never
stash, clean, reset, checkout, stage, follow an escaping symlink, or resolve an
ownership conflict by preference.

## Verify and repeat

```text
ultragoal --json fit verify --target <relative-path>
```

Verify actual discovery and behavior, preservation, idempotency, and rollback
where the desired outcome depends on them. A second inspect/plan over the same
unchanged fitted state must not invent new mutations. A failed or interrupted
apply reports preserved state, partial effects, rollback status, causal
diagnosis, and one exact recovery action.

Read, help, inspect, plan, and verify are Read effects with zero hidden writes.
Do not substitute a compatibility wrapper, generated tree, receipt, or static
fixture when the typed fit tool or required host behavior is unavailable.

## Output

Report target and classification, observed owners and conflicts, exact planned
or executed mutations, effect, authority, preservation result, verification,
idempotency, rollback or recovery state, unsupported capabilities, blockers,
and highest candidate-only ceiling.

Generated files, documentation, schemas, fixtures, tests, and receipts do not
prove installation, loaded behavior, repository preservation, idempotency,
rollback, discovery, runtime behavior, readiness, release, or completion.
