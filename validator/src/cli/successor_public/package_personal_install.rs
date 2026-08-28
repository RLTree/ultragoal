use super::*;
use crate::cli::successor::command_contract::{OptionName, PackageAction, ParsedValue};
use crate::distribution::{
    ConfinedRoot, PERSONAL_MARKETPLACE_SOURCE_RELATIVE, PackageIdentity,
    PersonalMarketplaceSourceObservation, PersonalMarketplaceUpdateAuthority,
    PersonalMarketplaceUpdateDisposition, PersonalMarketplaceUpdateStage, ReadOnlyTreeObservation,
    ReadOnlyWorkspace, SourceIdentity, capture_product_package, capture_product_package_with_cli,
    execute_personal_marketplace_update, verify_product_package,
};
use crate::plugin_product::agent_discovery::{
    HostPluginRegistryObservation, InstalledSourceAuthorityCapture,
    capture_installed_source_authority,
};
use crate::plugin_product::lifecycle::{
    LifecycleAuthorization, LifecycleIntent, LifecycleRequest, LifecycleState, PackageAuthority,
    PriorInstalledAuthority, Version,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::{self, File, OpenOptions};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

mod live_effects;

const PACKAGE_LIMIT: usize = 65 * 1024 * 1024;
const PLAN_LIMIT: u64 = 1024 * 1024;
const MARKETPLACE: &str = "local-harness-plugins";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct DirectoryAuthority {
    device: u64,
    inode: u64,
    owner: u32,
    mode: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct PersonalHomeAuthorityRecord {
    schema_version: String,
    canonical_path_sha256: String,
    home: DirectoryAuthority,
    codex_home: DirectoryAuthority,
    authority_sha256: String,
}

/// One retained authority for the exact canonical personal home and its
/// `.codex` object. Pathnames are reporting/admission inputs only; retained
/// descriptors and `ConfinedRoot` own every filesystem effect.
struct PersonalHomeAuthority {
    canonical: PathBuf,
    home: File,
    codex_home: File,
    root: ConfinedRoot,
    record: PersonalHomeAuthorityRecord,
}

impl PersonalHomeAuthority {
    fn capture(requested: &Path) -> Result<Self, &'static str> {
        let canonical = requested
            .canonicalize()
            .map_err(|_| "personal home authority is unavailable")?;
        if canonical != requested || !canonical.is_absolute() {
            return Err("personal home authority changed");
        }
        let named = fs::symlink_metadata(&canonical)
            .map_err(|_| "personal home authority is unavailable")?;
        if named.file_type().is_symlink() || !named.is_dir() {
            return Err("personal home authority is not one directory");
        }
        let mut options = OpenOptions::new();
        options
            .read(true)
            .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_DIRECTORY);
        let home = options
            .open(&canonical)
            .map_err(|_| "personal home authority descriptor is unavailable")?;
        let opened = home
            .metadata()
            .map_err(|_| "personal home authority descriptor is unavailable")?;
        if directory_authority(&named)? != directory_authority(&opened)? {
            return Err("personal home authority changed during capture");
        }
        let codex_descriptor = unsafe {
            libc::openat(
                home.as_raw_fd(),
                c".codex".as_ptr(),
                libc::O_RDONLY | libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_DIRECTORY,
            )
        };
        if codex_descriptor < 0 {
            return Err("personal Codex home authority is unavailable");
        }
        let codex_home = unsafe { File::from_raw_fd(codex_descriptor) };
        let codex_named = stat_codex_home(home.as_raw_fd())?;
        let codex_opened = codex_home
            .metadata()
            .map_err(|_| "personal Codex home authority descriptor is unavailable")?;
        let home_authority = directory_authority(&opened)?;
        let codex_authority = directory_authority(&codex_opened)?;
        if codex_named != codex_authority {
            return Err("personal Codex home authority changed during capture");
        }
        let home_id = digest_bytes(
            format!(
                "{}\0{}\0{}",
                canonical.display(),
                home_authority.device,
                home_authority.inode
            )
            .as_bytes(),
        );
        let root = ConfinedRoot::open_personal_home(&canonical, &home_id)
            .map_err(|_| "personal home confinement failed")?;
        let mut record = PersonalHomeAuthorityRecord {
            schema_version: "HarnessPersonalHomeAuthority-v1".to_owned(),
            canonical_path_sha256: digest_bytes(
                format!("personal-home-path-v1\0{}", canonical.display()).as_bytes(),
            ),
            home: home_authority,
            codex_home: codex_authority,
            authority_sha256: String::new(),
        };
        record.authority_sha256 = personal_home_authority_digest(&record)?;
        let authority = Self {
            canonical,
            home,
            codex_home,
            root,
            record,
        };
        authority.revalidate()?;
        Ok(authority)
    }

    fn revalidate(&self) -> Result<(), &'static str> {
        self.root
            .revalidate()
            .map_err(|_| "personal home authority changed")?;
        let named = fs::symlink_metadata(&self.canonical)
            .map_err(|_| "personal home authority is unavailable")?;
        let opened = self
            .home
            .metadata()
            .map_err(|_| "personal home authority descriptor is unavailable")?;
        let codex_opened = self
            .codex_home
            .metadata()
            .map_err(|_| "personal Codex home authority descriptor is unavailable")?;
        if self.canonical.canonicalize().ok().as_deref() != Some(self.canonical.as_path())
            || directory_authority(&named)? != self.record.home
            || directory_authority(&opened)? != self.record.home
            || stat_codex_home(self.home.as_raw_fd())? != self.record.codex_home
            || directory_authority(&codex_opened)? != self.record.codex_home
            || self.record.authority_sha256 != personal_home_authority_digest(&self.record)?
        {
            return Err("personal home or Codex home authority changed");
        }
        Ok(())
    }

    fn path(&self) -> &Path {
        &self.canonical
    }

    fn root(&self) -> ConfinedRoot {
        self.root.clone()
    }

    fn record(&self) -> &PersonalHomeAuthorityRecord {
        &self.record
    }

    fn codex_home_fd(&self) -> std::os::fd::RawFd {
        self.codex_home.as_raw_fd()
    }
}

fn directory_authority(metadata: &fs::Metadata) -> Result<DirectoryAuthority, &'static str> {
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err("personal authority object is not one directory");
    }
    Ok(DirectoryAuthority {
        device: metadata.dev(),
        inode: metadata.ino(),
        owner: metadata.uid(),
        mode: metadata.mode(),
    })
}

