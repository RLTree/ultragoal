Template choice: ChatPRD **Product Spec for AI Agents** (id 4317); all template sections are filled below. User-requested behavioral scope, safety, rollout and open questions are included in Spec details.

# Spec Header

## Field Descriptions

| Field | Value |
| --- | --- |
| Product | UltraGoal Next — Bend-first engineering requirements and evidence engine |
| Version | Next major redesign; release number TBD |
| Product owner | Terry Noblin |
| Implementation / evaluation owners | TBD at implementation authorization; one integration owner |
| Status | Implementation authorized and in progress under the sole active ExecPlan; acceptance evidence pending |
| Research cutoff | September 21, 2026, America/Los_Angeles |
| Current target | gpt-6-astra in Codex; future GPT variants require fresh qualification |
| Bend version policy | Use the newest official release; currently installed and checked: 2.0.25. Record exact identities per evidence run and requalify affected behavior after updates |
| Local candidate audited | HEAD f447a7a83e16f983f2ffe831e19ee7ba72ac8cec plus preserved pre-existing dirty/untracked work |
| Decision | Recommend the redesign, gated by measured Bend parsing/proof/adapter qualification and a controlled Astra evaluation |
| Authority | Full implementation and TypeSafe Jev spending authorized for development, evaluation/testing and production. Native installation/global changes, destructive EJ retirement, commits/publication and unrelated private-data disclosure retain their separate boundaries |
| Canonical companions | [Audit and disposition](AUDIT.md); [architecture](ARCHITECTURE.md); [migration and retirement](MIGRATION.md); [evaluation and implementation roadmap](EVALUATION.md) |

# The Problem

## Required Problem Statement

Capable agents doing substantial repository work need durable, executable requirements, trustworthy observations and fast feedback without carrying an elaborate process in context. Current UltraGoal spends machinery on broad state custody and orchestration around narrow checks; it also repeats inventory and parsing work and makes reuse depend on broad operation identities. Those mechanisms can consume latency and attention while leaving semantic defects, verification gaps and actual runtime outcomes unresolved.

The affected user is an operator such as Terry delegating ambitious work to gpt-6-astra. The job is to finish the original requested outcome, preserve unrelated work and important boundaries, and understand exactly what remains uncertain. The pain is repeated investigation, invalidated useful results, weak assurance described too broadly, and competing product instructions. Current frequency, attributable total latency and user-attention cost are **Unknown**; the audit establishes concrete mechanisms, not a measured Rust bottleneck or a proven product effect.

## Problem Details

| Question | Established answer |
| --- | --- |
| Which workflow? | Repository fitting, implementation feedback, verification, diagnosis/recovery and delivery across code and mixed documents |
| Why now? | September Astra guidance specifically calls for revisiting inherited skill/process overhead; Bend 2 introduces an opportunity to encode pure invariants; Jev offers bounded semantic decisions |
| What is broken or costly? | Repeated syn parsing, repeated inventory scans, broad cache key coupling; syntax-only routine behavior; unsupported public eval execution; no demonstrated current EJ task-outcome benefit |
| What remains essential? | Actual authorization, current observations, complete required coverage, preserved user work, explicit unknowns and recoverable failures |
| What is not established? | Dominant latency stage; Bend production speed/coverage; Jev accuracy on UG rules; current installed usable journey; benefits for future models |
| Existing alternatives? | Native Astra/Codex with project tools; simplified deterministic UG; current UG; EJ/Agentic resources. These are experimental controls, not alternative product directions |

## Evidence Inventory

