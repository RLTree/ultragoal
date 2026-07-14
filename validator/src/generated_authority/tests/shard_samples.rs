pub(super) struct SourceDefinitionSample<'a> {
    pub(super) schema: &'a str,
    pub(super) output: &'a str,
    pub(super) generator: &'a str,
    pub(super) sources: &'a str,
    pub(super) command: &'a str,
    pub(super) extra_field: &'a str,
    pub(super) trailing: &'a str,
}

impl Default for SourceDefinitionSample<'_> {
    fn default() -> Self {
        Self {
            schema: "GeneratedSurfaceAuthorityShard-v1",
            output: "generated/output.json",
            generator: "scripts/project-output",
            sources: "\"source/input.json\"",
            command: "scripts/project-output write",
            extra_field: "",
            trailing: "",
        }
    }
}

impl SourceDefinitionSample<'_> {
    pub(super) fn bytes(&self) -> Vec<u8> {
        format!(
            concat!(
                "{{\"schema_version\":\"{}\",",
                "\"contract_id\":\"harness-ultragoal-successor-contract-v2\",",
                "\"surfaces\":[{{\"disposition\":\"source_projection\",",
                "\"output\":\"{}\",\"generator\":\"{}\",",
                "\"canonical_sources\":[{}],",
                "\"regeneration_command\":\"{}\"{}}}]}}{}"
            ),
            self.schema,
            self.output,
            self.generator,
            self.sources,
            self.command,
            self.extra_field,
            self.trailing,
        )
        .into_bytes()
    }
}
