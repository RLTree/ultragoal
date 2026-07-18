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
        "74c10369674766de18369c919da85927109a6daefee7c392a9967dbed2c91726",
        &PS_CLI,
    ),
    spec(
        "command-argument-parser-root-to-ps-cli",
        "LEGACY-COMMAND:validator/src/argument_parser/mod.rs",
        KIND,
        "validator/src/argument_parser/mod.rs",
        "dcdcd25aea9a437b98c97a014bef0c7594ff5a263c179085127333efd06b4c75",
        &PS_CLI,
    ),
    spec(
        "command-argument-parser-specialized-to-ps-cli",
        "LEGACY-COMMAND:validator/src/argument_parser/specialized.rs",
        KIND,
        "validator/src/argument_parser/specialized.rs",
        "473c50f32c87994e127a3991ed1eb6d3883342a1b873452c5df38435b3e6d133",
        &PS_CLI,
    ),
    spec(
        "command-argument-parser-tests-to-ps-cli",
        "LEGACY-COMMAND:validator/src/argument_parser/tests.rs",
        KIND,
        "validator/src/argument_parser/tests.rs",
        "81ab34eec18f5acf33965a0a2ec6be0ba875ea1f6aa619184a1e41002f2a27ef",
        &PS_CLI,
    ),
    spec(
        "command-dispatcher-root-to-ps-cli",
        "LEGACY-COMMAND:validator/src/command/mod.rs",
        KIND,
        "validator/src/command/mod.rs",
        "58488e6a47a60d1f4956f0a5bfc675bdf57ab2715f6ba2214af30764d4a500dd",
        &PS_CLI,
    ),
];
