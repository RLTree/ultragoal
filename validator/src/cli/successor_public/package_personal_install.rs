use super::*;
use crate::cli::successor::command_contract::{OptionName, PackageAction, ParsedValue};
use crate::distribution::{
    PERSONAL_MARKETPLACE_SOURCE_RELATIVE, PersonalMarketplaceSourceObservation,
    ReadOnlyTreeObservation, ReadOnlyWorkspace, capture_product_package,
    capture_product_package_with_cli, verify_product_package,
};
use crate::plugin_product::agent_discovery::{
    HostPluginRegistryObservation, InstalledSourceAuthorityCapture,
    capture_installed_source_authority, parse_unpinned_host_plugin_registry_observation,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::ffi::OsString;
use std::fs::{self, File, OpenOptions};
use std::io::Read;
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::time::Duration;

const PACKAGE_LIMIT: usize = 65 * 1024 * 1024;
const HANDOFF_LIMIT: u64 = 1024 * 1024;
const FILE_LIMIT: usize = 256 * 1024 * 1024;
const CONFIG_LIMIT: usize = 1024 * 1024;
const MARKETPLACE: &str = "local-harness-plugins";
const PLUGIN_ID: &str = "harness-ultragoal@local-harness-plugins";
const PLUGIN_NAME: &str = "harness-ultragoal";
const HANDOFF_SCHEMA: &str = "HarnessPersonalMarketplaceInstallHandoff-v1";
const INSTALLED_SCHEMA: &str = "HarnessObservedInstalledAuthority-v4";
const HOST_TIMEOUT: Duration = Duration::from_secs(30);
const PROTECTED_PATHS: [&str; 4] = [
    "validator/src/distribution/host_effect/executor/tests/mod.rs",
    "validator/src/distribution/host_effect/mod.rs",
    "validator/src/distribution/host_effect/selected_codex_executable/selected_tests.rs",
    "validator/src/distribution/package/manifest_bind.rs",
];
const PROTECTED_MARKER: &str = ".ultragoal-e2e-unrelated-state.txt";

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
    canonical_home: String,
    canonical_codex_home: String,
    home: DirectoryAuthority,
    codex_home: DirectoryAuthority,
    authority_sha256: String,
}

/// Retained descriptors make observation resistant to pathname substitution.
/// This capability exposes no writer, recovery primitive, or confined root.
struct PersonalHomeAuthority {
    home_path: PathBuf,
    codex_home_path: PathBuf,
    home: File,
    codex_home: File,
    record: PersonalHomeAuthorityRecord,
}

impl PersonalHomeAuthority {
    fn capture(requested_home: &Path) -> Result<Self, &'static str> {
        let home_path = canonical_directory(requested_home, "personal home is unavailable")?;
        if home_path != requested_home {
            return Err("ambient HOME is not the canonical personal home");
        }
        let requested_codex = std::env::var_os("CODEX_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home_path.join(".codex"));
        let codex_home_path =
            canonical_directory(&requested_codex, "effective CODEX_HOME is unavailable")?;
        if codex_home_path != home_path.join(".codex") {
            return Err("effective CODEX_HOME is outside the supported personal scope");
        }
        let home = open_directory(&home_path)?;
        let codex_fd = unsafe {
            libc::openat(
                home.as_raw_fd(),
                c".codex".as_ptr(),
                libc::O_RDONLY | libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_DIRECTORY,
            )
        };
        if codex_fd < 0 {
            return Err("effective CODEX_HOME descriptor is unavailable");
        }
        let codex_home = unsafe { File::from_raw_fd(codex_fd) };
        let home_authority = directory_authority(
            &home
                .metadata()
                .map_err(|_| "personal home descriptor is unavailable")?,
        )?;
        let codex_authority = directory_authority(
            &codex_home
                .metadata()
                .map_err(|_| "CODEX_HOME descriptor is unavailable")?,
        )?;
        if home_authority
            != directory_authority(
                &fs::symlink_metadata(&home_path).map_err(|_| "personal home is unavailable")?,
            )?
            || codex_authority != stat_child_directory(home.as_raw_fd(), c".codex")?
        {
            return Err("personal HOME or CODEX_HOME changed during capture");
        }
        let mut record = PersonalHomeAuthorityRecord {
            schema_version: "HarnessPersonalHomeAuthority-v2".to_owned(),
            canonical_home: path_string(&home_path)?,
            canonical_codex_home: path_string(&codex_home_path)?,
            home: home_authority,
            codex_home: codex_authority,
            authority_sha256: String::new(),
        };
        record.authority_sha256 = home_authority_digest(&record)?;
        let authority = Self {
            home_path,
            codex_home_path,
            home,
            codex_home,
            record,
        };
        authority.revalidate()?;
        Ok(authority)
    }

