pub(super) const LOWER_DIGEST: &str =
    "0000000000000000000000000000000000000000000000000000000000000000";

pub(super) struct SourceProjectionSample<'a> {
    pub(super) schema: &'a str,
    pub(super) registry_sources: &'a str,
    pub(super) output: &'a str,
    pub(super) generator: &'a str,
    pub(super) sources: &'a str,
    pub(super) command: &'a str,
    pub(super) digest: &'a str,
    pub(super) extra_surface_field: &'a str,
    pub(super) trailing: &'a str,
}

impl Default for SourceProjectionSample<'_> {
    fn default() -> Self {
        Self {
            schema: "GeneratedSurfaceAuthority-v3",
            registry_sources: "\"migration/generated-surface-authority/source.json\"",
            output: "generated/output.json",
            generator: "scripts/project-output",
            sources: "\"source/input.json\"",
            command: "scripts/project-output write",
            digest: LOWER_DIGEST,
            extra_surface_field: "",
            trailing: "",
        }
    }
}

impl SourceProjectionSample<'_> {
    pub(super) fn bytes(&self) -> Vec<u8> {
        format!(
            concat!(
                "{{\"schema_version\":\"{}\",",
                "\"contract_id\":\"harness-ultragoal-successor-contract-v2\",",
                "\"registry_projection\":{{",
                "\"generator\":\"scripts/project-generated-authority\",",
                "\"canonical_sources\":[{}],",
                "\"regeneration_command\":\"scripts/project-generated-authority write\"}},",
                "\"surfaces\":[{{\"disposition\":\"source_projection\",",
                "\"output\":\"{}\",\"generator\":\"{}\",",
                "\"canonical_sources\":[{}],\"regeneration_command\":\"{}\",",
                "\"output_sha256\":\"{}\"{}}}]}}{}"
            ),
            self.schema,
            self.registry_sources,
            self.output,
            self.generator,
            self.sources,
            self.command,
            self.digest,
            self.extra_surface_field,
            self.trailing,
        )
        .into_bytes()
    }
}
