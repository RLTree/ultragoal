# UltraGoal Next: audit, disposition and research

Decision: build a smaller Bend-owned requirements/facts/evidence engine with internal calibrated Jev assessment. Keep useful guarantees, remove duplicated work and ownership, and qualify the replacement before retiring EJ. This document is evidence for the [PRD](PRD.md), not implementation authority.

Audit cutoff: September 21, 2026, America/Los_Angeles. Some retrieval timestamps are September 22 UTC, still September 21 locally. Mutable pages are identified by retrieval date; pinned commits establish source identity. No raw conversation histories, credential stores or live inference APIs were accessed.

## Local state and custody

| Surface | Observed identity | What this establishes |
| --- | --- | --- |
| Canonical checkout | HEAD f447a7a83e16f983f2ffe831e19ee7ba72ac8cec; plugin 0.0.42+codex.20260828085546 | Audited source plus ten pre-existing modified tracked files and untracked candidates/engines/evals/output/proposals |
| Worktree 1ebc | ae4273972ec4714ec14a376e1fb6a31f0017ff6d; 0.0.44+codex.20260904031120; six modified Rust files | Complete 12-file committed delta inspected, plus focused engine comparisons and all six dirty files |
| Worktree e7ee | eb51ba4d6ea8844197e855d42fa239ef898be47d; 0.0.12; no tracked changes | Historical candidate |
| Worktree usable-loop-master-candidate | 53d600076fc970ec0d4685e2f06d0f522f08ac76; 0.0.34; no tracked changes | Historical candidate |
| UG installed cache | 0.0.41+codex.20260824093100 manifest exists | Cache identity only; fresh runtime/discovery journey not performed |
| EJ marketplace source/cache | 0.12.0-dev.1+codex.20260914203419; engine 0.4.0-dev.1 | Seven important source/cache files byte-equal, including binary; not whole-package equivalence |
| EJ helper in this repository | engines/engineering-judgment version 0.2.0-dev.1 | Older artifact, not current installed behavior |
| Agentic Engineering | Local four-pack 4.0.0 material and adaptive 0.2.0-dev.1; worktree 07da adaptive 0.1.0 | Core 4.0.0 cache found; other packs’ current activation Unknown |

The [active plan](../../exec-plans/active/usable-product-milestone.md:94) historically reports 73/73 fit conditions and an installed 0.0.41 usable-loop HOLD due to product defects. It distinguishes source repairs from installed behavior. This audit does not promote that historical observation into a fresh installed pass or failure.

The newer 1ebc candidate adds 277/deletes 17 committed lines across 12 files, primarily guidance and package/version bindings. Four added source-text tests assert instruction presence, not agent outcomes. Fourteen relevant engine/template files are byte-identical to canonical source: repeated parsing/scans, whole-snapshot keys, broad reuse, mediated syntax-only behavior and unsupported eval all persist. Its six dirty Rust files (+24/−33) are byte-identical to the canonical dirty counterparts and change formatting/import/module ordering rather than observed behavior. Useful new guidance maps material facts to acceptance/evidence and makes approval sensitive to decision-relevant drift; retain these ideas in obligation/admission semantics and concise presentation. Evidence: [newer entry](/Users/terrynoblin/.codex/worktrees/1ebc/harness-ultragoal-plugin-proposal/skills/harness-ultragoal/SKILL.md:38), [guidance assertions](/Users/terrynoblin/.codex/worktrees/1ebc/harness-ultragoal-plugin-proposal/validator/tests/plugin_product_contract/source_contract/validate_descriptor_bindings.rs:201), [candidate scope](/Users/terrynoblin/.codex/worktrees/1ebc/harness-ultragoal-plugin-proposal/docs/exec-plans/active/usable-product-milestone.md:43). No alternate worktree tests were rerun.

Starting full tracked diff SHA-256: a704eb16034451923e200b099868cd7fb85f9be319fa4a9e0d44e91f2c105567. The existing active plan, root architecture, standards, skills, Rust changes, worktrees and historical artifacts were not edited. The five new proposal documents are the only intended repository source changes. Build products from the required existing check are not product implementation.

## Local disposition

Paths below are relative to repository root unless absolute. Source references identify inspected behavior; test-source references establish fixture presence unless fresh execution is explicitly stated.