    fn revalidate(&self) -> Result<(), &'static str> {
        if self.home_path.canonicalize().ok().as_deref() != Some(self.home_path.as_path())
            || self.codex_home_path.canonicalize().ok().as_deref()
                != Some(self.codex_home_path.as_path())
            || directory_authority(
                &self
                    .home
                    .metadata()
                    .map_err(|_| "personal home descriptor is unavailable")?,
            )? != self.record.home
            || directory_authority(
                &self
                    .codex_home
                    .metadata()
                    .map_err(|_| "CODEX_HOME descriptor is unavailable")?,
            )? != self.record.codex_home
            || stat_child_directory(self.home.as_raw_fd(), c".codex")? != self.record.codex_home
            || self.record.authority_sha256 != home_authority_digest(&self.record)?
        {
            return Err("personal HOME or CODEX_HOME authority changed");
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct BoundFile {
    canonical_path: String,
    sha256: String,
    byte_length: u64,
    unix_mode: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct HostContext {
    personal_home: PersonalHomeAuthorityRecord,
    config: BoundFile,
    profile_context: String,
    working_directory: String,
    context_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct SelectedCodex {
    executable: BoundFile,
    version: String,
    plugin_help_sha256: String,
    registry_observation_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct InstalledAuthority {
    schema_version: String,
    plugin_version: String,
    registry_observation_sha256: String,
    marketplace_source_path: String,
    marketplace_source_tree_sha256: String,
    marketplace_source_observation_sha256: String,
    marketplace_source_catalog_sha256: String,
    cache_path: String,
    cache_tree_sha256: String,
    cache_catalog_sha256: String,
    marketplace_runtime_sha256: String,
    cache_runtime_sha256: String,
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
            .map_err(|_| "installed marketplace source changed")?;
        self.cache
            .revalidate()
            .map_err(|_| "installed cache changed")?;
        self.source_tree
            .revalidate()
            .map_err(|_| "installed marketplace source tree changed")?;
        self.cache_tree
            .revalidate()
            .map_err(|_| "installed cache tree changed")
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
    archive_path: String,
    archive_sha256: String,
    inventory_sha256: String,
    runtime_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct DurableMarketplaceSource {
    catalog: BoundFile,
    catalog_name: String,
    canonical_path: String,
    tree_sha256: String,
    observation_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct SourceCandidate {
    head_commit: String,
    head_tree: String,
    branch: String,
    status_sha256: String,
    worktree_diff_sha256: String,
    staged_diff_sha256: String,
    untracked_content_sha256: String,
    dirty: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct ProtectedState {
    tracked_diff_sha256: String,
    marker_sha256: String,
    state_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct SupportedAction {
    executable: String,
    arguments: Vec<String>,
    working_directory: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct PersonalMarketplaceInstallHandoff {
    schema_version: String,
    handoff_sha256: String,
    effect: String,
    status: String,
    source: SourceCandidate,
    target: TargetPackage,
    release_cli: BoundFile,
    durable_marketplace_source: DurableMarketplaceSource,
    plugin_id: String,
    marketplace: String,
    host: HostContext,
    selected_codex: SelectedCodex,
    predecessor: InstalledAuthority,
    expected_target_source_tree_sha256: String,
    expected_target_cache_tree_sha256: String,
    expected_target_runtime_sha256: String,
    protected_state: ProtectedState,
    supported_action: SupportedAction,
    recovery_action: SupportedAction,
    consequences: Vec<String>,
    cancellation_boundary: String,
    restart_rule: String,
    claim_ceiling: String,
}

struct PreparedHandoff {
    record: PersonalMarketplaceInstallHandoff,
    home: PersonalHomeAuthority,
    installed: InstalledAuthorityObservation,
    registry: HostPluginRegistryObservation,
    marketplace_source: ReadOnlyTreeObservation,
}

impl PreparedHandoff {
    fn revalidate(
        &self,
        source_context: &LiveContext,
        observation_context: &LiveContext,
    ) -> Result<(), &'static str> {
        source_context
            .revalidate()
            .map_err(|_| "candidate changed before handoff publication")?;
        observation_context
            .revalidate()
            .map_err(|_| "host context changed before handoff publication")?;
        self.home.revalidate()?;
        self.installed.revalidate()?;
        self.marketplace_source
            .revalidate()
            .map_err(|_| "durable marketplace source changed before handoff publication")?;
        let observed = observe_supported_host(observation_context, &self.home)?;
        if observed.registry != self.registry {
            return Err("supported Codex listing changed before handoff publication");
        }
        Ok(())
    }
}

struct SupportedHostObservation {
    registry: HostPluginRegistryObservation,
    selected: SelectedCodex,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
enum VerificationClassification {
    ExactTarget,
    ExactPriorNoEffect,
    PartialOrStale,
    Ambiguous,
}

#[derive(Serialize)]
struct InstallVerification<'a> {
    schema_version: &'static str,
    handoff_sha256: &'a str,
    classification: VerificationClassification,
    effect: &'static str,
    next_action: Option<&'a SupportedAction>,
    claim_ceiling: &'static str,
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
        return handoff_failure("canonical personal HOME is unavailable");
    };
    match build_handoff(source_context, observation_context, home, input, cli) {
        Ok(prepared) => {
            if let Err(cause) = prepared.revalidate(source_context, observation_context) {
                return handoff_failure(cause);
            }
            match serde_json::to_vec(&prepared.record) {
                Ok(bytes) if public_output_allowed(bytes.len()) => RuntimeOutcome::payload(
                    ExitClass::Success,
                    bytes,
                    format!(
                        "supported install handoff {} effect=none",
                        prepared.record.handoff_sha256
                    ),
                ),
                _ => handoff_failure("supported install handoff encoding failed"),
            }
        }
        Err(cause) => handoff_failure(cause),
    }
}

pub(super) fn verify(
    source_context: &LiveContext,
    observation_context: &LiveContext,
    invocation: &ParsedInvocation,
    home: Option<&Path>,
) -> RuntimeOutcome {
    let Some(path) = verify_argument(invocation) else {
        return invalid_verify_invocation();
    };
    let Some(home) = home else {
        return verify_failure("canonical personal HOME is unavailable");
    };
    let bytes = match super::fit::external_plan_file::read_immutable_plan(path, HANDOFF_LIMIT) {
        Ok(bytes) => bytes,
        Err(_) => return verify_failure("handoff is not one immutable owner-only input"),
    };
    let canonical = bytes.strip_suffix(b"\n").unwrap_or(&bytes);
    let record: PersonalMarketplaceInstallHandoff = match serde_json::from_slice(canonical) {
        Ok(record) => record,
        Err(_) => return verify_failure("handoff does not match the closed schema"),
    };
    if validate_handoff_identity(&record).is_err() {
        return verify_failure("handoff identity was changed or substituted");
    }
    let source = match capture_source_candidate(source_context) {
        Ok(source) => source,
        Err(cause) => return verify_failure(cause),
    };
    if source != record.source {
        return verification_result(&record, VerificationClassification::PartialOrStale);
    }
    let protected = match capture_protected_state(source_context) {
        Ok(protected) => protected,
        Err(cause) => return verify_failure(cause),
    };
    if protected != record.protected_state {
        return verification_result(&record, VerificationClassification::PartialOrStale);
    }
    let durable = match capture_durable_marketplace_source(source_context, &record.target) {
        Ok((source, _observation)) => source,
        Err(_) => return verification_result(&record, VerificationClassification::PartialOrStale),
    };
    if durable != record.durable_marketplace_source {
        return verification_result(&record, VerificationClassification::PartialOrStale);
    }
    let authority = match PersonalHomeAuthority::capture(home) {
        Ok(authority) => authority,
        Err(_) => return verification_result(&record, VerificationClassification::Ambiguous),
    };
    let host = match capture_host_context(observation_context, &authority) {
        Ok(host) => host,
        Err(_) => return verification_result(&record, VerificationClassification::Ambiguous),
    };
    if host.personal_home != record.host.personal_home
        || host.profile_context != record.host.profile_context
        || host.working_directory != record.host.working_directory
    {
        return verification_result(&record, VerificationClassification::PartialOrStale);
    }
    let supported = match observe_supported_host(observation_context, &authority) {
        Ok(observation) => observation,
        Err(_) => return verification_result(&record, VerificationClassification::Ambiguous),
    };
    if supported.selected.executable != record.selected_codex.executable
        || supported.selected.version != record.selected_codex.version
        || supported.selected.plugin_help_sha256 != record.selected_codex.plugin_help_sha256
    {
        return verification_result(&record, VerificationClassification::PartialOrStale);
    }
    let installed = match installed_authority(&authority, &supported.registry) {
        Ok(observation) => observation,
        Err(_) => return verification_result(&record, VerificationClassification::PartialOrStale),
    };
    verification_result(
        &record,
        classify_installation(&record, &installed.authority),
    )
}

fn build_handoff(
    source_context: &LiveContext,
    observation_context: &LiveContext,
    home_path: &Path,
    input: &str,
    cli: &str,
) -> Result<PreparedHandoff, &'static str> {
    let (artifact, _catalog) = capture_accepted_candidate(source_context, input, cli)?;
    let home = PersonalHomeAuthority::capture(home_path)?;
    let supported = observe_supported_host(observation_context, &home)?;
    let installed = installed_authority(&home, &supported.registry)?;
    let target = target_package(&artifact, input)?;
    if !monotonic_successor(&installed.authority.plugin_version, &target.version) {
        return Err("target package is not one monotonic installed-version successor");
    }
    let host = capture_host_context(observation_context, &home)?;
    let release_cli = bind_workspace_file(source_context, cli, FILE_LIMIT)?;
    if release_cli.sha256 != target.runtime_sha256 {
        return Err("release CLI and packaged runtime differ");
    }
    let source = capture_source_candidate(source_context)?;
    let (durable_marketplace_source, marketplace_source) =
        capture_durable_marketplace_source(source_context, &target)?;
    let protected_state = capture_protected_state(source_context)?;
    let action = SupportedAction {
        executable: supported.selected.executable.canonical_path.clone(),
        arguments: vec![
            "plugin".to_owned(),
            "add".to_owned(),
            format!("{PLUGIN_NAME}@{MARKETPLACE}"),
        ],
        working_directory: host.working_directory.clone(),
    };
    let mut record = PersonalMarketplaceInstallHandoff {
        schema_version: HANDOFF_SCHEMA.to_owned(),
        handoff_sha256: String::new(),
        effect: "none".to_owned(),
        status: "ready-for-explicit-supported-codex-user-action".to_owned(),
        source,
        target: target.clone(),
        release_cli,
        durable_marketplace_source,
        plugin_id: PLUGIN_ID.to_owned(),
        marketplace: MARKETPLACE.to_owned(),
        host,
        selected_codex: supported.selected,
        predecessor: installed.authority.clone(),
        expected_target_source_tree_sha256: target.source_tree_sha256.clone(),
        expected_target_cache_tree_sha256: target.source_tree_sha256.clone(),
        expected_target_runtime_sha256: target.runtime_sha256.clone(),
        protected_state,
        supported_action: action.clone(),
        recovery_action: action,
        consequences: vec![
            "Supported Codex may replace its personal cache entry for this plugin version."
                .to_owned(),
            "Supported Codex may update personal config enablement for this plugin.".to_owned(),
            "UltraGoal does not execute, retry, compensate, restore, or remove personal state."
                .to_owned(),
        ],
        cancellation_boundary: "Cancellation before the supported command starts is no effect; after it starts, Codex and the user own completion and recovery.".to_owned(),
        restart_rule: "After the supported command is terminal, start a fresh Codex task and run package install-verify before separate discovery or runtime evaluation.".to_owned(),
        claim_ceiling: "read-only exact handoff only; personal installation, discovery, runtime behavior, readiness, and release remain unproved".to_owned(),
    };
    record.handoff_sha256 = handoff_digest(&record)?;
    validate_handoff_identity(&record)?;
    Ok(PreparedHandoff {
        record,
        home,
        installed,
        registry: supported.registry,
        marketplace_source,
    })
}

fn capture_accepted_candidate(
    context: &LiveContext,
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
        return Err("package or release CLI is outside the bounded package surface");
    }
    let catalog = InventoryBuilder::new(context)
        .build()
        .map_err(|_| "authority catalog is unavailable")?;
    let source = capture_product_package(context, &catalog)
        .map_err(|_| "current-source package capture failed")?;
    let payload = super::package_cli_payload::from_read_only(context, cli, source.candidate_id())
        .ok_or("release CLI payload is unavailable or not native")?;
    let artifact = capture_product_package_with_cli(context, &catalog, payload)
        .map_err(|_| "current-source package capture failed")?;
    verify_product_package(&artifact, context, &catalog)
        .map_err(|_| "current-source package verification failed")?;
    let archive = ReadOnlyWorkspace::open(context)
        .and_then(|workspace| workspace.inspect_file(input, PACKAGE_LIMIT))
        .map_err(|_| "package archive is unavailable")?
        .ok_or("package archive is unavailable")?;
    if archive != artifact.snapshot().archive() {
        return Err("package archive does not match the exact current candidate");
    }
    Ok((artifact, catalog))
}

fn target_package(
    artifact: &crate::distribution::ProductionPackageArtifact,
    input: &str,
) -> Result<TargetPackage, &'static str> {
    let snapshot = artifact.snapshot();
    let runtime = snapshot
        .entries()
        .iter()
        .find(|entry| entry.path == "runtime/ultragoal")
        .ok_or("packaged runtime is unavailable")?;
    Ok(TargetPackage {
        context_id: snapshot.identity().source().context_id().to_owned(),
        candidate_id: artifact.candidate_id().to_owned(),
        catalog_id: artifact.catalog_id().to_owned(),
        version: snapshot.identity().source().version().to_owned(),
        source_tree_sha256: snapshot.source_tree_sha256().to_owned(),
        archive_path: input.to_owned(),
        archive_sha256: snapshot.package_sha256().to_owned(),
        inventory_sha256: snapshot.inventory_sha256().to_owned(),
        runtime_sha256: runtime.sha256.clone(),
    })
}

fn capture_durable_marketplace_source(
    context: &LiveContext,
    target: &TargetPackage,
) -> Result<(DurableMarketplaceSource, ReadOnlyTreeObservation), &'static str> {
    const CATALOG_PATH: &str = ".agents/plugins/marketplace.json";
    let catalog = bind_workspace_file(context, CATALOG_PATH, CONFIG_LIMIT)?;
    let bytes = fs::read(&catalog.canonical_path)
        .map_err(|_| "workspace marketplace catalog is unreadable")?;
    let value: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|_| "workspace marketplace catalog is invalid")?;
    let catalog_name = value
        .get("name")
        .and_then(serde_json::Value::as_str)
        .filter(|name| !name.is_empty() && name.len() <= 128)
        .ok_or("workspace marketplace name is unavailable")?;
    let rows = value
        .get("plugins")
        .and_then(serde_json::Value::as_array)
        .filter(|rows| rows.len() == 1)
        .ok_or("workspace marketplace plugin source is ambiguous")?;
    let plugin = &rows[0];
    let source = plugin
        .get("source")
        .and_then(serde_json::Value::as_object)
        .ok_or("workspace marketplace plugin source is unavailable")?;
    if plugin.get("name").and_then(serde_json::Value::as_str) != Some(PLUGIN_NAME)
        || source.get("source").and_then(serde_json::Value::as_str) != Some("local")
    {
        return Err("workspace marketplace does not bind the exact local plugin");
    }
    let relative = source
        .get("path")
        .and_then(serde_json::Value::as_str)
        .filter(|path| *path == "./plugins/harness-ultragoal")
        .ok_or("workspace marketplace source path is not the canonical durable source")?;
    let relative = relative
        .strip_prefix("./")
        .ok_or("workspace marketplace source path is invalid")?;
    let path = context.worktree_root().join(relative);
    let canonical = canonical_directory(
        &path,
        "durable workspace-local marketplace source is not materialized",
    )?;
    if !canonical.starts_with(context.worktree_root()) {
        return Err("durable marketplace source escapes the workspace");
    }
    let observation = ReadOnlyTreeObservation::capture_root(&canonical, 4096, PACKAGE_LIMIT)
        .map_err(|_| "durable marketplace source observation is unavailable")?;
    if observation.tree_sha256() != target.source_tree_sha256 {
        return Err("durable marketplace source does not match the exact target package");
    }
    let record = DurableMarketplaceSource {
        catalog,
        catalog_name: catalog_name.to_owned(),
        canonical_path: path_string(&canonical)?,
        tree_sha256: observation.tree_sha256().to_owned(),
        observation_sha256: observation.observation_sha256().to_owned(),
    };
    observation
        .revalidate()
        .map_err(|_| "durable marketplace source changed during capture")?;
    Ok((record, observation))
}

/// Directly reads exact selected-binary, config, cached-catalog, source, and
/// cache objects. It does not invoke Codex: even a listing command may attempt
/// host maintenance before producing output and is therefore not read proof.
fn observe_supported_host(
    context: &LiveContext,
    home: &PersonalHomeAuthority,
) -> Result<SupportedHostObservation, &'static str> {
    home.revalidate()?;
    let codex = context
        .capabilities()
        .tool("codex")
        .filter(|tool| tool.available)
        .ok_or("selected Codex executable is unavailable")?;
    let selected_path = codex
        .executable
        .as_deref()
        .map(Path::new)
        .ok_or("selected Codex executable is unavailable")?;
    let expected_sha256 = codex
        .executable_sha256
        .as_deref()
        .filter(|value| valid_hex_digest(value))
        .map(|value| format!("sha256:{value}"))
        .ok_or("selected Codex executable identity is unavailable")?;
    let executable = bind_file(selected_path, FILE_LIMIT)?;
    if executable.sha256 != expected_sha256 {
        return Err("selected Codex executable content changed");
    }
    let binary = fs::read(&executable.canonical_path)
        .map_err(|_| "selected Codex executable is unreadable")?;
    let add_signature = b"Install a plugin from a configured marketplace snapshot";
    let remove_signature = b"Remove an installed plugin from local config and cache";
    if !contains_bytes(&binary, add_signature) || !contains_bytes(&binary, remove_signature) {
        return Err("selected Codex binary lacks the required supported action surfaces");
    }
    let version = selected_version_from_path(Path::new(&executable.canonical_path))?;
    let (config, config_bytes) = bind_personal_file(home, Path::new("config.toml"), CONFIG_LIMIT)?;
    let config_value: toml::Value =
        toml::from_slice(&config_bytes).map_err(|_| "Codex config shape is unavailable")?;
    let enabled = config_value
        .get("plugins")
        .and_then(toml::Value::as_table)
        .and_then(|plugins| plugins.get(PLUGIN_ID))
        .and_then(toml::Value::as_table)
        .and_then(|plugin| plugin.get("enabled"))
        .and_then(toml::Value::as_bool);
    if enabled != Some(true) {
        return Err("current plugin config enablement is absent or ambiguous");
    }
    let source_path = home.home_path.join(PERSONAL_MARKETPLACE_SOURCE_RELATIVE);
    let personal_source_relative =
        Path::new("local-marketplaces/harness-ultragoal-local/plugins/harness-ultragoal");
    let (marketplace_config, marketplace_config_bytes) = bind_personal_file(
        home,
        Path::new("local-marketplaces/harness-ultragoal-local/config.toml"),
        CONFIG_LIMIT,
    )?;
    if !configured_marketplace_is_exact(
        &marketplace_config_bytes,
        &context.worktree_root().join("plugins/harness-ultragoal"),
    ) {
        return Err(
            "configured marketplace identity or source is not the exact workspace-local authority",
        );
    }
    let (_manifest, manifest_bytes) = bind_personal_file(
        home,
        &personal_source_relative.join(".codex-plugin/plugin.json"),
        CONFIG_LIMIT,
    )?;
    let parsed_manifest =
        crate::plugin_manifest::parse(&manifest_bytes, crate::plugin_manifest::MANIFEST_LIMIT)
            .map_err(|_| "installed plugin manifest is invalid")?;
    if parsed_manifest.name != PLUGIN_NAME {
        return Err("installed plugin identity is substituted");
    }
    let (cached_catalog, cached_catalog_bytes) = bind_personal_file(
        home,
        Path::new("local-marketplaces/harness-ultragoal-local/.agents/plugins/marketplace.json"),
        CONFIG_LIMIT,
    )?;
    if !cached_catalog_binds_plugin(&cached_catalog_bytes) {
        return Err("cached marketplace does not bind the exact local plugin source");
    }
    let plugin_json = serde_json::to_vec(&serde_json::json!({
        "installed": [{
            "pluginId": PLUGIN_ID,
            "name": PLUGIN_NAME,
            "marketplaceName": MARKETPLACE,
            "version": parsed_manifest.version,
            "installed": true,
            "enabled": true,
            "source": {"source": "local", "path": source_path},
        }]
    }))
    .map_err(|_| "direct plugin observation encoding failed")?;
    let marketplace_json = serde_json::to_vec(&serde_json::json!({
        "marketplaces": [{"name": MARKETPLACE, "root": home.home_path}]
    }))
    .map_err(|_| "direct marketplace observation encoding failed")?;
    let registry = parse_unpinned_host_plugin_registry_observation(
        &plugin_json,
        &marketplace_json,
        Path::new(&executable.canonical_path),
        &executable.sha256,
    )
    .map_err(|_| "direct plugin or marketplace observation is unavailable or ambiguous")?;
    let selected = SelectedCodex {
        executable,
        version,
        plugin_help_sha256: digest_json(&(
            "descriptor-bound-supported-actions-v1",
            digest_bytes(add_signature),
            digest_bytes(remove_signature),
            config.sha256,
            marketplace_config.sha256,
            cached_catalog.sha256,
        ))?,
        registry_observation_sha256: registry.sha256().to_owned(),
    };
    home.revalidate()?;
    Ok(SupportedHostObservation { registry, selected })
}

fn installed_authority(
    home: &PersonalHomeAuthority,
    registry: &HostPluginRegistryObservation,
) -> Result<InstalledAuthorityObservation, &'static str> {
    home.revalidate()?;
    if registry.marketplace_root() != home.home_path
        || registry.installed_root() != home.home_path.join(PERSONAL_MARKETPLACE_SOURCE_RELATIVE)
    {
        return Err("observed marketplace source is outside the exact personal authority");
    }
    let session_id = digest_bytes(
        format!(
            "personal-installed-observation-v2\0{}\0{}",
            home.home_path.display(),
            registry.selected_codex_identity_sha256()
        )
        .as_bytes(),
    );
    let source = capture_installed_source_authority(
        registry.installed_root(),
        registry.sha256(),
        &session_id,
    )
    .map_err(|_| "installed marketplace source authority is unavailable")?;
    let cache_path = home.home_path.join(format!(
        ".codex/plugins/cache/{MARKETPLACE}/{PLUGIN_NAME}/{}",
        registry.plugin_version()
    ));
    let cache = capture_installed_source_authority(&cache_path, registry.sha256(), &session_id)
        .map_err(|_| "installed cache authority is unavailable")?;
    if source.plugin_version() != registry.plugin_version()
        || cache.plugin_version() != registry.plugin_version()
        || source.runtime_sha256() != cache.runtime_sha256()
    {
        return Err("registry, source, cache, or runtime identity diverges");
    }
    let source_tree =
        PersonalMarketplaceSourceObservation::capture(&home.home_path, registry.installed_root())
            .map_err(|_| "installed marketplace source tree is unavailable")?;
    let cache_tree = ReadOnlyTreeObservation::capture_root(&cache_path, 4096, PACKAGE_LIMIT)
        .map_err(|_| "installed cache tree is unavailable")?;
    if source_tree.tree_sha256() != cache_tree.tree_sha256() {
        return Err("installed marketplace source and cache trees diverge");
    }
    let mut authority = InstalledAuthority {
        schema_version: INSTALLED_SCHEMA.to_owned(),
        plugin_version: registry.plugin_version().to_owned(),
        registry_observation_sha256: registry.sha256().to_owned(),
        marketplace_source_path: path_string(registry.installed_root())?,
        marketplace_source_tree_sha256: source_tree.tree_sha256().to_owned(),
        marketplace_source_observation_sha256: source_tree.observation_sha256().to_owned(),
        marketplace_source_catalog_sha256: source.catalog_sha256().to_owned(),
        cache_path: path_string(&cache_path)?,
        cache_tree_sha256: cache_tree.tree_sha256().to_owned(),
        cache_catalog_sha256: cache.catalog_sha256().to_owned(),
        marketplace_runtime_sha256: source.runtime_sha256().to_owned(),
        cache_runtime_sha256: cache.runtime_sha256().to_owned(),
        installed_authority_sha256: String::new(),
    };
    authority.installed_authority_sha256 = installed_digest(&authority)?;
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

fn capture_host_context(
    context: &LiveContext,
    home: &PersonalHomeAuthority,
) -> Result<HostContext, &'static str> {
    home.revalidate()?;
    let config_path = home.codex_home_path.join("config.toml");
    let config = bind_file(&config_path, CONFIG_LIMIT)?;
    let profile_context = std::env::var("CODEX_PROFILE").unwrap_or_else(|_| "default".to_owned());
    if profile_context.is_empty()
        || profile_context.len() > 128
        || profile_context.chars().any(char::is_control)
    {
        return Err("effective Codex profile context is invalid");
    }
    let working_directory = canonical_directory(
        context.worktree_root(),
        "supported action working directory is unavailable",
    )?;
    let mut host = HostContext {
        personal_home: home.record.clone(),
        config,
        profile_context,
        working_directory: path_string(&working_directory)?,
        context_sha256: String::new(),
    };
    host.context_sha256 = host_context_digest(&host)?;
    Ok(host)
}

fn capture_source_candidate(context: &LiveContext) -> Result<SourceCandidate, &'static str> {
    let candidate = context.candidate();
    Ok(SourceCandidate {
        head_commit: candidate
            .head_commit
            .clone()
            .ok_or("source HEAD is unavailable")?,
        head_tree: candidate
            .head_tree
            .clone()
            .ok_or("source tree is unavailable")?,
        branch: candidate
            .branch
            .clone()
            .ok_or("source branch is unavailable")?,
        status_sha256: candidate.status_sha256.clone(),
        worktree_diff_sha256: candidate.worktree_diff_sha256.clone(),
        staged_diff_sha256: candidate.staged_diff_sha256.clone(),
        untracked_content_sha256: candidate.untracked_content_sha256.clone(),
        dirty: candidate.dirty,
    })
}

