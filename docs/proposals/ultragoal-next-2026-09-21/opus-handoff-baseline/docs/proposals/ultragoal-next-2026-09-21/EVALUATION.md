# UltraGoal Next: evaluation and implementation roadmap

Status: proposed thresholds and future experiments, not achieved results. Target model: gpt-6-astra. [PRD](PRD.md), [audit](AUDIT.md), [architecture](ARCHITECTURE.md), [migration](MIGRATION.md).

## Questions that decide the product

1. Does eliminating duplicate discovery/parsing and broad invalidation deliver useful feedback sooner without lost coverage?
2. Can Bend implement the selected parsers, facts, graph and admission laws with acceptable correctness, memory, throughput and build/runtime integration?
3. Do bounded Jev rules find consequential gaps or avoid expensive investigations that native Astra plus deterministic checks misses?
4. Does the integrated product improve completed outcomes, total cost and operator attention once overhead and failures are included?
5. Can verified UG replace EJ’s valuable responsibilities for actual consumers with a smaller coherent interface?

## Controls and ablations

| Arm | Purpose |
| --- | --- |
| N: native Astra/Codex + adopted repository tools | Measures baseline capable-model/harness behavior |
| U: current exact UG source/installed candidate, identified separately | Existing product comparator; do not substitute version identities |
| D: simplified incremental deterministic reference | Isolates shared parsing/facts, narrower reuse and simpler ownership |
| DJ: D + identical Jev rubrics | Isolates semantic contribution before language effects |
| B: equivalent Bend-first pure logic + required compatibility | Isolates implementation/runtime differences with same rule semantics |
| BJ: B + same Jev path | Proposed product |

D/DJ are temporary experimental controls, not authorization to substitute a non-Bend product. Reuse existing native reference implementations and fixtures to avoid building an unnecessary alternative engine. Stage comparisons: U→D for architecture; D→B for implementation; D→DJ and B→BJ for Jev; N/U→BJ for total product value.

Within selected N/B/BJ tasks, separately compare no UG instruction, compact conditional router, and retained targeted resource. Do not label scripts, references and executable controls collectively as “prompt.” No compulsory all-combinations campaign if earlier gates falsify a component.

Fix exact model slug, reasoning effort, service tier, host version, tool access, task inputs, verifier, warm/cold state and budgets within paired trials. Initial proposed Astra task effort is medium; include a smaller low-effort sensitivity stratum only after the primary comparison. Never use another model to fill missing Astra cells. Record aliases/provider changes and rerun affected qualification; no assumed monotonic improvement.

## Corpus and independent grading

Offline corpus: proposed 240 labeled scenarios across six strata (40 each): Rust; TS/JS; Python; mixed formats/docs; graph/lifecycle/authority; legitimate exceptions and malformed/adversarial inputs. Each has original requirement, valid reference behavior, at least one broken variant, exact oracle and supported-coverage notes. Reuse relevant existing cases; add only missing boundaries.

Semantic calibration: start with four high-value rule families (alignment, verification weakening, cross-surface consistency, recovery/reuse). Minimum proposed 100 positive and 100 negative/exception packets per family, with separate insufficient-evidence cases. Use public/synthetic data until private disclosure is approved. Freeze a held-out set separated by repository/task family; do not tune threshold on holdout labels.

Task pilot: 24 independent tasks across six strata, three randomized matched repeats per selected surviving arm. It detects gross failures and cost shape, not narrow noninferiority. Advancement campaign expands to at least 60 distinct tasks with three paired repeats and a precomputed sample-size plan based on pilot variance. Cluster intervals by task/repository, not by correlated repeat. Budget and full task list require later approval.

Grade completed deliverables against the original request, not the engine’s explanation. Use mechanical/runtime oracles for exact behavior and independent blinded rubric review where creative/doc quality requires judgment. The primary agent, Jev rule author and UG report cannot be the sole grader. Include deliberate verifier weakening and oracle tests; a deterministic verifier can still omit a requirement.

Keep all attempted cells, timeouts, failures, blocked tools and cancellations in cost denominators. Report accepted-run and attempted-run metrics separately. Do not erase a difficult cell or substitute an easier task after seeing results.

