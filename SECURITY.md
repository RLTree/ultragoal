# SECURITY

Harness Ultragoal operates on repositories, local state, subprocesses, plugin
installation paths, and host registries. Local data is untrusted. CLI args,
environment variables, filesystem metadata, manifests, cached state, receipts,
model output, tool output, and generated files must be parsed and authorized at
their owning boundary.

## Required Gates

- `scripts/check-product .` runs source/package-local compilation and the
  exact focused product-boundary checks selected for the candidate.
- `scripts/check-governance .` runs current source-law and standards
  projections only.
- `scripts/check .` is the compatibility aggregate of product and governance
  gates; it is not a coverage or full-repository proof route.
- `scripts/check-release .` performs clean exact-candidate preflight and then
  holds at the release authority boundary. A later release-surface transition
  requires separate Tree approval and an explicit implementation change.
- Built `ultragoal --root . check strict --claim cli-self-law-compliance` is
  an explicit, recursively zero-write compatibility adapter, not an ordinary
  product gate.

Passing one gate does not substitute for a different proof surface. None proves
package, installation, discovery, runtime, journey, or release.

## Trust Boundaries

Path confinement and descriptor-based identity checks live in the owning
filesystem adapters, including `audit/source_governance/`, `repository_fit/`,
`distribution/`, and `orchestration/`. Capture must bind the repository root,
every traversed ancestor, and the leaf; symlinks, special files, path escapes,
ancestor replacement, substitution, and ambiguous post-effect state fail
closed.

Root permits, leases, mutation grants, publication authority, and claim
decisions are sealed records. Only their production issuers may construct
them. A worker, fixture, receipt, schema, test constructor, or generated row
cannot authorize an effect or raise a claim.

External effects execute only through the supported host adapters after typed
preflight. Cancellation or failure after a possible effect is ambiguous until
reconciled; it must never be reported as success or silently retried.

Routine custody authority is structurally confined to one private owner and
one atomic transition boundary. Raw ledger handles, reservation tokens,
terminal writers, cleanup evidence constructors, recovery/takeover functions,
and rollback functions are not sibling or descendant APIs. Unauthorized code
may neither construct nor advance custody, even with a known local signing key;
privacy and executing transition controls carry this claim, while source-text
shape checks remain secondary.

Routine semantic events are private `HostState` operations, never files under
`validation_artifacts/` or a sibling event-store API. Each append and terminal
checkpoint is descriptor-bound to the exact candidate, target, context, and
source; terminal checkpoints require authentication. Readers cannot initialize
missing state or turn a partial observation into authority. Historical
read-only stale-auth access may report only settled `Complete`, `Failed`,
`Cancelled`, or `Incomplete` outcomes, never `Ambiguous`; mutation requires
the exact current head.

## Data And Process Rules

- Do not persist secrets, prompt bodies, transcripts, credentials, personal
  data, or unredacted host output in source, receipts, logs, fixtures, or
  telemetry.
- Secret references are resolved only at the effect boundary and secret values
  are never included in diagnostics.
- Subprocess programs, arguments, working directories, environment, timeouts,
  output limits, and cancellation behavior are typed and bounded before spawn.
- Network access, installation outside the candidate fixture, publishing,
  credential use, and destructive cleanup require the authority named by the
  product contract and explicit user approval where applicable.
- `check-release` is an authority-gated boundary, not release authority. It
  does not query or change external/package/install/release state or infer
  approval from environment, arguments, or local state.
- Dependencies require an adopted workspace change and lockfile update; tool
  projections must bind the exact dependency inputs and tool version.

## Negative Proof

Sensitive changes require positive behavior tests plus mutation, substitution,
race, special-file, traversal, stale-candidate, and false-pass controls. Read,
help, parse, inspect, query, and verification paths compare recursive tree and
Git status before and after at the same scope as the command. A green receipt
or test cannot repair missing production authorization.
