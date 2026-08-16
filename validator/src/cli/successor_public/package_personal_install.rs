use super::*;
use crate::cli::successor::command_contract::{OptionName, PackageAction, ParsedValue};
use crate::distribution::{
    ReadOnlyWorkspace, capture_product_package, capture_product_package_with_cli,
    verify_product_package,
};
use crate::plugin_product::agent_discovery::{
    HostPluginRegistryObservation, capture_installed_source_authority,
};
use crate::plugin_product::lifecycle::Version;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::Path;

const PACKAGE_LIMIT: usize = 65 * 1024 * 1024;
const PLAN_LIMIT: u64 = 1024 * 1024;
const MARKETPLACE: &str = "local-harness-plugins";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct InstalledAuthority {
    schema_version: String,
    plugin_version: String,
    registry_observation_sha256: String,
    marketplace_source_relative_path_sha256: String,
    marketplace_source_catalog_sha256: String,
    cache_catalog_sha256: String,
    marketplace_runtime_sha256: String,
    cache_runtime_sha256: String,
    selected_codex_identity_sha256: String,
    home_identity_sha256: String,
    installed_authority_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct TargetPackage {
    candidate_id: String,
    catalog_id: String,
    version: String,
    source_tree_sha256: String,
    archive_sha256: String,
    inventory_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct PersonalMarketplaceInstallPlan {
    schema_version: String,
    plan_sha256: String,
    effect: String,
    marketplace: String,
    lifecycle_intent: String,
    input: String,
    cli: String,
    before: InstalledAuthority,
    target: TargetPackage,
    apply_status: String,
    required_effects: Vec<String>,
    required_reconciliation: Vec<String>,
    rollback: Vec<String>,
    claim_ceiling: String,
}

#[derive(Serialize)]
struct PlanBinding<'a> {
    schema_version: &'a str,
    effect: &'a str,
    marketplace: &'a str,
    lifecycle_intent: &'a str,
    input: &'a str,
    cli: &'a str,
    before: &'a InstalledAuthority,
    target: &'a TargetPackage,
    apply_status: &'a str,
    required_effects: &'a [String],
    required_reconciliation: &'a [String],
    rollback: &'a [String],
    claim_ceiling: &'a str,
}

pub(super) fn plan(
    source_context: &LiveContext,
    observation_context: &LiveContext,
    invocation: &ParsedInvocation,
    home: Option<&Path>,
) -> RuntimeOutcome {
    let Some((input, cli)) = plan_arguments(invocation) else {
        return invalid_plan_invocation();
    };
    let Some(home) = home else {
        return plan_failure("personal home authority is unavailable");
    };
    match build_plan(source_context, observation_context, home, input, cli) {
        Ok(record) => match serde_json::to_vec(&record) {
            Ok(bytes) if public_output_allowed(bytes.len()) => RuntimeOutcome::payload(
                ExitClass::Success,
                bytes,
                format!(
                    "personal marketplace install plan {} effect=none",
                    record.plan_sha256
                ),
            ),
            _ => plan_failure("personal marketplace install plan encoding failed"),
        },
        Err(cause) => plan_failure(cause),
    }
}

pub(super) fn apply(
    source_context: &LiveContext,
    observation_context: &LiveContext,
    invocation: &ParsedInvocation,
    home: Option<&Path>,
) -> RuntimeOutcome {
    let Some((plan_path, accepted_plan)) = apply_arguments(invocation) else {
        return invalid_apply_invocation();
    };
    let Some(home) = home else {
        return apply_failure("personal home authority is unavailable");
    };
    if source_context.revalidate().is_err() || observation_context.revalidate().is_err() {
        return apply_failure("candidate context changed before plan admission");
    }
    let bytes = match super::fit::external_plan_file::read_immutable_plan(plan_path, PLAN_LIMIT) {
        Ok(bytes) => bytes,
        Err(_) => return apply_failure("accepted plan file is not one immutable owner-only input"),
    };
    let canonical = if bytes.ends_with(b"\n") && !bytes[..bytes.len() - 1].ends_with(b"\n") {
        &bytes[..bytes.len() - 1]
    } else {
        bytes.as_slice()
    };
    let record: PersonalMarketplaceInstallPlan = match serde_json::from_slice(canonical) {
        Ok(record) => record,
        Err(_) => return apply_failure("accepted plan does not match the closed plan schema"),
    };
    if record.plan_sha256 != accepted_plan
        || record.effect != "none"
        || record.apply_status != "hold-prior-installed-authority-adoption-not-implemented"
        || validate_plan_identity(&record).is_err()
    {
        return apply_failure("accepted plan identity or HOLD boundary was substituted");
    }
    let current = match build_plan(
        source_context,
        observation_context,
        home,
        &record.input,
        &record.cli,
    ) {
        Ok(current) => current,
        Err(_) => return apply_failure("installed or candidate authority changed after planning"),
    };
    if current != record
        || source_context.revalidate().is_err()
        || observation_context.revalidate().is_err()
    {
        return apply_failure("installed or candidate authority changed after planning");
    }
    RuntimeOutcome::failure(
        ExitClass::UnsupportedCapability,
        Diagnostic::new(
            DiagnosticId::DownstreamToolUnavailable,
            ExitClass::UnsupportedCapability,
            DiagnosticDetails {
                cause: "the accepted exact plan is current, but the existing lifecycle executor cannot represent the observed prior installed authority without relabeling it as the target package",
                affected_surface: "HCT-DISTRIBUTION personal marketplace install apply",
                repair: "add a typed prior-installed-authority adoption record to the lifecycle state model, then bind materialization, Codex add, reconciliation, and rollback to that record",
                effect: "none",
                rerun: "ultragoal --json package install-apply --plan <absolute-plan-path> --accept-plan <sha256>",
                ceiling: "plan admission passed; marketplace, plugin registry, cache, runtime, and host installation remain unchanged",
            },
        ),
    )
}

fn build_plan(
    source_context: &LiveContext,
    observation_context: &LiveContext,
    home: &Path,
    input: &str,
    cli: &str,
) -> Result<PersonalMarketplaceInstallPlan, &'static str> {
    if !super::package_dispatch::package_archive_input_allowed(input)
        || !super::package_cli_payload::allowed(cli)
    {
        return Err("package or candidate CLI input is outside the bounded package surface");
    }
    let catalog = InventoryBuilder::new(source_context)
        .build()
        .map_err(|_| "authority catalog is unavailable")?;
    let source = capture_product_package(source_context, &catalog)
        .map_err(|_| "current-source package capture failed")?;
    let payload =
        super::package_cli_payload::from_read_only(source_context, cli, source.candidate_id())
            .ok_or("candidate CLI payload is unavailable or not native")?;
    let artifact = capture_product_package_with_cli(source_context, &catalog, payload)
        .map_err(|_| "current-source package capture failed")?;
    verify_product_package(&artifact, source_context, &catalog)
        .map_err(|_| "current-source package verification failed")?;
    let archive = ReadOnlyWorkspace::open(source_context)
        .and_then(|workspace| workspace.inspect_file(input, PACKAGE_LIMIT))
        .map_err(|_| "package archive input is unavailable")?
        .ok_or("package archive input is unavailable")?;
    if archive != artifact.snapshot().archive() {
        return Err("input archive does not match the exact current-source package");
    }
    let registry = super::capabilities::observe_unpinned_host_registry(observation_context)
        .map_err(|_| "current installed plugin registry observation is unavailable")?;
    let before = installed_authority(home, &registry)?;
    let target_version = artifact.snapshot().identity().source().version();
    let before_version = Version::parse(&before.plugin_version)
        .map_err(|_| "installed plugin version is invalid")?;
    let target_version_parsed =
        Version::parse(target_version).map_err(|_| "target plugin version is invalid")?;
    if target_version_parsed <= before_version {
        return Err("target package is not a monotonic installed-version successor");
    }
    let target = TargetPackage {
        candidate_id: artifact.candidate_id().to_owned(),
        catalog_id: artifact.catalog_id().to_owned(),
        version: target_version.to_owned(),
        source_tree_sha256: artifact.snapshot().source_tree_sha256().to_owned(),
        archive_sha256: artifact.snapshot().package_sha256().to_owned(),
        inventory_sha256: artifact.snapshot().inventory_sha256().to_owned(),
    };
    let required_effects = vec![
        "atomically materialize exact target bytes into the observed personal marketplace source"
            .to_owned(),
        "execute the pinned Codex executable: plugin add harness-ultragoal@local-harness-plugins"
            .to_owned(),
    ];
    let required_reconciliation = vec![
        "reobserve the exact installed registry row and personal marketplace root".to_owned(),
        "verify installed source, cache, and runtime bytes equal the target package".to_owned(),
        "prove idempotent replay produces no second materialization or install effect".to_owned(),
    ];
    let rollback = vec![
        "restore the exact pre-plan marketplace source tree".to_owned(),
        "reinstall and reobserve the exact prior installed authority".to_owned(),
        "surface recovery-required if either restoration or reconciliation is ambiguous".to_owned(),
    ];
    let binding = PlanBinding {
        schema_version: "HarnessPersonalMarketplaceInstallPlan-v1",
        effect: "none",
        marketplace: MARKETPLACE,
        lifecycle_intent: "monotonic-update",
        input,
        cli,
        before: &before,
        target: &target,
        apply_status: "hold-prior-installed-authority-adoption-not-implemented",
        required_effects: &required_effects,
        required_reconciliation: &required_reconciliation,
        rollback: &rollback,
        claim_ceiling: "read-only exact install plan; no marketplace, registry, cache, runtime, or host mutation authorized",
    };
    let plan_sha256 = digest_json(&binding)?;
    drop(binding);
    let record = PersonalMarketplaceInstallPlan {
        schema_version: "HarnessPersonalMarketplaceInstallPlan-v1".to_owned(),
        plan_sha256,
        effect: "none".to_owned(),
        marketplace: MARKETPLACE.to_owned(),
        lifecycle_intent: "monotonic-update".to_owned(),
        input: input.to_owned(),
        cli: cli.to_owned(),
        before,
        target,
        apply_status: "hold-prior-installed-authority-adoption-not-implemented".to_owned(),
        required_effects,
        required_reconciliation,
        rollback,
        claim_ceiling: "read-only exact install plan; no marketplace, registry, cache, runtime, or host mutation authorized".to_owned(),
    };
    validate_plan_identity(&record)?;
    Ok(record)
}