| Area / finding | Disposition and rationale | Evidence |
| --- | --- | --- |
| Routine check is Rust syntax acceptance, not compilation or behavior | Rewrite suitable parsing/facts in Bend; keep assurance explicitly syntax-only; native tools own stronger semantics | [syntax evaluator](../../../validator/src/routine_work/behavior/rust_source_syntax.rs:151) |
| Child parses, then parent authentication calls evaluator again | One computation owner and typed observation validation; do not port duplicate parsing | [child](../../../validator/src/cli/successor_public/routine/behavior_child.rs:29), [parent](../../../validator/src/routine_work/behavior/rust_source_syntax.rs:133) |
| Governance audit scans three times; package path calls audit twice | Share one bounded snapshot and relevant live revalidation; retain race detection without universal rescans | [inventory](../../../validator/src/audit/source_governance/inventory/mod.rs:39), [package checks](../../../validator/src/audit/plugin/laws.rs:14), [dependency adapter](../../../validator/src/audit/plugin/dependency_adapter/mod.rs:10) |
| Source is parsed, text-filtered and parsed again for several rules | Shared parsed representation and reusable production/test fact views | [production source](../../../validator/src/audit/source_governance/production_source/mod.rs:19), [filter](../../../validator/src/audit/source_governance/production_source/filter/mod.rs:9), [typed boundary](../../../validator/src/audit/law/authority_surfaces/source/typed_boundary/mod.rs:32) |
| Reuse binds broad candidate/context/graph/plan identity | Separate computation keys from live operation admission; unrelated changes must not force recomputation | [plan IDs](../../../validator/src/routine_work/plan/mod.rs:95), [reuse decision](../../../validator/src/routine_work/reuse/decision.rs:50) |
| Failed/incomplete/stale/substituted results do not become hits | Retain as Bend admission laws and red fixtures | [reuse checks](../../../validator/src/routine_work/reuse/decision.rs:15), [tests](../../../validator/src/routine_work/tests/reuse/matching.rs:66) |
| Rename paths and unknown changes affect selection | Retain and strengthen with explicit membership/negative-query dependencies and indexed graph propagation | [selection](../../../validator/src/routine_work/plan/selection.rs:40), [planning fixtures](../../../validator/src/routine_work/tests/planning.rs:69) |
| Dirty capture and live revalidation repeat Git/tree/candidate probes | Measure discovery, reads, hashing and admission separately from parsing; narrow only after dependency proof | [local capture](../../../validator/src/routine_work/local/mod.rs:38), [revalidation](../../../validator/src/context/revalidate.rs:24) |
| Strict checks capture full before/after trees | Keep zero-write assurance at adapter tests/explicit full audit; remove compulsory cost from cheap affected path | [strict adapter](../../../validator/src/cli/successor_public/strict/mod.rs:23) |
| Fit materializes compiled template bytes | Agent authors repository-specific requirements; Bend checks proposed applicability and conflicts; host applies approved patch | [fit catalog](../../../validator/src/repository_fit/product_adapter/catalog.rs:72) |
| Syntax-only behavior surrounded by process custody, sandbox, grants and ledgers | Delegate host responsibilities; retain only required observation/storage primitives and matching laws | [grant scope](../../../validator/src/routine_work/runtime_adapter/mediator/grant_scope.rs:5), [framed input](../../../validator/src/routine_work/runtime_adapter/mediator/filesystem/framed_read_input.rs:9) |
| Orchestration owns leases, heartbeat, review, retries and acceptance | Bend schedules check dependencies; native host schedules agents/processes. Retire generic compulsory review lifecycle | [orchestration engine](../../../validator/src/orchestration/engine/mod.rs:7), [graph](../../../validator/src/orchestration/graph.rs:72) |
| Public eval run returns unsupported | Import bounded native evaluation evidence; do not recreate unsupported confinement infrastructure | [eval run](../../../validator/src/cli/successor_public/evaluation/run.rs:42) |
| Agentic advice supports no-op selection and unchanged suppression | Retain small conditional resources and stable concern suppression; simplify thirty-skill routing ceremony | [advisory selector](../../../validator/src/engineering_advisory/selection.rs:66), [current architecture](../../../ARCHITECTURE.md:149) |
| Universal 100% coverage and receipt-string checks | Replace default with property-specific assurance and separate coverage dimensions; existing adopted requirement changes need explicit revision | [standard](../../../agent-standards/02-boundaries-validation-and-enforcement.md:7), [package law](../../../validator/src/audit/plugin/laws.rs:38) |
| Namespace naming heuristics carry broad claim consequences | Keep objectively adopted namespace rules; use bounded semantic concerns for ambiguous legibility/ownership; remove unsupported universal gates | [identifier analysis](../../../validator/src/audit/namespace/source/identifiers.rs:29) |
| Custom installation/effect machinery persists despite native-owner direction | Use supported host installation; retain relevant race/preservation fixtures, not complete custom install engine | [active plan](../../exec-plans/active/usable-product-milestone.md:150), [host effects](../../../validator/src/distribution/host_effect/mod.rs:3) |

## EJ and Agentic Engineering: substance, not prose transplantation

