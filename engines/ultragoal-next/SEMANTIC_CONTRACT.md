# UltraGoal semantic interface v3 (rubric version 4)

This is the current source contract. A campaign must bind the exact built source,
request bytes and model identity. Version 3 has a qualified blocking route for
`recovery-state` contradictions only; other families remain advisory. Neither
this file nor a model opinion authorizes an effect or qualifies a native claim.
Historical v1/v2 campaigns and current development probes remain development data.

Each internal request contains one original obligation and two independent Choice
questions, `conflict` and `support`, using `jev-1.13.0`. Obtain their exact version3
instructions/options from Bend `SEMANTIC_PROBES`; prose is not a prompt generator.
Question IDs correlate answers; they do not convey inference instructions, and
questions cannot read one another's answers.

## Meaning and admission

The state schema is `ultragoal-semantic-state/3`: original `requirement`,
`requirement_origin`, an explicit `admission_boundary`, and supplied `context`.
A clause list or material contract cannot weaken the original requirement.
Semantic support concerns what the supplied materials establish within that
comparison. Native execution authenticity, identity, freshness, permission,
protected adoption and calibration are independently checked by code/host.
A report can support a proposition about its reported outcome without proving
that the run happened. A test body is not a run, and a proposal is not recovery.

Families: `alignment`, `verification`, `consistency`, `recovery-proposal`,
`recovery-state`, and `investigation-reuse`. Their question-specific forms separate
behavior description from reported outcome, assertion design from reported
verification, required passing results from bug-detecting failures, proposed
next actions from reported recovery, and finding usefulness from applicability.
Legacy `recovery` stays unavailable until explicitly mapped to its intended target.

## Declared material schema

`ultragoal-semantic-context/3` has `target`, nonempty `scope`, `material_contract`,
`clauses`, and `evidence`. A material contract names its `question`, declared
`closed_world` (`provided-materials`, `explicit-dependencies`, or `unknown`) and
linked `conditions`. A condition has `id`, `statement` and nonempty `evidence_ids`.
An explicit-dependency world must link a condition to a dependency inventory.
That label/link does not prove relevance or exhaustiveness. `provided-materials`
limits the comparison; it does not define missing helpers/types or imply that
unlisted dependencies do not exist.

Clauses have unique `id`, original-aligned `statement` and nonempty
`required_roles`. Evidence has unique `id`, `role`, declared `material_type`,
`version`, `content`, `provenance`, and `clause_ids` referencing existing clauses.
Evidence and clause arrays are bounded to64, conditions to32. Version3 rejects
unknown fields, unsupported role/type pairs, duplicate IDs and dangling links.
`valid` means schema/link validity only: not required-role coverage, cross-material
version identity, meaning, completeness or authenticated native evidence.

Roles include implementation, caller-expectation, asserted-test,
executed-observation, documentation, prior-finding, proposal, retrieval-metadata
and unclassified-source. Types distinguish source code/patches, interface
contracts, test source, execution reports, documents, findings, action proposals,
dependency inventories and selection reports. All role/type/version declarations
remain data. Version2 declared-context shape remains compatibility input.
Ordinary source captures use `ultragoal-source-context/3`, unclassified material
roles, actual source bindings and explicit missing required-surface context.

## Decisions and retained evidence

Conflict options are explicit-conflict/no-explicit-conflict/uncertain; support
options are complete-support/missing-support/uncertain. An available concrete
conflict takes precedence even if another surface is absent. No explicit conflict
plus complete support yields supported; other valid pairs yield insufficient;
missing/malformed probes yield unavailable. Simultaneous valid explicit conflict
and complete support retains `conflicting-valid-probes` for review. Keep both raw
distributions; do not invent a combined probability or statistical independence.
The operation budget remains eight questions (at most four paired obligations),
with bounded shared deadline/retries and all attempts/ambiguous usage retained.

Responses require exact model/question/option/type envelopes and finite in-range
numbers. Independent valid answers survive only a valid enclosing response.
Raw response decimals are authoritative; normalized binary64 values use the
explicit reporting tolerance. Missing data never defaults to clear. `--details`
retains the credential-checked exact final request JSON when available. Generic
`semantic --request ... --response ...` validates preserved bytes offline and
never claims a provider execution or current native observation.

## Optional retrieval

