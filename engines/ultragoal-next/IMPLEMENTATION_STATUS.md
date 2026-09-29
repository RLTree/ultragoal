# UltraGoal Next — implementation status

Current status (2026-09-28): b192 passed the installed functional loop and
deep-directory representative shape. The active dogfooding work is the
repository-owned Next gate, effect/dependency enforcement, typed handoffs and
fresh maintenance receiving. See the sole active ExecPlan for candidate and
claim state. The Opus rows below retain their historical evidence chronology.

Opus 5.5 continuation of the transferred implementation (base f447a7a83e16, transfer
bundle `.codex-worktree/transfers/20260923-opus55`). This file maps requirements to
implementation and verifiers. It is an evidence map, not another execution plan.
Evidence paths below are relative to `.codex-worktree/opus-work/` in the owner worktree.
Codex performs the final independent review; nothing here is a graduation claim.

## Toolchain

Bend 2.0.27 (global `~/.bend/bin/bend`, sha256 `b603e46e…`, recorded per build as
`bend_binary_sha256` in `identity.json`), Rust 1.95, AppleClang, O3. Builds b2–b5 used
2.0.25 (`3850c7cd…`); the global binary was replaced by 2.0.27 on 2026-09-23 20:14 and
every later build, including the delivered candidate, used it. An earlier revision of
this file said 2.0.25 and called 2.0.27 unqualified; that described a separately
downloaded binary, not the installed one. Emission is load-sensitive: 186–254 s on
an idle-to-busy machine, and one build (b14) exceeded the then 420-second native guard
under heavy swap (the limit was removed on 2026-09-24; build.py now only records stage times); a same-conditions A/B then measured 186 s (b13 source) vs 195 s
(b14 source) (`emit-ab/`). Clang adds ~29 s; peak emitter RSS ~28 GB; every build ran
under a memory/disk watchdog (`scripts/guarded.py`).

## Performance — what changed and why

Root cause of the retained-session slowdown (measured, `diag-alloc/`, `probes/eq/`):
`Equality.decide_char` matched each `Char` as its 32-bit `Word`, materializing ~72 MB
of bit lists per 230 K-character comparison. Combined with LIFO free-list recycling,
every later request touched increasingly scattered memory. Fixes, all behavior-preserving:

| Change | Evidence of equivalence |
| --- | --- |
| Reflection equality: native `U32.is_eq` + erased soundness lemmas (`Equality.bend`), same `decide_strings` signature and witness | Proof check; `proofs.py` mutations `string/list-equality-unsound` rejected |
| Allocation-free `text_eq`/`text_gt`/`text_contains` for per-row checks | Proof check; suites |
| Lean SHA-256 (register-style rounds/schedule window) and UTF-8 encoder, same `Sha256.hash` signature and refusal laws | 53 + 159 KB vectors equal to hashlib (`probes/sha`); `digests.py` |
| Wire fast paths, fused field scanner, `W.lines`, chunk UTF-8 decode, `row_onto` projection | `probes/wire` 810 cases, `probes/facts` 900 cases equal to frozen snapshot |
| Lazy `Facts.classify`, accumulated `bytes`, single-pass private-key scan | `probes/facts`, `probes/policy` 758 cases |
| Session line-delta transport (owner ↔ core) with exact reconstruction and stale-base refusal | Rust `delta_script_reconstructs_exact_lines`; `core_session.py` delta tests |
| `poll(2)` owner accept/watcher instead of 5 ms sleeps | Rust suite; session suites |
| Parallel descriptor-bound capture windows with in-order budget | Rust race/containment suite |

## Research Run adoption (2026-09-24)

Adopted from RR's measured Jev usage, with read-only access to RR. Evaluation is under
`eval/`: frozen corpora (`FREEZE.json`); group split (6 dev and 6 held-out groups per
corpus); author labels plus blind GPT-6-Luna xhigh labels (select: 117/121 author-critical
files agreed; advise: 138/144 useful and 48/48 best overlap). Rules were pre-registered
before any arm ran.

- **Head plus slices** for `select` and `advise` (`JevWindow.bend`; `Advice.order` gives
  the deterministic advice ranking). Select shortlist raised from 8 to 64 groups. Dev
  select critical-file recall: 0.356 with b13 vs 0.881 windowed (consensus labels: 0.375
  vs 0.911). Distractor flags unchanged (29). Input tokens 3.0× (89,932 → 267,773).
  Median latency 302 → 602 ms. 3 of ~400 judgments were rejected by strict probability
  normalization and named uninspected.
- **Both provider limits** replace the 80,000-byte cap: admission checks each built request
  against 64,000 request bytes and 32,000 state-plus-longest-question bytes.
  Measured requests: max 17,220 B; questions ~70% of select bytes. The 8-question cap
  binds first. The unenforced 12k/20k figures are proposed for removal
  (`integration/PROPOSED_CANONICAL_UPDATES.md` §4).