EJ was inspected as a product artifact, never used as this audit’s authority or evaluator. Current source is [the local plugin](../../../../../plugins/engineering-judgment/README.md); absolute links below avoid confusing it with this repository’s older helper.

| Useful idea or mechanism | Best replacement form | Evidence and limit |
| --- | --- | --- |
| Preserve requested outcome | Bend requirement identity/adoption law plus Jev alignment question | [EJ check](/Users/terrynoblin/plugins/engineering-judgment/skills/engineering-judgment/references/check.md:5), [decide](/Users/terrynoblin/plugins/engineering-judgment/skills/engineering-judgment/references/decide.md:7); guidance, not proven outcome benefit |
| Choose through discriminating evidence | Small conditional resource + Jev identifying decisive missing observation | [decide](/Users/terrynoblin/plugins/engineering-judgment/skills/engineering-judgment/references/decide.md:9); no mandatory alternatives for settled work |
| Verify at consumer boundary | Typed assurance matching, native observation, focused Jev adequacy assessment | [check](/Users/terrynoblin/plugins/engineering-judgment/skills/engineering-judgment/references/check.md:11) |
| Revise contradicted hypotheses | Bounded attempt facts, deterministic repeat count, Jev same-mechanism assessment, agent-owned next experiment | [recover](/Users/terrynoblin/plugins/engineering-judgment/skills/engineering-judgment/references/recover.md:5) |
| Cancellation/restart/partial results | Bend lifecycle laws and retained failure fixtures; native process owner | [recover](/Users/terrynoblin/plugins/engineering-judgment/skills/engineering-judgment/references/recover.md:3) |
| Legibility and ownership | Adopted deterministic fact checks; semantic questions for actual architectural ambiguity | [audit](/Users/terrynoblin/plugins/engineering-judgment/engines/legibility/src/audit.rs:18) |
| Universal 250-line authored-file cap | Retire universal default; optional repository-specific rule only after justified adoption | [inventory](/Users/terrynoblin/plugins/engineering-judgment/engines/legibility/src/inventory.rs:5); no inspected evidence validates threshold |
| Missing registry/unsupported language collapsed into failed boolean | Explicit unknown/incomplete versus known failure, while required obligations remain unresolved | [audit](/Users/terrynoblin/plugins/engineering-judgment/engines/legibility/src/audit.rs:90) |
| observe/compare/detail/links | Consolidate normalization/comparison/excerpts into Bend fact/evidence graph; retain original detail and partial states | [engine README](/Users/terrynoblin/plugins/engineering-judgment/engines/engineering-judgment/README.md:23); command help inspected, no task benefit demonstrated |
| Owned-process helper | Adapter regression/test fixture, not universal supervisor | [process asset](/Users/terrynoblin/plugins/engineering-judgment/skills/engineering-judgment/assets/process/README.md:3) |
| Paired measurement helper | Reuse pairing/missingness fixtures and deterministic arithmetic | [measurement asset](/Users/terrynoblin/plugins/engineering-judgment/skills/engineering-judgment/assets/measurement/README.md:3) |
| Thirty AE skills/four packs | Selective technical resources/examples; no competing UG routes; retain provenance | [AE README](/Users/terrynoblin/Projects/agentic-engineering/README.md:19); no AE retirement authorized |

Current EJ adds mandatory legibility checks, unlike the old optional observer. Its inventory reads a broad tree; source-context propagation runs up to N+1 rounds with repeated imports parsing, alias analysis and binding parses, followed by further module/production parses. Relevant source: [context loop](/Users/terrynoblin/plugins/engineering-judgment/engines/legibility/src/source_context/mod.rs:32), [imports](/Users/terrynoblin/plugins/engineering-judgment/engines/legibility/src/source_context/imports.rs:52), [bindings](/Users/terrynoblin/plugins/engineering-judgment/engines/legibility/src/source_context/bindings.rs:29), [syntax](/Users/terrynoblin/plugins/engineering-judgment/engines/legibility/src/syntax/mod.rs:102), [Cargo metadata](/Users/terrynoblin/plugins/engineering-judgment/engines/legibility/src/metadata.rs:37). This independently reinforces parse-once/shared-facts work; elapsed shares remain Unknown.

Current EJ [build report](/Users/terrynoblin/plugins/engineering-judgment/BUILD_VERIFICATION.json:11) retains 43 engine tests, 116 legibility tests, six receiving checks and 53 package tests. They were not rerun here. [Provenance](/Users/terrynoblin/plugins/engineering-judgment/provenance.json:128) says current model trials/human recipient use were not run, and restored enforcement is qualified on Darwin ARM64 only. Nested engine documentation still says 0.3.0-dev.1/both ARM64 platforms, contradicting current 0.4.0-dev.1 binary/provenance. Prefer exact artifact evidence and mark documentation drift.