| Evidence | Confidence and ceiling |
| --- | --- |
| [Current source audit](AUDIT.md#local-disposition) | High for specific implementation/call paths; not fresh runtime success |
| [Candidate and installation distinctions](AUDIT.md#local-state-and-custody) | High for manifest/worktree identities; installed behavior remains historically reported |
| [September 11 Astra guidance](https://developers.openai.com/blog/rethinking-skills-and-prompts-for-gpt-6-astra) | Current primary guidance, directly relevant; no controlled UG effect size |
| [Bend findings](AUDIT.md#bend-toolchain-and-proof-findings) | Exact local probes plus pinned upstream source; compiler/foreign/runtime trust remains bounded |
| [TypeSafe and community findings](AUDIT.md#jev-and-community-evidence) | Current API/source evidence and author experiments; no approved local Jev calibration |
| [Skills evidence](AUDIT.md#guidance-and-executable-harnesses-are-different-interventions) | Useful methodology; earlier model populations cannot establish Astra benefit |

## Problem Quality Bar

The problem names a user, workflow, observable friction and source-supported mechanisms. It deliberately does not turn unknown latency shares or historical reports into current performance claims. Before implementation optimization decisions become permanent, the first spike must establish stage-level costs and preserve findings under reuse.

## Agent Prompt: Strengthen The Problem

“For the chosen task and candidate, identify one missed outcome or repeated work item, its original requirement, the source/runtime observation supporting it, and the smallest measurement that could show the mechanism is not responsible. Keep unknown cost and frequency explicit.”

# The Bet

## Required Bet Format

If we replace UltraGoal’s broad process machinery with a Bend-owned incremental obligation/fact/evidence engine, internal bounded Jev assessments and narrow native adapters, then operators using gpt-6-astra will complete more valid outcomes with less delay and supervision, demonstrated by preserved required coverage, at least 20% lower median total task time and 30% lower operator interventions against current UG, while outperforming a simplified deterministic baseline on targeted semantic defects without materially increasing routine-work cost. These are **proposed targets**, not observed results.

## Bet Details

Ship one product, one entry point and one policy owner. Change agent behavior by providing precise relevant feedback and useful recovery, not prescribing a universal workflow. Let the primary agent choose approaches and optional investigations. Automate deterministic obligations and semantic gap detection at the appropriate boundary.

The assumptions are that repeated work is material, Bend can support the selected representations/parsers economically, useful Jev rubrics can be calibrated, and supported host observations are sufficient. Each has a decisive spike or ablation in [EVALUATION.md](EVALUATION.md). A failure changes a component or scope claim; it does not justify quietly restoring a whole Rust policy engine.

## Example Bet

After a writer changes, UG reuses unchanged parser facts, checks affected cancellation dependencies, and asks Jev a narrow question about removed assertions. It presents one concern with the exact diff and a cancellation test to run. An unrelated README edit neither reparses the writer nor makes old runtime evidence valid for a new writer revision.

## Good Bet Checklist

- [x] Names target user/model, mechanism and observable outcome.
- [x] Specifies baselines, quality constraints and failure thresholds.
- [x] Separates language-direction commitment from unproven speed claims.
- [x] Allows no-benefit results for individual rubrics and guidance.
- [x] Excludes unauthorized implementation and provider spending.

## Agent Prompt: Make The Bet Falsifiable

“Compare the same task, model effort, tools and original acceptance criteria with and without this mechanism. Count all attempted work and recovery. Reject the mechanism if its targeted benefit is absent, its required coverage regresses, or its cost exceeds the registered guardrail.”

# Success Criteria

## Required Success Criteria

All values below are proposed acceptance targets. The [evaluation document](EVALUATION.md#acceptance-targets-and-decisions) owns denominators, statistical interpretation and promotion decisions.

| Expected behavior | Signal / target | Anti-signal |
| --- | --- | --- |
| Fast ordinary feedback | Reference 10k-file/100MB fixture: warm unchanged p95 ≤200ms; one-file local feedback p95 ≤500ms; cold local indexing p95 ≤5s | Full reparse/scan after an unrelated edit |
| Faster completed work | Paired Astra tasks: median end-to-end time ≥20% lower vs current UG; routine overhead ≤5% vs simplified deterministic baseline | Faster check but slower or incomplete task |
| Less supervision | ≥30% fewer operator correction/approval-navigation interventions vs current UG, with no invented zero-attention assumptions | Repeated reminders or unchanged findings |
| Requirements remain visible | 100% of applicable mandatory obligations discharged at specified assurance or explicitly unresolved | Silent deletion, unsupported N/A, semantic success substituted for runtime evidence |
| Useful semantic contribution | ≥15 percentage-point recall gain on semantic defect set vs deterministic baseline; each blocking rubric precision ≥95%, recall ≥85% before promotion | Many alerts without actionable defects or excessive false intervention |
| Reuse remains correct | Zero stale/admission mismatches in adversarial suite and randomized differential sequences; equivalence proof for modeled pure core | Cache hit on changed scope/dependency/rule or missing coverage |
| Costs decrease overall | ≥20% lower total task model/provider cost vs current UG; Jev incremental provider spend ≤10% of deterministic-arm model spend | Savings omit retries, failed attempts or recovery |
| Smaller product | One policy engine/entry point; runtime source and transitive dependency footprint targets ≥50% below current package, measured separately | Hidden duplicated policy, retained unsupported commands or mandatory process panels |

## User Behavior Criteria

Operators can identify the unresolved requirement and next useful action from one compact response. Agents can continue independent authorized work while a bounded assessment runs. Help and inspection do not write files or call providers. Unchanged findings do not recur as new interventions. Native host approvals and user choices retain their meaning.

## AI Product Criteria

Jev is internal and consequential but bounded: validated Choice, Score and Noul outputs support calibrated semantic policies, abstain when evidence is inadequate, and never grant authority. Astra receives a small conditional entry point with task-relevant resources. Future model changes trigger versioned requalification rather than accumulating permanent compensating instructions.

## Example Success Criteria

A scope change adding a new writer must create or invalidate the relevant obligation even with no edits to old files. A Jev outage yields “semantic assessment unavailable” while cheap local checks complete. A missing compiler produces unknown verification rather than a pass. A legitimate large file is not rejected merely for exceeding EJ’s historical line cap.

## Success Criteria Quality Bar

Every criterion has a measurable signal and anti-signal. Performance and semantic values require measurement; guarantees have counterexample suites and explicit trust assumptions. Product quality is graded against original requirements by independent oracles, not UG’s own result strings.

## Agent Prompt: Generate Success Criteria

“For each adopted requirement, state what a user or independent verifier can observe, the assurance needed, a legitimate exception, and one broken implementation that must fail. Include latency/cost and operator attention only when measured.”

# The Evaluation

## Required Evaluation Summary

Use a staged evaluation: offline law/parser/adapter qualification; deterministic performance comparisons; approved public/synthetic Jev calibration; approved matched Astra task campaign; then an authorized installed dirty-repository journey. Compare Native, current UG, simplified incremental deterministic, that baseline with Jev, Bend without Jev, and Bend with Jev plus necessary adapters. Guidance ablations are separate.

## Measurement Plan

Record cold/warm/unchanged/affected-change timing by stage, time to useful feedback, total completion, defect misses, false interventions, coverage, invalidation errors, all attempts/cost/retries, and measured operator interventions. Include Rust, TS/JS, Python, mixed configs/docs, malformed input, scope changes, legitimate exceptions, outages, cancellation, concurrency and late responses. Use immutable task and verifier versions; publish no private data. Details and budget formula live in [EVALUATION.md](EVALUATION.md).

## Kill / Scale / Graduate Thresholds

Kill or disable a mechanism on any authority bypass, loss of unrelated work, stale evidence accepted, weakened mandatory requirement, or assurance inflation. Keep semantically unqualified rules in shadow/advisory mode. Scale only when benefit survives matched outcome checks. Graduate the replacement after performance/quality gates and supported installation journeys; EJ retirement remains a separately approved transition.

## AI Evaluation Plan

Calibrate rubrics per exact model/version and prevalence stratum, including ambiguous/no-evidence cases and prompt injection in source excerpts. Test schema violations with synthetic responses before any provider use. Fix target gpt-6-astra, effort and host version within pairs. Do not replace Astra with another model if unavailable; record blocked qualification.

## Example Decision Thresholds

If Bend parse/fact throughput is slower but eliminating duplicate work still meets product latency and footprint targets, retain the Bend design. If a mature Rust syntax adapter is needed for unsupported grammar, retain only that interface and its retirement criterion. If Jev raises more false holds than useful detections, disable that rubric while preserving independent deterministic obligations.

## Evaluation Quality Bar

The evaluation compares outcomes, not receipts; includes negative controls and easy saturated tasks; reports attempted as well as accepted runs; and separates guidance from executable machinery. No current controlled campaign proves these proposed targets.

## Agent Prompt: Design The Evaluation

“Freeze the original acceptance oracle and compare one changed mechanism at a time before the end-to-end comparison. Include defects the mechanism should find, valid cases it should leave alone, unavailable tools, and recovery costs. Show exclusions and uncertainty.”

# Build-Readiness Review

## Checklist

- [x] Product direction, ownership and non-goals are explicit.
- [x] Current artifacts and installed/evaluated/source distinctions audited.
- [x] Current Astra, Bend and TypeSafe evidence checked with material limits.
- [x] Representation, parsing, proof, Jev, adapters and failure behavior specified.
- [x] EJ valuable responsibilities mapped; future mutations and rollback planned.
- [x] Falsifiable acceptance plan and dependency-ordered slices defined.
- [ ] Exact Bend toolchain/runtime qualified for deployment and theorem admission.
- [ ] Stage-level performance baseline and parser coverage demonstrated.
- [ ] Per-rule Jev calibration and data-processing terms verified.
- [ ] Protected adoption channel and native host adapter qualified.
- [x] Full implementation and TypeSafe Jev development/evaluation/testing/production spending authorized.
- [ ] Separate native installation, destructive retirement and any additional paid primary-model campaign authority established when needed.

## Agent Prompt: Spec Readiness Critique

Readiness: **ready for an explicitly authorized technical qualification slice; not ready for production implementation claims or retirement**. Try to falsify dependency completeness, evidence assurance, protected adoption and resource bounds first. Do not confuse a detailed design with implementation proof.

# Implementation Handoff for PM + AI Engineer

## Product Tasks

| Task | Owner | Completion criterion |
| --- | --- | --- |
| Approve scope and first slice | Terry | Explicit implementation authorization with preserved dirty-work boundary |
| Freeze outcome tasks/oracles and budget | Evaluation owner, TBD | Independent acceptance and all-attempt cost accounting |
| Qualify host/adoption/fit journey | Integration owner, TBD | Dirty work preserved; no hidden writes; missing capability explicit |
| Integrate EJ responsibilities and migrate consumers | Integration owner after parity | No competing entry point; approved rollback rehearsed |

## AI Engineering Tasks

| Task | Owner | Completion criterion |
| --- | --- | --- |
| Instrument current repeated work and build equivalent Bend spike | Engine owner, TBD | Stage timings, exact coverage comparison, bounded malformed-input outcomes |
| Define LAWS/PROOF and incremental graph | Bend owner, TBD | Named laws plus red implementations rejected; native checks pass |
| Implement minimal adapters | Adapter owner, TBD | Real OS/transport behavior tested; one policy owner |
| Develop TypeSafe contract/rubrics | Semantic owner, TBD | Strict synthetic validation, then approved calibrated pilot |
| Run matched Astra outcome evaluation | Independent evaluation owner, TBD | Original requirements graded; thresholds met with uncertainty reported |

## Users & jobs-to-be-done

- **Primary agent:** keep important obligations current while inventing and executing its own approach; obtain relevant facts once and receive discriminating feedback.
- **Repository operator:** delegate substantial work, authorize consequential effects knowingly, preserve unrelated state and see credible completion limits.
- **Maintainer/evaluator:** reproduce findings, update rules/model versions and retire mechanisms whose benefit disappears.

## Scope of the agent/product behavior

In scope: executable adopted requirements; minimal repository fitting; incremental facts/checks; internal semantic assessment; native observation; diagnosis/recovery; separate engine/project proof integration; mixed-deliverable evidence; compact CLI/plugin guidance.

Out of scope: a new generative supervisor, autonomous permissions service, replacement host scheduler, generic mandatory reviewers, universal language semantics, automatic compaction replacement, broad skill installation, unapproved private-data processing, or declaring software correct from source syntax and model confidence.

## Failure modes & safety

| Failure | Required response |
| --- | --- |
| Untrusted/malformed/oversized input | Typed bounded failure; preserve useful unaffected results; no partial parse promoted to complete |
| Unknown dependency or missed scope membership | Conservative expansion or explicit incomplete coverage |
| Late/duplicate/cancelled semantic response | Reject current admission; exact duplicate idempotence; no state revival |
| Provider outage/schema drift/budget exhausted | Unavailable/unknown; finish independent local work; bounded retry only |
| Requirement or verifier weakened | Compare against protected adopted baseline; cannot self-approve |
| Unsupported host observation or proof dependency | Claim ceiling lowered; exact missing verifier/action shown |
| Contradictory implementation evidence | Reopen affected obligation; propose discriminating investigation rather than repeating the same mechanism |

## Spec details

The [architecture](ARCHITECTURE.md) owns the design of obligations/facts/evidence, computation reuse versus live admission, Bend-native parsing, Rust exceptions, proof assumptions, Jev interface/calibration, persistence, concurrency and compact commands. The [audit](AUDIT.md) owns consequential disposition and source provenance. The [migration plan](MIGRATION.md) owns future replacement/retirement mutations. The [evaluation roadmap](EVALUATION.md) owns exact targets and slices. These are companion views of this proposal, not additional active plans.

Non-negotiable invariants: code owns identity, applicability, freshness, budgets, thresholds, mandatory coverage and consequences; host owns actual permission/effects; semantic opinions cannot manufacture proof; adopted requirements cannot silently weaken; incomplete checks remain incomplete; unrelated candidate changes do not invalidate independent computations; actual adapter behavior is tested separately from pure laws.

## Rollout & monitoring

Proceed from offline qualification to shadow results, limited adopted semantic policy, supported host journey, and measured replacement. Preserve old package/configuration until rollback is demonstrated. Monitor timing by stage, cache/admission errors, coverage gaps, rubric precision/recall, all-attempt spend and repeated interventions. Monitoring is local aggregate data by default; no transcript collection or hidden background work. Requalify on material compiler, host, rule, model, parser or disclosure-policy changes.

## Open questions

1. Which stage actually dominates current latency? Source proves duplicated work, not its share.
2. Does the latest Bend compiler/runtime pass UG’s claimed laws and operational qualification on its recorded exact version?
3. Which Rust grammar/format responsibilities can immediately meet parity in Bend?
4. Which Jev rubrics improve Astra task outcomes, and what per-rule thresholds earn consequential use?
5. What provider retention/disclosure terms and actual rate/batch limits apply?
6. Which native host channel can protect adoption provenance and supply trustworthy observations?
7. Which representative real repositories and any separate primary-model campaign will be used? TypeSafe Jev spending is already fully authorized.

Defaults are resolved in the design where safe. These uncertainties require evidence or later authority, not an ordinary design-choice questionnaire.

# Compact One-Page Version

## Problem

UG currently repeats analysis and owns broad process machinery while much useful verification remains narrow or incomplete. Astra needs precise obligations and evidence with less inherited ceremony.

## Bet

A Bend-owned incremental engine with internal bounded Jev and narrow native adapters can improve completed outcomes while reducing latency, cost and operator attention.

## Success Criteria

Preserve every mandatory obligation and assurance boundary; achieve proposed ≥20% lower task time/cost and ≥30% fewer interventions versus current UG; meet affected-feedback targets; demonstrate useful calibrated semantic coverage and zero stale-result acceptance in the qualification corpus.

## Evaluation

Compare native Astra and current/simplified/Bend designs, with and without Jev. Grade original requirements independently. Full implementation and TypeSafe Jev spending are authorized and underway; destructive EJ retirement remains a separately approved transition.