fn capture_protected_state(context: &LiveContext) -> Result<ProtectedState, &'static str> {
    let git = context
        .capabilities()
        .tool("git")
        .filter(|tool| tool.available)
        .and_then(|tool| tool.executable.as_deref())
        .map(Path::new)
        .ok_or("Git is unavailable for protected-state observation")?;
    let mut arguments = vec![
        OsString::from("diff"),
        OsString::from("--binary"),
        OsString::from("--"),
    ];
    arguments.extend(PROTECTED_PATHS.iter().map(OsString::from));
    let output =
        crate::context::run_bounded(git, &arguments, context.worktree_root(), HOST_TIMEOUT)
            .map_err(|_| "protected tracked diff observation failed")?;
    let marker = ReadOnlyWorkspace::open(context)
        .and_then(|workspace| workspace.inspect_file(PROTECTED_MARKER, CONFIG_LIMIT))
        .map_err(|_| "protected marker observation failed")?
        .ok_or("protected marker is unavailable")?;
    let tracked_diff_sha256 = digest_bytes(&output.stdout);
    let marker_sha256 = digest_bytes(&marker);
    let state_sha256 = digest_json(&(tracked_diff_sha256.as_str(), marker_sha256.as_str()))?;
    Ok(ProtectedState {
        tracked_diff_sha256,
        marker_sha256,
        state_sha256,
    })
}

