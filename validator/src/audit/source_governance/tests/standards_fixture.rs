use super::fixture::SourceRoot;

pub(super) fn seed(root: &SourceRoot) {
    let (json, tsv, audit, policy_shard, audit_shard) = standards_indexes();
    root.write("agent-standards/enforcement.json", &json);
    root.write("agent-standards/enforcement.tsv", &tsv);
    root.write("agent-standards/enforcement-audit.tsv", &audit);
    root.write("agent-standards/policy/fixture.json", &policy_shard);
    root.write("agent-standards/audit/fixture.json", &audit_shard);
    root.executable("scripts/project-agent-standards", "#!/bin/sh\nexit 0\n");
    root.write("templates/agent-standards/enforcement.json", &json);
    root.write("templates/agent-standards/enforcement.tsv", &tsv);
    root.write("templates/agent-standards/enforcement-audit.tsv", &audit);
    root.write("docs/generated/context.json", "{}\n");
    let policy_source = "agent-standards/policy/fixture.json";
    let audit_source = "agent-standards/audit/fixture.json";
    let mut rows = vec![
        tool_projection(
            "Cargo.lock",
            "cargo",
            "stable",
            &["Cargo.toml", "rust-toolchain.toml", "validator/Cargo.toml"],
            "cargo generate-lockfile --offline",
            b"# fixture lock\n",
        ),
        projection(
            "agent-standards/enforcement-audit.tsv",
            audit_source,
            audit.as_bytes(),
        ),
        projection(
            "agent-standards/enforcement.json",
            policy_source,
            json.as_bytes(),
        ),
        projection(
            "agent-standards/enforcement.tsv",
            policy_source,
            tsv.as_bytes(),
        ),
        projection(
            "templates/agent-standards/enforcement-audit.tsv",
            audit_source,
            audit.as_bytes(),
        ),
        projection(
            "templates/agent-standards/enforcement.json",
            policy_source,
            json.as_bytes(),
        ),
        projection(
            "templates/agent-standards/enforcement.tsv",
            policy_source,
            tsv.as_bytes(),
        ),
        tool_projection(
            "pnpm-lock.yaml",
            "pnpm",
            "10.13.1",
            &["package.json", "pnpm-workspace.yaml"],
            "pnpm install --lockfile-only --ignore-scripts",
            b"lockfileVersion: '9.0'\n",
        ),
        retained_context(),
    ];
    rows.sort_by(|left, right| left.output.cmp(&right.output));
    let definitions = rows
        .iter()
        .map(|row| row.definition.as_str())
        .collect::<Vec<_>>()
        .join(",\n    ");
    let aggregates = rows
        .iter()
        .map(|row| row.aggregate.as_str())
        .collect::<Vec<_>>()
        .join(",\n    ");
    root.write(
        "migration/generated-surface-authority/fixture.json",
        &format!(
            "{{\n  \"schema_version\": \"GeneratedSurfaceAuthorityShard-v1\",\n  \"contract_id\": \"harness-ultragoal-successor-contract-v2\",\n  \"surfaces\": [\n    {definitions}\n  ]\n}}\n"
        ),
    );
    root.write(
        "migration/generated-surface-authority.json",
        &format!(
            "{{\n  \"schema_version\": \"GeneratedSurfaceAuthority-v3\",\n  \"contract_id\": \"harness-ultragoal-successor-contract-v2\",\n  \"registry_projection\": {{\n    \"generator\": \"scripts/project-generated-authority\",\n    \"canonical_sources\": [\n      \"migration/generated-surface-authority/fixture.json\"\n    ],\n    \"regeneration_command\": \"scripts/project-generated-authority write\"\n  }},\n  \"surfaces\": [\n    {aggregates}\n  ]\n}}\n"
        ),
    );
}

struct ProjectionRows {
    output: String,
    definition: String,
    aggregate: String,
}

fn projection(output: &str, source: &str, bytes: &[u8]) -> ProjectionRows {
    let digest = crate::digest::bytes(bytes).replace("sha256:", "");
    let definition = format!(
        "{{\n      \"disposition\": \"source_projection\",\n      \"output\": \"{output}\",\n      \"generator\": \"scripts/project-agent-standards\",\n      \"canonical_sources\": [\n        \"{source}\"\n      ],\n      \"regeneration_command\": \"scripts/project-agent-standards write\"\n    }}"
    );
    ProjectionRows {
        output: output.to_string(),
        definition,
        aggregate: format!(
            "{{\n      \"disposition\": \"source_projection\",\n      \"output\": \"{output}\",\n      \"generator\": \"scripts/project-agent-standards\",\n      \"canonical_sources\": [\n        \"{source}\"\n      ],\n      \"regeneration_command\": \"scripts/project-agent-standards write\",\n      \"output_sha256\": \"{digest}\"\n    }}"
        ),
    }
}

