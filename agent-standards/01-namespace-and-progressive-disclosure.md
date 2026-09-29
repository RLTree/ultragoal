# Namespace and Progressive Disclosure

## Names expose responsibility

Paths, modules, functions, tests, commands, state roots, and identifiers should
name the product behavior or domain responsibility they own. Keep compatibility
vocabulary at its parser, schema, or external-contract boundary and route into
clear internal ownership.

Avoid generic mixed-responsibility buckets when a domain name is available.
Split a surface when current navigation, ownership, or testing shows an actual
boundary; do not impose universal line counts or naming taxonomies.

## Every active file has a current use

An active file should route, specify, implement, configure, validate, recover,
or document a current behavior. Git owns ordinary history. Retain an old file
in the working tree only for a named compatibility, migration, fixture,
recovery, legal, or irreproducible-evidence use, and keep it out of current
authority unless that use requires otherwise.

## Progressive disclosure

Always-loaded files are routers. Start from the current goal, active plan, code
owner, and one relevant standard or domain document. Load more only when a
concrete dependency, contradiction, failure, or decision requires it.

`ARCHITECTURE.md` maps code and state ownership. The active ExecPlan owns
program state. Specialized root documents own noninferable domain semantics.
Source, tests, schemas, and tools own deterministic behavior.

## Documentation freshness

Update a document when changed behavior makes that document materially false.
Regenerate generated documents through their named owner. Do not edit every
document, create a backlog row, or refresh a receipt merely because a nearby
source file changed.

When an affected document cannot be updated safely, record the exact stale
statement, owner, consequence, and claim ceiling in the active ExecPlan.

## Removal boundary

Delete only after current-reader analysis shows no live consumer or a migration
has moved that consumer. Do not confuse an obsolete name or stale vocabulary
with deletion evidence.
