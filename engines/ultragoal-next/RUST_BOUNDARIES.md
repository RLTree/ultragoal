# Rust boundary qualification

Bend owns the query graph, parse sharing, logical scheduling, predicate decisions,
semantic rubrics, disclosure eligibility, grouping/ranking and admission. The
native host owns permissions, execution authority and agent lifecycle. This file
records the current boundary and remaining departures; it is not permission to
retain a broad Rust subsystem.

| Capability | Current source and interface | Evidence / limits | Retirement or further qualification |
| --- | --- | --- | --- |
| R1 descriptor-bound filesystem observation | `fs_adapter/`, `inventory.rs`, `relative.rs`, `os.rs`: confined enumeration, no-follow reads, identities, read errors, membership checkpoints and revalidation; selected captures run in bounded parallel windows with the total byte budget applied in path order (same accept/reject set as a serial pass) | Race/symlink/rename/root-replacement/dirty-preservation tests; captures actual selected bytes. Before/after checks do not establish immutable native-verifier input custody. Bend supplies logical scope/read plans and exclusions. | Retain until Bend has an equivalent qualified descriptor/custody API. Keep file/buffer bounds physical; do not move rule selection or authority here. |
| R1 physical private-core transport | `core_session.rs`, `index.rs`, `process.rs`: one child per invocation, versioned sequence/length framing, bounded queues/streams, deadline/cancel/close, physical 4MiB batching, exact-byte transport interning, for a retained core a line delta (keep/skip/literal) against the frame it last sent, verified by replay before sending, and for a check selecting over 2MiB of content, CHUNK frames (~1MiB each, 8MiB per request) to a dedicated core whose CH/PC rows are appended verbatim to the evaluation frame | Private-session, UTF-8, truncation, sequence, late/cancel, cumulative512MiB and process-lifetime tests. Index uses one core; Bend owns eight-frame CPU batches and parse work. | No Rust worker pool or logical scheduler. Reassess copies/crossings on final medium-tier workload; no FFI expansion without measurement. |
| R2 direct secure TypeSafe transport | `provider.rs` owns semantic preparation and attempt/ledger accounting; private `provider/credential.rs` owns environment/Keychain lookup; private `provider/dispatch.rs` alone builds the fixed curl transport from a `Screened` handle after credential and Bend secret-screen admission. The endpoint is fixed (or compile-time synthetic loopback), TLS uses an empty environment/no proxy, response I/O is bounded at 512 KiB, and attempts pair with ledger reservations. | Strict envelope, wrong model/question/kind, malformed number/distribution, incomplete-answer, credential-refusal and no-handle-before-screen controls; ignored, unreachable and early-handle mutants were rejected by actual adapter tests. Secrets do not enter argv or persisted request bodies. | Retain until equivalent TLS/credential/cancellation support is qualified in Bend. Native permission and disclosure authority remain external. |
| R3 mature Rust syntax compatibility | `native_syntax.rs`, generated `native_identity.rs`: `syn` source parse of captured UTF-8 bytes | Named Rust syntax surface only; source/wrapper/version identity bound. No macro expansion, type checking, build or test claim. | Retire only after required syntax corpus, malformed-input and latency equivalence. Keep native compiler semantics native. |
| R3 fixed native parser/check adapters | `main.rs` named Python/TypeScript/C11 routes and `proof.rs`: captured bytes or bounded local check-only closure to fixed tools; no arbitrary command executor | Actual tool/input/argv/terminal records and named journey tests. C11 excludes preprocessing/headers; Python/TypeScript paths are qualified only for the documented macOS layout. Cancellation is an incomplete terminal observation. | The current candidate implements consumed current-pending-call handles and complete captured-stdin request/observation binding for fixed Python/TypeScript/C11 syntax routes. Imported reports cannot become current calls. This is a local bridge/OS/tool assumption, not protected host attestation. The full qualified host/project-runtime channel remains unavailable. |
| Physical artifact translation | `main.rs`, `context.rs`: parse report envelopes, validate source spans against captured bytes, bind returned alias references, serialize original requirement/origin/revision and predicate surface | Current input/span/alias/provenance tests, including changed hidden aliases and unchanged vendor-request bytes. The original requirement is retained; a narrow predicate pass does not certify original-outcome alignment. | Keep serialization and byte containment here. Representation choices must be explicit and must not become semantic authority. |

## Pressure and credential custody in this candidate

`fs_adapter/stream.rs` samples current headroom before hashing and before each source page. It retains the open descriptor and acknowledged Bend pages for a transient shortage, then returns a typed `Pressure { offset }` after the bounded wait. `segmented.rs::mixed_chunks` treats only that pressure variant as an incomplete source: earlier completed CH rows stay in the same report, the pressured source enters Bend as an unavailable F, and the report names its path and next byte offset. That offset is diagnostic; the next operation recaptures from byte zero under fresh descriptor, source, contract and membership checks. Cancellation, source change and other failures remain terminal. A missing or changed source never inherits a prior verified result.

`provider.rs::key` owns explicit environment selection and fallback to the existing narrow `research-run.typesafe` Keychain service. `keychain_key` is the only direct Keychain command. Both return a validated credential type or a fixed secret-free error; malformed present environment input fails closed. The credential is used only in the fixed TypeSafe transport and never appears in argv or reports. The project-owned `docs/legibility/` registry records these and the resource/source-stream boundaries exactly. It does not approve the broader inherited Rust effect sites or establish Bend/Python semantic coverage.

## Remaining ownership debt, not qualified exceptions

