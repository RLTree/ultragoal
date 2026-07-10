# Ultra Orchestration and Write-Scope Safety

## Root authority

The Ultra root is the sole owner of live baseline recomputation, contract adoption, shared authority, global dependency decisions, integration, merge/rebase, release identity, migration registry, claim decisions, and final reporting. Workers submit requested shared-authority changes; only the root applies them.

- adopted contract and all machine-readable authority registries
- workspace Cargo.toml, Cargo.lock, and dependency policy
- `.codex-plugin/plugin.json`, `.agents/plugins/marketplace.json`, `.codex/agents/`, and authoritative `AGENTS.md` changes
- public CLI command catalog and root dispatcher
- global generated inventory schemas and canonical outputs
- claim graph, claim decisions, readiness/release/completion state
- migration alias registry and compatibility deadlines
- integration branches, merges, rebases, release tags, and publishing

## Dependency order

| Node | Work package | Depends on | Owner | Completion condition |
| --- | --- | --- | --- | --- |
| N00-LIVE-BASELINE | Live baseline and contract adoption | none | Ultra Root | Live context report, adopted contract digest, unresolved external decisions, and root-only authority map. |
| N01-CONTEXT-EFFECT | Live context and effect model | N00-LIVE-BASELINE | Authority Kernel | Pure context construction and structural effect enforcement pass negative fixtures. |
| N02-INVENTORY | Canonical inventory and generated authority | N01-CONTEXT-EFFECT | Product Architecture | Deterministic inventory covers active and legacy surfaces with no parallel authority. |
| N03-CLI-STATE | CLI grammar, typed state, diagnostics, and capture | N01-CONTEXT-EFFECT, N02-INVENTORY | CLI Product | Product-semantic commands, zero-write reads, stable machine output, repair, and next pass. |
| N04-DISTRIBUTION | Package, marketplace, installation, and discovery | N02-INVENTORY, N03-CLI-STATE | Distribution and Release Engineering | Deterministic package and honest source-to-runtime identity ladder pass supported-host probes. |
| N05-FIT | Fresh setup and retrofit reconciliation | N02-INVENTORY, N03-CLI-STATE | Repository Fitting | Inspect/plan/apply/verify behavior passes fresh, partial, conflict, dirty, and rollback fixtures. |
| N06-ROUTINE | Routine impact, capture, fixtures, and verified reuse | N02-INVENTORY, N03-CLI-STATE | Routine Development | Dirty-tree fast path, conservative closure, cache validation, and fallback behavior pass. |
| N07-OBSERVABILITY | Local observability, diagnosis, and optional export | N03-CLI-STATE | Observability | Local causal query and privacy controls pass; optional export passes only when configured. |
| N08-PLUGIN-PRODUCT | Plugin front door, canonical skills, and journeys | N02-INVENTORY, N03-CLI-STATE, N04-DISTRIBUTION, N05-FIT, N06-ROUTINE, N07-OBSERVABILITY | Plugin Product | Eight-skill topology and plugin-led representative routes are discoverable and behaviorally closed. |
| N09-AGENTS-REPO | Read-only agents and agent-first repository guidance | N02-INVENTORY, N05-FIT | Ultra Root | Current discovery paths, least privilege, root/scoped instructions, and fresh-agent comprehension pass. |
| N10-ORCHESTRATION | Adaptive orchestration and write leases | N03-CLI-STATE, N06-ROUTINE, N09-AGENTS-REPO | Ultra Root | Disjoint leases, worker schemas, independent acceptance, recovery, and root reconciliation pass. |
| N11-EVAL-RESEARCH | Evaluation, improvement, and research promotion | N06-ROUTINE, N07-OBSERVABILITY, N08-PLUGIN-PRODUCT | Agent Quality | Task/scorer validity, vendor-neutral records, negative controls, and reviewed promotion pass. |
| N12-CLAIMS | Claim graph and proof reconciliation | N03-CLI-STATE, N04-DISTRIBUTION, N05-FIT, N06-ROUTINE, N07-OBSERVABILITY, N10-ORCHESTRATION, N11-EVAL-RESEARCH | Proof Authority | One claim graph, independent reviewers, false-pass controls, and exact ceilings pass. |
| N13-REAL-JOURNEYS | Representative product and quality-in-use journeys | N08-PLUGIN-PRODUCT, N09-AGENTS-REPO, N10-ORCHESTRATION, N11-EVAL-RESEARCH, N12-CLAIMS | Independent Review | Fresh user/agent, maintainer, failure/recovery, and adversarial journeys pass on live candidate. |
| N14-MIGRATION | Migration routes and compatibility | N02-INVENTORY, N03-CLI-STATE, N08-PLUGIN-PRODUCT, N09-AGENTS-REPO, N12-CLAIMS | Maintenance and Migration | Every legacy surface has measured route/removal/archive disposition and compatible behavior where promised. |
| N15-RETIREMENT | Old-surface retirement and authority cleanup | N12-CLAIMS, N14-MIGRATION | Maintenance and Migration | No retired writer/reader/public route remains beyond adopted compatibility; duplicate authority is absent. |
| N16-RELEASE | Release candidate proof | N04-DISTRIBUTION, N13-REAL-JOURNEYS, N15-RETIREMENT | Distribution and Release Engineering | Exact live release proof, package/provenance verification, supported install/discovery, and open decisions resolve. |
| N17-COMPLETION | Integrated goal completion reconciliation | N13-REAL-JOURNEYS, N15-RETIREMENT, N16-RELEASE | Ultra Root | Root independently reconciles all required claims and reports complete only if every exact live surface passes. |