- **Usage ledger** (`--usage-ledger`): one line per attempt before dispatch, one when it
  settles. Receipts carry `usage` and `payload` composition. There is no spend limit.
  On dev, ledger totals equal receipt totals for every arm.
- **Score rounding** (RR score profile v3): a Score is accepted within
  (L(L-1)/2+1)×0.005 of its probabilities' mean. Found by this evaluation: 39 of 389 b13
  advice judgments (all requirement-map Scores) were rejected as malformed. Dev
  advise useful@3: 0.833 with b13 vs 0.972 after windows and fix. Uninspected: 39 → 1.
  On the 19 cases without Scores, windowing alone moved useful@3 from 0.930 to 0.965.
- **Held-out, run once** on b17 vs b13 (`eval/HELDOUT.json`):
  - select critical recall 0.387 → 0.919 (consensus labels: 0.393 → 0.934); distractor
    flags 34 → 36; tokens 3.1×; median 308 → 626 ms.
  - advise useful@3 0.847 → 0.958 (consensus labels: 0.819 → 0.931); uninspected 31 → 0;
    injected candidate in the top 3: 1 → 0; tokens +16%.
  - Held-out groups are now consumed.
- **Requirement checks unchanged**: one requirement per request, same probes and thresholds.
- **Not adopted (negative results, kept in `eval/runs/dev/`):**
  - Situation wording for select (`relevance/4`): recall 0.864 vs 0.881, and distractor
    flags 36 vs 29.
  - Situation wording for advise (v2): useful@3 0.958 vs 0.972, below the pre-registered
    +0.03 bar.
  - Both were removed from the product. Their texts are in
    `eval/NEGATIVE_*.bend.txt`.
- Limits: the advise corpus is near ceiling (top-1 useful 1.0 in every fixed arm), so it
  separates arms weakly. Corpora are synthetic, and the labels come from one author plus
  one blind model.

## Requirement → implementation → verifier

