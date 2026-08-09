# Goal Contract

## Authority and superseded history

Name this contract's authority, the prior contract or process it supersedes,
and any frozen compatibility inputs that remain readable but cannot schedule
work or raise a claim.

Name the sole active ExecPlan. Keep current program status there, architecture
in `ARCHITECTURE.md`, and planning/evidence/recovery rules in `PLANS.md`.

## User, job, and outcome

- **User:**
- **Job:**
- **Product outcome:**
- **Mission outcome:**

## Observable journey

List the smallest end-to-end sequence that an external evaluator can observe.
Bind it to one exact candidate, target, effect scope, and acceptance oracle.

## Non-goals

List outcomes this contract does not authorize or prove.

## Protected invariants

- authorize before consequential effects;
- parse untrusted input before behavior;
- preserve unrelated work and root/target custody;
- separate source, package, install, discovery, runtime, journey, and release;
- fail closed on ambiguous identity, effect, or recovery; and
- prevent generated artifacts, agents, reviewers, or model output from minting
  root authority or raising a claim ceiling.

## Human decisions

List decisions with no safe default: external or destructive effects,
credentials, production targets, publication/release, ambiguous retirement,
and material scope expansion.

## Claim ceiling

Name the exact claim, required surface, supported envelope, prohibited
substitutes, and honest pre-acceptance states such as `partial`,
`blocked_by_product`, `blocked_by_environment_or_authority`, or
`inconclusive`.