Topological order: `N00-LIVE-BASELINE -> N01-CONTEXT-EFFECT -> N02-INVENTORY -> N03-CLI-STATE -> N04-DISTRIBUTION -> N05-FIT -> N06-ROUTINE -> N07-OBSERVABILITY -> N09-AGENTS-REPO -> N08-PLUGIN-PRODUCT -> N10-ORCHESTRATION -> N11-EVAL-RESEARCH -> N12-CLAIMS -> N13-REAL-JOURNEYS -> N14-MIGRATION -> N15-RETIREMENT -> N16-RELEASE -> N17-COMPLETION`.

## Write leases

| Scope | Owner | Depends on | Exclusive paths | Exclusive semantics | Generated outputs | Forbidden |
| --- | --- | --- | --- | --- | --- | --- |
| WS-PLUGIN | Plugin Product | WS-CLI-CORE, WS-DISTRIBUTION, WS-FIT, WS-ROUTINE, WS-OBSERVE | skills/, docs/product-journeys/, tests/plugin_journeys/ | canonical skill bodies, front-door routing, plugin product journey prose and fixtures | generated/plugin-route-index.json | root-only plugin version fields, claim decisions, Cargo lock/workspace dependencies |
| WS-AGENTS | Ultra Root | WS-FIT | docs/agent-roles/, tests/agent_discovery/ | read-only custom agent role proposals, worker lease templates, agent discovery fixtures | generated/agent-role-index.json | writing `.codex/agents/` or other host-protected configuration, live worker lease grants, root integration, claim decisions |
| WS-CLI-CORE | CLI Product | none | validator/src/cli/, validator/src/context/, validator/src/state/, validator/tests/cli_contract/ | typed CLI parser, LiveContext, effect classes, finding/repair/next schemas, machine output and exit codes | generated/command-catalog.json, generated/effect-catalog.json | workspace dependencies, public command catalog final adoption, claim decisions |
| WS-DISTRIBUTION | Distribution and Release Engineering | WS-CLI-CORE | validator/src/distribution/, install/, tests/distribution/ | package writer, marketplace/install/discovery adapters, provenance adapters | generated/package-inventory.json, generated/marketplace-fixtures.json | plugin release version, publishing, signing identity decision, claim decisions |
| WS-FIT | Repository Fitting | WS-CLI-CORE | validator/src/fit/, templates/repository-fit/, tests/fit/ | fit inspect/plan/apply/verify, ownership and rollback, fresh and retrofit templates | generated/fit-template-index.json | modifying live user repositories outside fixtures, claim decisions, root AGENTS adoption |
| WS-ROUTINE | Routine Development | WS-CLI-CORE | validator/src/impact/, validator/src/fixtures/, tests/routine/ | impact graph, verified reuse, fixture scheduling, routine/strict profile planning | generated/impact-node-catalog.json, generated/fixture-catalog.json | claim decisions, release profile final adoption, workspace dependency edits |
| WS-OBSERVE | Observability | WS-CLI-CORE | validator/src/observe/, schemas/observability/, tests/observability/ | semantic event schema/store/query, causal explanation, optional export adapters, privacy controls | generated/event-catalog.json | telemetry consent decision, external collector configuration, claim decisions |
| WS-EVAL | Agent Quality | WS-ROUTINE, WS-OBSERVE | validator/src/eval/, evals/, tests/eval/ | vendor-neutral eval schemas, task/scorer audit, failure harvesting, promotion candidate records | generated/eval-task-catalog.json | law promotion, claim decisions, vendor account changes |
| WS-MIGRATION | Maintenance and Migration | WS-PLUGIN, WS-AGENTS, WS-CLI-CORE | validator/src/migrate/, migration/, tests/migration/ | legacy inventory, compatibility route implementations, retirement scans and plans | generated/migration-registry.json | destructive cleanup without authority, compatibility deadline decision, claim decisions |

A lease is valid only after canonical path, semantic-symbol, generated-output, fixture, and external-effect overlap checks. Path isolation or worktrees alone are insufficient. Any new overlap invalidates both leases until the root reallocates ownership.

Host-protected `.codex/`, `.agents/`, and Git metadata are never worker lease paths. Workers return exact proposed edits in `requested_root_changes`; only the root may apply them through exposed approval/capability boundaries.

## WorkerResult-v1

Every implementation worker returns a machine-readable record with:

- worker, lease ID, candidate/context ID, base and final commit/worktree state;
- paths, semantic symbols, generated outputs, fixtures, and effects touched;
- requirements and dependency nodes addressed;
- exact changes and rationale;
- commands/tests run, captured artifacts, failures, and skipped work;
- unresolved dependencies and requested root-only changes;
- self-review limitations and explicit no-claim statement.

Independent review reproduces the material evidence and attacks scope escape, stale context, regressions, reward hacking, security, recovery, and maintainability. The root records accept, reject, or rework before integration.

## Recovery

Ordinary technical blockers trigger repair or replanning of dependency-closed work; independent nodes continue. Only genuine external authority, unavailable required access, or destructive-action decisions stop dependent work. A stop records preserved state, highest honest ceiling, exact blocker, and exact next action.
