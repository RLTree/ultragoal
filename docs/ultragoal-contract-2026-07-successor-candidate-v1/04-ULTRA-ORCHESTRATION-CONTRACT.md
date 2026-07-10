# Ultra Orchestration Contract

## Purpose

This contract is designed for a Sol Ultra root agent operating through Codex after independent review and adoption. It defines how the product is completed without encoding a permanent list of historical lanes.

The client for this authoring pass did not expose independently verifiable model or reasoning metadata. The design therefore targets the requested runtime but does not claim that runtime was active.

## Root authority

The root Ultra agent is the sole integration and candidate claim authority. It owns:

- the current `AuditContext` and candidate identity;
- adaptive work-package graph construction;
- product-law interpretation;
- shared-authority serialization;
- write ownership and conflict prevention;
- worker acceptance or rejection;
- reconciliation and merge order;
- strict verification and independent review routing;
- cleanup, retirement, and residual-risk accounting;
- current state, exact next action, stop reason, and claim ceiling.

The root MAY delegate exploration, implementation, testing, or review. It MUST NOT delegate final integrated claim authority.

## State-derived decomposition

The root begins from current repository and product state, not a stored lane plan:

1. Freeze a read-only discovery context.
2. Evaluate required product surfaces and laws.
3. Build findings with dependency and repair metadata.
4. Group compatible findings into product-semantic work packages.
5. Compute prerequisite, conflict, and shared-authority edges.
6. Select read-only reconnaissance and adversarial work before material writes when contracts are uncertain.
7. Stabilize canonical schemas and ownership.
8. Assign bounded write packages.
9. Recompute the graph after every accepted material change.

The root may instantiate package families such as authority kernel, plugin distribution, repository fitting, routine loop, observability and repair, strict proof, orchestration runtime, improvement loop, maintenance, or real-use validation. These are semantic templates, not a fixed implementation list. A family is instantiated only if current findings require it.

## Work-package schema

Every active work package MUST conform to:

```json
{
  "work_package_id": "WP-plugin-discovery-01",
  "semantic_owner": "distribution_and_discovery",
  "objective": "Reconcile installed plugin identity through application discovery.",
  "laws": ["HUL-DISCOVERY-001", "HUL-PLUGIN-001"],
  "context_id": "sha256:...",
  "safety_class": "parallel_disjoint_write",
  "prerequisites": ["WP-package-snapshot-01"],
  "reads": [".codex-plugin/**", "install/**"],
  "writes": ["validator/src/discovery/**", "fixtures/discovery/**"],
  "shared_authority": [],
  "external_effects": [],
  "acceptance": ["AC-DISCOVERY-INSTALL-IDENTITY", "AC-DISCOVERY-NEW-TASK"],
  "required_outputs": ["code_diff", "test_result", "finding_delta", "risk_delta"],
  "claim_effect": "none_until_root_reconciliation",
  "timeout_policy": "progress_or_recover",
  "retirement_on_accept": []
}
```

IDs identify an instance, not an architectural lane. Replanning may retire an instance and create a new one without changing product law IDs.

## Typed safety classes

| Safety class | Parallel rule | Typical work |
|---|---|---|
| `parallel_read_only` | May run concurrently against a frozen context | inventory, research, architecture mapping, adversarial analysis |
| `parallel_disjoint_write` | May run only with proven non-overlapping writes and compatible generated outputs | isolated implementation and fixtures |
| `serial_shared_authority` | Root serializes all mutations and immediately recomputes dependent state | laws, schemas, manifests, command catalog, package inventory, claim graph |
| `serial_external_state` | Root serializes and requires explicit authority, before/after observation, and recovery plan | install, cache mutation, application state, network release, destructive cleanup |
| `root_only_claim` | Cannot be delegated | integrated status, completion, release, retirement authorization |

Path disjointness is necessary but insufficient. Two workers conflict if they change the same semantic authority, generator/input pair, public command behavior, schema family, or evidence identity even when paths differ.

## Shared-authority surfaces

These surfaces are serial by default:

- stable product laws and diagnostic identifiers;
- `AuditContext` and authority-graph schemas;
- public CLI command and exit-code catalog;
- canonical plugin component metadata and package inventory;
- generated-source ownership and generators;
- claim and evidence graph;
- migration aliases and deprecation schedule;
- product-surface inventory;
- integrated completion decision.

A worker may propose a patch to a shared-authority surface, but the root must apply or accept it in isolation and invalidate affected work before parallel work resumes.

## Reconnaissance roles

Reconnaissance is read-only and answers a bounded question with source references, confidence, and unresolved ambiguity. It MUST NOT:

- edit files or generate receipts;
- treat historical state as current;
- expand its own scope;
- make completion claims;
- turn an uncertain inference into a dependency fact.

The root should prefer parallel reconnaissance for independent product surfaces, current primary-source research, and adversarial architecture review.

## Write-owning workers

Write ownership begins only after:

- relevant laws and acceptance criteria are stable;
- exact write and generated-output sets are known;
- dependency and conflict edges are current;
- user changes in the write set are classified;
- a bounded verification path exists;
- external effects, if any, are explicitly authorized.

A worker owns only the assigned paths and semantic surface. Discovering a needed shared or out-of-scope change requires a blocker report, not an opportunistic edit.

## Worker output schema

Every worker report MUST contain:

```json
{
  "work_package_id": "WP-example-01",
  "context_id_started": "sha256:...",
  "status": "candidate_complete",
  "owned_changes": [
    {"path": "validator/src/example.rs", "kind": "modified", "semantic_role": "validator"}
  ],
  "unowned_changes_observed": [],
  "commands": [
    {"argv": ["cargo", "test", "example"], "exit_code": 0, "result_ref": "capture://..."}
  ],
  "acceptance_results": [
    {"id": "AC-EXAMPLE", "status": "observed", "evidence_ref": "capture://..."}
  ],
  "findings_delta": {"resolved": [], "introduced": [], "unchanged": []},
  "risk_delta": [],
  "generated_outputs": [],
  "retirement_candidates": [],
  "claim_requested": "none",
  "blocker": null,
  "exact_next_action": "Root independently reruns AC-EXAMPLE."
}
```

Allowed statuses are `working`, `candidate_complete`, `blocked`, `stale`, `drifting`, `rejected`, and `retired`. A worker cannot report `accepted`, `integrated`, or `complete`; those are root decisions.

## Root acceptance schema

The root accepts a worker only after recording:

```json
{
  "work_package_id": "WP-example-01",
  "decision": "accepted",
  "current_context_id": "sha256:...",
  "ownership_reconciled": true,
  "diff_reviewed": true,
  "acceptance_reproduced": ["AC-EXAMPLE"],
  "independent_review": "not_yet_required|passed|failed",
  "new_findings": [],
  "invalidated_packages": [],
  "merge_order": 4,
  "claim_effect": "observation_only",
  "next_action": "Recompute affected work graph."
}
```

Acceptance means the change entered the integrated candidate. It does not imply product completion.

## Serial and parallel execution rules

1. The root MUST freeze the graph version before dispatch.
2. Parallel workers MUST share the same relevant base identity.
3. A shared-authority change invalidates dependent unaccepted results.
4. Generated outputs belong to the worker that owns the generator input; no second worker edits the output.
5. Test execution may be parallel only when output directories, ports, fixtures, and external services are isolated.
6. Installation, application registry, release, and shared cache mutations are serialized.
7. The root MUST inspect actual diffs and unowned changes before acceptance.
8. Failed or blocked work does not stall unrelated legal nodes.
9. The graph is recomputed after material acceptance, rejection, or external-state change.

## Independent review and falsification

Independent reviewers are read-only by default and receive:

- the product laws and requested claim;
- the candidate identity and affected surfaces;
- implementation diff and generated-output provenance;
- acceptance evidence with command captures;
- known limitations and excluded surfaces;
- explicit falsification prompts.

They MUST test or analyze:

- wrong-surface substitution;
- stale and tampered evidence;
- omitted generated or package resources;
- command bypass and hidden writes;
- dirty-tree and partial-availability behavior;
- overlapping worker ownership;
- operator discoverability and repair comprehension;
- path, secret, supply-chain, and telemetry boundary failures;
- performance scope and cache honesty;
- cleanup and deprecated-authority residue.

The root records findings and either creates new semantic work packages or lowers the claim ceiling. “Reviewer approves” without evidence is advisory only.

## Recovery contract

### Stale worker

A worker is stale when relevant context inputs changed. Stop acceptance, preserve its report, compute the semantic diff, and either rebase/reverify or retire the package. Never relabel stale output as current.

### Blocked worker

A blocked report MUST name the missing authority or prerequisite, current evidence, smallest unblock, and safe unrelated work. The root resolves the dependency or replans; it does not ask the worker to guess.

### Drifting worker

Drift exists when reads, writes, objective, or proposed authority exceed the package. Interrupt, preserve the bounded useful delta, reclassify changes, and either split a new package or reject unowned edits.

### Incomplete worker

If execution stops, the root inspects actual state, not the worker’s intended plan. It records completed changes, partial effects, unverified outputs, and recovery/rollback. A replacement worker receives this reconciled state.

### Conflicting workers

Quarantine both results, identify semantic ownership, choose a canonical base, and re-run acceptance serially. Line-level merge success does not resolve authority conflict.

### Root interruption

Durable state MUST be reconstructable from source, the canonical work graph, command captures, and explicit external-state observations. The root resumes by freezing a new AuditContext and invalidating incongruent work; it does not trust a stale progress narrative.

## Current-state and next-action artifacts

The orchestration read model MUST expose:

- current context and graph version;
- active, ready, blocked, stale, and review work packages;
- ownership and conflicts;
- last accepted change and invalidations;
- required product surfaces and claim ceilings;
- exact next root action;
- exact next safe worker action, if any;
- honest stop reason.

`ultragoal status` and `ultragoal next` are read-only projections of this model. They MUST NOT create work packages, change receipts, or mutate goal state.

## Completion and stop behavior

The root stops and reports a bounded ceiling when:

- an explicit user decision or authority is required;
- required external state is unavailable and cannot be safely simulated;
- product laws or ownership are materially ambiguous;
- independent proof cannot be reproduced;
- candidate identity changed during strict proof;
- a security or preservation risk cannot be bounded;
- the minimum required surface remains blocked.

The root does not manufacture work to keep agents busy. When no legal work package exists, it returns the exact unblock or the highest honest claim ceiling.

## Orchestration completion criteria

The orchestration subsystem is not claimable until behavioral tests demonstrate:

- state-derived decomposition changes when repository state changes;
- disjoint read/write work runs legally in parallel;
- semantic conflicts serialize even across different paths;
- shared-authority change invalidates dependent work;
- worker self-claims cannot raise authority;
- stale, blocked, drifting, incomplete, and conflicting workers recover correctly;
- root restart reconstructs accurate current state;
- independent falsification findings re-enter the work graph;
- fixed historical lane IDs are absent from the generated implementation plan;
- final claim uses the minimum reconciled surface ceiling.