fn classify_installation(
    record: &PersonalMarketplaceInstallHandoff,
    observed: &InstalledAuthority,
) -> VerificationClassification {
    if observed == &record.predecessor {
        return VerificationClassification::ExactPriorNoEffect;
    }
    if observed.plugin_version == record.target.version
        && observed.marketplace_source_tree_sha256 == record.expected_target_source_tree_sha256
        && observed.cache_tree_sha256 == record.expected_target_cache_tree_sha256
        && observed.marketplace_runtime_sha256 == record.expected_target_runtime_sha256
        && observed.cache_runtime_sha256 == record.expected_target_runtime_sha256
    {
        return VerificationClassification::ExactTarget;
    }
    VerificationClassification::PartialOrStale
}

fn verification_result(
    record: &PersonalMarketplaceInstallHandoff,
    classification: VerificationClassification,
) -> RuntimeOutcome {
    let next_action = (!matches!(classification, VerificationClassification::ExactTarget))
        .then_some(&record.recovery_action);
    let result = InstallVerification {
        schema_version: "HarnessPersonalMarketplaceInstallVerification-v1",
        handoff_sha256: &record.handoff_sha256,
        classification,
        effect: "none",
        next_action,
        claim_ceiling: match classification {
            VerificationClassification::ExactTarget => {
                "exact disk and supported-host target observed; fresh-task discovery and runtime behavior remain separate"
            }
            VerificationClassification::ExactPriorNoEffect => {
                "exact predecessor observed; no installation effect is claimed"
            }
            VerificationClassification::PartialOrStale => {
                "partial or stale personal state observed; no success, repair, or retry is claimed"
            }
            VerificationClassification::Ambiguous => {
                "personal state is ambiguous; preserve it and use only the named supported user action"
            }
        },
    };
    match serde_json::to_vec(&result) {
        Ok(bytes) if public_output_allowed(bytes.len()) => RuntimeOutcome::payload(
            if matches!(classification, VerificationClassification::ExactTarget) {
                ExitClass::Success
            } else {
                ExitClass::ActionableFinding
            },
            bytes,
            format!("install verification {classification:?} effect=none"),
        ),
        _ => verify_failure("install verification encoding failed"),
    }
}

