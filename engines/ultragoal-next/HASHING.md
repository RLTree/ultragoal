# Canonical hashing and measured host boundary

Bend now computes SHA-256 for rule/semantic computation identities and parser/fact
computation identities before sending them across the process boundary. The Rust
bridge validates the canonical digest representation and does not hash those IDs
again. Actual query-cache reuse still requires equality of the complete structured
arguments; a digest match is not a proof witness.

`Sha256.bend` implements the bounded SHA-256 byte algorithm; `Utf8Encode.bend`
constructs canonical UTF-8 without normalization, and `Digest.bend` provides the
text/identity boundary. Invalid scalar values, non-byte elements, incorrect byte
lengths and inputs beyond16MiB are refused. An unavailable identity is not a valid
64-character digest and cannot silently become a reusable identity in the bridge.
Formatter/input-refusal laws supplement independent byte/vector tests; they do
not constitute a proof of cryptographic security or NIST module validation.

Algorithm reference: [NIST FIPS180-4, August2015](https://nvlpubs.nist.gov/nistpubs/FIPS/NIST.FIPS.180-4.pdf), sections4.1.2,4.2.2,5 and6.2.
Published example checks: [NIST NSRL test data](https://www.nist.gov/itl/ai/ai-standards-and-guidelines-group/nsrl-test-data).
Development evidence is retained under `/tmp/ug-next-hash-work-20260923`: 24initial
vectors/boundaries,332raw-byte differential cases including16MiB,309UTF-8 cases,
and six malformed byte/length/scalar refusals. The canonical computation-key
integration also has independent hashlib checks in `tests/digests.py`.

SHA-256 rounds keep the eight working words and a sixteen-word schedule window in
U32 parameters, and the UTF-8 encoder accumulates length/validity without a record
per character. The public `Sha256.hash(bytes, length)` signature and its refusal
laws are proved by the build; outputs match hashlib on block-boundary, multibyte and
159 KB inputs (`.codex-worktree/opus-work/probes/sha`) and `tests/digests.py`.

## Narrow temporary host exception

Raw OS-capture identities, native tool/runtime fingerprints, transport request/
response byte identities, and build/package custody still use the host SHA-256
primitive. These occur at native byte/I/O or bootstrap boundaries; they do not
choose rules, approve evidence, or replace the constructor-proven query cache.
This is a measured temporary optimization and integration limit, not an alleged
Bend language limitation or a permanent Rust hashing subsystem.

A framed comparison on the current implementation (fresh process per request, 9
alternating rounds, digests equal to hashlib; `.codex-worktree/opus-work/diag-cold/hash.jsonl`)
measured the `HASH` command at 98 ms for 1 MiB and 784 ms for 8 MiB, against 6.1 ms and
30 ms for host SHA-256 with the same framing. Most of the Bend time is framing and
parsing (a parse-only request takes 584 ms at 8 MiB); Bend's UTF-8 encode plus SHA-256
costs about 25 ms/MiB, host SHA-256 about 1.8 ms/MiB in process. Moving raw-capture
hashing into Bend would add about 2.4 CPU-seconds per 100 MB, so the exception stays.
Small canonical computation keys stay in Bend and no longer cross as long strings.
Duplicating or serially crossing full native byte buffers merely to hash them is
not adopted on these data.

Exit criteria: preserve published/random/malformed-input equivalence, bind
canonical bytes/algorithm identity, and demonstrate a batched or native-accelerated
Bend path that meets the final capture/transport time and memory budgets on the
supported workload. Reassess raw source capture separately from binary tool,
credential-transport and pre-core bootstrap identities. Final engine-level
latency and boundary-overhead qualification remain required; this helper pilot
alone does not discharge them.