fn installed_authority(
    home: &Path,
    registry: &HostPluginRegistryObservation,
) -> Result<InstalledAuthority, &'static str> {
    let canonical_home = home
        .canonicalize()
        .map_err(|_| "personal home authority is unavailable")?;
    let marketplace_root = registry.marketplace_root();
    if marketplace_root != canonical_home {
        return Err("observed personal marketplace root does not equal the canonical home root");
    }
    let relative = registry
        .installed_root()
        .strip_prefix(marketplace_root)
        .ok()
        .and_then(Path::to_str)
        .ok_or("installed source is outside the observed marketplace root")?;
    if !relative.starts_with(".codex/local-marketplaces/")
        || !relative.ends_with("/plugins/harness-ultragoal")
        || Path::new(relative).is_absolute()
    {
        return Err("installed source path is not the canonical Harness marketplace member");
    }
    let session_id = digest_bytes(
        format!(
            "personal-installed-authority-session-v1\0{}\0{}",
            canonical_home.display(),
            registry.selected_codex_identity_sha256()
        )
        .as_bytes(),
    );
    let source = capture_installed_source_authority(
        registry.installed_root(),
        registry.sha256(),
        &session_id,
    )
    .map_err(|_| "installed marketplace source authority could not be captured")?;
    let cache_path = canonical_home.join(format!(
        ".codex/plugins/cache/{MARKETPLACE}/harness-ultragoal/{}",
        registry.plugin_version()
    ));
    let cache = capture_installed_source_authority(&cache_path, registry.sha256(), &session_id)
        .map_err(|_| "installed cache authority could not be captured")?;
    if source.plugin_version() != registry.plugin_version()
        || cache.plugin_version() != registry.plugin_version()
    {
        return Err("installed source, cache, and registry versions diverge");
    }
    let source_runtime = source.runtime_sha256().to_owned();
    let cache_runtime = cache.runtime_sha256().to_owned();
    if source_runtime != cache_runtime {
        return Err("installed marketplace and cache runtime bytes diverge");
    }
    source
        .revalidate()
        .map_err(|_| "installed marketplace source changed during capture")?;
    cache
        .revalidate()
        .map_err(|_| "installed cache changed during capture")?;
    let mut authority = InstalledAuthority {
        schema_version: "HarnessObservedInstalledAuthority-v1".to_owned(),
        plugin_version: registry.plugin_version().to_owned(),
        registry_observation_sha256: registry.sha256().to_owned(),
        marketplace_source_relative_path_sha256: digest_bytes(
            format!("personal-marketplace-source-v1\0{relative}").as_bytes(),
        ),
        marketplace_source_catalog_sha256: source.catalog_sha256().to_owned(),
        cache_catalog_sha256: cache.catalog_sha256().to_owned(),
        marketplace_runtime_sha256: source_runtime,
        cache_runtime_sha256: cache_runtime,
        selected_codex_identity_sha256: registry.selected_codex_identity_sha256().to_owned(),
        home_identity_sha256: digest_bytes(
            format!("personal-home-v1\0{}", canonical_home.display()).as_bytes(),
        ),
        installed_authority_sha256: String::new(),
    };
    authority.installed_authority_sha256 = installed_authority_digest(&authority)?;
    Ok(authority)
}