fn validate_handoff_identity(
    record: &PersonalMarketplaceInstallHandoff,
) -> Result<(), &'static str> {
    if record.schema_version != HANDOFF_SCHEMA
        || record.effect != "none"
        || record.status != "ready-for-explicit-supported-codex-user-action"
        || record.plugin_id != PLUGIN_ID
        || record.marketplace != MARKETPLACE
        || record.host.personal_home.schema_version != "HarnessPersonalHomeAuthority-v2"
        || record.host.personal_home.authority_sha256
            != home_authority_digest(&record.host.personal_home)?
        || record.host.context_sha256 != host_context_digest(&record.host)?
        || record.predecessor.schema_version != INSTALLED_SCHEMA
        || record.predecessor.installed_authority_sha256 != installed_digest(&record.predecessor)?
        || !monotonic_successor(&record.predecessor.plugin_version, &record.target.version)
        || record.release_cli.sha256 != record.target.runtime_sha256
        || record.durable_marketplace_source.tree_sha256 != record.target.source_tree_sha256
        || record.durable_marketplace_source.canonical_path
            != record.host.working_directory.to_owned() + "/plugins/harness-ultragoal"
        || record.expected_target_source_tree_sha256 != record.target.source_tree_sha256
        || record.expected_target_cache_tree_sha256 != record.target.source_tree_sha256
        || record.expected_target_runtime_sha256 != record.target.runtime_sha256
        || record.supported_action != record.recovery_action
        || record.supported_action.executable != record.selected_codex.executable.canonical_path
        || record.supported_action.arguments
            != ["plugin", "add", "harness-ultragoal@local-harness-plugins"]
        || record.supported_action.working_directory != record.host.working_directory
        || record.protected_state.state_sha256
            != digest_json(&(
                record.protected_state.tracked_diff_sha256.as_str(),
                record.protected_state.marker_sha256.as_str(),
            ))?
        || record.handoff_sha256 != handoff_digest(record)?
    {
        return Err("handoff contract changed");
    }
    Ok(())
}

fn handoff_digest(record: &PersonalMarketplaceInstallHandoff) -> Result<String, &'static str> {
    let mut binding = record.clone();
    binding.handoff_sha256.clear();
    digest_json(&binding)
}

fn installed_digest(authority: &InstalledAuthority) -> Result<String, &'static str> {
    let mut binding = authority.clone();
    binding.installed_authority_sha256.clear();
    digest_json(&binding)
}

fn home_authority_digest(authority: &PersonalHomeAuthorityRecord) -> Result<String, &'static str> {
    let mut binding = authority.clone();
    binding.authority_sha256.clear();
    digest_json(&binding)
}

fn host_context_digest(host: &HostContext) -> Result<String, &'static str> {
    let mut binding = host.clone();
    binding.context_sha256.clear();
    digest_json(&binding)
}