fn stat_codex_home(home: std::os::fd::RawFd) -> Result<DirectoryAuthority, &'static str> {
    let mut value = std::mem::MaybeUninit::<libc::stat>::uninit();
    let result = unsafe {
        libc::fstatat(
            home,
            c".codex".as_ptr(),
            value.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    };
    if result != 0 {
        return Err("personal Codex home authority is unavailable");
    }
    let value = unsafe { value.assume_init() };
    if value.st_mode & libc::S_IFMT != libc::S_IFDIR {
        return Err("personal Codex home authority is not one directory");
    }
    Ok(DirectoryAuthority {
        device: value.st_dev as u64,
        inode: value.st_ino as u64,
        owner: value.st_uid,
        mode: value.st_mode as u32,
    })
}

fn personal_home_authority_digest(
    authority: &PersonalHomeAuthorityRecord,
) -> Result<String, &'static str> {
    #[derive(Serialize)]
    struct Binding<'a> {
        schema_version: &'a str,
        canonical_path_sha256: &'a str,
        home: &'a DirectoryAuthority,
        codex_home: &'a DirectoryAuthority,
    }
    digest_json(&Binding {
        schema_version: &authority.schema_version,
        canonical_path_sha256: &authority.canonical_path_sha256,
        home: &authority.home,
        codex_home: &authority.codex_home,
    })
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct InstalledAuthority {
    schema_version: String,
    plugin_version: String,
    registry_observation_sha256: String,
    marketplace_source_relative_path_sha256: String,
    marketplace_source_tree_sha256: String,
    marketplace_source_observation_sha256: String,
    marketplace_source_catalog_sha256: String,
    cache_catalog_sha256: String,
    cache_tree_sha256: String,
    marketplace_runtime_sha256: String,
    cache_runtime_sha256: String,
    selected_codex_identity_sha256: String,
    personal_home_authority_sha256: String,
    installed_authority_sha256: String,
}

struct InstalledAuthorityObservation {
    authority: InstalledAuthority,
    source: InstalledSourceAuthorityCapture,
    cache: InstalledSourceAuthorityCapture,
    source_tree: PersonalMarketplaceSourceObservation,
    cache_tree: ReadOnlyTreeObservation,
}