| Requirement (PRD/EVALUATION) | Status | Evidence |
| --- | --- | --- |
| Warm unchanged p95 ≤ 200 ms, fresh frontends, one owner/core, 10k files/100 MB | **Met.** b27 interleaved A/B p95 156 / 175 ms (rotation benchmark, accepted 2026-09-24); b28 installed journey, 30 warm checks through a Sol-started session: p95 156 ms at load ~38. Quiet-host rerun on the final candidate in DELIVERY | `runs/ab-warm-b27-{1,2}/`, `journey-b28/` |
| One-file affected p95 ≤ 500 ms | **Met**: p95 177 ms | same |
| Cold index ≤ 5 s, 10k/100 MB | Streamed over two core lanes (b28). Distinct-content 100 MB, installed b28: 27/27 runs, p95 3.24 s at load 17–31 (b27: p95 5.75 s at load 25–54, 20 of 27 failures under load from the core's fixed read budget, fixed in b28) | `journey-b28/evidence/scale/`, `integration/STREAMING_RUST_LOG.md` |
| Whole-repository content obligations | **Delivered and streamed (b28).** Content streams in bounded CHUNK requests; chunk cores list their files as CF rows expanded at one FC marker (`chunk_listing_exact` proven), coverage names omitted in-scope paths and unavailable counts, and a failed `absent-text`/`absent-path`/`json-syntax` names its counterexample file and line. Main frames that would pass 16 MiB name the omitted units (`evaluation frame bound`) | `tests/chunk_differential.py` (0 mismatches over 3,300 random splits, witnesses checked against a reference); `ChunkProof.bend`; DELIVERY (10×) |
| Index report at real 100 MB and 10× | **Built in scratch (index-report, 2026-09-25); not yet in a candidate.** The report is streamed (fact sets written one per line as the core answers, no whole-report hold or bound) and plain line facts are written as `line_runs`, losslessly; `select` reads it as a stream and accepts the old form (frontend 0.5 GB instead of 1.7 GB at 111 MB; the same answer on 42 of 42 per-repository and 14 of 14 union questions). Index requests are 16 MiB, each on a fresh core. Seven development repositories as one tree (111 MB of text): completes in 10.8–22.6 s, peak 2.8–2.9 GB, report 37 MB (was refused, `resource_bound`, after 4.7–5.1 GB; per-repository reports 432 MB); every fact equals the per-repository reports. 10× fixture: 2.4–2.7 GB peak and 205–207 s CPU (was 4.06 GB, 295 s). 1× cold index unchanged (median 2.76 vs 2.91 s). Old core `SELECT` removed | `integration/INDEX_REPORT_LOG.md`; `tests/index_report.py` |
| Full-index warm retention | **Not delivered.** `index` is per-request | design in DELIVERY |
| Growth over long sessions | Cause measured: constant instructions per request while the runtime allocator's free lists scatter cells (IPC 4.06 → 2.54). The owner replaces its core every 10 evaluations with a fresh core primed by the same frame bytes. Owner lifetime and idle bounds are runtime inputs; an expired handle fails with `error: session_unavailable` | `tests/retained_session.py`; `diag-warm/` |
| Incremental = full recompute | b28: 10,000 × 8-step randomized production-path differential, 328,828 results, 0 mismatches, 0 crashes, 0 timeouts | `differential/runs/b28-10k/` |
| Law mutations on production paths | 16/16 rejected (adds `chunk-listing-empty-digest`, `chunk-listing-drops-last-row`) | `runs/verify-b28/proofs.log` |
| Regression suites | b28: 21 suites (156 tests) pass. The invalidation and source-structure suites run probe programs that had not been rebuilt since 2026-09-23 while their imports changed; `build.py` now rebuilds both with every candidate. Rust: 50/51 on b28 (one planner test hard-coded the pre-Step-7 frame cost; fixed to derive its budget from the frame) | `runs/verify-b28/` |
| Select at 10k/100 MB | **Built in scratch (select-build, 2026-09-24); not yet in a candidate.** File-first retrieval replaces window substring scoring and the chunk tournament: BM25F over identifier-split, stemmed tokens (path ×3, definition names ×2, body), a pool of 4G files reranked by best window (reciprocal rank), G = 128 groups (`--shortlist 1..512`), in four Bend stages whose statistics are sums, so any chunking gives one answer (12 proved laws; 13 law mutations rejected). Development recall 0.913 at G = 128, 0.826 at 64 (was 0.357 at 64). Differential 0 of 42 questions against the fixed-point reference; chunking 0 differences over 120 random cases and on helix and django at 4 KiB. Wall time against b29's chunk rounds: 10k fixture 1.9–2.6 s vs 3.6–3.8 s; django 13 s vs 40 s; 10× 22 s vs 29–31 s at 1.43 vs 1.75 GiB. Real text at 111 MB (stage level): ranking 10.4 s at load 130, of which term counting 8.7 s; index-time term counts (+9% index) would remove it, designed not built | `integration/SELECT_BUILD_LOG.md`; `tests/select_{differential,chunks,disclosure,recall}.py` |
| Jev rubric calibration (consequential ≥ 0.95 precision, LB ≥ 0.90, recall ≥ 0.85) | **Met for `recovery-state` only, on two independent held-out sets:** v2 on b27 (precision 1.000, LB 0.960, recall 0.948) and v3 on b28 with fit-v3 (0.984, 0.945, 0.941). Advisory elsewhere on v3: alignment 0.955/0.906/0.826, consistency 0.989/0.942/0.829, verification 0.945/0.885/0.851, investigation-reuse 0.792/0.706/0.429. Fit-v3 versus fit-v2 on the same v3 responses: recall +5.4 points (95% 3.5–7.5), precision −1.5 (−2.6 to −0.5) | `heldout-v2/scores/REPORT.json`, `heldout-v3/scores/REPORT.json` |
| Semantic decision ownership | Bend applies per-family fitted gates (`SEMANTIC_CALIBRATED`, family as the tenth field) and decides blocking (`SEMANTIC_ADMIT`: qualified family, admitted state unknown, `--semantic-blocking` on); Rust forwards validated probabilities and applies Bend's admission | `semantic_materials.py`; `semantic_blocking_live.py` (live: blocks on, advisory off) |
| Dirty-tree fit/apply/verify/recovery | **Installed CL-USABLE-LOOP passed on b28** (10/10 steps, unrelated state and `.git` byte-identical). b27 was partial: session socket path limit (D1), no failure location (D2), undocumented `fact-*` rules (D3), unreliable index (D4), all fixed in b28; the warm session needs a host that allows local sockets (D5, documented) | `journey/`, `journey-b28/` |
| Package/discovery/runtime | Exact b192 source/package was installed and passed fresh agent, dirty-tree, default Jev/session and deep-directory receiving; current dogfooding edits need a new source-bound build and receiving | Main `.codex-worktree/final-b192/`; `package_guard.py` |
| EJ migration/rollback | Prepared, zero drift, not executed (separate authority) | `migration/` |
| Original-outcome comparison, cost, attention | Pilot (6 cells, b27): agents used only `inspect` because task repositories had no adopted contract. Redesigned with identical adopted contracts in every arm; re-pilot on b28 | `campaign/` |
| Repository `scripts/check` | See DELIVERY for the exact result | `runs/scripts-check.log` |

## Trust boundaries (unchanged in kind)

Session sockets, PIDs and nonces are local assumptions, not protected provenance.
The delta transport relies on the same trusted owner that already carries frame bytes;
it adds no digest or disk-derived witness. Semantic output never grants permission,
proof, freshness or runtime evidence. One qualified policy decision can fail a pending
obligation (recovery-state contradictions, `--semantic-blocking`); it never verifies one.