`relevance/3` keeps at most 64 optional groups, judged in head-plus-slices windows. Exact duplicate bodies group
only within the same grammar, with all local path/hash/span/ID provenance and
binding anchors retained. The provider sees one excerpt and at most16 duplicate
paths, with unshown aliases explicitly unassessed. Full prepared provenance
invalidates prior reuse even if the displayed vendor request is unchanged.
No summaries are generated, mandatory material is never pruned, and shared text
does not prove path-specific behavior. Candidate recall is measured on a synthetic
labelled corpus (`IMPLEMENTATION_STATUS.md`, Research Run adoption); omitted-evidence
recovery remains a separate evaluation obligation.

The27 prospective v3 development cases, blind judgments, and separate five-case
clarifications are preserved. The unknown-helper assertion case remains a false
concern under the blind interpretation. These probes do not qualify consequential
rules or supply an independent error-rate estimate; untouched confirmation data
and original-outcome evaluation are still required.

## Rubric version 4 and fitted gate policy

Probe text version 4 adds three general rules derived from development-split errors:
unsupplied helpers, configuration, placeholders, external defaults and unstated units
leave a clause unsupported and are not themselves conflicts; conflicts are checked
exactly (comparison boundaries, inclusive ranges, timestamp versus date, side-effect
order, positional syntaxes, every clause of a multi-part requirement); complete support
requires every required value to come from supplied content.

Bend's `SEMANTIC_CALIBRATED` decides version-4 answers with the fitted policy
`ug-semantic-fit-v3` (`.codex-worktree/opus-work/fit/policy-v3.json`, from `fit_v3.py`;
every decision carries its sha256). The frame carries the rubric family as a tenth field.
An answer qualifies for an option when that option leads the runner-up by more than 1e-4
(a tied top never qualifies), reaches min_p, and leads by at least min_margin.
- **Contradicted:** the conflict probe qualifies for explicit-conflict at the family's gate (min_p, min_margin):
  - alignment 0.505, 0.035;
  - verification 0.625, 0.265;
  - consistency 0.875, 0.775;
  - investigation-reuse 0.745, 0.515;
  - recovery-state and families without development data keep the v2 gate, 0.805, 0.635.
- **Supported:** otherwise, when the support probe qualifies for complete-support (0.915, 0.835).
- **Insufficient:** otherwise.

Gates sit half a 0.01 step below grid values. They were fitted on development
probabilities only: the v1 development split plus the consumed held-out v2, 1,569
consensus cases in all. The fit uses a 0.03 knife-edge exclusion and grouped
cross-validation to the blocking rule's bar: out-of-fold precision ≥ 0.95 with a lower
bound ≥ 0.90. Held-out v3 was scored once: only recovery-state met the full
blocking bar (precision 0.984, lower bound 0.945, recall 0.941). Other versions,
invalid numbers or unvalidated answers use the uncalibrated combination or remain
unavailable. The two distributions stay separate; no joint probability is formed.

`SEMANTIC_ADMIT` makes a decision consequential only where held-out evidence qualifies
it. A `contradicted` decision under this policy fails the obligation only when all of
these hold:
- the family qualified on held-out data (recovery-state, held-out v2: precision 1.000,
  Wilson lower bound 0.960, recall 0.948, under the gate it keeps);
- the admitted state is `unknown`;
- `--semantic-blocking` is `on`, the default; `off` keeps every decision advisory.

Every other decision leaves the admitted state unchanged.

Select relevance uses the same policy (`RELEVANCE_DECISIONS`): an excerpt is hidden as
irrelevant only when the answer qualifies for irrelevant (0.885, 0.805); on development
data this hid 269 excerpts, none from critical files (argmax hid 358, 16 critical). No
relevant gate met the precision bar, so nothing is flagged relevant; every other answer
stays surfaced as insufficient, with the model's choice kept as `model_choice`.
Choice answers are valid when their probabilities sum to 1 within 0.005 per option (the
provider rounds to two decimals); Score sums stay strict.

Previous calibration (fixed 0.6/0.95 thresholds, superseded): 880 authored synthetic cases in five families, labels withheld from
two independent blind labelers (agreement 97.7% development, 95.7% held-out), split by
scenario group and frozen before any call. Thresholds were fitted on development only;
the held-out split was run once. Held-out: concern precision 0.933 (Wilson LB 0.890),
recall 0.886, insufficient falsely cleared 1/133, true supports retained 68/131. No
family meets the consequential blocking bar (precision ≥ 0.95 with LB ≥ 0.90, recall
≥ 0.85); consistency is closest (0.980, LB 0.897, recall 0.909). Semantic results are
therefore advisory, suitable for triage and for assessed-clear with abstention, and
never block or discharge obligations. Record:
`.codex-worktree/opus-work/calibration/CALIBRATION_RECORD.json`.