fn installed_authority_digest(authority: &InstalledAuthority) -> Result<String, &'static str> {
    #[derive(Serialize)]
    struct Binding<'a> {
        schema_version: &'a str,
        plugin_version: &'a str,
        registry_observation_sha256: &'a str,
        marketplace_source_relative_path_sha256: &'a str,
        marketplace_source_catalog_sha256: &'a str,
        cache_catalog_sha256: &'a str,
        marketplace_runtime_sha256: &'a str,
        cache_runtime_sha256: &'a str,
        selected_codex_identity_sha256: &'a str,
        home_identity_sha256: &'a str,
    }
    digest_json(&Binding {
        schema_version: &authority.schema_version,
        plugin_version: &authority.plugin_version,
        registry_observation_sha256: &authority.registry_observation_sha256,
        marketplace_source_relative_path_sha256: &authority.marketplace_source_relative_path_sha256,
        marketplace_source_catalog_sha256: &authority.marketplace_source_catalog_sha256,
        cache_catalog_sha256: &authority.cache_catalog_sha256,
        marketplace_runtime_sha256: &authority.marketplace_runtime_sha256,
        cache_runtime_sha256: &authority.cache_runtime_sha256,
        selected_codex_identity_sha256: &authority.selected_codex_identity_sha256,
        home_identity_sha256: &authority.home_identity_sha256,
    })
}

