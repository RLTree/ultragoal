use super::model::{PS_CLI, RouteSpec, spec};

const KIND: &str = "legacy-command-authority";

pub(super) const ROUTES: [RouteSpec; 2] = [
    spec(
        "command-argument-parser-public-arguments-to-ps-cli",
        "LEGACY-COMMAND:validator/src/argument_parser/public_arguments.rs",
        KIND,
        "validator/src/argument_parser/public_arguments.rs",
        "54cabbaa8a0f68e92c1eab095a62a5655c704052697c9d3c9c9e17bb86d054a9",
        &PS_CLI,
    ),
    spec(
        "command-argument-parser-root-to-ps-cli",
        "LEGACY-COMMAND:validator/src/argument_parser/mod.rs",
        KIND,
        "validator/src/argument_parser/mod.rs",
        "abdd64e6f2f3ab70b7e236b49b8888c0f6f2bab38812df7620f907b962f9a66d",
        &PS_CLI,
    ),
];