impl InstalledAuthorityObservation {
    fn revalidate(&self) -> Result<(), &'static str> {
        self.source
            .revalidate()
            .map_err(|_| "installed marketplace source changed during capture")?;
        self.cache
            .revalidate()
            .map_err(|_| "installed cache changed during capture")?;
        self.source_tree
            .revalidate()
            .map_err(|_| "installed marketplace source tree changed during capture")?;
        self.cache_tree
            .revalidate()
            .map_err(|_| "installed cache tree changed during capture")?;
        if self.cache_tree.tree_sha256() != self.authority.cache_tree_sha256 {
            return Err("installed cache tree changed during capture");
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct TargetPackage {
    context_id: String,
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
    personal_home: PersonalHomeAuthorityRecord,
    before: InstalledAuthority,
    target: TargetPackage,
    apply_status: String,
    required_effects: Vec<String>,
    required_reconciliation: Vec<String>,
    rollback: Vec<String>,
    claim_ceiling: String,
}

struct PreparedInstallPlan {
    record: PersonalMarketplaceInstallPlan,
    registry: HostPluginRegistryObservation,
    installed: InstalledAuthorityObservation,
    personal_home: PersonalHomeAuthority,
}

impl PreparedInstallPlan {
    fn revalidate(
        &self,
        source_context: &LiveContext,
        observation_context: &LiveContext,
    ) -> Result<(), &'static str> {
        source_context
            .revalidate()
            .map_err(|_| "candidate source changed before plan publication")?;
        observation_context
            .revalidate()
            .map_err(|_| "host observation context changed before plan publication")?;
        self.personal_home.revalidate()?;
        let registry = super::capabilities::observe_unpinned_host_registry(observation_context)
            .map_err(registry_observation_failure_cause)?;
        if registry != self.registry {
            return Err("installed registry changed before plan publication");
        }
        self.installed.revalidate()?;
        self.personal_home.revalidate()?;
        source_context
            .revalidate()
            .map_err(|_| "candidate source changed before plan publication")?;
        observation_context
            .revalidate()
            .map_err(|_| "host observation context changed before plan publication")
    }
}

#[derive(Serialize)]
struct PlanBinding<'a> {
    schema_version: &'a str,
    effect: &'a str,
    marketplace: &'a str,
    lifecycle_intent: &'a str,
    input: &'a str,
    cli: &'a str,
    personal_home: &'a PersonalHomeAuthorityRecord,
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
        Ok(prepared) => match serde_json::to_vec(&prepared.record) {
            Ok(bytes) if public_output_allowed(bytes.len()) => {
                if let Err(cause) = prepared.revalidate(source_context, observation_context) {
                    return plan_failure(cause);
                }
                RuntimeOutcome::payload(
                    ExitClass::Success,
                    bytes,
                    format!(
                        "personal marketplace install plan {} effect=none",
                        prepared.record.plan_sha256
                    ),
                )
            }
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
        || record.effect != "personal-marketplace-update"
        || record.apply_status != "ready-exact-live-personal-marketplace-adapter"
        || validate_plan_identity(&record).is_err()
    {
        return apply_failure("accepted plan identity or HOLD boundary was substituted");
    }
    let personal_home = match PersonalHomeAuthority::capture(home) {
        Ok(authority) if authority.record() == &record.personal_home => authority,
        _ => return apply_failure("accepted personal home authority changed after planning"),
    };
    // Do not require the host to still be exact-prior here. A fresh process
    // must be able to admit the one exact materialized-but-not-installed state
    // and drive the existing rollback transaction. The live adapter acquires
    // the exclusive lease, captures the required exact prior cache snapshot,
    // and compares the typed registry/source/cache/runtime observations before
    // any package or install effect.
    let (artifact, catalog) =
        match capture_accepted_candidate(source_context, &record.input, &record.cli) {
            Ok(candidate) => candidate,
            Err(_) => return apply_failure("candidate authority changed after planning"),
        };
    if target_package(&artifact) != record.target
        || source_context.revalidate().is_err()
        || observation_context.revalidate().is_err()
    {
        return apply_failure("installed or candidate authority changed after planning");
    }
    let (authority, prior) = match accepted_update_authority(&record, &target_package(&artifact)) {
        Ok(accepted) => accepted,
        Err(cause) => return apply_failure(cause),
    };
    let cancellation = crate::distribution::host_effect::HostEffectCancellation::default();
    let _signal_guard = match cancellation.install_process_signal_source() {
        Ok(guard) => guard,
        Err(cause) => return apply_failure(cause),
    };
    let mut effects = match live_effects::LivePersonalMarketplaceUpdateEffects::new(
        source_context,
        observation_context,
        personal_home,
        record.clone(),
        prior,
        artifact,
        catalog,
        cancellation,
    ) {
        Ok(effects) => effects,
        Err(cause) => return apply_failure(cause),
    };
    let report = execute_personal_marketplace_update(&authority, &mut effects);
    if let Err(cause) = effects.finish() {
        return match report.disposition {
            PersonalMarketplaceUpdateDisposition::Applied => apply_effect_failure(
                cause,
                "observed-exact-target; pinned-executable-finalization-failed",
                "preserve the observed target and reconcile the exact selected Codex executable before any retry",
            ),
            PersonalMarketplaceUpdateDisposition::RecoveredAfterFailure => {
                let (stage_cause, _) = recovered_stage_diagnostic(report.failed_stage);
                apply_effect_failure(
                    stage_cause,
                    "restored-exact-prior; pinned-executable-finalization-failed",
                    "preserve the restored prior state and reconcile the exact selected Codex executable before deriving a new plan",
                )
            }
            PersonalMarketplaceUpdateDisposition::RecoveryAmbiguous => {
                let (stage_cause, _) = ambiguous_stage_diagnostic(report.failed_stage);
                apply_effect_failure(
                    stage_cause,
                    "ambiguous-recovery-and-executable-finalization",
                    "reconcile every marketplace, registry, cache, runtime, and selected-Codex authority before any retry",
                )
            }
            PersonalMarketplaceUpdateDisposition::RefusedBeforeEffect => {
                let (stage_cause, _) = refused_stage_diagnostic(report.failed_stage);
                apply_failure(stage_cause)
            }
            PersonalMarketplaceUpdateDisposition::ReusedVerifiedTarget => apply_failure(cause),
        };
    }
    match report.disposition {
        PersonalMarketplaceUpdateDisposition::Applied => RuntimeOutcome::payload(
            ExitClass::Success,
            Vec::new(),
            format!(
                "personal marketplace install applied plan={} effect=marketplace-source+plugin-install",
                record.plan_sha256
            ),
        ),
        PersonalMarketplaceUpdateDisposition::ReusedVerifiedTarget => RuntimeOutcome::payload(
            ExitClass::Success,
            Vec::new(),
            format!(
                "personal marketplace install reused plan={} effect=none",
                record.plan_sha256
            ),
        ),
        PersonalMarketplaceUpdateDisposition::RecoveredAfterFailure => {
            let (cause, repair) = recovered_stage_diagnostic(report.failed_stage);
            apply_effect_failure(cause, "restored-exact-prior", repair)
        }
        PersonalMarketplaceUpdateDisposition::RefusedBeforeEffect => {
            let (cause, repair) = refused_stage_diagnostic(report.failed_stage);
            apply_effect_failure(cause, "none", repair)
        }
        PersonalMarketplaceUpdateDisposition::RecoveryAmbiguous => {
            let (cause, repair) = ambiguous_stage_diagnostic(report.failed_stage);
            apply_effect_failure(cause, "ambiguous-recovery-required", repair)
        }
    }
}

fn recovered_stage_diagnostic(
    stage: Option<PersonalMarketplaceUpdateStage>,
) -> (&'static str, &'static str) {
    match stage {
        Some(PersonalMarketplaceUpdateStage::InterruptedAfterMaterialization) => (
            "personal marketplace update recovered exact prior after stage=interrupted-after-materialization",
            "derive a fresh exact plan after verifying the restored source, registry, cache, runtime, and child-group lease are terminal",
        ),
        Some(PersonalMarketplaceUpdateStage::MaterializeTarget) => (
            "personal marketplace update recovered exact prior after stage=materialize-target",
            "derive a fresh exact plan after verifying source materialization custody and the restored predecessor",
        ),
        Some(PersonalMarketplaceUpdateStage::ReconcileMaterializedTarget) => (
            "personal marketplace update recovered exact prior after stage=reconcile-materialized-target",
            "derive a fresh exact plan only after the target-source observation path is repaired",
        ),
        Some(PersonalMarketplaceUpdateStage::InstallTarget) => (
            "personal marketplace update recovered exact prior after stage=install-target",
            "derive a fresh exact plan only after the pinned child execution or cancellation cause is resolved",
        ),
        Some(PersonalMarketplaceUpdateStage::ReconcileTarget) => (
            "personal marketplace update recovered exact prior after stage=reconcile-target",
            "derive a fresh exact plan only after target registry, cache, runtime, and source observation agree",
        ),
        _ => (
            "personal marketplace update recovered exact prior after stage=unknown-forward-stage",
            "preserve the restored prior state and independently reconcile the redacted stage before deriving a new plan",
        ),
    }
}

fn refused_stage_diagnostic(
    stage: Option<PersonalMarketplaceUpdateStage>,
) -> (&'static str, &'static str) {
    match stage {
        Some(PersonalMarketplaceUpdateStage::InitialObservation) => (
            "personal marketplace update refused before effect at stage=initial-observation",
            "restore the exact typed home, registry, source, cache-tree, runtime, plan, and selected-Codex observations before retrying",
        ),
        Some(PersonalMarketplaceUpdateStage::MaterializeTarget) => (
            "personal marketplace update refused before effect at stage=materialize-target",
            "clear the production cancellation source, then derive a fresh exact plan before retrying",
        ),
        _ => (
            "personal marketplace update refused before effect at stage=unknown-admission-stage",
            "restore exact plan and predecessor custody and rederive a fresh plan before retrying",
        ),
    }
}

fn ambiguous_stage_diagnostic(
    stage: Option<PersonalMarketplaceUpdateStage>,
) -> (&'static str, &'static str) {
    match stage {
        Some(PersonalMarketplaceUpdateStage::RestorePriorTree) => (
            "personal marketplace recovery is ambiguous at stage=restore-prior-tree",
            "preserve all state and reconcile the exact marketplace source tree before any child or install retry",
        ),
        Some(PersonalMarketplaceUpdateStage::ReinstallPrior) => (
            "personal marketplace recovery is ambiguous at stage=reinstall-prior",
            "preserve all state and reconcile the exact prior registry, cache, runtime, and selected-Codex child result before any retry",
        ),
        Some(PersonalMarketplaceUpdateStage::ReconcilePrior) => (
            "personal marketplace recovery is ambiguous at stage=reconcile-prior",
            "preserve all state and independently reobserve every exact prior authority before deriving another plan",
        ),
        _ => (
            "personal marketplace recovery is ambiguous at stage=unknown-recovery-stage",
            "preserve all state and reconcile every marketplace, registry, cache, runtime, home, and child-group authority before any retry",
        ),
    }
}

fn build_plan(
    source_context: &LiveContext,
    observation_context: &LiveContext,
    home: &Path,
    input: &str,
    cli: &str,
) -> Result<PreparedInstallPlan, &'static str> {
    let (artifact, _catalog) = capture_accepted_candidate(source_context, input, cli)?;
    let personal_home = PersonalHomeAuthority::capture(home)?;
    let registry = super::capabilities::observe_unpinned_host_registry(observation_context)
        .map_err(registry_observation_failure_cause)?;
    let installed = installed_authority(&personal_home, &registry)?;
    let before = installed.authority.clone();
    let target = target_package(&artifact);
    let before_version = Version::parse(&before.plugin_version)
        .map_err(|_| "installed plugin version is invalid")?;
    let target_version_parsed =
        Version::parse(&target.version).map_err(|_| "target plugin version is invalid")?;
    if target_version_parsed
        .precedence_cmp(&before_version)
        .map_err(|_| "installed or target plugin version is invalid")?
        != std::cmp::Ordering::Greater
    {
        return Err("target package is not a monotonic installed-version successor");
    }
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
        schema_version: "HarnessPersonalMarketplaceInstallPlan-v5",
        effect: "personal-marketplace-update",
        marketplace: MARKETPLACE,
        lifecycle_intent: "monotonic-update",
        input,
        cli,
        personal_home: personal_home.record(),
        before: &before,
        target: &target,
        apply_status: "ready-exact-live-personal-marketplace-adapter",
        required_effects: &required_effects,
        required_reconciliation: &required_reconciliation,
        rollback: &rollback,
        claim_ceiling: "read-only exact install plan; apply still requires this immutable plan identity and may update only the bound personal marketplace and Codex plugin layers",
    };
    let plan_sha256 = digest_json(&binding)?;
    drop(binding);
    let record = PersonalMarketplaceInstallPlan {
        schema_version: "HarnessPersonalMarketplaceInstallPlan-v5".to_owned(),
        plan_sha256,
        effect: "personal-marketplace-update".to_owned(),
        marketplace: MARKETPLACE.to_owned(),
        lifecycle_intent: "monotonic-update".to_owned(),
        input: input.to_owned(),
        cli: cli.to_owned(),
        personal_home: personal_home.record().clone(),
        before,
        target,
        apply_status: "ready-exact-live-personal-marketplace-adapter".to_owned(),
        required_effects,
        required_reconciliation,
        rollback,
        claim_ceiling: "read-only exact install plan; apply still requires this immutable plan identity and may update only the bound personal marketplace and Codex plugin layers".to_owned(),
    };
    validate_plan_identity(&record)?;
    Ok(PreparedInstallPlan {
        record,
        registry,
        installed,
        personal_home,
    })
}