fn validate_plan_identity(record: &PersonalMarketplaceInstallPlan) -> Result<(), &'static str> {
    if record.schema_version != "HarnessPersonalMarketplaceInstallPlan-v1"
        || record.marketplace != MARKETPLACE
        || record.lifecycle_intent != "monotonic-update"
        || record.before.installed_authority_sha256 != installed_authority_digest(&record.before)?
    {
        return Err("plan contract changed");
    }
    let binding = PlanBinding {
        schema_version: "HarnessPersonalMarketplaceInstallPlan-v1",
        effect: &record.effect,
        marketplace: &record.marketplace,
        lifecycle_intent: &record.lifecycle_intent,
        input: &record.input,
        cli: &record.cli,
        before: &record.before,
        target: &record.target,
        apply_status: &record.apply_status,
        required_effects: &record.required_effects,
        required_reconciliation: &record.required_reconciliation,
        rollback: &record.rollback,
        claim_ceiling: &record.claim_ceiling,
    };
    (record.plan_sha256 == digest_json(&binding)?)
        .then_some(())
        .ok_or("plan identity changed")
}

fn plan_arguments(invocation: &ParsedInvocation) -> Option<(&str, &str)> {
    let ParsedInvocation {
        command: SuccessorCommand::Package(PackageAction::InstallPlan),
        effect: EffectClass::Read,
        arguments,
        ..
    } = invocation
    else {
        return None;
    };
    if arguments.len() != 2 {
        return None;
    }
    let input = arguments.iter().find(|row| row.name == OptionName::Input)?;
    let cli = arguments.iter().find(|row| row.name == OptionName::Cli)?;
    match (&input.value, &cli.value) {
        (ParsedValue::RelativePath(input), ParsedValue::RelativePath(cli)) => {
            Some((input.as_str(), cli.as_str()))
        }
        _ => None,
    }
}

