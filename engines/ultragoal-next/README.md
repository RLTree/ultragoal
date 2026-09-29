# UltraGoal Next implementation

The source-bound `b192382363d860b4` candidate passed the installed functional loop and deep 10k-file/100 MB receiving; [initial dogfooding](../../docs/proposals/ultragoal-next-2026-09-21/DOGFOOD_COMPLETION_SPEC.md) still requires current development routes and enforceable ownership. This is not a public release or EJ retirement claim. Bend owns contract parsing, source facts, query selection, checks, evidence admission, provider policy and report disposition. Rust supplies bounded filesystem/process/HTTPS operations, native byte hashes under the measured exception, JSON wire compatibility and named native syntax adapters.

For maintenance, use the [capability and receiving map](docs/legibility/capability-map.md)
and `scripts/check-next` from the repository root. The legacy `validator/` tree
has separate compatibility ownership.

## Build and use

Run `python3 build.py` with the selected Bend 2.0.27 release. The build disables Bend telemetry, checks `PROOF.bend` without executing it, builds the Rust bridge offline with its lockfile, compiles Bend once, and records before/after source hashes and exact binaries/tool identities in `target/release/identity.json`. `ultragoal` and `ug-core` must remain beside each other. Checks never compile the engine after a user edit.

```sh
target/release/ultragoal inspect
target/release/ultragoal fit --root /absolute/repository
target/release/ultragoal check --root /absolute/repository --contract /absolute/contract.tsv --no-cache
target/release/ultragoal index --root /absolute/repository --scope prefix:src/ --no-cache > /tmp/index.json
target/release/ultragoal select --root /absolute/repository --report /tmp/index.json --query 'cancellation preserves original output'
target/release/ultragoal explain --report /tmp/check.json --id cancellation
```

For semantic obligations whose selected evidence already has disclosure authority,
add its truthful `--disclosure synthetic|public|project` class; the installed
narrow Keychain route then works without a credential flag. `--local` is the
explicit provider off switch and leaves semantic obligations unresolved.

`index` produces optional source facts, not adopted requirements or a verification pass. `select` revalidates source spans and membership, uses Bend lexical ranking for a shortlist of 128 groups by default (`--shortlist 1..512`), and judges them with Jev relevance questions in head-plus-slices windows when disclosure is explicitly enabled. It returns original excerpts and exact byte spans, retained anchors, changed/new dependencies and omissions. It creates no summaries, changes no primary model or KV cache, and never filters mandatory checks or binding instructions. An unchanged prior selection can be supplied with `--prior`; reported prior advice stays unverified and does not become a current semantic result. `--semantic` requests a fresh optional assessment.

`fit` returns a concrete proposed patch without applying it. Existing contracts are preserved. A proposed contract must be reviewed against the original request and adopted through the user's actual host/CI boundary. `--baseline` detects removed or changed baseline obligations; an ordinary caller-provided file is not an authenticated adoption channel.

For native compatibility, `verify --rust FILE`, `verify --python FILE`, and `verify --typescript FILE` parse captured bytes without executing the source. The Python/TypeScript tool locations currently qualify the local macOS layout only. `verify --stdin-c FILE` checks a narrow C11 translation unit through captured stdin, with default compiler configuration and standard includes disabled. Preprocessor tokens, line splices and pragmas are outside that envelope. These routes prove named syntax surfaces, not type checking, tests, project runtime or host attestation.

## Contract and report

The UTF-8 version-1 contract is tab-separated. It starts with `UG<TAB>1`. Each `O` row has these eleven fields:

```text
O  id  revision  statement  origin  selector  projection  rule  argument  assurance  mandatory
```

Use `%25`, `%09`, `%0A` and `%0D` for percent, tab, newline and carriage return inside fields. Malformed escapes, duplicate IDs, unknown fields/versions, missing requirements and injected transport records are rejected. Selectors are exact paths (optionally `file:path`), explicit `prefix:path`, or `*`. Projections are `membership`, `content`, or `unknown`; content-consuming rules cannot pretend to depend only on names. Assurance is `exact`, `semantic`, `runtime` or `formal`. This initial contract admits mandatory rows only; it does not silently infer exemptions or remove requirements.