Evaluate retrieval/candidate construction separately from Jev selection using the exact candidate set available at decision time. Report automatic coverage over all eligible events, selective error over automatically handled cases (undefined if none), critical misses over actual critical cases, and review burden. Correlated judgments from one document are not independent task successes. Once heldout failures guide tuning, that split is no longer untouched final evaluation. Shadow comparisons use original decision-time state and never execute a duplicate business operation.

For repository exploration, compare efficient lexical/structural retrieval, the same candidates with Jev selection, and the end-to-end agent outcome when qualification is available. Count filesystem/index/parse time, provider latency, returned context bytes/tokens, subsequent reads, duplicate investigations, false-omission recovery and total cost. Record critical evidence recall at both candidate and final-bundle stages; low reading volume alone is not success. Include relevant evidence outside the first shortlist, sparse/misleading names, contradictory chunks, no valid match, stale prior findings, changed dependencies and misleading summaries. Required-check coverage must remain identical across arms. Existing context-cache costs belong in the comparison; a provider's cheaper per-token price alone does not establish routing savings.

## Coverage dimensions

Report denominators and uncovered classes separately:

| Dimension | Denominator and admissible evidence |
| --- | --- |
| Language/construct support | Declared parser/extractor construct corpus, including versions/editions and unsupported syntax |
| Deterministic obligations | Applicable adopted rule/input combinations; exact evaluator result |
| Formal engine properties | Named theorem/source/compiler/assumption tuples completed; not lines of code “proved” |
| Project proofs | Supported project law closure and proof tool identity; absent capability stays unknown/not supported |
| Semantic assessment | Eligible concern/rubric/model packets, clear/concern/abstention/unavailable counts and labels |
| Runtime observation | Required user/consumer surfaces actually exercised on exact relevant inputs |

A semantic response cannot fill the runtime/proof denominator. More questions or more proof files cannot increase coverage unless they cover distinct adopted obligations. Not-applicable decisions need an auditable basis and can themselves be challenged.

## Performance measurement

Reference fixture tiers (targets, not existing inventory measurements): small 1k files/10MB; medium 10k/100MB; large 100k/1GB, with deterministic synthetic composition and representative real public repositories. Record hardware/OS, cores, memory, disk/cache state, source/tool identities and active load. The main acceptance tier is medium on a pinned Apple Silicon development host; Linux qualification is separate.

Cold means a fresh UG cache and process, with OS-cache condition recorded; do not require privileged page-cache flushing. Warm means previously parsed/fact state with a fresh command process. Also measure within-operation reuse. Package/build compilation is timed separately and never hidden inside check throughput.

September 23 operational build decision: use a 420-second aggregate timeout for
Bend C emission plus native C compilation during ongoing development. The former
300-second script timeout was not a PRD performance requirement. A measured
270.08-second emission plus a 29.60-second `-O2` compilation left effectively no
margin; allow bounded compiler variability while retaining runtime optimization
when its measurements justify it. Record flags, stage wall/CPU times, peak memory
where available and total build time; terminate/reap owned process groups on
expiry. Preserve every prior 300-second failure as failed under its original
configuration. This is a prospective execution guard, not a claim that a seven-
minute or high-memory build is acceptable on all supported developer hosts.
Build repeatability/resource suitability remain qualification work. Cold/warm
runtime, coverage, proof and original-outcome thresholds are unchanged.

For the explicit foreground session mode, “fresh command process” means a new
frontend PID using the same qualified local computation owner/core incarnation.
Report session startup/cold population separately and amortized over the declared
command sequence; include frontend startup, IPC, fresh observation and admission
in every warm latency. Instrument retained value construction/reuse, invalidated
nodes, bytes and memory. An advisory disk comparison or a process kept alive
without retained parsed/fact values does not meet this definition. Report cold
standalone, session cold, fresh-frontend session warm and within-operation reuse
separately. Session warm results carry the same explicit local trust ceiling as
the architecture; they do not establish protected host/adoption assurance.

For each tier run at least 30 repeats for each relevant case: unchanged; one implementation edit; test-only edit; unrelated doc edit; add/delete/rename; changed scope/ignore membership; changed rule; configuration/lockfile; tool version; unknown dependency; concurrent mutation. Report median/p95 and bytes/allocations/peak memory where tooling supports them.

Instrument independent stages:

1. Discovery/enumeration and Git status.
2. Reading and decoding.
3. Tokenization and syntax parsing, including parse counts.
4. Semantic fact extraction, including repeated visits.
5. Dependency/query membership analysis and invalidation.
6. Deterministic rule evaluation and evidence composition.
7. Hashing, serialization, byte copying and language/process crossings.
8. Process/compiler/build startup.
9. Native test/probe runtime.
10. Model queue, network, inference, validation and retries.

The parser spike compares current repeated work with shared inventory/parses before changing language. Bend then processes the same immutable bytes/facts. Preserve supported grammar and findings; “faster” by silently dropping constructs fails. For linked-list strings versus bounded arrays, measure construction/copy cost and peak memory, not only token-loop throughput.

## Acceptance targets and decisions

All numerical targets are proposed. They are chosen to demand a noticeable improvement while keeping ordinary work responsive; adjust only before the confirmatory experiment with written rationale.

| Metric | Proposed threshold | Rationale / decision |
| --- | --- | --- |
| Critical correctness | Zero observed authority bypass, user-work loss, silent requirement weakening or stale evidence admission in corpus | Any case blocks promotion; zero observations is not a universal proof |
| Required coverage | 100% of applicable mandatory obligations discharged at proper assurance or explicitly unresolved | No hidden omissions; “unresolved” is honest coverage, not product success |
| Overall task satisfaction | ≥95% in qualified task set; paired noninferiority lower 95% bound above −3pp versus N and U | Both threshold and uncertainty required; insufficient sample means inconclusive |
| Useful local feedback | Medium tier warm unchanged p95 ≤200ms; affected one-file ≤500ms; cold ≤5s | Keeps cheap checks outside main task critical path; large tier target cold ≤30s, warm ≤1s |
| Total task time | Median ≥20% improvement vs U; paired 95% interval excludes no improvement | A check microbenchmark alone cannot graduate product |
| Routine overhead | BJ time and total model/provider cost ≤5% above D on no-defect routine cases | Stops pervasive unnecessary assessments; semantic eligibility should skip these cases |
| Semantic gain | ≥15pp recall gain over D/B on targeted gap corpus; rubric recall ≥85%, precision ≥95% for consequential intervention | Require 95% precision lower bound ≥90%; rare/high-impact rules may need larger samples; otherwise advisory only |
| Abstention | At least 95% of insufficient-evidence fixtures not falsely cleared; all malformed responses rejected | Explicitly reward unknown rather than guessed certainty |
| Jev latency | p95 ≤1.5s per qualified packet when service healthy; operation deadline ≤3s by default | Proposed service target, not vendor SLA; timeout returns unresolved |
| Cost | ≥20% lower all-attempt task model/provider cost vs U; Jev spend ≤10% of D model spend | Include main agent, reviewers, cached input, retries, failures, recovery and billable ambiguous calls |
| Attention | ≥30% fewer correction/approval-navigation interventions vs U | Count actual operator events/time; safety approvals are not obstacles to bypass |
| Reuse | Differential equivalence for 100k randomized mutation sequences plus named adversarial cases; no errors | Property proof covers modeled assumptions, adapters require their own tests |
| Boundary overhead | IPC/serialization p95 <10% of local affected-path time budget | Optimize batching before considering unstable FFI |
| Footprint | One engine policy owner; target ≥50% less runtime source and transitive dependency count vs current package | Report shipped bytes/startup and build dependencies separately; never delete required coverage to meet count |
| Guidance | Default UG selected entry ≤500 words, resources conditional | Proposed usability ceiling, not a universal optimal prompt length; measure actual loaded context and behavior |

Semantic thresholds are tuned per rubric/model and intervention cost, not a universal confidence value. Track reliability diagrams/calibration error and precision-recall; compare selected-option probability, Choice confidence, Noul probability and Score distributions on their actual semantics. For unresolved contradiction or unavailable evaluator, no threshold may convert missing evidence into a pass.

Kill: critical correctness failure, unsupported assurance, or repeated no-benefit/high-cost mechanism. Revise: preserve core while adjusting offending parser, adapter, rubric, evidence packet or guidance. Scale: provisional targets met with sufficient uncertainty bounds. Graduate: all required gates plus supported installed journey and a proposed ten-task limited-use observation period meet targets without critical failure. Retirement remains separately authorized.