fn capture_accepted_candidate(
    source_context: &LiveContext,
    input: &str,
    cli: &str,
) -> Result<
    (
        crate::distribution::ProductionPackageArtifact,
        crate::inventory::AuthorityCatalog,
    ),
    &'static str,
> {
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
    source_context
        .revalidate()
        .map_err(|_| "candidate source changed after package capture")?;
    Ok((artifact, catalog))
}

fn target_package(artifact: &crate::distribution::ProductionPackageArtifact) -> TargetPackage {
    TargetPackage {
        context_id: artifact
            .snapshot()
            .identity()
            .source()
            .context_id()
            .to_owned(),
        candidate_id: artifact.candidate_id().to_owned(),
        catalog_id: artifact.catalog_id().to_owned(),
        version: artifact.snapshot().identity().source().version().to_owned(),
        source_tree_sha256: artifact.snapshot().source_tree_sha256().to_owned(),
        archive_sha256: artifact.snapshot().package_sha256().to_owned(),
        inventory_sha256: artifact.snapshot().inventory_sha256().to_owned(),
    }
}

fn registry_observation_failure_cause(
    failure: super::capabilities::RegistryObservationFailure,
) -> &'static str {
    match failure {
        super::capabilities::RegistryObservationFailure::Unavailable(code) => code,
        super::capabilities::RegistryObservationFailure::Blocked(id) => id.code(),
    }
}

fn installed_authority(
    home: &PersonalHomeAuthority,
    registry: &HostPluginRegistryObservation,
) -> Result<InstalledAuthorityObservation, &'static str> {
    home.revalidate()?;
    let canonical_home = home.path();
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
    if relative != PERSONAL_MARKETPLACE_SOURCE_RELATIVE || Path::new(relative).is_absolute() {
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
    let cache_relative = format!(
        ".codex/plugins/cache/{MARKETPLACE}/harness-ultragoal/{}",
        registry.plugin_version()
    );
    let cache_path = canonical_home.join(&cache_relative);
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
    let source_tree =
        PersonalMarketplaceSourceObservation::capture(&canonical_home, registry.installed_root())
            .map_err(|_| "installed marketplace source tree could not be captured")?;
    let marketplace_source_tree_sha256 = source_tree.tree_sha256().to_owned();
    let marketplace_source_observation_sha256 = source_tree.observation_sha256().to_owned();
    let cache_tree = ReadOnlyTreeObservation::capture_root(&cache_path, 4096, PACKAGE_LIMIT)
        .map_err(|_| "installed cache tree could not be captured")?;
    let cache_tree_sha256 = cache_tree.tree_sha256().to_owned();
    if cache_tree_sha256 != marketplace_source_tree_sha256 {
        return Err("installed source and cache full trees diverge");
    }
    let mut authority = InstalledAuthority {
        schema_version: "HarnessObservedInstalledAuthority-v3".to_owned(),
        plugin_version: registry.plugin_version().to_owned(),
        registry_observation_sha256: registry.sha256().to_owned(),
        marketplace_source_relative_path_sha256: digest_bytes(
            format!("personal-marketplace-source-v1\0{relative}").as_bytes(),
        ),
        marketplace_source_tree_sha256,
        marketplace_source_observation_sha256,
        marketplace_source_catalog_sha256: source.catalog_sha256().to_owned(),
        cache_catalog_sha256: cache.catalog_sha256().to_owned(),
        cache_tree_sha256,
        marketplace_runtime_sha256: source_runtime,
        cache_runtime_sha256: cache_runtime,
        selected_codex_identity_sha256: registry.selected_codex_identity_sha256().to_owned(),
        personal_home_authority_sha256: home.record().authority_sha256.clone(),
        installed_authority_sha256: String::new(),
    };
    authority.installed_authority_sha256 = installed_authority_digest(&authority)?;
    let observation = InstalledAuthorityObservation {
        authority,
        source,
        cache,
        source_tree,
        cache_tree,
    };
    observation.revalidate()?;
    home.revalidate()?;
    Ok(observation)
}