Rules include `member`, `path-member`, `absent-path`, `contains`, `absent-text`, `json-syntax`, `fact-heading`, `fact-import`, `fact-definition`, `fact-json-key`, `fact-json-valid`, `native` and `semantic`. A failed `absent-text`, `absent-path` or `json-syntax` obligation names its offending file (and, for text, the line) in `counterexample` and in its next action. Text/fact predicates have their literal declared meaning. A lexical definition/import candidate is not proof of language semantics or import resolution. Unsupported grammar and unknown dependencies remain unknown. JSON numbers retain lexical precision; duplicate decoded keys and malformed Unicode escapes are rejected. JSON depth is bounded at 64. Text/Markdown/source-line facts retain original UTF-8 byte positions; JSON AST key facts explicitly lack original-key spans.

The JSON envelope includes all requested obligations, computed versus currently admitted state, coverage, facts, computation identities, scoped timing and recovery actions. Exit codes are 0 for the requested local surface, 1 for failure, 2 for unresolved evidence, 3 for usage/input/tool errors and 130 for interruption. `indexed` is a distinct optional-data result, not verification. `--local` never drops semantic/runtime obligations to manufacture a successful full-contract report.

## Reuse, custody and recovery

The graph separates membership queries from content dependencies and current admission. An unrelated content change does not change a names-only query. Source and parser/rule identities are distinct; build-generated `Identity.bend` binds relevant implementation/tool closures. Identical content/grammar fact extraction can reuse within a trusted invocation. The report counts actual parses and that reuse separately.

An explicitly selected `--cache DIR` stores only compact digest-keyed reported comparisons; no cached result is sent back to exact evaluation. Its checksum detects corruption, not a malicious same-user rewrite. Disk results never discharge exact obligations. No privileged signing or authorization subsystem was introduced to make a mutable cache trustworthy. A qualified host provenance channel remains necessary for stronger persistent reuse.

Captured content and metadata are revalidated before current admission. Changed-and-restored inputs reopen affected admission; old computation remains visible as historical work. This observation is not represented as a qualified immutable whole-workspace snapshot. Missing host attestation leaves required runtime obligations unresolved.

`--journal FILE` explicitly requests a compact append-only operation report. `explain --journal FILE` replays it in Bend: exact duplicates are idempotent; conflicts, partial writes and malformed rows are visible; cancellation/supersession cannot be revived by a late completion. The journal is reported evidence, not authority to resume or replay an external operation. No automatic provider replay occurs after a crash. Signal cancellation terminates the invocation's owned process group; provider cancellation cannot guarantee cancellation of billing.

The descriptor adapter records symlinks, nonregular inputs, changing captures and invalid UTF-8; affected obligations remain unknown while unaffected observations survive. Ordinary checks use bounded CHUNK requests, and Bend folds their partials ([sessions reference](plugin/skills/harness-ultragoal/references/sessions.md)). Read planning and the final check accept ordered physical pages, so a large listing is not pruned to one 16 MiB frame. A selected large logical source is hashed, delivered in UTF-8 pages and checked again through one held descriptor; Bend verifies its byte offsets, sequence and final SHA before parsing. Multiple selected sources keep one logical path each. Index writes fact sets as cores answer, one set per line. Host headroom is remeasured at dispatch and each source page; batch size and concurrency adapt, and transient pressure pauses with the current descriptor and completed pages retained before a named offset is returned. There is no fixed total file-count or source-size refusal. A 16 MiB physical core frame, 256 MiB ordinary answer and provider envelopes still apply to each unit; current device pressure can leave an explicitly incomplete result. Ordinary cores rotate before their 512 MiB cumulative traffic budget, while retained source/rank/advice/check sessions use a live headroom budget. `.git`, `target` and `node_modules` are excluded from the snapshot universe. These are operational bounds, not latency acceptance targets.