- Canonical rule/semantic and parser/fact computation IDs now hash in Bend.
  `main.rs::hash` remains for raw OS/tool/transport/bootstrap byte identities as
  the explicitly scoped temporary optimization in `HASHING.md`. Its helper
  comparison and correctness corpus are retained; final engine boundary-cost
  qualification and replacement criteria remain open. This is not a language
  limitation or a blanket permanent ownership exemption.
- `JevLayout.bend` owns supported Choice option sets, canonical ordering,
  unique names and nonempty descriptions. Rust decodes JSON, binds the planned
  names to the exact request and translates vendor probabilities mechanically.
  The layout checkpoint passed all four option sets and malformed-input tests.
- `JevWindow.bend` owns request windows for `select` and `advise`: the head and
  slices under both provider limits (64,000 request bytes, 32,000 bytes of state
  plus longest question), omitted items, and the merge (first accepted judgment
  wins; a judgment from a request that did not carry the item is refused; a missing
  one is named uninspected). Rust builds each planned body, dispatches at most two at
  a time and appends the usage ledger; Bend admission (`Policy.provider`) checks
  both limits on each built body before dispatch.
  `Advice.order` owns the deterministic advice ranking.
- `Secrets.bend` owns credential screening (file names, credential-shaped text,
  service-account and configuration-JWT content), adopted from the jev-codex
  token-saver and checked against it differentially (698 cases, 0 mismatches). It
  gates disclosure and screens each whole request body (`SECRET_SCREEN`).
- `ExcerptPlan.bend` owns source-window selection, including eight contributing
  spans and the 4096-byte window limit. Rust validates captured spans, batches
  their coordinates, and checks returned coordinates against those sources before
  slicing actual bytes. Binding sources remain retained; ranking and duplicate
  grouping also run in Bend. The integrated window route passed candidate tests;
  the separate prototype also has255legacy-oracle matches and five refusal cases.
- Rust copies operation receipts and advisory disk comparisons; it must never
  deserialize a user-editable result as a proof witness or adopted authority.
  Current reports admit zero disk results as exact evidence. A scoped native
  thread overlaps advisory-cache byte hashing/reading with Bend evaluation and
  joins on success or failure; it never supplies cached answers to that evaluation.

## Native and adoption ceiling

The current fixed adapters do not supply a protected general host observation
channel or protected adoption baseline. A complete host request needs operation,
request/obligation, tool/argv/environment, scope, immutable input, expected output
and deadline bindings. Its observation needs host execution identity, actual
producer/input binding, terminal/stream/artifact facts and current admission.
Until that receiving boundary is qualified, required project runtime and adoption
remain explicitly unresolved. A self-authored approved file is not a substitute.

## Explicit computation lifetime

| State or effect | Actual lifetime and owner | Permitted entry and validation |
| --- | --- | --- |
| CLI observation | One frontend command in `main.rs`/`inventory.rs`; descriptor-bound files close after the attempt | Captures, hashes and current revalidation precede Bend admission; imported reports remain reported. |
| Task session endpoint | One foreground task owner in `session_transport.rs`; owner-only socket/handle removed on close or expiry | `session --root --directory`; peer UID, root/core/endpoint identity, sequence and deadlines are checked on each request. |
| Core and physical lane | `core_session.rs` owns a child or a rotating dedicated lane; Bend owns retained graph values | Only framed bounded requests; malformed/late/incomplete replies fail closed. No disk result constructs a graph value. |
| Jev credential | One call in private `provider/credential.rs`; no persistent plaintext cache | `key` selects an explicit env value or narrow Keychain item, validates representation, returns a typed credential or fixed refusal. |
| Jev attempt and ledger | `provider.rs` owns each dispatched request and its reservation/settlement | Bend disclosure/budget admission and secret screen precede dispatch; ambiguous timeout/settlement remains unknown, never replayed as success. |

These are local host boundaries, not protected adoption or authenticated host
attestation. The material-effect contract and the offline Next gate check their
declared owners; native receiving still tests the actual paths.

`session_transport.rs` extends one fixed core's lifetime only after an explicit
foreground `session` command. It owns restricted local framing, peer-UID checks,
endpoint/incarnation/workspace/core binding, request/deadline/byte bounds and
cleanup. The owner may replace its core with a fresh core of the same binary,
primed by re-sending the same frame bytes; no state crosses processes. It
executes no verifier/provider/permission/adoption action. Fresh
frontends continue through the existing descriptor-bound observation and current
admission paths. Bend owns typed parse retention and reverse-closure invalidation;
no serialized disk result becomes a constructor witness. Same-user socket/PID/
nonce and ordinary file stamps are not protected host provenance. See the session
reference for the supported commands, current retention envelope and limits.

## Session transport and semantic decision

- Owner→core evaluation bodies after the first are line deltas: transport
  compression inside one owner/core incarnation. Bend
  rebuilds the lines from its retained values, checks that the base is consumed
  exactly and the base sequence matches, and otherwise answers
  `delta base unavailable` without changing state; the owner then resends the full
  frame. No digest or disk result substitutes for the bytes.
- The owner waits with `poll(2)` for connections and disconnects.
- `provider.rs` takes the semantic rubric version from Bend's probe rows and
  forwards the validated option probabilities to Bend's `SEMANTIC_CALIBRATED`
  decision; it applies no threshold itself.

## Qualification

Current candidate evidence, measurements and unmet targets are in
IMPLEMENTATION_STATUS.md. Native session results are reported local computation
under trusted-running-code/input assumptions, not protected provenance. Peer-UID
checks and restrictive modes are implemented; no cross-user experiment
establishes a stronger claim.