fn installed_authority_digest(authority: &InstalledAuthority) -> Result<String, &'static str> {
    #[derive(Serialize)]
    struct Binding<'a> {
        schema_version: &'a str,
        plugin_version: &'a str,
        registry_observation_sha256: &'a str,
        marketplace_source_relative_path_sha256: &'a str,
        marketplace_source_tree_sha256: &'a str,
        marketplace_source_observation_sha256: &'a str,
        marketplace_source_catalog_sha256: &'a str,
        cache_catalog_sha256: &'a str,
        cache_tree_sha256: &'a str,
        marketplace_runtime_sha256: &'a str,
        cache_runtime_sha256: &'a str,
        selected_codex_identity_sha256: &'a str,
        personal_home_authority_sha256: &'a str,
    }
    digest_json(&Binding {
        schema_version: &authority.schema_version,
        plugin_version: &authority.plugin_version,
        registry_observation_sha256: &authority.registry_observation_sha256,
        marketplace_source_relative_path_sha256: &authority.marketplace_source_relative_path_sha256,
        marketplace_source_tree_sha256: &authority.marketplace_source_tree_sha256,
        marketplace_source_observation_sha256: &authority.marketplace_source_observation_sha256,
        marketplace_source_catalog_sha256: &authority.marketplace_source_catalog_sha256,
        cache_catalog_sha256: &authority.cache_catalog_sha256,
        cache_tree_sha256: &authority.cache_tree_sha256,
        marketplace_runtime_sha256: &authority.marketplace_runtime_sha256,
        cache_runtime_sha256: &authority.cache_runtime_sha256,
        selected_codex_identity_sha256: &authority.selected_codex_identity_sha256,
        personal_home_authority_sha256: &authority.personal_home_authority_sha256,
    })
}

fn adopted_lifecycle_plan(
    record: &PersonalMarketplaceInstallPlan,
) -> Result<
    (
        crate::plugin_product::lifecycle::LifecyclePlan,
        PackageIdentity,
        PriorInstalledAuthority,
    ),
    &'static str,
> {
    let prior_package_authority = PackageAuthority {
        version: Version::parse(&record.before.plugin_version)
            .map_err(|_| "prior installed version is invalid")?,
        package_sha256: record.before.installed_authority_sha256.clone(),
        inventory_sha256: record.before.marketplace_source_catalog_sha256.clone(),
        candidate_id: record.before.registry_observation_sha256.clone(),
    };
    let prior = PriorInstalledAuthority::new(
        prior_package_authority.clone(),
        record.before.installed_authority_sha256.clone(),
    )
    .map_err(|_| "prior installed authority is invalid")?;
    let target = PackageIdentity::new(
        SourceIdentity::new(
            record.target.context_id.clone(),
            record.target.candidate_id.clone(),
            "harness-ultragoal".to_owned(),
            record.target.version.clone(),
            record.target.catalog_id.clone(),
            record.target.inventory_sha256.clone(),
        )
        .map_err(|_| "target source identity is invalid")?,
        record.target.source_tree_sha256.clone(),
        record.target.archive_sha256.clone(),
    )
    .map_err(|_| "target package identity is invalid")?;
    let target_authority = PackageAuthority {
        version: Version::parse(target.source().version())
            .map_err(|_| "target package version is invalid")?,
        package_sha256: target.archive_sha256().to_owned(),
        inventory_sha256: target.source().accepted_inventory_sha256().to_owned(),
        candidate_id: target.source().candidate_id().to_owned(),
    };
    let before = LifecycleState {
        installed: Some(prior_package_authority.clone()),
        cache: Some(prior_package_authority),
        generation: 0,
        recovery_required: false,
    };
    let lifecycle = crate::plugin_product::lifecycle::plan(
        &before,
        &LifecycleRequest {
            intent: LifecycleIntent::MonotonicUpdate,
            target: Some(target_authority),
            prior_authority: None,
            authorization: LifecycleAuthorization {
                allow_host_write: true,
                allow_downgrade: false,
                expected_installed_sha256: Some(record.before.installed_authority_sha256.clone()),
            },
        },
    )
    .map_err(|_| "monotonic lifecycle adoption plan is invalid")?;
    Ok((lifecycle, target, prior))
}

fn accepted_update_authority(
    record: &PersonalMarketplaceInstallPlan,
    current_target: &TargetPackage,
) -> Result<(PersonalMarketplaceUpdateAuthority, PriorInstalledAuthority), &'static str> {
    if current_target != &record.target || validate_plan_identity(record).is_err() {
        return Err("accepted candidate authority changed after planning");
    }
    let (_, target, prior) = adopted_lifecycle_plan(record)
        .map_err(|_| "typed prior installed authority adoption failed")?;
    if target.archive_sha256() != current_target.archive_sha256
        || target.tree_sha256() != current_target.source_tree_sha256
    {
        return Err("accepted target package authority changed after planning");
    }
    let authority = PersonalMarketplaceUpdateAuthority::new(
        record.plan_sha256.clone(),
        prior.clone(),
        record.before.marketplace_source_tree_sha256.clone(),
        record.target.archive_sha256.clone(),
        record.target.source_tree_sha256.clone(),
    )
    .map_err(|_| "atomic marketplace update authority is invalid")?;
    Ok((authority, prior))
}