fn bind_workspace_file(
    context: &LiveContext,
    relative: &str,
    maximum: usize,
) -> Result<BoundFile, &'static str> {
    bind_file(&context.worktree_root().join(relative), maximum)
}

fn bind_file(path: &Path, maximum: usize) -> Result<BoundFile, &'static str> {
    let canonical = path
        .canonicalize()
        .map_err(|_| "bound file is unavailable")?;
    if !canonical.is_absolute() {
        return Err("bound file is not one canonical path");
    }
    let metadata = fs::symlink_metadata(&canonical).map_err(|_| "bound file is unavailable")?;
    if metadata.file_type().is_symlink()
        || !metadata.is_file()
        || metadata.len() as usize > maximum
        || metadata.nlink() != 1
    {
        return Err("bound file is unsafe or too large");
    }
    let bytes = fs::read(&canonical).map_err(|_| "bound file could not be read")?;
    let after = fs::symlink_metadata(&canonical).map_err(|_| "bound file changed")?;
    if metadata.dev() != after.dev()
        || metadata.ino() != after.ino()
        || metadata.len() != after.len()
        || metadata.mtime() != after.mtime()
        || metadata.mtime_nsec() != after.mtime_nsec()
    {
        return Err("bound file changed during capture");
    }
    Ok(BoundFile {
        canonical_path: path_string(&canonical)?,
        sha256: digest_bytes(&bytes),
        byte_length: metadata.len(),
        unix_mode: metadata.mode(),
    })
}

fn bind_personal_file(
    home: &PersonalHomeAuthority,
    relative: &Path,
    maximum: usize,
) -> Result<(BoundFile, Vec<u8>), &'static str> {
    home.revalidate()?;
    let mut file = open_relative_file(&home.codex_home, relative)?;
    let metadata = file
        .metadata()
        .map_err(|_| "personal authority file is unavailable")?;
    if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.nlink() != 1 {
        return Err("personal authority file is unsafe");
    }
    if metadata.len() > maximum as u64 {
        return Err("personal authority file is too large");
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    Read::by_ref(&mut file)
        .take(maximum as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "personal authority file is unreadable")?;
    if bytes.len() > maximum {
        return Err("personal authority file is too large");
    }
    let after = file
        .metadata()
        .map_err(|_| "personal authority file changed")?;
    let rebound = open_relative_file(&home.codex_home, relative)?
        .metadata()
        .map_err(|_| "personal authority file changed")?;
    if !same_file_observation(&metadata, &after) || !same_file_observation(&metadata, &rebound) {
        return Err("personal authority file changed during capture");
    }
    let canonical_path = home.codex_home_path.join(relative);
    home.revalidate()?;
    Ok((
        BoundFile {
            canonical_path: path_string(&canonical_path)?,
            sha256: digest_bytes(&bytes),
            byte_length: metadata.len(),
            unix_mode: metadata.mode(),
        },
        bytes,
    ))
}

fn open_relative_file(root: &File, relative: &Path) -> Result<File, &'static str> {
    let components = relative.components().collect::<Vec<_>>();
    if components.is_empty() || components.len() > 32 {
        return Err("personal authority file path is invalid");
    }
    let mut current = root
        .try_clone()
        .map_err(|_| "personal authority descriptor is unavailable")?;
    for (index, component) in components.iter().enumerate() {
        let std::path::Component::Normal(name) = component else {
            return Err("personal authority file path is invalid");
        };
        let name = std::ffi::CString::new(name.as_bytes())
            .map_err(|_| "personal authority file path is invalid")?;
        let mut flags = libc::O_RDONLY | libc::O_CLOEXEC | libc::O_NOFOLLOW;
        if index + 1 != components.len() {
            flags |= libc::O_DIRECTORY;
        }
        let fd = unsafe { libc::openat(current.as_raw_fd(), name.as_ptr(), flags) };
        if fd < 0 {
            return Err("personal authority file is unavailable");
        }
        current = unsafe { File::from_raw_fd(fd) };
    }
    Ok(current)
}

fn same_file_observation(left: &fs::Metadata, right: &fs::Metadata) -> bool {
    left.dev() == right.dev()
        && left.ino() == right.ino()
        && left.len() == right.len()
        && left.mode() == right.mode()
        && left.uid() == right.uid()
        && left.mtime() == right.mtime()
        && left.mtime_nsec() == right.mtime_nsec()
}

fn canonical_directory(path: &Path, cause: &'static str) -> Result<PathBuf, &'static str> {
    let canonical = path.canonicalize().map_err(|_| cause)?;
    let metadata = fs::symlink_metadata(&canonical).map_err(|_| cause)?;
    if canonical != path || metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(cause);
    }
    Ok(canonical)
}

fn open_directory(path: &Path) -> Result<File, &'static str> {
    let mut options = OpenOptions::new();
    options
        .read(true)
        .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_DIRECTORY);
    options
        .open(path)
        .map_err(|_| "directory descriptor is unavailable")
}

fn directory_authority(metadata: &fs::Metadata) -> Result<DirectoryAuthority, &'static str> {
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err("authority object is not one directory");
    }
    Ok(DirectoryAuthority {
        device: metadata.dev(),
        inode: metadata.ino(),
        owner: metadata.uid(),
        mode: metadata.mode(),
    })
}

fn stat_child_directory(
    parent: std::os::fd::RawFd,
    name: &std::ffi::CStr,
) -> Result<DirectoryAuthority, &'static str> {
    let mut value = std::mem::MaybeUninit::<libc::stat>::uninit();
    let result = unsafe {
        libc::fstatat(
            parent,
            name.as_ptr(),
            value.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    };
    if result != 0 {
        return Err("child directory authority is unavailable");
    }
    let value = unsafe { value.assume_init() };
    if value.st_mode & libc::S_IFMT != libc::S_IFDIR {
        return Err("child authority is not one directory");
    }
    Ok(DirectoryAuthority {
        device: value.st_dev as u64,
        inode: value.st_ino as u64,
        owner: value.st_uid,
        mode: value.st_mode as u32,
    })
}

fn monotonic_successor(prior: &str, target: &str) -> bool {
    let prior = crate::plugin_product::lifecycle::Version::parse(prior);
    let target = crate::plugin_product::lifecycle::Version::parse(target);
    matches!(
        (prior, target),
        (Ok(prior), Ok(target))
            if target.precedence_cmp(&prior).ok() == Some(std::cmp::Ordering::Greater)
    )
}

fn selected_version_from_path(executable: &Path) -> Result<String, &'static str> {
    let release = executable
        .parent()
        .and_then(Path::parent)
        .and_then(Path::file_name)
        .and_then(|value| value.to_str())
        .ok_or("selected Codex version path is unavailable")?;
    let version = release
        .split_once('-')
        .map(|(version, _)| version)
        .ok_or("selected Codex version path is unavailable")?;
    if version.is_empty()
        || version.len() > 64
        || !version
            .bytes()
            .all(|byte| byte.is_ascii_digit() || byte == b'.')
    {
        return Err("selected Codex version path is invalid");
    }
    Ok(version.to_owned())
}

fn cached_catalog_binds_plugin(bytes: &[u8]) -> bool {
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(bytes) else {
        return false;
    };
    value
        .get("plugins")
        .and_then(serde_json::Value::as_array)
        .is_some_and(|rows| {
            rows.len() == 1
                && rows[0].get("name").and_then(serde_json::Value::as_str) == Some(PLUGIN_NAME)
                && rows[0]
                    .get("source")
                    .and_then(serde_json::Value::as_object)
                    .and_then(|source| source.get("source"))
                    .and_then(serde_json::Value::as_str)
                    == Some("local")
                && rows[0]
                    .get("source")
                    .and_then(serde_json::Value::as_object)
                    .and_then(|source| source.get("path"))
                    .and_then(serde_json::Value::as_str)
                    == Some("./plugins/harness-ultragoal")
        })
}