## Jev

The direct endpoint is `https://api.typesafe.ai/v1/systemone`, with explicit model `jev-1.13.0`. Credential lookup happens only after request admission, via `TYPESAFE_API_KEY` or an explicitly selected macOS Keychain service. Keys never enter argv, source, reports or persisted request configuration. An active credential found inside a packet prevents dispatch.

Example synthetic-only development call:

```sh
target/release/ultragoal semantic --request /tmp/public-synthetic-request.json --disclosure synthetic --keychain-service research-run.typesafe
```

Choice, Noul and Score preserve their distinct raw meanings. Bend rejects nonfinite/out-of-range values, invalid probability mass, inconsistent selected options and weighted Score levels. The adapter rejects duplicate JSON keys, wrong model/kind/question/option sets, wrong Score legend/order and incomplete response shapes. A choice must hold the highest probability exactly; a top probability shared with another option is a valid tie that never qualifies as a decision (`advisory-tied`). Unknown/malformed/unavailable results never default to clear. A Score is accepted when it lies within the two-decimal wire rounding of its probabilities' mean, (L(L-1)/2+1)×0.005 for L levels; probability normalization stays strict. Every attempted request is retained, including schema failures and ambiguous timeouts, and `--usage-ledger FILE` records each attempt before dispatch and again when it settles. curl runs with an empty environment and `--noproxy '*'`; exits 6 and 7 (resolve, connect) settle as `verified_not_sent` at zero and may be retried, while timeouts and other transport failures stay `unknown`. Responses are capped at 512 KiB, depth 32 and 8,192 values. Retry is bounded by the same operation deadline and admitted attempt count; schema failure is not retried. A test build compiled with `UG_TYPESAFE_PROVIDER=synthetic-loopback-v1` posts to `127.0.0.1:$UG_SYNTHETIC_PROVIDER_PORT` with a fixed bearer and never reads the key; any other value fails closed, and `build.py` refuses to build the product with it set.

The [version-2 semantic contract](SEMANTIC_CONTRACT.md) separates alignment, verification adequacy/performance, consistency, proposed recovery actions, achieved recovery and investigation reuse. Each internal request contains one original obligation. Legacy `recovery` is unavailable until explicitly migrated to `recovery-proposal`, `recovery-state` or `investigation-reuse`; requirements and assurance are never silently lowered. Explicit role/clause context is Bend-validated; ordinary files retain unclassified roles. Semantic results are advisory with one qualified exception: under the fitted policy `ug-semantic-fit-v3`, a contradiction in the `recovery-state` family, which met the blocking bar on two independent held-out sets, fails an obligation whose admitted state is otherwise unknown. `--semantic-blocking off` keeps every semantic result advisory. Optional relevance selection retains counterevidence and reserves shortlist space for lexical definition/assertion/import candidates. These hints do not establish parsed roles or full retrieval coverage. Known private state/credential paths and private-key blocks are excluded from disclosure, without treating their absence as complete evidence. The caller must still supply an authorized, minimal disclosure scope. No personal conversation histories are used.

## Verification and claim ceilings

