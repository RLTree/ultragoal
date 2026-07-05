# Review Target And Archive Receipts

The validator receipt proves package/static behavior. Reviewer sign-off and zip
handoff need separate proof anchors so neither one signs a moving target.

The validator `package_digest` intentionally excludes generated audit
artifacts. The compact review-loop index is source content, so changes to
review-history routing stale the source digest instead of drifting outside
proof.

## Review Target

Generate the current review target after the canonical validator pass:

```text
cargo run --offline -- --root . review-target --receipt validation_artifacts/review/review-target-receipt.json
```

The receipt is intentionally detached from the repo archive. It contains
`review_target_digest`, `included_path_count`, and a digest of the exact
included path list. It hashes manifest-owned source content while excluding:

- `validation_artifacts/ultragoal-audit/`

`plugin-manifest-draft.json` is hashed in normalized form with detached
generated-artifact paths removed. This keeps reviewer sign-off and validator
package identity stable when generated receipts are refreshed, while
history-routing edits still change the digest.

Reviewer approval is current only when all four reviewers evaluate the same
`review_target_digest` and the same current validator receipt. Historical review
rounds are never standing approval.

Use a canonical non-symlink output root for detached receipts. On macOS, avoid
symlinked temporary roots; the validator intentionally rejects symlink
components in output paths.

## Review Archive Anchor

Material sign-off review may build a deterministic archive, but only as a
detached review anchor. This receipt gives reviewers one immutable zip identity
to inspect; it is not upload, release, distribution, publication, or install
proof.

```text
cargo run --offline -- --root . archive --zip validation_artifacts/review/harness-ultragoal-plugin-proposal.candidate.zip --receipt validation_artifacts/review/candidate-archive-receipt.json --archive-purpose candidate_review_anchor
```

The archive builder writes deterministic zip entries under one root directory,
sorts entries, fixes entry timestamps, rejects symlinks, and rejects archive
junk such as `__MACOSX`, `.DS_Store`, `.serena`, bytecode, and cache files. The
archive receipt records the zip SHA-256, entry count, root names, manifest
digest, package digest, hygiene result, `archive_purpose`, and claim ceiling.

The archive digest is a review artifact identity, not an upload artifact
identity. It remains valid only for the sign-off round that cites the same
current validator receipt and `review_target_digest`.

## Upload Distribution Archive

`upload_distribution` is intentionally not mintable by the archive command until
the gated promotion path consumes and joins all required approval evidence:
current validator receipt, current review-target receipt, candidate archive
receipt, and a four-persona sign-off review receipt bound to the same
anchors. Until that promotion path exists, the only supported archive purpose is
`candidate_review_anchor`.

Do not claim an upload or distribution archive SHA-256 from a candidate archive.