## Required negative and failure cases

- Malformed JSON/UTF-8, duplicate keys, deep nesting, oversize source, unsupported syntax, partial extraction and adversarial strings.
- New/deleted/renamed files, case/Unicode collisions, symlink changes, scope membership, ignored/generated inputs, configuration, lockfile/tool/rule/rubric/model updates and dynamic unknown dependencies. A membership-only query must reuse after unrelated content changes and invalidate after a matching path is added/deleted.
- Missing compiler, failed test process, output truncation, success-looking text after nonzero exit, no executed test, changed assertions, legitimate expected-value changes and placeholders.
- Provider timeout, 429/5xx, invalid payload, wrong model/question/option, NaN/out-of-range probabilities, unavailable usage, duplicate response, pre/post-cancel response and deadline exhausted during retry.
- Concurrent evaluations, host restart, journal partial write, conflicting duplicate event, superseded contract, missing host event and late admission. Replay an old successful verifier output under current digests; mutate and restore input during execution; reject both unless the observation proves a qualified immutable input binding.
- Proof TODO/open claim, false theorem, unsafe/transitive unsafe/foreign dependency, omitted LAWS import, changed law, compiler bug regression, and proof check attempting runtime effects.
- Legitimate large/cohesive files, language unsupported by a rule, generated code, well-justified exceptions and unrelated edits that must preserve reusable work.
- Semantic mechanics: same-batch answer dependency refused/staged; omitted correct candidate; all candidates wrong; subset evidence insufficient for a universal claim without inventing contradiction; conflicting Noul/Choice; inconsistent Score expectation/legend; order permutation; empty automatic subset with undefined selective error.
- Dirty tracked/untracked repository fitting, partial patch application, concurrent user edits and rollback without overwrites.

Engine law suite includes deliberately broken implementations: stale identity accepted, unavailable coerced to clear, removed mandatory item, replay after cancellation, quota underflow, order-dependent merge and incomplete invalidation. The corresponding proof or boundary test must reject each. Project proof tests separately establish adapter behavior and actual imported theorem identity.

## Cost and authorization

Terry subsequently authorized full implementation and all TypeSafe Jev API spending for development, evaluation/testing and production. That authority is active and must not be requested again. Begin with offline synthetic-response/native checks, then direct live Jev qualification using minimal relevant synthetic/public/project evidence, bounded operations and recorded usage. Any separate paid primary-model API campaign, unrelated private-data disclosure or global host mutation retains its own authority boundary. Operational request/time/token budgets prevent runaway work; they do not reimpose a human spending-approval gate for Jev.

Estimate: total spend = sum across all attempts of input, cached input, cache-write, output/reasoning, tool/provider and retry charges, plus external compute. Use current authorized account pricing at launch. The September 21 TypeSafe list price ($0.042/M input, free output) suggests 1,000 packets averaging 12k input tokens would list at about $0.504 before retries/other terms; this is arithmetic illustration, not authorization, invoice forecast or an Astra campaign budget. Astra expense is TBD until tasks/token pilot and current billing surface are known.

Repository artifacts remain local by default. Public/synthetic data is the pilot default; private source/transcripts require separate permission and retention terms. No raw conversation histories are needed for initial cases.

## Dependency-ordered implementation roadmap

