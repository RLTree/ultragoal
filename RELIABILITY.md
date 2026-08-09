# RELIABILITY

Harness Ultragoal must remain truthful across interruption, retry, concurrent
workers, partial host effects, and stale candidates. A run may be resumed only
from durable, candidate-bound state whose authority and causal history can be
revalidated.

## Retry And Idempotency

- Parser, authorization, confinement, identity, digest, schema, policy, and
  claim failures are terminal for that attempt.
- Host or process failures are retryable only when the owning typed error says
  so and no effect may already have occurred.
- Effectful operations reserve authority before mutation, record their durable
  transition, and reconcile the observed post-state before completion.
- Repeat setup, retrofit, installation, and routine-use operations must either
  reuse verified state or produce the same supported outcome without duplicate
  effects.

Routine snapshot stability is measured over the bound Git-visible repository
state. Ignored build output is outside dirty-state authority; selected inputs
and declared output scopes are still independently revalidated before and
after effects.

## State And Recovery

Product state is owned by the typed stores under `state/`, `orchestration/`,
`routine_work/`, `repository_fit/`, `distribution/`, and `observability/`.
Repository fixtures and tests must use explicit isolated roots; they must not
read or mutate the user's live plugin cache or application registry.

Append-only journals, leases, reservations, recovery records, and terminal
observations retain causal authority. Summaries and receipts are projections
and can be rebuilt or rejected; they cannot overwrite the underlying record.
Cleanup is legal only after terminal reconciliation proves that no unique
state, pending effect, or recovery authority remains.

Routine execution keeps reservation, child/process-group custody, staged
outputs, cleanup observation, terminal precommit, and terminal publication in
one private typed owner. Expiry permits recovery investigation but never proves
owner death or authorizes takeover. Failure, cancellation, timeout, panic, or
output ambiguity stays nonterminal until the owner proves that no child or
staged custody remains. Reuse is a new durable attempt bound to the prior
committed record; it is not a read-only shortcut to completion.

The same private routine `HostState` owner appends descriptor-bound semantic
events and authenticated terminal checkpoints for the exact candidate, target,
context, and source. A checkpoint projection joins those records; an
interruption in the append window may therefore leave a pending append absent
or present. Reads never bootstrap missing state. Historical read-only access
under stale authority may expose only settled `Complete`, `Failed`,
`Cancelled`, or `Incomplete` states and must never project `Ambiguous` as a
settled result; every mutation revalidates the exact current head.

An interrupted operation follows this sequence:

1. reopen the exact state root without initializing missing authority;
2. bind the current candidate, target identity, journal head, and permit;
3. classify the last durable transition and observed host state;
4. reconcile to committed, rolled back, refused, or ambiguous;
5. issue a new action only from the reconciled state.

A mutable local store cannot independently prove that its own complete history
was not rolled back. Until an external monotonic head is available, a nonempty
routine-authority store refuses fresh-process open, takeover, reuse, and
mutation. Same-process evidence cannot promote cross-process recovery or
installed interruption/recovery claims. The local store also provides no
cross-process rollback or recovery proof.

## Concurrency

Concurrent implementation uses disjoint run-scoped leases. Concurrent Rust
checks use isolated `CARGO_TARGET_DIR` values and up to 16 build/test threads.
Product operations use bounded leases and process locks at their effect
boundary; a lock is held through mutation and terminal state publication.
Unknown ownership, expired authority, lock loss, or conflicting state fails
closed.

## Feedback Gates

- Product boundary gate: `scripts/check-product .` for source/package-local
  compilation and exact focused product-boundary checks.
- Governance projection gate: `scripts/check-governance .` for current
  source-law and standards projections only.
- Compatibility aggregate: `scripts/check .` runs product plus governance;
  it is neither a coverage nor a full-repository proof route.
- Release preflight: `scripts/check-release .` requires a clean exact
  committed candidate, then stops at authority HOLD. A later release-surface
  action requires separate Tree approval and an explicit implementation
  change.
- Public self-law compatibility adapter: built `ultragoal --root . check strict
  --claim cli-self-law-compliance`, with a recursive before/after snapshot;
  it is not an ordinary product gate.

None of these proves package, installation, discovery, runtime, journey, or
release. A stale binary, cached target, old receipt, or prior digest is not
current evidence.

## Observability

Stable diagnostics identify the failed boundary and causal next action without
echoing untrusted or private input. Local events support diagnosis and recovery;
optional export remains privacy-preserving and cannot become an authorization
channel. Completion requires a terminal observation bound to the same
candidate and state transition as the effect it describes.
