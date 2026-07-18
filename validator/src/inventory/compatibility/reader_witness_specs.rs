#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ArtifactSpec {
    pub path: &'static str,
    pub sha256: &'static str,
}

pub(crate) const ACCEPTED_CONTEXT_ARTIFACTS: [ArtifactSpec; 19] = [
    ArtifactSpec {
        path: "schemas/codex-registry-exposure.schema.json",
        sha256: "c6f814afae2631fa02f693d6e9785115dda7331ed5a7a7e7be94955f1e7e4173",
    },
    ArtifactSpec {
        path: "validator/src/audit/plugin/registry/live/mod.rs",
        sha256: "7ab5996f8b9b22d83f21a107fde4772511542dae0a52fc92f291e87c6fbdbf51",
    },
    ArtifactSpec {
        path: "validator/src/audit/plugin/registry/live/raw.rs",
        sha256: "a0b93154ae043a22a3d7f287c2e2c4663e2cf128bcdb419ef1ea118d8dfc0a6f",
    },
    ArtifactSpec {
        path: "validator/src/audit/plugin/registry/live/reviewers.rs",
        sha256: "e05afc06f83658c43ce502573b4af38ac72adcafc1656ca9697a79c43c21647b",
    },
    ArtifactSpec {
        path: "validator/src/claim_semantics/coverage/receipt/exclusions.rs",
        sha256: "3ac6ecd75a072db612af32f89000257f5723f8beb242af8f2319b17eb09febf6",
    },
    ArtifactSpec {
        path: "validator/src/claim_semantics/plugin_policy/mod.rs",
        sha256: "0402169f674c6c026613107c2364f2f54089730e70a744556ab4a4a4d0b67e9b",
    },
    ArtifactSpec {
        path: "validator/src/claim_semantics/plugin_policy/tests.rs",
        sha256: "ffe717cbb8d1fb2d9de5f6121c27e7d1d8a78a08c0c9342fcc74c97f7a3de23f",
    },
    ArtifactSpec {
        path: "validator/src/cli/control/plane/registry/mod.rs",
        sha256: "eb2f6c7341c665bf35f2f1ffbcdf3972899b17ff19941907b3b02d84314fcf4c",
    },
    ArtifactSpec {
        path: "validator/src/cli/control/plane/registry/agent_rows.rs",
        sha256: "8a850630845f3d6cb4944dff0467937e653f017547356e3c6f21c62bd9e55fa1",
    },
    ArtifactSpec {
        path: "validator/src/cli/live_loop/surfaces/input_spec/path_rules.rs",
        sha256: "7880797e1832bd81c850b7ef61e5029710e61efe3b5fca6b37d45651afd08df7",
    },
    ArtifactSpec {
        path: "validator/src/schema_catalog/fixture_schema_rules.rs",
        sha256: "ba00909a97a944cf8f33b555cfb8c7f15c5a91a98bd0e201bfc5f667bbd1260e",
    },
    ArtifactSpec {
        path: "validator/src/schema_catalog/schema/patterns.rs",
        sha256: "03f579dd5c5c8107c28ed6650063e170e120dbf0799295ca7232589c03e80743",
    },
    ArtifactSpec {
        path: "validator/src/self_tests/claim/plugin/policy.rs",
        sha256: "a968480da4f951344338a16c71cbeb92b82c26adf106c4677c70875bd63949df",
    },
    ArtifactSpec {
        path: "validator/src/self_tests/cli/control/registry.rs",
        sha256: "904e019aa7adca8cf8ccb7863d2a92a5ec0854d5f4d3c4321492662710035fe8",
    },
    ArtifactSpec {
        path: "validator/src/self_tests/command/round_receipts.rs",
        sha256: "7c6997dffa60c0eb7141ae3e9de8f38ea2eee17909f5746429ae1716ed050043",
    },
    ArtifactSpec {
        path: "validator/src/self_tests/plugin/registry/fixture.rs",
        sha256: "ed85a08e23c9b242b7ee9d8a9535edeee05fb56b32b3834161059eab52b8dc67",
    },
    ArtifactSpec {
        path: "validator/src/self_tests/plugin/registry/guard.rs",
        sha256: "f46b39c6472e1d1cb8dce9d4c03276f6b4066812b1865f158c1c60487799a4da",
    },
    ArtifactSpec {
        path: "validator/src/self_tests/plugin/registry/mod.rs",
        sha256: "17c0ac91315b48eaeb5c74aed34f8cfa931c2a21dc71ceeddb0f7164af45118b",
    },
    ArtifactSpec {
        path: "validator/src/self_tests/schema/rules.rs",
        sha256: "a0cda442c0f33b9a4f4dab4b67ac4367d601f5e383efc7b6a3272c3f0cbb6023",
    },
];