fn configured_marketplace_is_exact(bytes: &[u8], expected_source: &Path) -> bool {
    let Ok(text) = std::str::from_utf8(bytes) else {
        return false;
    };
    let Ok(value) = toml::from_str::<toml::Value>(text) else {
        return false;
    };
    let marketplace = value
        .get("marketplaces")
        .and_then(toml::Value::as_table)
        .and_then(|marketplaces| marketplaces.get(MARKETPLACE))
        .and_then(toml::Value::as_table);
    let plugin = value
        .get("plugins")
        .and_then(toml::Value::as_table)
        .and_then(|plugins| plugins.get(PLUGIN_ID))
        .and_then(toml::Value::as_table);
    marketplace.is_some_and(|marketplace| {
        marketplace.get("source_type").and_then(toml::Value::as_str) == Some("local")
            && marketplace.get("source").and_then(toml::Value::as_str) == expected_source.to_str()
    }) && plugin
        .is_some_and(|plugin| plugin.get("enabled").and_then(toml::Value::as_bool) == Some(true))
}

fn contains_bytes(haystack: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty() && haystack.windows(needle.len()).any(|row| row == needle)
}

fn path_string(path: &Path) -> Result<String, &'static str> {
    path.to_str()
        .filter(|value| !value.is_empty() && value.len() <= 4096)
        .map(str::to_owned)
        .ok_or("path is not one bounded UTF-8 path")
}

fn valid_hex_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn digest_json(value: &impl Serialize) -> Result<String, &'static str> {
    serde_json::to_vec(value)
        .map(|bytes| digest_bytes(&bytes))
        .map_err(|_| "identity encoding failed")
}

fn digest_bytes(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
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
    let input = arguments
        .iter()
        .find(|argument| argument.name == OptionName::Input)?;
    let cli = arguments
        .iter()
        .find(|argument| argument.name == OptionName::Cli)?;
    match (&input.value, &cli.value) {
        (ParsedValue::RelativePath(input), ParsedValue::RelativePath(cli))
            if super::package_dispatch::package_archive_input_allowed(input.as_str())
                && super::package_cli_payload::allowed(cli.as_str()) =>
        {
            Some((input.as_str(), cli.as_str()))
        }
        _ => None,
    }
}

fn verify_argument(invocation: &ParsedInvocation) -> Option<&Path> {
    let ParsedInvocation {
        command: SuccessorCommand::Package(PackageAction::InstallVerify),
        effect: EffectClass::Read,
        arguments,
        ..
    } = invocation
    else {
        return None;
    };
    match arguments.as_slice() {
        [argument] if argument.name == OptionName::Handoff => match &argument.value {
            ParsedValue::HostPath(path) if path.is_valid() => Some(path.as_path()),
            _ => None,
        },
        _ => None,
    }
}

fn invalid_plan_invocation() -> RuntimeOutcome {
    failure(
        DiagnosticId::UnexpectedArguments,
        ExitClass::InvalidInvocation,
        "package install-plan requires exact --input and --cli paths",
        "supply the exact current package archive and release CLI",
        "ultragoal --json package install-plan --input target/ultragoal/package-a.hugpkg --cli target/ultragoal/release/ultragoal",
    )
}

fn invalid_verify_invocation() -> RuntimeOutcome {
    failure(
        DiagnosticId::UnexpectedArguments,
        ExitClass::InvalidInvocation,
        "package install-verify requires one immutable --handoff host path",
        "supply the exact owner-only handoff emitted by package install-plan",
        "ultragoal --json package install-verify --handoff <exact-handoff-file>",
    )
}

fn handoff_failure(cause: &'static str) -> RuntimeOutcome {
    let repair = match cause {
        "configured marketplace identity or source is not the exact workspace-local authority" => {
            "preserve personal state; establish one supported configured marketplace whose exact source is the catalog-bound workspace path, then rerun without asking UltraGoal to edit host state"
        }
        "durable workspace-local marketplace source is not materialized" => {
            "materialize the exact verified package at the already configured catalog-bound workspace source, reconcile its complete tree, then rerun"
        }
        _ => {
            "preserve personal state and restore exact package, host-listing, HOME, CODEX_HOME, config/profile, working-directory, and protected custody before deriving a handoff"
        }
    };
    failure(
        DiagnosticId::AuthorityRequired,
        ExitClass::BlockedAuthority,
        cause,
        repair,
        "ultragoal --json package install-plan --input target/ultragoal/package-a.hugpkg --cli target/ultragoal/release/ultragoal",
    )
}

fn verify_failure(cause: &'static str) -> RuntimeOutcome {
    failure(
        DiagnosticId::StaleContext,
        ExitClass::ActionableFinding,
        cause,
        "preserve personal state and re-observe it through one fresh immutable handoff; do not repair or retry through UltraGoal",
        "ultragoal --json package install-verify --handoff <exact-handoff-file>",
    )
}