fn validate_plan_identity(record: &PersonalMarketplaceInstallPlan) -> Result<(), &'static str> {
    if record.schema_version != "HarnessPersonalMarketplaceInstallPlan-v5"
        || record.effect != "personal-marketplace-update"
        || record.personal_home.schema_version != "HarnessPersonalHomeAuthority-v1"
        || record.before.schema_version != "HarnessObservedInstalledAuthority-v3"
        || record.marketplace != MARKETPLACE
        || record.lifecycle_intent != "monotonic-update"
        || record.apply_status != "ready-exact-live-personal-marketplace-adapter"
        || record.personal_home.authority_sha256
            != personal_home_authority_digest(&record.personal_home)?
        || record.before.personal_home_authority_sha256 != record.personal_home.authority_sha256
        || record.before.cache_tree_sha256 != record.before.marketplace_source_tree_sha256
        || record.before.installed_authority_sha256 != installed_authority_digest(&record.before)?
    {
        return Err("plan contract changed");
    }
    let binding = PlanBinding {
        schema_version: "HarnessPersonalMarketplaceInstallPlan-v5",
        effect: &record.effect,
        marketplace: &record.marketplace,
        lifecycle_intent: &record.lifecycle_intent,
        input: &record.input,
        cli: &record.cli,
        personal_home: &record.personal_home,
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

fn apply_effect_failure(
    cause: &'static str,
    effect: &'static str,
    repair: &'static str,
) -> RuntimeOutcome {
    RuntimeOutcome::failure(
        ExitClass::ActionableFinding,
        Diagnostic::new(
            DiagnosticId::StaleContext,
            ExitClass::ActionableFinding,
            DiagnosticDetails {
                cause,
                affected_surface: "HCT-DISTRIBUTION personal marketplace lifecycle",
                repair,
                effect,
                rerun: "package install-plan then package install-apply",
                ceiling: "the accepted update did not prove an exact installed target; do not infer install success or repeat an effect until exact authority is rederived",
            },
        ),
    )
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
    #[cfg(unix)]
    use std::fs;
    #[cfg(unix)]
    use std::os::unix::fs::MetadataExt;
    #[cfg(unix)]
    use std::path::{Path, PathBuf};
    #[cfg(unix)]
    use std::sync::atomic::{AtomicU64, Ordering};

    const TARGET_VERSION: &str = "0.0.41+codex.20260824093100";
    #[cfg(unix)]
    const INSTALLED_VERSION: &str = "0.0.39+codex.20260820190706";
    #[cfg(unix)]
    static NEXT_PERSONAL_FIXTURE: AtomicU64 = AtomicU64::new(0);

    fn digest(byte: char) -> String {
        format!("sha256:{}", byte.to_string().repeat(64))
    }

    fn record() -> PersonalMarketplaceInstallPlan {
        let mut personal_home = PersonalHomeAuthorityRecord {
            schema_version: "HarnessPersonalHomeAuthority-v1".to_owned(),
            canonical_path_sha256: digest('f'),
            home: DirectoryAuthority {
                device: 1,
                inode: 2,
                owner: 501,
                mode: (libc::S_IFDIR | 0o700) as u32,
            },
            codex_home: DirectoryAuthority {
                device: 1,
                inode: 3,
                owner: 501,
                mode: (libc::S_IFDIR | 0o700) as u32,
            },
            authority_sha256: String::new(),
        };
        personal_home.authority_sha256 = personal_home_authority_digest(&personal_home).unwrap();
        let mut before = InstalledAuthority {
            schema_version: "HarnessObservedInstalledAuthority-v3".to_owned(),
            plugin_version: "0.0.39+codex.20260820190706".to_owned(),
            registry_observation_sha256: digest('a'),
            marketplace_source_relative_path_sha256: digest('0'),
            marketplace_source_tree_sha256: digest('6'),
            marketplace_source_observation_sha256: digest('7'),
            marketplace_source_catalog_sha256: digest('b'),
            cache_catalog_sha256: digest('c'),
            cache_tree_sha256: digest('6'),
            marketplace_runtime_sha256: digest('d'),
            cache_runtime_sha256: digest('d'),
            selected_codex_identity_sha256: digest('e'),
            personal_home_authority_sha256: personal_home.authority_sha256.clone(),
            installed_authority_sha256: String::new(),
        };
        before.installed_authority_sha256 = installed_authority_digest(&before).unwrap();
        let mut record = PersonalMarketplaceInstallPlan {
            schema_version: "HarnessPersonalMarketplaceInstallPlan-v5".to_owned(),
            plan_sha256: String::new(),
            effect: "personal-marketplace-update".to_owned(),
            marketplace: MARKETPLACE.to_owned(),
            lifecycle_intent: "monotonic-update".to_owned(),
            input: "target/ultragoal/package-0.0.41.hugpkg".to_owned(),
            cli: "target/ultragoal/release/ultragoal".to_owned(),
            personal_home,
            before,
            target: TargetPackage {
                context_id: digest('0'),
                candidate_id: digest('1'),
                catalog_id: digest('2'),
                version: TARGET_VERSION.to_owned(),
                source_tree_sha256: digest('3'),
                archive_sha256: digest('4'),
                inventory_sha256: digest('5'),
            },
            apply_status: "ready-exact-live-personal-marketplace-adapter".to_owned(),
            required_effects: vec!["materialize".to_owned(), "install".to_owned()],
            required_reconciliation: vec!["registry".to_owned(), "cache".to_owned()],
            rollback: vec!["restore".to_owned()],
            claim_ceiling: "read-only exact install plan; apply still requires this immutable plan identity and may update only the bound personal marketplace and Codex plugin layers".to_owned(),
        };
        rebind_plan(&mut record);
        record
    }

    fn rebind_plan(record: &mut PersonalMarketplaceInstallPlan) {
        let binding = PlanBinding {
            schema_version: &record.schema_version,
            effect: &record.effect,
            marketplace: &record.marketplace,
            lifecycle_intent: &record.lifecycle_intent,
            input: &record.input,
            cli: &record.cli,
            personal_home: &record.personal_home,
            before: &record.before,
            target: &record.target,
            apply_status: &record.apply_status,
            required_effects: &record.required_effects,
            required_reconciliation: &record.required_reconciliation,
            rollback: &record.rollback,
            claim_ceiling: &record.claim_ceiling,
        };
        record.plan_sha256 = digest_json(&binding).unwrap();
    }

    #[cfg(unix)]
    struct PersonalInstallFixture {
        root: PathBuf,
        home: PathBuf,
        installed: PathBuf,
        cache: PathBuf,
    }

    #[cfg(unix)]
    impl PersonalInstallFixture {
        fn new(marketplace_directory: &str) -> Self {
            let requested_root = std::env::temp_dir().join(format!(
                "hul-personal-install-plan-{}-{}",
                std::process::id(),
                NEXT_PERSONAL_FIXTURE.fetch_add(1, Ordering::Relaxed),
            ));
            fs::create_dir(&requested_root).unwrap();
            let root = requested_root.canonicalize().unwrap();
            let home = root.join("home");
            let installed = home.join(format!(
                ".codex/local-marketplaces/{marketplace_directory}/plugins/harness-ultragoal"
            ));
            let cache = home.join(format!(
                ".codex/plugins/cache/{MARKETPLACE}/harness-ultragoal/{INSTALLED_VERSION}"
            ));
            fs::create_dir_all(&home).unwrap();
            materialize_installed_fixture(&installed);
            materialize_installed_fixture(&cache);
            Self {
                root,
                home,
                installed,
                cache,
            }
        }

        fn registry(&self) -> HostPluginRegistryObservation {
            HostPluginRegistryObservation::fixture(&self.installed, &self.home, INSTALLED_VERSION)
        }
    }

    #[cfg(unix)]
    impl Drop for PersonalInstallFixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[cfg(unix)]
    fn materialize_installed_fixture(target: &Path) {
        let repository = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("validator has a repository parent");
        fs::create_dir_all(target.join(".codex/agents")).unwrap();
        for entry in fs::read_dir(repository.join(".codex/agents")).unwrap() {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_file() {
                fs::copy(
                    entry.path(),
                    target.join(".codex/agents").join(entry.file_name()),
                )
                .unwrap();
            }
        }
        fs::create_dir_all(target.join(".codex-plugin")).unwrap();
        let mut manifest: serde_json::Value = serde_json::from_slice(
            &fs::read(repository.join(".codex-plugin/plugin.json")).unwrap(),
        )
        .unwrap();
        manifest["version"] = serde_json::Value::String(INSTALLED_VERSION.to_owned());
        fs::write(
            target.join(".codex-plugin/plugin.json"),
            serde_json::to_vec_pretty(&manifest).unwrap(),
        )
        .unwrap();
        fs::create_dir_all(target.join("runtime")).unwrap();
        fs::write(target.join("runtime/ultragoal"), b"fixture-runtime").unwrap();
    }

    #[cfg(unix)]
    fn tree_snapshot(root: &Path) -> Vec<(String, String, u32, u64, Vec<u8>)> {
        fn visit(root: &Path, current: &Path, rows: &mut Vec<(String, String, u32, u64, Vec<u8>)>) {
            let mut entries = fs::read_dir(current)
                .unwrap()
                .map(Result::unwrap)
                .collect::<Vec<_>>();
            entries.sort_by_key(|entry| entry.file_name());
            for entry in entries {
                let path = entry.path();
                let metadata = fs::symlink_metadata(&path).unwrap();
                let relative = path
                    .strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .into_owned();
                if metadata.is_dir() {
                    rows.push((
                        relative,
                        "directory".to_owned(),
                        metadata.mode(),
                        metadata.nlink(),
                        Vec::new(),
                    ));
                    visit(root, &path, rows);
                } else if metadata.is_file() {
                    rows.push((
                        relative,
                        "regular".to_owned(),
                        metadata.mode(),
                        metadata.nlink(),
                        fs::read(path).unwrap(),
                    ));
                } else {
                    rows.push((
                        relative,
                        "special".to_owned(),
                        metadata.mode(),
                        metadata.nlink(),
                        Vec::new(),
                    ));
                }
            }
        }
        let mut rows = Vec::new();
        visit(root, root, &mut rows);
        rows
    }

    #[cfg(unix)]
    #[test]
    fn installed_authority_accepts_the_exact_live_personal_marketplace_shape_without_mutation() {
        let fixture = PersonalInstallFixture::new("harness-ultragoal-local");
        let before = tree_snapshot(&fixture.root);

        let home = PersonalHomeAuthority::capture(&fixture.home).unwrap();
        let observation = installed_authority(&home, &fixture.registry()).unwrap();

        assert_eq!(observation.authority.plugin_version, INSTALLED_VERSION);
        assert!(
            observation
                .authority
                .marketplace_source_tree_sha256
                .starts_with("sha256:")
        );
        assert!(
            observation
                .authority
                .marketplace_source_observation_sha256
                .starts_with("sha256:")
        );
        assert_ne!(
            observation.authority.marketplace_source_tree_sha256,
            observation.authority.marketplace_source_observation_sha256
        );
        observation.revalidate().unwrap();
        assert_eq!(tree_snapshot(&fixture.root), before);
    }

    #[cfg(unix)]
    #[test]
    fn installed_authority_observation_rejects_later_source_change() {
        let fixture = PersonalInstallFixture::new("harness-ultragoal-local");
        let home = PersonalHomeAuthority::capture(&fixture.home).unwrap();
        let observation = installed_authority(&home, &fixture.registry()).unwrap();

        fs::write(
            fixture.installed.join("runtime/ultragoal"),
            b"changed-runtime",
        )
        .unwrap();

        assert_eq!(
            observation.revalidate().unwrap_err(),
            "installed marketplace source changed during capture"
        );
    }

    #[cfg(unix)]
    #[test]
    fn installed_authority_rejects_a_foreign_marketplace_directory_before_capture() {
        let fixture = PersonalInstallFixture::new("foreign-marketplace");
        let before = tree_snapshot(&fixture.root);

        let home = PersonalHomeAuthority::capture(&fixture.home).unwrap();
        let failure = match installed_authority(&home, &fixture.registry()) {
            Ok(_) => panic!("foreign marketplace unexpectedly admitted"),
            Err(failure) => failure,
        };

        assert_eq!(
            failure,
            "installed source path is not the canonical Harness marketplace member"
        );
        assert_eq!(tree_snapshot(&fixture.root), before);
    }

    #[cfg(unix)]
    #[test]
    fn personal_home_authority_rejects_alternate_home_and_same_path_inode_substitution() {
        let first = PersonalInstallFixture::new("harness-ultragoal-local");
        let second = PersonalInstallFixture::new("harness-ultragoal-local");
        let retained = PersonalHomeAuthority::capture(&first.home).unwrap();
        let alternate = PersonalHomeAuthority::capture(&second.home).unwrap();
        assert_ne!(retained.record(), alternate.record());

        let held = first.root.join("held-home");
        fs::rename(&first.home, &held).unwrap();
        fs::create_dir(&first.home).unwrap();
        fs::create_dir(first.home.join(".codex")).unwrap();
        assert_eq!(
            retained.revalidate().unwrap_err(),
            "personal home authority changed"
        );
    }

    #[cfg(unix)]
    #[test]
    fn installed_authority_rejects_prior_cache_full_tree_mismatch() {
        let fixture = PersonalInstallFixture::new("harness-ultragoal-local");
        fs::write(fixture.cache.join("cache-only.txt"), b"not in source").unwrap();
        let home = PersonalHomeAuthority::capture(&fixture.home).unwrap();
        let failure = installed_authority(&home, &fixture.registry())
            .err()
            .expect("mismatched cache tree must fail closed");
        assert_eq!(failure, "installed source and cache full trees diverge");
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
        let mut cache_tree = original.clone();
        cache_tree.before.cache_tree_sha256 = digest('9');
        cache_tree.before.installed_authority_sha256 =
            installed_authority_digest(&cache_tree.before).unwrap();
        rebind_plan(&mut cache_tree);
        assert!(validate_plan_identity(&cache_tree).is_err());
        let mut home = original.clone();
        home.personal_home.home.inode += 1;
        home.personal_home.authority_sha256 =
            personal_home_authority_digest(&home.personal_home).unwrap();
        home.before.personal_home_authority_sha256 = home.personal_home.authority_sha256.clone();
        home.before.installed_authority_sha256 = installed_authority_digest(&home.before).unwrap();
        rebind_plan(&mut home);
        assert!(validate_plan_identity(&home).is_ok());
        assert_ne!(home.personal_home, original.personal_home);
        let mut directory_observation = original.clone();
        directory_observation
            .before
            .marketplace_source_observation_sha256 = digest('8');
        assert!(validate_plan_identity(&directory_observation).is_err());
        let mut inner_schema = original.clone();
        inner_schema.before.schema_version = "HarnessObservedInstalledAuthority-v1".to_owned();
        inner_schema.before.installed_authority_sha256 =
            installed_authority_digest(&inner_schema.before).unwrap();
        rebind_plan(&mut inner_schema);
        assert!(validate_plan_identity(&inner_schema).is_err());
        let mut outer_schema = original.clone();
        outer_schema.schema_version = "HarnessPersonalMarketplaceInstallPlan-v3".to_owned();
        rebind_plan(&mut outer_schema);
        assert!(validate_plan_identity(&outer_schema).is_err());
        let mut target = original.clone();
        target.target.version = "0.0.39".to_owned();
        assert!(validate_plan_identity(&target).is_err());
        let mut sequence = original;
        sequence.required_effects.swap(0, 1);
        assert!(validate_plan_identity(&sequence).is_err());
    }

    #[test]
    fn apply_admission_revalidates_the_accepted_candidate_without_replanning_the_predecessor() {
        struct Interrupted {
            observation: crate::distribution::PersonalMarketplaceUpdateObservation,
            restorations: usize,
            reinstalls: usize,
        }

        impl crate::distribution::PersonalMarketplaceUpdateEffects for Interrupted {
            fn observe(
                &mut self,
                _authority: &PersonalMarketplaceUpdateAuthority,
            ) -> Result<crate::distribution::PersonalMarketplaceUpdateObservation, &'static str>
            {
                Ok(self.observation)
            }

            fn materialize_target(
                &mut self,
                _authority: &PersonalMarketplaceUpdateAuthority,
            ) -> Result<(), &'static str> {
                Err("already materialized")
            }

            fn install_target(
                &mut self,
                _authority: &PersonalMarketplaceUpdateAuthority,
            ) -> Result<(), &'static str> {
                Err("installation must not run before restart recovery")
            }

            fn restore_prior_tree(
                &mut self,
                _authority: &PersonalMarketplaceUpdateAuthority,
            ) -> Result<(), &'static str> {
                self.restorations += 1;
                self.observation = exact_update_observation(
                    crate::distribution::PersonalMarketplaceAuthorityMatch::Prior,
                );
                Ok(())
            }

            fn reinstall_prior(
                &mut self,
                _authority: &PersonalMarketplaceUpdateAuthority,
            ) -> Result<(), &'static str> {
                self.reinstalls += 1;
                Ok(())
            }
        }

        let accepted = record();
        let (authority, prior) = accepted_update_authority(&accepted, &accepted.target).unwrap();
        assert_eq!(authority.plan_sha256(), accepted.plan_sha256);
        assert_eq!(authority.prior(), &prior);
        assert_eq!(
            authority.prior_tree_sha256(),
            accepted.before.marketplace_source_tree_sha256
        );
        assert_eq!(
            authority.target_tree_sha256(),
            accepted.target.source_tree_sha256
        );
        let mut interrupted = Interrupted {
            observation: crate::distribution::PersonalMarketplaceUpdateObservation {
                marketplace_root_bound: true,
                source_tree: crate::distribution::PersonalMarketplaceAuthorityMatch::Target,
                installed: crate::distribution::PersonalMarketplaceAuthorityMatch::Target,
                cache: crate::distribution::PersonalMarketplaceAuthorityMatch::Prior,
                runtime: crate::distribution::PersonalMarketplaceAuthorityMatch::Other,
                registry: crate::distribution::PersonalMarketplaceAuthorityMatch::Prior,
            },
            restorations: 0,
            reinstalls: 0,
        };
        let report = execute_personal_marketplace_update(&authority, &mut interrupted);
        assert_eq!(
            report.disposition,
            PersonalMarketplaceUpdateDisposition::RecoveredAfterFailure
        );
        assert_eq!((interrupted.restorations, interrupted.reinstalls), (1, 1));
    }

    fn exact_update_observation(
        value: crate::distribution::PersonalMarketplaceAuthorityMatch,
    ) -> crate::distribution::PersonalMarketplaceUpdateObservation {
        crate::distribution::PersonalMarketplaceUpdateObservation {
            marketplace_root_bound: true,
            source_tree: value,
            installed: value,
            cache: value,
            runtime: value,
            registry: value,
        }
    }

    #[test]
    fn registry_observation_failure_keeps_its_typed_cause() {
        assert_eq!(
            registry_observation_failure_cause(
                super::capabilities::RegistryObservationFailure::Blocked(
                    crate::plugin_product::agent_discovery::AgentDiscoveryErrorId::IdentityMismatch,
                ),
            ),
            "identity-mismatch"
        );
        assert_eq!(
            registry_observation_failure_cause(
                super::capabilities::RegistryObservationFailure::Unavailable(
                    "codex-observation-unavailable",
                ),
            ),
            "codex-observation-unavailable"
        );
    }

    #[test]
    fn public_failure_diagnostics_preserve_stable_stage_causality_and_next_action() {
        assert_eq!(
            recovered_stage_diagnostic(Some(PersonalMarketplaceUpdateStage::InstallTarget)),
            (
                "personal marketplace update recovered exact prior after stage=install-target",
                "derive a fresh exact plan only after the pinned child execution or cancellation cause is resolved",
            )
        );
        assert_eq!(
            ambiguous_stage_diagnostic(Some(PersonalMarketplaceUpdateStage::ReinstallPrior)),
            (
                "personal marketplace recovery is ambiguous at stage=reinstall-prior",
                "preserve all state and reconcile the exact prior registry, cache, runtime, and selected-Codex child result before any retry",
            )
        );
        assert_eq!(
            refused_stage_diagnostic(Some(PersonalMarketplaceUpdateStage::InitialObservation)).0,
            "personal marketplace update refused before effect at stage=initial-observation"
        );
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

        let mut substituted_cachebuster = original.before;
        substituted_cachebuster.plugin_version = "0.0.39+codex.substituted-cachebuster".to_owned();
        assert_ne!(
            installed_authority_digest(&substituted_cachebuster).unwrap(),
            substituted_cachebuster.installed_authority_sha256
        );
    }

    #[test]
    fn accepted_plan_adopts_prior_authority_without_relabeling_it_as_target() {
        let record = record();
        let (lifecycle, target, prior) = adopted_lifecycle_plan(&record).unwrap();
        assert_eq!(lifecycle.intent, LifecycleIntent::MonotonicUpdate);
        assert_eq!(lifecycle.before.generation, 0);
        assert_eq!(lifecycle.before.installed.as_ref(), Some(prior.authority()));
        assert_eq!(lifecycle.rollback_state, lifecycle.before);
        assert_eq!(
            lifecycle
                .expected_after
                .installed
                .as_ref()
                .unwrap()
                .package_sha256,
            target.archive_sha256()
        );
        assert_ne!(prior.authority().package_sha256, target.archive_sha256());

        let mut downgraded = record.clone();
        downgraded.target.version = downgraded.before.plugin_version.clone();
        assert!(adopted_lifecycle_plan(&downgraded).is_err());
        let mut cachebuster_only = record.clone();
        cachebuster_only.target.version = "0.0.39+codex.different-cachebuster".to_owned();
        assert!(adopted_lifecycle_plan(&cachebuster_only).is_err());
        let mut aliased = record;
        aliased.target.archive_sha256 = aliased.before.installed_authority_sha256.clone();
        assert!(adopted_lifecycle_plan(&aliased).is_err());
    }

    #[test]
    fn public_grammar_separates_read_only_plan_from_external_apply_acceptance() {
        let plan = parse_args([
            "--json",
            "package",
            "install-plan",
            "--input",
            "target/ultragoal/package-0.0.41.hugpkg",
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