fn apply_arguments(invocation: &ParsedInvocation) -> Option<(&Path, &str)> {
    let ParsedInvocation {
        command: SuccessorCommand::Package(PackageAction::InstallApply),
        effect: EffectClass::ExternalWrite,
        arguments,
        ..
    } = invocation
    else {
        return None;
    };
    if arguments.len() != 2 {
        return None;
    }
    let plan = arguments.iter().find(|row| row.name == OptionName::Plan)?;
    let accepted = arguments
        .iter()
        .find(|row| row.name == OptionName::AcceptPlan)?;
    match (&plan.value, &accepted.value) {
        (ParsedValue::HostPath(plan), ParsedValue::Identifier(accepted)) => {
            Some((plan.as_path(), accepted.as_str()))
        }
        _ => None,
    }
}

fn digest_json(value: &impl Serialize) -> Result<String, &'static str> {
    serde_json::to_vec(value)
        .map(|bytes| digest_bytes(&bytes))
        .map_err(|_| "canonical identity encoding failed")
}

fn digest_bytes(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

fn plan_failure(cause: &'static str) -> RuntimeOutcome {
    failure(cause, "read", "package install-plan")
}

fn apply_failure(cause: &'static str) -> RuntimeOutcome {
    failure(cause, "none", "package install-apply")
}

fn failure(cause: &'static str, effect: &'static str, command: &'static str) -> RuntimeOutcome {
    RuntimeOutcome::failure(
        ExitClass::ActionableFinding,
        Diagnostic::new(
            DiagnosticId::StaleContext,
            ExitClass::ActionableFinding,
            DiagnosticDetails {
                cause,
                affected_surface: "HCT-DISTRIBUTION personal marketplace lifecycle",
                repair: "restore exact package, installed-registry, marketplace, cache, runtime, and selected-Codex custody before retrying",
                effect,
                rerun: command,
                ceiling: "no marketplace, plugin registry, cache, runtime, or host installation effect occurred",
            },
        ),
    )
}

fn invalid_plan_invocation() -> RuntimeOutcome {
    plan_failure("package install-plan requires exact --input and --cli paths")
}

fn invalid_apply_invocation() -> RuntimeOutcome {
    apply_failure(
        "package install-apply requires one immutable --plan and exact --accept-plan identity",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::successor::parse_args;

    fn digest(byte: char) -> String {
        format!("sha256:{}", byte.to_string().repeat(64))
    }

    fn record() -> PersonalMarketplaceInstallPlan {
        let mut before = InstalledAuthority {
            schema_version: "HarnessObservedInstalledAuthority-v1".to_owned(),
            plugin_version: "0.0.35".to_owned(),
            registry_observation_sha256: digest('a'),
            marketplace_source_relative_path_sha256: digest('0'),
            marketplace_source_catalog_sha256: digest('b'),
            cache_catalog_sha256: digest('c'),
            marketplace_runtime_sha256: digest('d'),
            cache_runtime_sha256: digest('d'),
            selected_codex_identity_sha256: digest('e'),
            home_identity_sha256: digest('f'),
            installed_authority_sha256: String::new(),
        };
        before.installed_authority_sha256 = installed_authority_digest(&before).unwrap();
        let mut record = PersonalMarketplaceInstallPlan {
            schema_version: "HarnessPersonalMarketplaceInstallPlan-v1".to_owned(),
            plan_sha256: String::new(),
            effect: "none".to_owned(),
            marketplace: MARKETPLACE.to_owned(),
            lifecycle_intent: "monotonic-update".to_owned(),
            input: "target/ultragoal/package-0.0.36.hugpkg".to_owned(),
            cli: "target/ultragoal/release/ultragoal".to_owned(),
            before,
            target: TargetPackage {
                candidate_id: digest('1'),
                catalog_id: digest('2'),
                version: "0.0.36".to_owned(),
                source_tree_sha256: digest('3'),
                archive_sha256: digest('4'),
                inventory_sha256: digest('5'),
            },
            apply_status: "hold-prior-installed-authority-adoption-not-implemented".to_owned(),
            required_effects: vec!["materialize".to_owned(), "install".to_owned()],
            required_reconciliation: vec!["registry".to_owned(), "cache".to_owned()],
            rollback: vec!["restore".to_owned()],
            claim_ceiling: "read-only exact install plan; no marketplace, registry, cache, runtime, or host mutation authorized".to_owned(),
        };
        let binding = PlanBinding {
            schema_version: &record.schema_version,
            effect: &record.effect,
            marketplace: &record.marketplace,
            lifecycle_intent: &record.lifecycle_intent,
            input: &record.input,
            cli: &record.cli,
            before: &record.before,
            target: &record.target,
            apply_status: &record.apply_status,
            required_effects: &record.required_effects,
            required_reconciliation: &record.required_reconciliation,
            rollback: &record.rollback,
            claim_ceiling: &record.claim_ceiling,
        };
        record.plan_sha256 = digest_json(&binding).unwrap();
        record
    }

    #[test]
    fn exact_plan_identity_rejects_every_authority_or_effect_relabel() {
        let original = record();
        assert!(validate_plan_identity(&original).is_ok());
        let mut effect = original.clone();
        effect.effect = "external-write".to_owned();
        assert!(validate_plan_identity(&effect).is_err());
        let mut authority = original.clone();
        authority.before.cache_catalog_sha256 = digest('9');
        assert!(validate_plan_identity(&authority).is_err());
        let mut target = original.clone();
        target.target.version = "0.0.35".to_owned();
        assert!(validate_plan_identity(&target).is_err());
        let mut sequence = original;
        sequence.required_effects.swap(0, 1);
        assert!(validate_plan_identity(&sequence).is_err());
    }

    #[test]
    fn prior_authority_digest_is_independent_from_target_package_identity() {
        let original = record();
        assert_ne!(
            original.before.installed_authority_sha256,
            original.target.archive_sha256
        );
        let mut relabeled = original.before.clone();
        relabeled.installed_authority_sha256 = original.target.archive_sha256;
        assert_ne!(
            installed_authority_digest(&relabeled).unwrap(),
            relabeled.installed_authority_sha256
        );
    }

    #[test]
    fn public_grammar_separates_read_only_plan_from_external_apply_acceptance() {
        let plan = parse_args([
            "--json",
            "package",
            "install-plan",
            "--input",
            "target/ultragoal/package-0.0.36.hugpkg",
            "--cli",
            "target/ultragoal/release/ultragoal",
        ])
        .unwrap();
        let ParseOutcome::Invocation(plan) = plan else {
            panic!("expected install plan invocation")
        };
        assert_eq!(plan.effect, EffectClass::Read);
        let apply = parse_args([
            "--json",
            "package",
            "install-apply",
            "--plan",
            "/private/tmp/hul-install-plan.json",
            "--accept-plan",
            "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        ])
        .unwrap();
        let ParseOutcome::Invocation(apply) = apply else {
            panic!("expected install apply invocation")
        };
        assert_eq!(apply.effect, EffectClass::ExternalWrite);
    }
}
