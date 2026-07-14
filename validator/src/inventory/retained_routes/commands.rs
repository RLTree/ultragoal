use super::model::{PS_CLI, RouteSpec, spec};

const KIND: &str = "legacy-command-authority";

pub(super) const ROUTES: [RouteSpec; 6] = [
    spec(
        "command-argument-parser-authority-to-ps-cli",
        "LEGACY-COMMAND:validator/src/argument_parser/authority.rs",
        KIND,
        "validator/src/argument_parser/authority.rs",
        "7b5f6b3cb322f8523bebbb839d5cd152ac3c7158004a3b1e02b1e73feb5a5be7",
        &PS_CLI,
    ),
    spec(
        "command-argument-parser-help-request-to-ps-cli",
        "LEGACY-COMMAND:validator/src/argument_parser/help_request.rs",
        KIND,
        "validator/src/argument_parser/help_request.rs",
        "d65afba98d06646bca33d23658069ede5fd38eceddce2ed62584b042f0dd1937",
        &PS_CLI,
    ),
    spec(
        "command-argument-parser-root-to-ps-cli",
        "LEGACY-COMMAND:validator/src/argument_parser/mod.rs",
        KIND,
        "validator/src/argument_parser/mod.rs",
        "9af5bc7c96b9ff5e92d1f68095f6058e0018826a5e70324d907ae3c77e056ed7",
        &PS_CLI,
    ),
    spec(
        "command-argument-parser-specialized-to-ps-cli",
        "LEGACY-COMMAND:validator/src/argument_parser/specialized.rs",
        KIND,
        "validator/src/argument_parser/specialized.rs",
        "a4d02928f6938c747fc66be5ba6325abbe2bb3d8e745c4edfbf50f0e3ea0bab0",
        &PS_CLI,
    ),
    spec(
        "command-argument-parser-tests-to-ps-cli",
        "LEGACY-COMMAND:validator/src/argument_parser/tests.rs",
        KIND,
        "validator/src/argument_parser/tests.rs",
        "2f6f455a18902834db7a0b90d4dd5b272d2e7b38bfd8031447c1c6b82632cfdb",
        &PS_CLI,
    ),
    spec(
        "command-dispatcher-root-to-ps-cli",
        "LEGACY-COMMAND:validator/src/command/mod.rs",
        KIND,
        "validator/src/command/mod.rs",
        "84d68b3a59a38c438a10828d7214a84ef2c0154842424568e6cb5189121a9d4b",
        &PS_CLI,
    ),
];
