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
installed interruption/recovery claims.

## Concurrency

Concurrent implementation uses disjoint run-scoped leases. Concurrent Rust
checks use isolated `CARGO_TARGET_DIR` values and up to 16 build/test threads.
Product operations use bounded leases and process locks at their effect
boundary; a lock is held through mutation and terminal state publication.
Unknown ownership, expired authority, lock loss, or conflicting state fails
closed.

## Feedback Gates

- Fast source gate: `python3 scripts/check-python-source-laws .`.
- Focused Rust gate: `cargo test -p ultragoal <filter> --offline --jobs 16 --
  --test-threads 16` with a lane-local target directory.
- Rust warning gate: `RUSTFLAGS=-Dwarnings cargo check -p ultragoal
  --all-targets --offline --jobs 16`.
- Full repository gate: `scripts/check .`.
- Public self-law gate: built `ultragoal --root . check strict --claim
  cli-self-law-compliance`, with a recursive before/after snapshot.

Broad gates run only on a dependency-closed freeze. A stale binary, cached
target, old receipt, or prior digest is not current evidence.

## Observability

Stable diagnostics identify the failed boundary and causal next action without
echoing untrusted or private input. Local events support diagnosis and recovery;
optional export remains privacy-preserving and cannot become an authorization
channel. Completion requires a terminal observation bound to the same
candidate and state transition as the effect it describes.