### Historical evaluations

| Report | Reported finding | Allowed inference |
| --- | --- | --- |
| [EJ 0.2 integrated](../../../target/engineering-judgment-integrated/REPORT.md:3) | 27 episodes; eight primary pairs, both 7/8 request satisfaction; EJ aggregate tokens +22.2%, uncached input +26%; fewer total tokens in 5/8 pairs; no observed helper use | Mixed Astra Medium guidance results; aggregate tail costs matter; no helper efficacy established |
| Same preparation study | Some later-task savings did not repay preparation in observed use | Measure amortization rather than assume fitting always pays |
| [EJ 0.6 assessment](../../../target/engineering-judgment-0.6-assessment/REPORT.md:3) | Both mandatory outcomes met; Native had compatibility/verification advantages; schedule numbers were virtual | No general EJ advantage or actual latency result |
| [EJ 0.7 pilot](../../../target/engineering-judgment-0.7-pilot/REPORT.md:3) | Native/rich/compact guidance saturated; 12/12 producer completions; evaluation overhead exceeded producer fresh tokens | Need difficult discriminating tasks plus ordinary overhead controls; not proof guidance is useless |

Reports were read without underlying raw histories. Their task/model/candidate identities differ from current EJ mandatory enforcement; no causal benefit is imported.

## Parsing and performance findings

The user’s parsing hypothesis is supported as a priority, not yet established as the dominant wall-clock component. Source demonstrates repeated inventory, reading, syntax parsing, fact extraction and broad invalidation in both UG and EJ. It does not establish that Rust’s implementation language causes the delay.

475 historical JSON performance receipts exist. Latest generated timestamp observed was July 12, 2026. A representative [receipt](../../../validation_artifacts/performance/cli-performance-test-1783838146688650000.json:61) reports 10,431ms wrapper duration but failed, with an empty speed_proof.nodes list, null CPU/I/O and no supported claims. It cannot answer the current latency question.

The first spike must count/time discovery, read/decoding, tokenization/syntax, fact extraction, repeated parse calls, dependency propagation, hashing, serialization/crossings, full-tree invalidation, startup/build and model/network separately. Compare current source, shared-fact deterministic baseline and equivalent Bend implementation over identical captured bytes. Native compiler/type/macro semantics remain separate. No new profiler/engine was implemented in this planning task.

## Current Astra and native host