| Requirement | Current verifier / important limit |
| --- | --- |
| Contract, parser and stream integrity | `tests/journey.py`, `tests/json_parser.py`; fragmented reads, UTF-8, malformed JSON, duplicate keys, complete frames, injected records |
| Reuse and admission | Actual CLI differential mutations, forged-cache regression, membership changes, explicit unknown dependencies, baseline comparison, changed-and-restored source |
| Pure laws | `PROOF.bend --check-only`; lifecycle, unknown/mandatory preservation, assurance separation, quota helper, actual query-key invariants, conditional trusted parse reuse and full wire string round-trip |
| Counterexamples | `tests/proofs.py` deliberately breaks admission, lifecycle, quota, mandatory coverage, query fingerprints and parse reuse; missing/open proof imports must fail |
| Native boundaries | Captured C/Rust/Python/TypeScript syntax, signal cancellation, bounded output; no full-project runtime or qualified host identity claim |
| Optional evidence selection | `tests/context_journey.py`; exact spans, retained anchors, changed/new dependencies, fabricated text, reported-prior reuse; critical recall outside a labelled corpus remains unknown |
| Semantic service | `pilot.py`; original public synthetic packets/responses and all attempts. Development labels are not a held-out calibration set |
| Performance | `evaluate.py benchmark`; fresh process/stage/wall timing with explicit scope and all-attempt ledger. No language-speedup or end-to-end task-benefit inference |
| Migration | `migration.py`; exact listed release bytes archived/restored in an empty directory. No install, global consumer migration or retirement |

The proof set does not prove native filesystem completeness, compiler correctness, provider truth, protected adoption, universal grammar support or whole-project correctness. Wire round-trip uses an explicit finite-bit proof helper to avoid relying on negative literal-pattern refinement. The conditional parse-reuse theorem requires a genuinely valid cached parse; mutable disk entries cannot provide that premise.

`package.py` prepares a local same-name plugin candidate and manifest, without installation or publication. Source, package, installation, discovery, native runtime and product outcomes remain separate. The repository's legacy aggregate check is still required at integration; its known sandbox-activation failure must be reported rather than repaired by rebuilding privileged legacy ownership.

Still unqualified: protected adoption/native host attestation, trustworthy disk-persistent exact-result reuse, held-out blocking for families other than recovery-state, matched task/attention/cost benefit, complete external consumer migration and EJ retirement. Installed discovery, dirty recovery and the representative deep-directory scale shape passed on b192; current dogfooding edits need a fresh bound build and receiving. These limits do not imply public-release readiness.

`verify --bend-proof FILE [--root DIR]` captures a bounded local Bend import closure, including adjacent LAWS, and checks it without execution. Relative .bend imports and Base are supported; remote, foreign, unsafe, incomplete and unsupported import syntax are refused. The closure is limited to64files/4MiB and retains exact compiler/Base/source identities. Source-line law names are candidates, not an independently parsed theorem inventory. No runtime or protected-host claim follows.

Context selection `--details` includes the full locally prepared candidate metadata and which entries entered the shortlist, plus a provenance digest and the original index path/digest. Default responses keep this metadata recoverable without dumping all source text.

`semantic --request REQUEST.json --response RESPONSE.json` validates preserved response bytes offline through the same numeric/schema/Bend checks. It reports imported advisory evidence, performs no provider request or credential lookup, and cannot claim original execution/freshness. This allows receiving-adapter repairs to recover retained raw evidence without repeating paid calls.

`tests/index_scale.py` exercises indexing and selection across the former16MiB whole-frame boundary. In-process fact lookup uses the Bend standard map; it contains only parses computed during that invocation. Candidate windows retain adjacent short source lines when a different line exceeds4KiB, and report the oversized line explicitly. The original frozen V2 semantic evaluation executable remains separate from later indexing/transport candidates.

