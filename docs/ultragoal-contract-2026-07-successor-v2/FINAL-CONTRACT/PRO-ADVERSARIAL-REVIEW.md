# Independent Pro Adversarial Review

## Review result

The candidate bundle was coherent but not acceptable as final law. It retained circular authority, broad lane-era migration burden, duplicate proof/state surfaces, an overgrown CLI, incomplete current plugin/agent discovery contracts, and mechanisms that could reward proof-shaped output. This successor contract resolves those defects at the design level; live implementation remains unverified.

## Material falsifications

| ID | Severity | Finding | Evidence | Contract resolution |
| --- | --- | --- | --- | --- |
| F-001 | critical | Archive-wide provenance is incomplete | Candidate, tracked source, and fixture digest authorities verify, but 249 copied metadata/evidence/research/legacy entries lack a supplied archive-wide digest authority. | Treat those entries as unverified context; preserve assistant-computed digests and never promote them without independent corroboration. |
| F-002 | critical | Snapshot cannot prove live state | Archived branch, HEAD, status, worktrees, runtime, plugin installation, and command output are historical observations only. | Ultra must recompute every live precondition before acting or claiming. |
| F-003 | critical | Candidate law binding is circular | Candidate wording conditions binding behavior on implemented validators/proof, permitting absent validators to weaken law. | Separate adopted normative status, implementation status, proof eligibility, and claim decision. |
| F-004 | high | Candidate requirement trace is too coarse | The candidate has one row per broad law and cannot localize implementation/proof responsibility. | Replace with granular stable requirement IDs and full source/surface/tool/claim mappings. |
| F-005 | high | Current plugin manifest is lane-era | The snapshot manifest description and routes retain gate/lane product semantics. | Rewrite around plugin journeys and bind manifest contents to package inventory. |
| F-006 | high | Plugin draft is not a supported runtime manifest | The large draft inventory is useful internal data but conflates generated inventory with host plugin wiring. | Use supported manifest fields and generate internal inventories separately. |
| F-007 | high | Repo marketplace surface is missing | Only a personal-marketplace example is present; current repo-scoped marketplace conventions are not implemented. | Add and test repo and personal distribution surfaces independently. |
| F-008 | high | Three skills lack required discovery metadata | agent-improvement-loop, agent-observability-stack, and product-fitness-gate lack required `name`/`description` frontmatter. | Consolidate topology and validate every active skill metadata block. |
| F-009 | high | Skill topology duplicates concepts | Fourteen active skills overlap around fitting, proof/product gates, orchestration, and observability. | Consolidate to eight product-semantic skills with measured legacy routes. |
| F-010 | high | Custom agents use the wrong current discovery path | Snapshot TOMLs live in `custom-agents/`, while current project-scoped Codex custom agents use `.codex/agents/`. | Move canonical read-only roles and verify discovery in a new run. |
| F-011 | high | Static write roles overgrant authority | Static lane-era roles make ownership durable and broad rather than run-scoped and dependency-aware. | Keep custom agents read-only; create explicit disjoint write leases per run. |
| F-012 | critical | Lane/gate migration is pervasive | Tracked snapshot includes 13,542 lane references across 872 files and 10,808 gate references across 1,221 files. | Use semantic inventory, compatibility routing, active-reference scans, and staged retirement rather than renaming only visible docs. |
| F-013 | high | Hard-coded GPT-5.5 assumptions remain | Tracked snapshot contains 297 GPT-5.5 references across 172 files. | Classify historical references; capability-probe or parameterize active model assumptions. |
| F-014 | high | Public CLI surface is overgrown | The handwritten `Command` enum exposes 37 variants with overlapping authority and internal terminology. | Collapse to ten product-semantic groups and generate compatibility routes. |
| F-015 | critical | CLI parse errors may mutate observability state | The parser error path invokes observability emission, violating no-hidden-write expectations for parsing/read surfaces. | Enforce structural zero-write parsing/help/query behavior and test filesystem/event-store immutability. |
| F-016 | high | Handwritten parser increases drift and ambiguity | Manual parsing duplicates grammar, help, validation, and compatibility logic. | Adopt a typed parser and generate command/effect catalogs. |
| F-017 | medium | Candidate custom tooling is over-fragmented | Nineteen candidate tools split tightly coupled authority and invite orchestration overhead. | Consolidate to twelve semantic Harness-owned tools with explicit APIs and proof. |
| F-018 | critical | Duplicate finalizers and claim authorities remain | Audit, product, standards, final-packet, and transactional-finalization surfaces can compete. | Make HCT-CLAIMS the sole machine claim authority and retire projections/writers. |
| F-019 | critical | Proof-shaped output can be rewarded | Receipts, generated rows, telemetry, and green tests are abundant enough to become optimization targets. | Install explicit surrogate-evidence guards, negative controls, and behavior-grounded scoring. |
| F-020 | high | Mutable receipts/current-state files are too authoritative | State projections and receipts appear across many command-specific flows. | Recompute from typed context-bound facts; treat projections as disposable views. |
| F-021 | high | Some host proof requirements are impossible without exposed APIs | A snapshot or local tool cannot prove host-internal cache, selected model/mode, or undisclosed registry state. | Probe only exposed surfaces, report unknown, and lower dependent claims. |
| F-022 | high | Installation and discovery are conflated | Catalog files, installed bytes, host registration, and runtime selection are not consistently separated. | Implement identity-preserving transition claims from package through runtime. |
| F-023 | high | Routine work inherits excessive ceremony | Lane/gate architecture encourages broad proof on ordinary changes. | Use conservative affected sets and reserve claim-specific strict proof for boundaries. |
| F-024 | critical | Dirty-tree preservation is not a universal invariant | Scattered commands may assume clean state or write unrelated artifacts. | Centralize context/effects and test staged, modified, untracked, and worktree cases. |
| F-025 | critical | Telemetry is at risk of becoming proof authority | Observability and proof surfaces are tightly intertwined. | Make events diagnostic evidence only; prohibit event rows from setting claims. |
| F-026 | high | Plugin hooks and extensions cross trust boundaries | Plugin components can execute with host permissions and require explicit trust. | Inventory components, minimize them, disclose effects, and test disabled/denied states. |
| F-027 | high | Evaluation core is vendor-adapter heavy | Promptfoo and OpenAI-specific modules risk owning canonical semantics and future deprecation exposure. | Keep vendor-neutral canonical schemas and adapters; migrate deprecated surfaces. |
| F-028 | high | Candidate research is already temporally stale | GPT-5.6, Goal, plugin marketplace, and custom-agent documentation changed by July 10, 2026. | Refresh primary sources and require freshness checks for mutable capability claims. |
| F-029 | critical | Path-only write scopes cannot prevent semantic collision | Workers can edit different paths that generate or govern the same authority. | Lease path, semantic symbols, generated outputs, fixtures, and external effects together. |
| F-030 | critical | Shared authority needs a sole root owner | Workspace manifests, command catalogs, claim graphs, migration registries, and releases cannot be safely co-owned. | Workers request changes; root applies, integrates, and reconciles. |
| F-031 | medium | External stop conditions are under-specified | Signing, distribution, telemetry, licensing, and destructive cleanup require real authority. | Represent them as open decisions with claim-local effects and safe defaults. |
| F-032 | critical | CLI implementation can crowd out the plugin product | Repository volume and candidate detail center heavily on validators and receipts. | Make plugin journeys, skill quality, installation/discovery, and fresh-user behavior first-class nodes and claims. |
| F-033 | high | Legacy aliases lack complete retirement policy | Scattered compatibility and historical references do not carry owner, measurement, deadline, or removal proof. | Generate a single migration registry and enforce route lifecycle. |
| F-034 | high | Fresh-agent navigation is not proven | File presence and extensive documentation do not establish comprehensibility or correct route selection. | Run new-task skill/agent/repository journeys without hidden context. |
| F-035 | critical | Release and completion can be conflated | A package or green finalizer can be mistaken for integrated product completion. | Use separate claim IDs and exact prerequisites; completion remains root-reconciled. |
| F-036 | critical | Protected host configuration cannot be leased to workspace-write workers | Current Codex sandbox documentation protects project `.git`, `.agents`, and `.codex` paths; treating those paths as ordinary worker write scopes creates an impossible or bypass-seeking workflow. | Keep protected configuration root-owned. Workers return exact requested changes; the root independently reviews and applies them only through exposed host authority. |