fn failure(
    id: DiagnosticId,
    exit: ExitClass,
    cause: &'static str,
    repair: &'static str,
    rerun: &'static str,
) -> RuntimeOutcome {
    RuntimeOutcome::failure(
        exit,
        Diagnostic::new(
            id,
            exit,
            DiagnosticDetails {
                cause,
                affected_surface: "HCT-DISTRIBUTION supported personal-install handoff",
                repair,
                effect: "none",
                rerun,
                ceiling: "UltraGoal performed no personal marketplace, cache, config, registry, install, removal, retry, rollback, restore, or recovery effect",
            },
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(seed: char) -> String {
        format!("sha256:{}", seed.to_string().repeat(64))
    }

    fn file(path: &str, seed: char) -> BoundFile {
        BoundFile {
            canonical_path: path.to_owned(),
            sha256: digest(seed),
            byte_length: 1,
            unix_mode: 0o100600,
        }
    }

    fn installed(version: &str, seed: char) -> InstalledAuthority {
        let mut value = InstalledAuthority {
            schema_version: INSTALLED_SCHEMA.to_owned(),
            plugin_version: version.to_owned(),
            registry_observation_sha256: digest('1'),
            marketplace_source_path:
                "/home/.codex/local-marketplaces/harness-ultragoal-local/plugins/harness-ultragoal"
                    .to_owned(),
            marketplace_source_tree_sha256: digest(seed),
            marketplace_source_observation_sha256: digest('2'),
            marketplace_source_catalog_sha256: digest('3'),
            cache_path: format!(
                "/home/.codex/plugins/cache/local-harness-plugins/harness-ultragoal/{version}"
            ),
            cache_tree_sha256: digest(seed),
            cache_catalog_sha256: digest('4'),
            marketplace_runtime_sha256: digest(seed),
            cache_runtime_sha256: digest(seed),
            installed_authority_sha256: String::new(),
        };
        value.installed_authority_sha256 = installed_digest(&value).unwrap();
        value
    }

    fn record() -> PersonalMarketplaceInstallHandoff {
        let mut home = PersonalHomeAuthorityRecord {
            schema_version: "HarnessPersonalHomeAuthority-v2".to_owned(),
            canonical_home: "/home".to_owned(),
            canonical_codex_home: "/home/.codex".to_owned(),
            home: DirectoryAuthority {
                device: 1,
                inode: 2,
                owner: 3,
                mode: 0o40700,
            },
            codex_home: DirectoryAuthority {
                device: 1,
                inode: 4,
                owner: 3,
                mode: 0o40700,
            },
            authority_sha256: String::new(),
        };
        home.authority_sha256 = home_authority_digest(&home).unwrap();
        let mut host = HostContext {
            personal_home: home,
            config: file("/home/.codex/config.toml", '5'),
            profile_context: "default".to_owned(),
            working_directory: "/workspace".to_owned(),
            context_sha256: String::new(),
        };
        host.context_sha256 = host_context_digest(&host).unwrap();
        let target = TargetPackage {
            context_id: digest('6'),
            candidate_id: digest('7'),
            catalog_id: digest('8'),
            version: "0.0.42+codex.20260828085546".to_owned(),
            source_tree_sha256: digest('b'),
            archive_path: "target/ultragoal/package-a.hugpkg".to_owned(),
            archive_sha256: digest('c'),
            inventory_sha256: digest('d'),
            runtime_sha256: digest('e'),
        };
        let protected_state = ProtectedState {
            tracked_diff_sha256: digest('f'),
            marker_sha256: digest('a'),
            state_sha256: digest_json(&(digest('f'), digest('a'))).unwrap(),
        };
        let selected = SelectedCodex {
            executable: file("/usr/local/bin/codex", '9'),
            version: "0.150.1".to_owned(),
            plugin_help_sha256: digest('0'),
            registry_observation_sha256: digest('1'),
        };
        let action = SupportedAction {
            executable: "/usr/local/bin/codex".to_owned(),
            arguments: vec!["plugin".to_owned(), "add".to_owned(), PLUGIN_ID.to_owned()],
            working_directory: "/workspace".to_owned(),
        };
        let mut record = PersonalMarketplaceInstallHandoff {
            schema_version: HANDOFF_SCHEMA.to_owned(),
            handoff_sha256: String::new(),
            effect: "none".to_owned(),
            status: "ready-for-explicit-supported-codex-user-action".to_owned(),
            source: SourceCandidate {
                head_commit: "1".repeat(40),
                head_tree: "2".repeat(40),
                branch: "codex/test".to_owned(),
                status_sha256: digest('3'),
                worktree_diff_sha256: digest('4'),
                staged_diff_sha256: digest('5'),
                untracked_content_sha256: digest('6'),
                dirty: true,
            },
            target: target.clone(),
            release_cli: file("/workspace/target/ultragoal/release/ultragoal", 'e'),
            durable_marketplace_source: DurableMarketplaceSource {
                catalog: file("/workspace/.agents/plugins/marketplace.json", 'c'),
                catalog_name: "harness-ultragoal-local".to_owned(),
                canonical_path: "/workspace/plugins/harness-ultragoal".to_owned(),
                tree_sha256: target.source_tree_sha256.clone(),
                observation_sha256: digest('d'),
            },
            plugin_id: PLUGIN_ID.to_owned(),
            marketplace: MARKETPLACE.to_owned(),
            host,
            selected_codex: selected,
            predecessor: installed("0.0.41+codex.20260824093100", 'a'),
            expected_target_source_tree_sha256: target.source_tree_sha256.clone(),
            expected_target_cache_tree_sha256: target.source_tree_sha256.clone(),
            expected_target_runtime_sha256: target.runtime_sha256.clone(),
            protected_state,
            supported_action: action.clone(),
            recovery_action: action,
            consequences: vec!["bounded".to_owned()],
            cancellation_boundary: "before start is no effect".to_owned(),
            restart_rule: "fresh task".to_owned(),
            claim_ceiling: "handoff only".to_owned(),
        };
        record.handoff_sha256 = handoff_digest(&record).unwrap();
        record
    }

    #[test]
    fn handoff_identity_binds_every_authority_and_rejects_tampering() {
        let record = record();
        validate_handoff_identity(&record).unwrap();
        let mut cases = Vec::new();
        let mut source = record.clone();
        source.source.status_sha256 = digest('9');
        cases.push(source);
        let mut home = record.clone();
        home.host.personal_home.home.inode += 1;
        cases.push(home);
        let mut codex = record.clone();
        codex.selected_codex.version = "0.151.0".to_owned();
        cases.push(codex);
        let mut marketplace_source = record.clone();
        marketplace_source.durable_marketplace_source.tree_sha256 = digest('9');
        cases.push(marketplace_source);
        let mut profile = record.clone();
        profile.host.profile_context = "other".to_owned();
        cases.push(profile);
        let mut cwd = record.clone();
        cwd.host.working_directory = "/other".to_owned();
        cases.push(cwd);
        let mut protected = record.clone();
        protected.protected_state.marker_sha256 = digest('8');
        cases.push(protected);
        for changed in cases {
            assert!(validate_handoff_identity(&changed).is_err());
        }
    }

    #[test]
    fn verification_classifies_exact_target_prior_and_mixed_state() {
        let record = record();
        assert_eq!(
            classify_installation(&record, &record.predecessor),
            VerificationClassification::ExactPriorNoEffect
        );
        let mut target = installed(&record.target.version, 'b');
        target.marketplace_runtime_sha256 = record.target.runtime_sha256.clone();
        target.cache_runtime_sha256 = record.target.runtime_sha256.clone();
        target.installed_authority_sha256 = installed_digest(&target).unwrap();
        assert_eq!(
            classify_installation(&record, &target),
            VerificationClassification::ExactTarget
        );
        let mut mixed = target;
        mixed.cache_tree_sha256 = digest('c');
        mixed.installed_authority_sha256 = installed_digest(&mixed).unwrap();
        assert_eq!(
            classify_installation(&record, &mixed),
            VerificationClassification::PartialOrStale
        );
    }

    #[test]
    fn selected_action_is_current_supported_codex_only_and_never_an_executor() {
        let record = record();
        assert_eq!(
            record.supported_action.arguments,
            ["plugin", "add", "harness-ultragoal@local-harness-plugins"]
        );
        assert_eq!(record.supported_action, record.recovery_action);
        assert_eq!(record.effect, "none");
    }

    #[test]
    fn direct_marketplace_config_requires_exact_alias_source_and_enablement() {
        let source = Path::new("/workspace/plugins/harness-ultragoal");
        let exact = r#"
[marketplaces.local-harness-plugins]
source_type = "local"
source = "/workspace/plugins/harness-ultragoal"

[plugins."harness-ultragoal@local-harness-plugins"]
enabled = true
"#;
        assert!(configured_marketplace_is_exact(exact.as_bytes(), source));
        for changed in [
            exact.replace(
                "/workspace/plugins/harness-ultragoal",
                "/private/tmp/stale-harness-source",
            ),
            exact.replace("local-harness-plugins", "different-marketplace"),
            exact.replace("enabled = true", "enabled = false"),
        ] {
            assert!(!configured_marketplace_is_exact(changed.as_bytes(), source));
        }
    }

    #[test]
    fn active_and_packaged_readers_expose_no_personal_mutation_adapter() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let forbidden = [
            ["install", "apply"].join("-"),
            "reviewed lifecycle adapter".to_owned(),
            "restores the prior authority".to_owned(),
            "automatic retry".to_owned(),
        ];
        for relative in [
            "skills/improve-and-maintain/SKILL.md",
            "docs/install-and-visibility.md",
            "docs/plugin-resource-map.md",
            "validator/src/cli/successor/catalog/observability_and_package.rs",
            ".codex-plugin/plugin.json",
            "plugin-manifest-draft.json",
        ] {
            let bytes = fs::read_to_string(root.join(relative)).unwrap();
            for denied in &forbidden {
                assert!(
                    !bytes.contains(denied),
                    "active/package reader {relative} retained {denied}"
                );
            }
        }
    }
}