Optional indexes use source-spans/2: source-line text is represented by null plus exact byte offsets and the file digest, avoiding a second copy of repository contents in the report. Plain line facts, nine in ten of all facts, are written as `line_runs` (`[first line, start byte, length, ...]`, each next line starting one byte after the previous end; [format](plugin/skills/harness-ultragoal/references/commands.md#facts-and-selection)); on the seven development repositories this makes the report 37 MB instead of 432 MB for 111 MB of text, and `select` reads either form as a stream. JSON metadata with no source span retains its value. Selection freshly captures the source, verifies its digest and span boundaries, and reconstructs exact excerpts; no generated summary becomes evidence. Inline/1 check reports remain supported. Separate parser keys prevent mixing inline facts with compact span facts.

Run `python3 benchmark_index.py --files 10000 --bytes-per-file 10000 --repeats 30 --out /tmp/NEW-UNUSED-OUTPUT` for the full mixed-language reference index. It retains source/executor custody, actual per-file sizes/hashes, all-attempt ledger, source identity checks and separate serialization/output timing events. The existing evaluate.py benchmark remains the narrower membership-plus-one-predicate journey.

`advise --request CONTEXT.json` uses internal Bend-authored Noul and Score questions for test choice, causal diagnosis, requirement mapping, change-impact prioritization and reuse usefulness. It returns all candidates and complete attempt evidence without executing actions or dropping checks. Explicit synthetic/public/project disclosure flags require preexisting user authority; no new permission comes from a flag or model response. Advisory utility can be used while mandatory evidence admission remains unqualified.

The index bridge interns repeated, byte-identical source bodies only within each transport frame. Unique bodies remain inline, and frames with no duplicates use the plain protocol so they pay no Bend atom-table overhead. Bend rejects duplicate, missing, malformed or digest-inconsistent references and restores exact source rows before scope selection, parse planning or fact computation. This is transport compression; it adds no trusted persistent cache. Native CPU parallel lets evaluate ordered batches of eight frames, and balanced ParseWork jobs parallelize distinct source parses. The stream rejects malformed lengths, invalid UTF-8, truncation, missing/trailing terminators, more than128frames, frames over16MiB and aggregate encoded input over512MiB. Restored references are limited to100,000files/128MiB per frame. The Rust process adapter retains bounded stdout/stderr, whole-operation deadline and cancellation; its pipe-draining threads do not schedule parsing. `tests/stream.py` covers batch rollover, thread-order equivalence, exact interning equivalence and rejection paths. Performance is measured separately for each candidate; removing the process pool alone did not meet the reference latency target.

Rust/Python/ECMAScript indexes now include [bounded source structure](SOURCE_GRAMMARS.md), exact header/name/body spans, Python suites and import targets, and opaque Rust macro/attribute markers. Existing line facts remain. `mixed-spans/1` check reports retain line/name quotes and rehydrate null body/parameter spans from freshly validated source. Parser limitations and unknown dependency closure are explicit. Rule identities include all runtime Bend modules, while parser identities bind their parser closure; both include the installed Bend/Base/effect source identities, checked before and after a build.

Production parse reuse now groups aliases under a typed canonical parse job. Each request retains its path, digest, format and exact source; grammar is derived from that path. A repeated alias requires constructive argument equality, while the first alias is reflexive. LAWS.grouped_parse_row_correct covers the emitted map key and all fact-row fields against the original request, including the actual parser result. The legacy conditional blob helpers remain compatibility tests and are not the production reuse path. This law does not establish filesystem truth, complete dependency discovery, native language validity, or the full cross-revision query graph. Performance qualification for this stronger representation remains open until the final integrated candidate is measured.

Each CLI invocation now uses one private compiled-core session for planning, validation and index batches. S1 transport envelopes bind sequence, completion marker and exact byte length. The Rust adapter bounds allocation, queue time, deadline/cancellation and child cleanup; Bend handles request validation and CPU work batches. The session closes before final CLI success, rejects unexpected trailing responses, and is not a daemon or a replacement host scheduler. The private session retains the bounded production query graph across evaluation frames. Each predicate depends on its current rule-specific source projection. Exact constructive argument equality admits cached computation; lifecycle, coverage, exclusions and assurance are checked again for every result. Removed nodes are retired, malformed evaluation clears graph state, and transport failure ends the process. This is in-invocation state; editable disk caches remain advisory.

Query graph implementation: `QueryPlan.bend` builds a deduplicated two-level graph (selected input projection → predicate); `QueryGraph.bend` updates current dependency values in order and reuses only `QueryValue.bend` results proven to equal `QueryCompute.compute` for the complete current arguments. `QueryGraphProof.bend`, bound into `LAWS.bend`/`PROOF.bend`, proves actual revision output equals a full recomputation, including deletion/reset and cache retirement. The rule-specific projection adapter is checked independently against the uncached evaluator; this theorem does not certify filesystem observation, projection completeness or native tool custody. Unknown input and operation admission remain fresh. Chunked content projections retain every CH state, id and order while retaining only PC rows for the obligation's exact selector; unrelated selector partials cannot dirty an exact-path predicate, and a changed or omitted CH conservatively recomputes. Full CF membership and U/I/lifecycle admission still use the complete current frame. The pressure offset names a fresh-recapture retry, not byte-level resume. Membership projections omit content; content/fact/native inputs track their consumed data. Current scheduling scans the bounded graph in dependency order; a separate reverse-edge worklist and cross-invocation retained computation remain unfinished.

Per-revision graph limits remain 2,048 nodes and 16 MiB charged payload/dependency arguments. A projection shared by several predicates is built and charged once. Physical frame, provider and deadline envelopes remain; retained page traffic follows current host headroom rather than the ordinary session's fixed cumulative budget. A resource refusal is incomplete work, never a verified obligation.

Optional selection groups byte-identical excerpts only when their grammars match. Each selected group retains every original candidate ID/path/content digest/span in its `aliases`; binding instruction/requirement anchors remain separate and unranked. Jev receives the representative excerpt plus at most the Bend-declared 16 duplicate paths, with any unshown aliases explicitly unassessed. Full provenance binds the local prepared-evidence identity and prior comparisons; redundant digests and repeated bodies are not sent as semantic evidence. Path-specific behavior is never inferred solely from shared text. `relevance/3` remains uncalibrated advisory.

Fixed Python/TypeScript/C11 verification now plans a captured-input request in
Bend and executes it through a consumed local pending-call handle. The observation
retains operation/request bindings, actual child PID, complete/incomplete input
and streams, terminal/cancellation/deadline facts and current source comparison.
The PID is local observation, not a protected host execution ID. Requested-tool
pre/post hashes assume the loaded tool/runtime stays stable; they cannot rule out
replacement and restoration. `explain --report` treats a stored observation as a
reported import and never admits it as a new current call. General project tests
and protected adoption remain external host/CI qualification boundaries.

Build qualification is separate from check throughput. `build.py` sets no time limit and records each stage's time and memory. Builds take about 190–290 s of emission on a quiet host, longer under load, and 29 s of Clang, with emitter peak resident memory around 26–28 GB on this 64 GB Apple Silicon host; smaller-memory hosts and other platforms remain unqualified. Build metadata records the exact SDK/compiler/flags, generated C digest and per-stage time/memory. The packaged runtime is compiled once, not after user source edits.

`fit --root DIR --contract REQUIREMENTS.tsv` prepares byte-exact forward and
inverse patches from explicit requirements. Existing requirements must remain
preserved under Bend's contract comparison; removal or revision changes produce
no patch. Selected fit inputs use descriptor-bound no-follow capture, so linked
or dangling configuration targets remain unavailable rather than being replaced.
The report binds observed root/target/input identities and keeps adoption
unverified. The native host reviews and revalidates before applying; inverse
patches must be refused after user edits. UG fit does not write a target or grant
permission. The no-contract route remains a manifest-derived proposal template,
not evidence that its test command or original-outcome coverage is sufficient.

Explicit opt-in foreground sessions are described in [session usage and limits](plugin/skills/harness-ultragoal/references/sessions.md). Current source integrates typed query-parse retention and physical local transport; final candidate/runtime/performance verification remains required. Default commands remain one-shot.

## Current status

This candidate continues the transferred implementation. Current requirement
coverage, measurements, failures and evidence locations are in
[IMPLEMENTATION_STATUS.md](IMPLEMENTATION_STATUS.md). Retained sessions use a
line-delta transport between owner and core; see the session reference.