## Fresh Codex-agent attack

**Attack:** Start from a subdirectory, load only normal project guidance, and ask for setup, routine work, diagnosis, strict proof, or migration without knowledge of this chat. Seed stale commands, overlapping skills, missing frontmatter, wrong custom-agent paths, and unsupported host features.

**Required outcome:** The agent discovers one front door, loads only the needed workflow, distinguishes capability from requirement, avoids hidden writes, reports causal repairs, and never invents runtime/model/host proof. Failure blocks `CL-RUNTIME`, `CL-FIT`, or `CL-REAL-JOURNEY` as applicable.

## Product-user attack

**Attack:** Acquire through repo and personal marketplace paths, install wrong and right versions, restart/new-task the host, fit fresh and conflicting repositories, work on a dirty tree, diagnose a seeded failure, query local events offline, and attempt a release.

**Required outcome:** Every transition names identity and truth surface; local work is preserved; routes close behaviorally; unavailable optional capabilities degrade; package/install/discovery/runtime are not conflated.

## Security-reviewer attack

**Attack:** Path traversal, symlink swaps, alternate worktrees, untracked collisions, output floods, cancellation, secret canaries, denied network, malicious repository instructions, hook/connector denial, worker scope escalation, and external/destructive action without authority.

**Required outcome:** No unauthorized side effect; exact finding and repair; preserved state; least privilege; no secret persistence/export; dependent claim only is lowered.

## Proof-falsifier attack

**Attack:** Forge or duplicate receipts/events, edit current-state projections, provide green tests against a broken real route, reuse evidence from another commit/worktree/toolchain, sign the wrong artifact, provide provenance for a package that does not install, and optimize an eval scorer through verbosity or proof artifacts.

**Required outcome:** HCT-CLAIMS rejects wrong-surface, stale, unbound, self-reconciled, or surrogate evidence. The named behavior and false-pass controls remain decisive.

## Maintainer attack

**Attack:** Add a new skill, host adapter, semantic tool, evaluation task, and compatibility route; deprecate an old command; diagnose an authority conflict; regenerate inventories; and identify release blockers.

**Required outcome:** One source of truth, stable IDs, clear owners, bounded extension points, deterministic generation, no hidden circular dependency, and an exact migration/proof path.

## Residual uncertainty

- No live repository or current installed-plugin behavior was available.
- Cargo/Rust was unavailable in this review runtime, so no Rust validation ran.
- Host APIs, model/mode metadata, account eligibility, and permissions must be probed live.
- Distribution, license, supported matrix, telemetry, signing, hooks, compatibility duration, and destructive cleanup require external decisions.
- The archive provenance gap for 249 copied context entries remains and is explicitly excluded from trusted snapshot proof.

## Final challenge

A polished implementation can still fail if it optimizes to generated evidence or makes the CLI complete while the plugin journeys remain confusing. The Ultra root must repeatedly test the system as a user, fresh agent, reviewer, attacker, and maintainer and must lower claims whenever those perspectives disagree.
