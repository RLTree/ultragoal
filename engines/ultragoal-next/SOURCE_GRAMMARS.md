# Bounded source structure

The Rust, Python and ECMAScript fact producers retain the existing line facts and
add structure derived from tokens, delimiter trees and Python indentation suites.
They do not establish complete language validity, type correctness, import
resolution, macro expansion or runtime behavior. Named native syntax adapters
remain separate producers.

| Surface | Implemented envelope | Verification |
| --- | --- | --- |
| Lexing | Quoted strings, escaped quotes, Python triple quotes, comments, UTF-8 byte offsets, LF/CRLF, Python backslash-LF continuation | Exact source-slice and line assertions, strings/comments containing fake declarations |
| Delimiters | Nested parentheses, brackets and braces, maximum depth 64 | Matched, mismatched, missing and excessive delimiters |
| Python suites | Spaces-only indentation, nested suites, blank/comment lines and dedents; maximum suite depth 64 | Nested definitions, missing bodies, inconsistent dedents, tabs and depth boundaries |
| Named headers | Basic named functions/generators, parameter groups, body/suite spans, Rust signatures, classes and basic Rust type bodies | Function/class names and exact spans; numeric names refused |
| Python imports | Dotted targets, aliases, comma lists, relative from-import targets and parenthesized imported-name groups | Targets compared separately from imported names; from-clauses are consumed once |
| ECMAScript imports | Literal side-effect imports, from-specifiers, re-export specifiers; dynamic imports identified separately | Exact literal spans; dynamic resolution remains unknown |
| Rust token constructs | Macro definitions/invocations and attributes treated as opaque token trees | Fake function headers inside them are not emitted; expansion/attribute limitations retained |

The lexical driver has a 200,000-step bound. Helper scans consume finite input;
source size is bounded by the enclosing input transport. Module paths and header
suffixes have separate explicit bounds. Unsupported forms produce
`structure-limitation`, including raw Rust literals/identifiers, unsupported
lifetimes, Python interpolated literals, non-ASCII identifiers, and ECMAScript
template, regex/division, angle/JSX/generic disambiguation. Type annotations,
generics, expressions and statements are not generally validated. Names are
identifier-shaped words; native reserved-word and contextual validity remain
outside this producer's claim. Absence of a limitation does not mean that every
construct or dependency was analyzed.

Every successful structure parse explicitly reports dependency closure as
unknown. Extracted static import syntax must not be used as a certificate that
all relevant files, packages, configuration, generated inputs or runtime
dependencies were captured.

Facts carry exact byte spans or explicit line-zero metadata. Index reports use
`source-spans/2`. Check reports for these producers use `mixed-spans/1`: existing
line quotes and name quotes are retained, while parameter/body spans have null
values and require fresh source rehydration. Metadata retains its derived value.
The selector validates all bounds, UTF-8 boundaries and file digests. Nested
spans cannot shorten a retained excerpt or consume another window slot when
already covered. Old inline reports remain supported.

Run `bend tests/SourceProbe.bend -o target/release/source-probe` and then
`python3 tests/source_structure.py` for the focused executable parser tests.
The full CLI context/index regressions verify the serialization and consumer
boundary. These tests are not a full grammar or dependency-graph proof.