fn tool_projection(
    output: &str,
    tool: &str,
    version: &str,
    sources: &[&str],
    command: &str,
    bytes: &[u8],
) -> ProjectionRows {
    let digest = crate::digest::bytes(bytes).replace("sha256:", "");
    let sources = sources
        .iter()
        .map(|source| format!("        \"{source}\""))
        .collect::<Vec<_>>()
        .join(",\n");
    let definition = format!(
        "{{\n      \"disposition\": \"tool_projection\",\n      \"output\": \"{output}\",\n      \"tool\": \"{tool}\",\n      \"tool_version\": \"{version}\",\n      \"canonical_sources\": [\n{sources}\n      ],\n      \"regeneration_command\": \"{command}\"\n    }}"
    );
    ProjectionRows {
        output: output.to_string(),
        aggregate: format!(
            "{{\n      \"disposition\": \"tool_projection\",\n      \"output\": \"{output}\",\n      \"tool\": \"{tool}\",\n      \"tool_version\": \"{version}\",\n      \"canonical_sources\": [\n{sources}\n      ],\n      \"regeneration_command\": \"{command}\",\n      \"output_sha256\": \"{digest}\"\n    }}"
        ),
        definition,
    }
}

fn retained_context() -> ProjectionRows {
    let digest = crate::digest::bytes(b"{}\n").replace("sha256:", "");
    let row = format!(
        "{{\n      \"disposition\": \"retained_context\",\n      \"output\": \"docs/generated/context.json\",\n      \"sha256\": \"{digest}\",\n      \"reason\": \"fixture retained context is immutable and non-authoritative\",\n      \"replacement_targets\": [\n        \"HCT-CLAIMS\"\n      ],\n      \"preserve\": true,\n      \"physical_deletion_authorized\": false\n    }}"
    );
    ProjectionRows {
        output: "docs/generated/context.json".to_string(),
        definition: row.clone(),
        aggregate: row,
    }
}

fn standards_indexes() -> (String, String, String, String, String) {
    let mut ids = crate::audit::agent::standards::ids::REQUIRED_IDS.to_vec();
    ids.sort_unstable();
    let mut json_rows = Vec::new();
    let mut audit_rows = Vec::new();
    let mut tsv = String::from(
        "id\tsource_law\trequired_behavior\tenforcement_status\tgate_or_fixture_path\towner_lane\tclaim_ids_affected\tcurrent_status\tblocker_or_repair_action\trequired_execplan_refs\n",
    );
    let mut audit = String::from(
        "standard_id\taudit_status\tevidence_path\tevidence_digest\taudited_at\tclaim_ceiling_impact\n",
    );
    let evidence = crate::digest::bytes(b"pub fn product() {}\n");
    for id in ids {
        json_rows.push(format!(
            "{{\n      \"id\": \"{id}\",\n      \"source_law\": \"fixture law {id}\",\n      \"required_behavior\": \"fixture behavior {id}\",\n      \"enforcement_status\": \"mechanized\",\n      \"gate_or_fixture_path\": \"validator/src/lib.rs\",\n      \"owner_lane\": \"fixture\",\n      \"claim_ids_affected\": \"completion\",\n      \"current_status\": \"active\",\n      \"blocker_or_repair_action\": \"repair fixture {id}\",\n      \"required_execplan_refs\": [\n        \"plan.md\"\n      ]\n    }}"
        ));
        tsv.push_str(&format!("{id}\tfixture law {id}\tfixture behavior {id}\tmechanized\tvalidator/src/lib.rs\tfixture\tcompletion\tactive\trepair fixture {id}\tplan.md\n"));
        audit.push_str(&format!(
            "{id}\tpass\tvalidator/src/lib.rs\t{evidence}\t2026-07-13T00:00:00Z\tcompletion\n"
        ));
        audit_rows.push(format!(
            "{{\n      \"standard_id\": \"{id}\",\n      \"audit_status\": \"pass\",\n      \"evidence_path\": \"validator/src/lib.rs\",\n      \"evidence_digest\": \"{evidence}\",\n      \"audited_at\": \"2026-07-13T00:00:00Z\",\n      \"claim_ceiling_impact\": \"completion\"\n    }}"
        ));
    }
    let json = format!(
        "{{\n  \"schema\": \"harness-ultragoal.agent-standards-enforcement.v1\",\n  \"rows\": [\n    {}\n  ]\n}}\n",
        json_rows.join(",\n    ")
    );
    let policy_shard = format!(
        "{{\n  \"schema\": \"harness-ultragoal.agent-standards-policy-shard.v1\",\n  \"rows\": [\n    {}\n  ]\n}}\n",
        json_rows.join(",\n    ")
    );
    let audit_shard = format!(
        "{{\n  \"schema\": \"harness-ultragoal.agent-standards-audit-shard.v1\",\n  \"rows\": [\n    {}\n  ]\n}}\n",
        audit_rows.join(",\n    ")
    );
    (json, tsv, audit, policy_shard, audit_shard)
}