Primary sources freshly opened: [Astra model page](https://developers.openai.com/api/docs/models/gpt-6-astra), [model guidance](https://developers.openai.com/api/docs/guides/latest-model), [September 11 guidance](https://developers.openai.com/blog/rethinking-skills-and-prompts-for-gpt-6-astra), [hooks](https://learn.chatgpt.com/docs/hooks), [approvals/security](https://learn.chatgpt.com/docs/agent-approvals-security), [async tools](https://developers.openai.com/api/docs/guides/async-tool-calling), [compaction](https://developers.openai.com/api/docs/guides/compaction). Mutable docs’ publication dates are Unknown unless explicitly dated.

The API lists gpt-6-astra with 1,050,000 context, 128,000 maximum output, text/image input, function calling and structured outputs; reasoning low through max. The local Codex CLI reports 0.155.1; its model cache (client metadata 0.155.0, fetched September 21 local) lists Astra, low through ultra, 272,000 default and 872,000 maximum context. These are different product surfaces; do not apply API context or reasoning limits blindly to Codex. Cache/catalog availability is not an inference or quality test.

Official model guidance documents stronger long-task coherence, async calling and steering, alongside sensitivity to instructions, unnecessary clarification, low delegation and excessive testing in some tasks. Design inference: remove ritual, retain targeted scope/completion cues, and evaluate actual workflows. No capability claim makes authority, freshness or evidence unnecessary.

Native hosts already provide sandbox/approval, process and task facilities; this session’s available tools confirm those interfaces exist. Specific UG hook activation was not installed or tested. Hooks cover many local tools, but hosted tools and specialized paths can bypass them; write_stdin does not rerun PreToolUse. Therefore hooks are a feedback opportunity, not universal mediation. Use native compaction; keep structured UG facts separately rather than invent a replacement transcript manager. Async tool APIs support waiting/continuation, but do not guarantee every Codex integration exposes the same protocol.

## Guidance and executable harnesses are different interventions

| Source/version | Evidence | Consequence and transfer limit |
| --- | --- | --- |
| [Eric Provencher, OpenAI, Sept. 11](https://developers.openai.com/blog/rethinking-skills-and-prompts-for-gpt-6-astra) | Primary guidance for precise triggers, progressive disclosure, narrow AGENTS context and calibrated testing/persistence | Directly supports simplifying Astra-facing instructions; not a controlled UG benchmark |
| [SkillsBench v4, June 14](https://arxiv.org/html/2602.12670v4) | 87 tasks/eight domains/18 paired configurations; mean 33.9→50.5%; skills include scripts/templates/resources | Author result, not pure prose effect; no Astra cohort; no universal optimal skill count |
| [SWE-Skills-Bench v1, March 16](https://arxiv.org/html/2603.15401v1) | Claude Code + Haiku 4.5 only; 49 skills/565 tasks; 89.8→91.0%, tokens +10.5%; many ceiling cases | Modest marginal effect in that population; cannot infer Astra’s effect or executable enforcement value |
| [SkillsBench v1.1 release](https://github.com/benchflow-ai/skillsbench/releases/tag/v1.1), b63b7b2 | Native task packages require BenchFlow >=0.6.3,<0.7 | Pin runner, task and verifier separately |
| [Current leaderboard](https://www.skillsbench.ai/leaderboard), labelled July 16 recomputation | 24 paired plus one one-sided configuration; no Astra row | Not the paper cohort |
| [Dataset README](https://huggingface.co/datasets/benchflow/skillsbench-leaderboard/blob/main/README.md), f104580 | June 16 export; reviewed non-timeout scoring wording differs from paper’s fixed trial frame | Do not merge headline numbers or hide timeouts |

Enduring controls concern requirements, observations, coverage, boundaries and honest results. Replaceable scaffolding includes generic “think harder,” broad mandatory reading, fixed reviewer phases, repeated reminders and fixed context recipes. Specialized procedural resources may still be useful. Evaluate guidance-only, executable deterministic, semantic and combined interventions separately; future model improvements are not assumed monotonic.

## Bend toolchain and proof findings

**Current installation after the user’s update: Bend 2.0.25**, confirmed by /Users/terrynoblin/.bend/bin/bend version. The latest official release is [v2.0.25](https://github.com/bendlang/bend/releases/tag/v2.0.25), published September 21 at 17:52:42Z, release commit c65bcb788dbfb298bb434c1d858b47c193841dc0. Current main remains [a49524265bdfa5753a4bf38e25f0574a705dd868](https://github.com/bendlang/bend/commit/a49524265bdfa5753a4bf38e25f0574a705dd868), September 21 17:54:47Z—the exact source already audited. Its sole post-release change updates flake version/archive hashes. There are no newer upstream capabilities relative to that source baseline.

The original audit’s local probes used 2.0.5 under /Users/terrynoblin/.bend/app/2.0.5/RGtZ77/bend2 with Bun 1.3.11. Those observations are historical; the user replaced that installation. The new package is a Darwin ARM64 executable with Base/effects and guide resources under ~/.bend/bend2 and ~/.bend/guide.

| Changes now available locally versus 2.0.5 | Consequence for UG |
| --- | --- |
| Check-only mode, LAWS-import safeguard, transitive unsafe/foreign reporting and soundness repairs | Use check-only proof admission and explicit trust diagnostics; update the qualification regression corpus |
| Literal checking, record layout, string/word matching and template-checking improvements | Remeasure on the latest release; old compiler timing is not a current baseline |
| Array.map improvements, file size/offset/byte operations, TCP receive deadlines, CLI arguments and macOS FIFO handling | Exercise Bend’s available runtime operations before adding adapters; ordinary file access does not establish secure snapshots or atomic durability |
| Packaged executable and explicit update mechanism | The old automatic-updating launcher is no longer the active program; daily metadata checking can be disabled per invocation |

These changes are documented in the [pinned changelog](https://github.com/bendlang/bend/blob/a49524265bdfa5753a4bf38e25f0574a705dd868/CHANGELOG.md); they are not measured UG performance gains. No new standard TLS/HTTP/parser ecosystem or stable native-library ABI was found, so R1–R3 remain narrowly justified.

Fresh 2.0.25 checks used BEND_NO_TELEMETRY=1, including bend guide. Existing induction and pair/PROOF fixtures passed; open/hole/false/type-error fixtures failed. Checking the existing hello fixture returned only the check result and did not print main’s “42.” A scratch PROOF.bend omitting its sibling LAWS.bend failed with exit 1. A scratch unsafe recursive definition and ordinary caller both appeared in transitive diagnostics, with exit 0; this confirms exit zero is insufficient for trusted proof admission. These are smoke probes, not complete compiler qualification.

Current executable SHA-256: 3850c7cd281a687715a181ad6a2ecdef041704f320ea2b4304cf9e802309203c. Base: e5639663177f2de93ef34867c029698aa4e68a98d46629f0b15452b67b99d798. Guide: 9e4643649b8ce8c8a066b60741eeec1a3d902c0e1ed86d1e5bf0340b4fede746.

### Historical 2.0.5 observations

The original required bend guide/version invocation revealed old launcher behavior: background metadata telemetry/update checking and an attempted ~/.bend/last write. The write was denied by sandbox; telemetry success is Unknown; no update was reported. Further original probes used the Bun compiler entrypoint directly. This is historical behavior, not a description of the replacement executable. The current CLI has an explicit update command and BEND_NO_TELEMETRY=1 bypasses its daily metadata check/cache write. No global configuration was changed.

Historical 2.0.5 main.ts SHA-256: 34a8a791b02ce92f247bda4cabcd3ff4eabe500002fedbec4867b5db77ee1feb. bend.ts: fe3c2b0b306fccbe44efec349d8f339b6efdaceb090b3d3049c74a1a6015c859. base.bend: b2d53bbd83639c3ae27260b318efa09de9df6006a556ac6ef41c104ea164917a.

| Original local 2.0.5 probe | Observed result |
| --- | --- |
| Existing induction.bend and pair/PROOF.bend | Exit 0, complete terms |
| Existing open.bend and hole.bend | Exit 1, incomplete claims/TODO |
| Existing false.bend and type-error.bend | Exit 1, equality/type mismatch |
| In-memory nondecreasing Empty recursion | Rejected |
| Same definition with @unsafe | Compiler API accepted; unsafe definition identified; not a fresh CLI-exit measurement |
| Malformed syntax; JSON.read/Parser.digits | Syntax rejected with location; library names undefined |
| Foreign direct Empty return | Rejected; foreign effects require IO type |

Existing fixtures were under /Users/terrynoblin/learning-projects/bend-course-work/runtime/fixtures/. Six tiny process timings ranged 66.6–74.5ms; they are not a throughput benchmark.

Current primary evidence changes the proof and integration design:

- [README limitations](https://github.com/bendlang/bend/blob/a49524265bdfa5753a4bf38e25f0574a705dd868/README.md#L215-L248): Bend 2 differs from Bend 1/HVM; linked-list strings, missing standard TLS/HTTP/JSON/regex and whole-program compilation constrain performance/compatibility.
- [Changelog](https://github.com/bendlang/bend/blob/a49524265bdfa5753a4bf38e25f0574a705dd868/CHANGELOG.md): September 19–21 soundness repairs address closed-Empty admissions. Formerly installed 2.0.5 cannot be treated as qualified merely because ordinary examples passed.
- [Current reporter](https://github.com/bendlang/bend/blob/a49524265bdfa5753a4bf38e25f0574a705dd868/bend2/main.ts#L499-L541) tracks transitive unsafe/foreign dependencies, while Base is trusted. Exit zero can include unsafe code.
- [Lean scope](https://github.com/bendlang/bend/blob/a49524265bdfa5753a4bf38e25f0574a705dd868/bend2/bend.lean#L92-L110) excludes substantial compiler pipeline concerns; Lean proof checking was not run here.
- [WONTFIX library target](https://github.com/bendlang/bend/blob/a49524265bdfa5753a4bf38e25f0574a705dd868/WONTFIX.txt#L94-L96) and [effect ABI](https://github.com/bendlang/bend/blob/a49524265bdfa5753a4bf38e25f0574a705dd868/guide/EFFECTS.md#L125-L129): no supported pure-def native library target/stable effect ABI. Start with a bounded process seam.
- [Guide](https://github.com/bendlang/bend/blob/a49524265bdfa5753a4bf38e25f0574a705dd868/guide/GUIDE.md#L144-L167): balanced fork/join work matters; no work-stealing assumption. [WONTFIX](https://github.com/bendlang/bend/blob/a49524265bdfa5753a4bf38e25f0574a705dd868/WONTFIX.txt#L37-L38) says repeated calls are not generally eliminated automatically.
- [Current CLI](https://github.com/bendlang/bend/blob/a49524265bdfa5753a4bf38e25f0574a705dd868/bend2/main.ts#L578-L628) adds check-only and LAWS import safeguards absent from former 2.0.5. Both are now locally smoke-tested on 2.0.25. Ordinary checking can execute main; proof admission needs a non-executing surface.

Inference: Bend should own suitable parsing and pure logic; narrow OS/TLS/native grammar adapters are justified by concrete gaps. There is no evidence for retaining broad Rust policy, graph or parsing ownership. Production readiness and speed remain unproved.

## Jev and community evidence

All TypeSafe documentation was fetched September 21; publication dates Unknown unless stated.

| Source | Verified contract or observation | Design consequence |
| --- | --- | --- |
| [API](https://docs.typesafe.ai/api), [models](https://docs.typesafe.ai/models) | POST https://api.typesafe.ai/v1/systemone; state/model/questions → model/answers/usage. jev-1.13.0; mutable aliases; 64k whole request, 32k state+longest question. Listed $0.042/M input; output free | Direct transport, version pin, bounded relevant packets; listed pricing/limits are not measured service guarantees |
| [Choice](https://docs.typesafe.ai/primitives/choice), [Score](https://docs.typesafe.ai/primitives/score), [Noul](https://docs.typesafe.ai/primitives/noul), [confidence](https://docs.typesafe.ai/confidence) | Choice distribution/confidence differ; Score probability-weighted level with distribution; Noul yes probability has no confidence. Confidence page formula is approximate | Per-rule calibration; preserve output semantics, no universal threshold |
| [Limitations, reviewed Sept. 17](https://docs.typesafe.ai/model-jaggedness/jev-1.13) | Literal reading, arithmetic/dates, indirection, irrelevant state, adversarial content and inconsistency | Prepare facts/excerpts; no arithmetic/identity/permission decisions; explicit abstention |
| [SDK options](https://docs.typesafe.ai/sdk/javascript/api/interfaces/RequestOptions), [retries](https://docs.typesafe.ai/sdk/javascript/api/interfaces/RetryPolicy) | Defaults include 10s per-attempt timeout, two retries, no total retry budget | UG owns total deadline and request/cost quota |
| [SDK v0.6.0 source](https://github.com/typesafe-ai/typesafe-sdk-js/blob/66880ccded6cb642dc1809620c2b108c33730214/src/client.ts#L342) | Parsed JSON is cast to T, not fully runtime-validated | Strict UG answer validation required |
| [Privacy](https://typesafe.ai/legal/privacy-policy), Nov. 19 2025; [DPA](https://typesafe.ai/legal/data-processing), Apr. 24 2026 | Vendor says no training/fine-tuning on inputs; purpose-based retention; enterprise ZDR not default | Account-specific retention/residency remain Unknown; no private upload without authority |
| [Pi Warden Sept. 21 calibration](https://github.com/DevMortimer/pi-warden/blob/5eb3841c3c893789ac40a471f31f6f9183c932d5/eval/reports/2026-09-21-calibration-0.33.3/report.md) | Author replay: 18,075 calls, 48 default holds and 27 model-derived regret labels with zero overlap; off_task AUC .51; sparse labels | Interventions are not user benefit. Evaluate targeted defects and operator outcomes; do not adopt generic Jev permission supervision |
| [Pi Warden stuck detector](https://github.com/DevMortimer/pi-warden/blob/5eb3841c3c893789ac40a471f31f6f9183c932d5/src/stuck.ts) | Deterministic exact repeats plus semantic same-strategy detection | Good bounded recovery pattern; published efficacy not Astra-specific |
| [Bicameral, Sept. 16](https://github.com/AbdelStark/bicameral/tree/3bea244b072cdacd3c8a85aec8d788a3ae0ac1cc) | Versioned packs, cancellation, pure policy/fake tests; cache lacks model identity; tests offline | Borrow narrow interfaces/fixtures, correct cache key; no live efficacy claim |
| [Hermes-Jev, Sept. 20](https://github.com/keeltrace/hermes-jev/tree/a3aeedc0006797c244ef29d7e085616a5253e627) and [benchmark policy](https://github.com/keeltrace/hermes-jev/blob/a3aeedc0006797c244ef29d7e085616a5253e627/docs/BENCHMARKING.md) | Async/stale-result and failure fingerprint handling; direct TypeSafe option, OpenRouter default; small historical live evidence | Retain lifecycle patterns, not the supervisor/compactor product; use direct API |
| [FastMCP #5170](https://github.com/PrefectHQ/fastmcp/pull/5170), merged Sept. 21 22:55:56Z, 5a876c4 | Author tool retrieval: 187 tools/374 model-written queries; ~30% BM25 vs ~82% Jev top-1, label issues; Luna-generated queries | Test semantic retrieval where lexical misses are costly; not evidence of Astra task completion gains |
| [Codex compaction correction, Sept. 19](https://github.com/jcressler/fast-jev-compaction-codex/blob/8dc4bc70f4943454a81bf29af02e2737f445639f/benchmarks/AUTOMATIC-EXPLICIT-RESULTS-AUDIT-2026-09-19.md) | 36 completed runs, 24 Jev selections; all arms 12/12; latency/token intervals cross zero; relevant facts persisted in files; Luna target | Reliable fixture integration, no established improvement/deterioration; do not replace native compaction on this evidence |

Pi Warden reports errors at an apparent 32-question limit absent from fetched official API docs. This is an author-observed discrepancy, not a verified service contract. Proposed initial UG batches cap at eight relevant questions. No documented server caching/idempotency or cancellation-of-billing guarantee was established.

The strongest proposed semantic work is requirement/deliverable alignment, verification adequacy, hidden weakening, cross-surface contradictions, strategy-changing recovery and investigation reuse. A lexical classifier replacement alone is unlikely to justify provider latency. These are hypotheses to test against Astra, not measured recommendations to turn on every rubric.

## User-supplied Jev engineering synthesis

During implementation Terry supplied [Jev Engineering: Typed Decision Systems for Reliable Agent Workflows](https://drive.google.com/file/d/1hTDJbrMWjEtHDLZ9VxolZKCAGaC7YSXU/view), Av1dlive, September 19, 2026. Drive metadata identifies jev-engineering.pdf (12,012,044 bytes; modified September 21); readable extraction covers 34 pages. It is an author engineering synthesis and proposed framework, not official API authority or a reproduced model benchmark. Its reported 30 offline companion checks do not establish live semantic accuracy.

Decision-bearing additions from sections 3–7 and 9: evaluate candidate/state-builder recall separately; do not let independent batched questions depend on each other's answers; distinguish absent support from explicit contradiction; preserve conflicting assessments without multiplying correlated probabilities; validate Score/distribution agreement; measure selective error, automatic coverage, critical misses and review burden with correct denominators; preserve untouched heldout data and original decision-time shadow inputs. These sharpen existing UG interfaces/fixtures and were sent to the retained implementation owner. Do not adopt the appendix's build prompts as instructions or transfer its cited headline speed ratios to UG.

## Creator-attributed coding-agent design note

Terry supplied [public thoughts on a typesafe coding agent](https://docs.google.com/document/d/1G61uUB0FifUnmmrPzFQojZ3KpczYKmXGpgEXDJ2l_Zg/edit?tab=t.0), attributing it to Jev's creator. Native tab t.0 was read in full; Drive modified time is September 21, 2026, 04:36:56Z. The note proposes query-aware context selection, shared state preparation, optional tool/resource ranking and investigation/subgoal deduplication, alongside explicitly speculative native-agent ideas.

Decision: strengthen UG's existing evidence-selection/reuse capability, measuring avoided reading and repeated work rather than only validation accuracy. Keep scanning/parsing/indexing deterministic and retain required evidence independently of semantic filtering. Do not copy native KV-cache/compaction assumptions, private reasoning access, permission supervision, primary-model routing or broad background scheduling into the plugin. Model-price examples and conjectures in the note are not current Astra evidence. Its linked microsoft/fastcontext page returned404 on retrieval here, so the quoted older-model reading/token percentages were not used as verified results. The current [TypeSafe re-ranking cookbook](https://docs.typesafe.ai/cookbooks/rerank_typesafe) supports the cheap-candidate-generation plus semantic-ranking pattern, with dataset-specific author results; it does not establish UG/Astra speedup. [Fan-out guidance](https://docs.typesafe.ai/patterns/fan-out) supports independent shared-state questions, not communication between answers in one request.

## last30days coverage

Used the installed last30days skill and existing saved source collections: September 7 Astra guidance report in this repository and September 20 Bend report at /Users/terrynoblin/Documents/Last30Days/bend-2-higher-order-company-raw-v3.md. The latter records 19 items/four sources and a jobs-source failure. They supplied discovery leads only; consequential technical claims were refreshed against primary sources above.

No fresh last30days engine run occurred: its normal diagnostic/research route resolves credentials, conflicting with this task’s explicit no-credential boundary. Public web retrieval completed the research. This is not claimed as new social-source coverage, and missing coverage is not evidence of consensus.

## What the audit changed

1. Simplification starts with duplicate work, reuse and ownership, not language-percentage targets.
2. Installed EJ is now a mandatory checker, so old optional-helper evaluations cannot justify importing it.
3. Current Bend proof support has concrete compiler/version/unsafe limitations; qualification comes before broad formal claims.
4. Native host facilities remove the rationale for a second general execution/scheduling system, while incomplete hook coverage limits prevention claims.
5. Jev becomes an internal, targeted assessment path with real consequences under adopted policy; broad supervision and compaction are not defaults.
6. Grammar support, deterministic rule coverage, formal properties, semantic assessment and runtime observation remain separate dimensions.

## Verification and limits

Fresh verification status is recorded in [EVALUATION.md](EVALUATION.md#verification-of-this-planning-package). Existing source tests/evaluation reports are not silently presented as rerun. No successor implementation or end-to-end speed/quality gain is claimed.

Known source-document contradiction: [RELIABILITY.md](../../../RELIABILITY.md:63) describes a fresh-process reuse refusal that differs from current active-plan/architecture descriptions. Resolve against selected implementation before migrating; do not make prose parity the oracle.
