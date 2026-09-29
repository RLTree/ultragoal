# CLI Authority and Proof Surfaces

## Typed public behavior

Public CLI grammar, effects, failures, and claim ceilings come from the typed
catalog and runtime implementation. Prose may explain them but cannot invent a
route, authorize an effect, or turn a failed command into success.

Parse and bind untrusted input before behavior. Effectful commands must name
their target and authority, revalidate current identity before effect, and fail
closed on ambiguity, substitution, duplicate effect, or unsafe recovery.

## Proof surfaces remain separate

Source, package, install, host discovery, cache, runtime exposure, product
journey, and release are different surfaces. Evidence from one supports only
that surface and its declared predecessors. Matching names or versions do not
prove matching bytes.

Tests, static review, package construction, installation, and reviewer
agreement do not by themselves prove a usable journey. A higher claim requires
fresh observation on that higher surface.

## Evidence economy

Routine help, inspection, planning, diagnosis, next-action, and local checks
leave no tracked authority artifact. Keep reproducible results in terminal or
CI output. Persist only when a named claim, cross-process custody,
irreproducible observation, recovery transition, or authorized release needs a
durable record.

Any retained artifact is candidate-, operation-, input-, and surface-bound and
states its claim ceiling and invalidation trigger. Artifact existence never
promotes a claim, and copied or stale artifacts cannot substitute for a fresh
operation.

## Agent-readable failures

Failures should identify the causal boundary, affected surface, effect status,
smallest safe repair, exact rerun or next action, and resulting ceiling without
leaking private paths or secrets. Broad audits are boundary checks; inner repair
loops use the narrowest failing command and current state.
