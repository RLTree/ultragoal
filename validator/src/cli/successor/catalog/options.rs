use super::super::command_contract::{
    CommandDescriptor, OptionName, OptionSpec, SuccessorCommand, ValueKind,
};
use crate::context::EffectClass;

const TARGET: OptionSpec = option(OptionName::Target, ValueKind::RelativePath, false);
const PLAN: OptionSpec = option(OptionName::Plan, ValueKind::RelativePath, true);
const ACCEPT_PLAN: OptionSpec = option(OptionName::AcceptPlan, ValueKind::Identifier, true);
const CLAIM: OptionSpec = option(OptionName::Claim, ValueKind::Identifier, true);
const FINDING: OptionSpec = option(OptionName::Finding, ValueKind::Identifier, false);
const FILTER: OptionSpec = option(OptionName::Filter, ValueKind::Identifier, false);
const OUTPUT: OptionSpec = option(OptionName::Output, ValueKind::RelativePath, true);
const APPROVE_EXPORT: OptionSpec = option(OptionName::ApproveExport, ValueKind::Flag, true);
const PROVIDER: OptionSpec = option(OptionName::Provider, ValueKind::Identifier, true);
const APPROVE_PUBLISH: OptionSpec = option(OptionName::ApprovePublish, ValueKind::Flag, true);
const SPEC: OptionSpec = option(OptionName::Spec, ValueKind::RelativePath, true);
const INPUT: OptionSpec = option(OptionName::Input, ValueKind::RelativePath, true);
const CANDIDATE: OptionSpec = option(OptionName::Candidate, ValueKind::Identifier, true);
const REGISTRY: OptionSpec = option(OptionName::Registry, ValueKind::RelativePath, false);
const APPROVE_RETIREMENT: OptionSpec = option(OptionName::ApproveRetirement, ValueKind::Flag, true);
const PACKAGE_ROOT: OptionSpec = option(OptionName::PackageRoot, ValueKind::HostPath, false);

pub(super) const TARGET_OPTION: &[OptionSpec] = &[TARGET];
pub(super) const FIT_APPLY: &[OptionSpec] = &[TARGET, PLAN, ACCEPT_PLAN];
pub(super) const STRICT_OPTIONS: &[OptionSpec] = &[TARGET, CLAIM];
pub(super) const FINDING_OPTION: &[OptionSpec] = &[FINDING];
pub(super) const CLAIM_OUTPUT: &[OptionSpec] = &[CLAIM, OUTPUT];
pub(super) const FILTER_OPTION: &[OptionSpec] = &[FILTER];
pub(super) const EXPORT_OPTIONS: &[OptionSpec] = &[OUTPUT, APPROVE_EXPORT];
pub(super) const OUTPUT_OPTION: &[OptionSpec] = &[OUTPUT];
pub(super) const INPUT_OPTION: &[OptionSpec] = &[INPUT];
pub(super) const PUBLISH_OPTIONS: &[OptionSpec] = &[INPUT, PROVIDER, APPROVE_PUBLISH];
pub(super) const SPEC_OPTION: &[OptionSpec] = &[SPEC];
pub(super) const SPEC_OUTPUT: &[OptionSpec] = &[SPEC, OUTPUT];
pub(super) const INPUT_OUTPUT: &[OptionSpec] = &[INPUT, OUTPUT];
pub(super) const CANDIDATE_OUTPUT: &[OptionSpec] = &[CANDIDATE, OUTPUT];
pub(super) const ADAPTER_OPTIONS: &[OptionSpec] = &[SPEC, PROVIDER];
pub(super) const REGISTRY_OPTION: &[OptionSpec] = &[REGISTRY];
pub(super) const MIGRATE_APPLY: &[OptionSpec] = &[PLAN, ACCEPT_PLAN];
pub(super) const MIGRATE_RETIRE: &[OptionSpec] = &[PLAN, APPROVE_RETIREMENT];
pub(super) const CAPABILITIES_OPTION: &[OptionSpec] = &[PACKAGE_ROOT];

const fn option(name: OptionName, kind: ValueKind, required: bool) -> OptionSpec {
    OptionSpec {
        name,
        kind,
        required,
    }
}

pub(super) const fn descriptor(
    command: SuccessorCommand,
    subcommand: Option<&'static str>,
    effect: EffectClass,
    purpose: &'static str,
    options: &'static [OptionSpec],
) -> CommandDescriptor {
    CommandDescriptor {
        command,
        subcommand,
        effect,
        purpose,
        options,
    }
}