| Slice | Dependencies | Bounded deliverable and proof surface | Stop / promotion |
| --- | --- | --- | --- |
| P0 baseline and toolchain qualification | Explicit implementation authorization | Instrument current source in disposable checkout; record candidate; stage timings. Resolve/use newest official Bend release and record exact identity per run. Qualify ordinary/open/false/unsafe/foreign/import and recent soundness regressions; confirm non-executing check path | No trusted proof claim until compiler/assumptions recorded; requalify affected behavior on updates; no broad engine port |
| P1 one end-to-end obligation | P0 | Bend parser for small contract/JSON subset with explicit coverage, reusable fact, deterministic check, evidence admission and explain result on fixture | Valid/invalid/malformed paths; parser and admission laws; direct native differential checks |
| P2 incremental membership/graph | P1 | Add/delete/rename/config/unknown dependencies; content reuse separate from live admission | Incremental equivalence proof under assumptions + differential mutation corpus |
| P3 native observation bridge | P1; P2 for affected-path timings | Narrow descriptor/storage and compiler observation adapters; complete VerificationRequest/NativeObservation binding; compiled Bend process seam; no general executor | Actual race/crash/cancellation/IPC tests, stale-output replay and concurrent-mutation rejection; unsupported host remains unknown |
| P4 internal Jev contract | P2/P3 | Fixed rubric packets, synthetic responses, strict validation, quotas, stale result handling and explain | Offline fake-provider corpus first; live calibration requires separate authorization |
| P5 calibrated semantic pilot | P4 + provider/data/budget authority | Initial four rule families on public/synthetic holdout; compare D/DJ and B/BJ | Only qualifying rubrics gain consequential policy use |
| P6 compact plugin + fit/verification journey | P2/P3; P5 where semantic policy adopted | One router, native tool integration, explicit cache/provider modes, dirty target preservation | Authorized supported-host install/discovery/runtime/recovery journey |
| P7 Astra product comparison | P5/P6 + model campaign authority | Matched pilot then sufficient confirmatory cells against original acceptance | Outcome/time/cost/attention gates; no winner from saturated cases alone |
| P8 EJ integration/retirement | P7 + consumer migration + separate retirement authority | Approved exact mutation/rollback manifest from migration document | One product owner/entry point, fresh consumer checks and rollback rehearsal |

Parallelism is useful for disjoint parser/adapter fixture development after shared types are fixed. The integration owner owns schema and state transitions; no parallel competing policy implementations. Do not attach guessed calendar durations to unmeasured Bend/toolchain work.

## Verification of this planning package

Fresh, non-destructive checks during this audit:

- Read local source/manifests, three worktree identities and protected dirt; current installed behavior was not retested.
- Read current primary sources and pinned community implementation/report artifacts. No live inference/campaign or private upload.
- Original Bend 2.0.5 pure fixture probes: two valid cases passed, four open/false/type-error cases rejected; in-memory unsafe/malformed/missing-library probes qualified the proof boundary. After the user updated, installed 2.0.25 was verified against latest release metadata. Fresh check-only probes passed the two valid cases, rejected all four invalid/incomplete cases, did not execute the hello fixture’s main, rejected omitted LAWS import, and reported both an unsafe definition and its caller while exiting zero. Current 2.0.25 is now locally smoke-tested, not fully qualified for UG.
- EJ binary version/help observed; seven important installed/source artifacts matched, not the whole package.
- Existing `CARGO_NET_OFFLINE=true scripts/check` ran. Release build/check and 46 selected test invocations passed, then integration test `routine_public_production_contract::journeys::dirty_public_effect_executes_once_then_exact_repeat_reuses_without_mutation` failed with `MEDIATOR-SANDBOX-ACTIVATION-UNAVAILABLE` (aggregate exit 101). This is an unresolved environment/runtime boundary; no implementation repair or elevated retry was made. The aggregate therefore did not pass or reach its governance stage.
- Separate `scripts/check-governance` passed: zero changed standards projections, generated authority current, 28 typed Python sources passed, and 119 standards rows accepted.
- Document validation checked 45 template/requested headings, balanced code fences and 94 local file/anchor links with no missing targets. All ten pre-existing modified tracked-file hashes and the full tracked diff matched the entry snapshot. New writing was confined to this proposal directory.
- Independent read-only design review identified four material gaps: native input/observation binding, membership fingerprint breadth, newer-worktree delta coverage, and exact EJ retirement paths. All four were corrected and the reviewer found no remaining material gap in those revised areas. This is planning review, not adapter execution or product qualification.
- Bend-update follow-up: latest release and current upstream commit were refreshed, bend guide and nine bounded check-only scenarios were run on installed 2.0.25, and the latest-release policy was added without changing engine source. All 94 local file-link targets remain valid and protected tracked files remain unchanged. The repeated aggregate repository check again passed 46 selected invocations before the same MEDIATOR-SANDBOX-ACTIVATION-UNAVAILABLE integration failure (exit 101); the separate governance check passed. This does not attribute the existing runtime failure to Bend.

These checks validate the planning package and the stated narrow existing surfaces, not the proposed engine’s efficacy or readiness. Full product check remains HOLD at the exact sandbox-activation gap; no source repair, installation, model campaign, credential access, EJ retirement, commit or publication occurred.
