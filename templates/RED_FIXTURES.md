# Red Fixtures

Canonical fixture identity is schema-owned by `schemas/common-defs.schema.json#/$defs/requiredRedFixtureId`; catalog shape is owned by `schemas/red-fixtures-catalog.schema.json`.

`templates/RED_FIXTURES.json` records expected packet paths, digests, expected errors, and expected failing checks. Current validator disposition lives only in generated `validation_artifacts/ultragoal-audit/red-fixture-report.json`. `fixtures/red/*.json` are compact executable JSON Patch specs against their declared valid base fixture.

Do not restate the required fixture list in prose. Regenerate this explanation from schemas if the list changes.
