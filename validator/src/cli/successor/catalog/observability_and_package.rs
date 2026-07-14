use super::super::command_contract::{
    CommandDescriptor, ObserveAction, PackageAction, SuccessorCommand,
};
use super::options::{
    EXPORT_OPTIONS, FILTER_OPTION, INPUT_OPTION, INPUT_OUTPUT, OUTPUT_OPTION, PUBLISH_OPTIONS,
    descriptor,
};
use crate::context::EffectClass;

pub(super) const COMMANDS: &[CommandDescriptor] = &[
    descriptor(
        SuccessorCommand::Observe(ObserveAction::Query),
        Some("query"),
        EffectClass::Read,
        "Query local semantic events without access-metadata writes.",
        FILTER_OPTION,
    ),
    descriptor(
        SuccessorCommand::Observe(ObserveAction::Export),
        Some("export"),
        EffectClass::ExternalWrite,
        "Export explicitly approved semantic data to an external destination.",
        EXPORT_OPTIONS,
    ),
    descriptor(
        SuccessorCommand::Package(PackageAction::Inventory),
        Some("inventory"),
        EffectClass::WorkspaceWrite,
        "Write a deterministic package inventory from canonical source.",
        OUTPUT_OPTION,
    ),
    descriptor(
        SuccessorCommand::Package(PackageAction::Build),
        Some("build"),
        EffectClass::WorkspaceWrite,
        "Build deterministic package bytes and local provenance inputs.",
        OUTPUT_OPTION,
    ),
    descriptor(
        SuccessorCommand::Package(PackageAction::Verify),
        Some("verify"),
        EffectClass::Read,
        "Verify package bytes and inventory without mutation.",
        INPUT_OPTION,
    ),
    descriptor(
        SuccessorCommand::Package(PackageAction::InstallTest),
        Some("install-test"),
        EffectClass::WorkspaceWrite,
        "Exercise an isolated install and write a local verification artifact.",
        INPUT_OUTPUT,
    ),
    descriptor(
        SuccessorCommand::Package(PackageAction::Publish),
        Some("publish"),
        EffectClass::ExternalWrite,
        "Publish only an explicitly approved package through a named provider.",
        PUBLISH_OPTIONS,
    ),
];
