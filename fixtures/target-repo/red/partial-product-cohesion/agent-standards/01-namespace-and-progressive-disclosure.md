# Namespace And Progressive Disclosure

## Namespace Law

The filesystem is an agent-facing interface. Directory structure, filenames,
CLI commands, and state roots must explain domain responsibility before a file
is opened.

- Paths answer "what does this do" by themselves. Avoid junk drawers such as
  `utils`, `helpers`, `misc`, and vague `common` directories for domain logic.
- Name surfaces by the operator or reader's domain task, not implementation
  accidents or historical shims.
- Prefer small, well-scoped files. Large files degrade context quality and get
  truncated in agent context.
- Repeated prefixes across more than two files usually mean a missing
  subdirectory with the prefix removed.
- Compatibility exceptions must name the external contract that makes the
  less-ideal name worth keeping.

## Codemap

`ARCHITECTURE.md` is the bird's-eye map of the repo. It answers where things
live, what boundaries exist, and what must not depend on what. It is a map, not
an encyclopedia.

- Keep it short enough that every contributor reads it.
- Name modules, types, commands, and state roots. Avoid link farms that rot.
- Call out absences and invariants, such as forbidden dependencies or
  restricted bind addresses.
- Update it when architecture changes, not as a retrospective apology.

## Progressive Disclosure

Context is scarce. `AGENTS.md` stays short and routes to detailed standards,
plans, specs, check entry points, and proof locations.

- A giant instruction file crowds out the task, the code, and the relevant
  docs. Use routed standards modules and specialized root docs instead of
  bloating one always-loaded file.
- When everything is important, nothing is. Put task-specific guidance in
  routed docs and make the routing obvious.
- The repo is the source of truth. Decisions, plans, debt, standards, and
  proof live in version control or in named generated artifacts.
- What an agent cannot discover and load in context effectively does not
  exist.
