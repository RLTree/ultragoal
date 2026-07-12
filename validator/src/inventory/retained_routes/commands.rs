use super::model::{PS_CLI, RouteSpec, spec};

const KIND: &str = "legacy-command-authority";

pub(super) const ROUTES: [RouteSpec; 6] = [
    spec(
        "command-argument-parser-authority-to-ps-cli",
        "LEGACY-COMMAND:validator/src/argument_parser/authority.rs",
        KIND,
        "validator/src/argument_parser/authority.rs",
        "e4a5c21667f27c779585f01faa57f2e5f8c435354e4e4ee4033237c1138ef109",
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
        "2c57b102a69a9704f5b36a5a5b7c6206222425040f326a1f917b5b24b5c2ecf0",
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
        "3761b5e9c2df5b2ca1100130e0ca3ba158fafa7ad1fac355b0a3a2801c1bb10b",
        &PS_CLI,
    ),
    spec(
        "command-dispatcher-root-to-ps-cli",
        "LEGACY-COMMAND:validator/src/command/mod.rs",
        KIND,
        "validator/src/command/mod.rs",
        "48199baa10e970396107664625e1a830688895d9cbfd45df571ecb6ba7f281e2",
        &PS_CLI,
    ),
];
