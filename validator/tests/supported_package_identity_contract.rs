use serde_json::json;
use std::collections::BTreeSet;
use std::fs;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use ultragoal::distribution::{
    CacheExpectation, CodexPlugin, DistributionErrorId, ExpectedPrior, HostCapabilityDeclaration,
    InstallEffects, InstallPlan, InstallScope, JourneyBinding, MarketplaceScope, PackageEffects,
    PackageIdentity, PackagePlan, RuntimeProbePlan, SurfaceIdentity, build_package, install,
    observe_codex_marketplace, observe_discovery, plan_codex_marketplace, plan_package,
    reconcile_cache_read_only, registry_document, unavailable_marketplace,
    verify_bound_surface_chain, verify_marketplace, verify_package,
};
use ultragoal::orchestration::{
    Actor, ArtifactWorkspace, Binding, CanonicalPath, EffectClass, EffectGrant, LeaseSpec,
    OwnedScope, PrerequisiteEvidence, Principal, SafetyClass, ScopePolicy, WorkPackage,
    WorkerResultV1,
};

const CONTEXT: &str = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const CANDIDATE: &str = "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const SUBSTITUTE: &str = "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
const WRONG_HOME: &str = "sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
const WRONG_TREE: &str = "sha256:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";
const R3_CONTEXT: &str = "sha256:94705c615b06f6c59c08a3234713d0cf6c57b12d557bffe8121e7a8d18ae5168";
const R3_CANDIDATE: &str =
    "sha256:2f1c22491962bd8f3211d4b71709f98406c318b42bcacc48e8c9ff5fb2773ce1";
const R3_RESULT_PATH: &str =
    "docs/ultragoal-successor-live/worker-results/SUPPORTED-PACKAGE-IDENTITY-074.json";
const R3_WORK_PACKAGE_PATH: &str =
    "docs/ultragoal-successor-live/work-packages/SUPPORTED-PACKAGE-IDENTITY-074-R3.json";
const R3_WORK_PACKAGE_SHA256: &str =
    "sha256:a786a53849d5503b76908c6e0e2a4ec1a421be8f68d0180934bbe3746149c6b0";
static NEXT: AtomicU64 = AtomicU64::new(0);

#[derive(Default)]
struct Sink(Option<Vec<u8>>);

impl PackageEffects for Sink {
    fn read_package(&mut self, _: usize) -> Result<Option<Vec<u8>>, ()> {
        Ok(self.0.clone())
    }

    fn compare_exchange_package(
        &mut self,
        expected: Option<&str>,
        replacement: Option<&[u8]>,
    ) -> Result<bool, ()> {
        if self.0.as_deref().map(digest).as_deref() != expected {
            return Ok(false);
        }
        self.0 = replacement.map(<[u8]>::to_vec);
        Ok(true)
    }
}

#[derive(Default)]
struct Installed(Option<Vec<u8>>);

impl InstallEffects for Installed {
    fn read_installed(&mut self, _: &str, _: usize) -> Result<Option<Vec<u8>>, ()> {
        Ok(self.0.clone())
    }

    fn compare_exchange_installed(
        &mut self,
        _: &str,
        expected: &ExpectedPrior,
        replacement: Option<&[u8]>,
    ) -> Result<bool, ()> {
        let matches = match expected {
            ExpectedPrior::Absent => self.0.is_none(),
            ExpectedPrior::ExactDigest(expected) => {
                self.0.as_deref().map(digest).as_deref() == Some(expected)
            }
        };
        if matches {
            self.0 = replacement.map(<[u8]>::to_vec);
        }
        Ok(matches)
    }
}

struct Fixture(PathBuf);

impl Fixture {
    fn new(label: &str) -> Self {
        let base = std::env::var_os("HUL_SUPPORTED_PACKAGE_SCRATCH_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);
        let root = base.join(format!(
            "archive-{label}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(root.join("source")).unwrap();
        fs::create_dir_all(root.join("home")).unwrap();
        fs::create_dir_all(root.join("project")).unwrap();
        let manifest = json!({
            "name":"harness-ultragoal",
            "version":"0.0.11",
            "description":"Repository fit, routine work, diagnosis, proof, and migration.",
            "author":{"name":"Terry Noblin","email":"tree@terrynoblin.dev","url":"https://terrynoblin.dev"},
            "homepage":"https://terrynoblin.dev/harness-ultragoal",
            "repository":"https://github.com/terrynoblin/harness-ultragoal",
            "license":"UNLICENSED",
            "keywords":["agent-first","verification"],
            "skills":"./skills/",
            "interface":{
                "displayName":"Harness Ultragoal",
                "shortDescription":"One evidence-bound front door.",
                "longDescription":"Route repository work through explicit authority and effects.",
                "developerName":"Terry Noblin",
                "category":"Productivity",
                "capabilities":["Read","Write"],
                "websiteURL":"https://terrynoblin.dev/harness-ultragoal",
                "defaultPrompt":["Classify this repository task and route it safely."],
                "brandColor":"#3B82F6"
            }
        });
        fs::write(
            root.join("source/plugin.json"),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();
        fs::write(
            root.join("source/skill-one.md"),
            b"---\nname: harness-ultragoal\ndescription: Harness front door\n---\n",
        )
        .unwrap();
        fs::write(
            root.join("source/skill-two.md"),
            b"---\nname: prove\ndescription: Proof workflow\n---\n",
        )
        .unwrap();
        let runtime = root.join("runtime-probe.sh");
        fs::write(
            &runtime,
            br##"#!/bin/sh
candidate="${1:-$HUL_CANDIDATE_ID}"
printf '%s\n' "HUL_RUNTIME_OBSERVATION={\"schema\":\"harness-ultragoal.runtime-probe.v1\",\"context_id\":\"$HUL_CONTEXT_ID\",\"candidate_id\":\"$candidate\",\"plugin_id\":\"$HUL_PLUGIN_ID\",\"version\":\"$HUL_VERSION\",\"package_sha256\":\"$HUL_PACKAGE_SHA256\",\"installed_tree_sha256\":\"$HUL_TREE_SHA256\",\"home_id\":\"$HUL_HOME_ID\",\"project_id\":\"$HUL_PROJECT_ID\",\"host_id\":\"$HUL_HOST_ID\",\"capability_sha256\":\"$HUL_CAPABILITY_SHA256\",\"binding_sha256\":\"$HUL_BINDING_SHA256\",\"executable_sha256\":\"$HUL_EXECUTABLE_SHA256\",\"session_nonce\":\"$HUL_SESSION_NONCE\"}"
"##,
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&runtime, fs::Permissions::from_mode(0o755)).unwrap();
        }
        Self(root)
    }

    fn plan(&self) -> PackagePlan {
        let entries = [
            json!({"path":".codex-plugin/plugin.json","source_path":"source/plugin.json","role":"manifest","executable":false}),
            json!({"path":"skills/harness-ultragoal/SKILL.md","source_path":"source/skill-one.md","role":"skill","executable":false}),
            json!({"path":"skills/prove/SKILL.md","source_path":"source/skill-two.md","role":"skill","executable":false}),
        ];
        let spec = serde_json::to_vec(&json!({
            "schema":"harness-ultragoal.package-plan.v1",
            "context_id":CONTEXT,
            "candidate_id":CANDIDATE,
            "plugin_id":"harness-ultragoal",
            "version":"0.0.11",
            "source_date_epoch":1_700_000_000u64,
            "entries":entries
        }))
        .unwrap();
        plan_package(&self.0, &spec).unwrap()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

struct EntryLayout {
    path: Range<usize>,
    mode: usize,
    role: usize,
    length: usize,
    digest: Range<usize>,
    payload: Range<usize>,
}

struct Layout {
    metadata: Vec<Range<usize>>,
    epoch: usize,
    count: usize,
    entries: Vec<EntryLayout>,
}

fn layout(bytes: &[u8]) -> Layout {
    let mut offset = 8;
    let metadata = (0..7).map(|_| string_range(bytes, &mut offset)).collect();
    let epoch = offset;
    offset += 8;
    let count = offset;
    let entry_count = u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap()) as usize;
    offset += 4;
    let mut entries = Vec::new();
    for _ in 0..entry_count {
        let path = string_range(bytes, &mut offset);
        let mode = offset;
        offset += 4;
        let role = offset;
        offset += 1;
        let length = offset;
        let payload_length =
            u64::from_be_bytes(bytes[offset..offset + 8].try_into().unwrap()) as usize;
        offset += 8;
        let digest = string_range(bytes, &mut offset);
        let payload = offset..offset + payload_length;
        offset += payload_length;
        entries.push(EntryLayout {
            path,
            mode,
            role,
            length,
            digest,
            payload,
        });
    }
    assert_eq!(offset, bytes.len());
    Layout {
        metadata,
        epoch,
        count,
        entries,
    }
}

fn string_range(bytes: &[u8], offset: &mut usize) -> Range<usize> {
    let length = u16::from_be_bytes(bytes[*offset..*offset + 2].try_into().unwrap()) as usize;
    *offset += 2;
    let range = *offset..*offset + length;
    *offset += length;
    range
}

fn candidate() -> (Fixture, PackagePlan, Vec<u8>) {
    let fixture = Fixture::new("candidate");
    let plan = fixture.plan();
    let archive = build_package(&plan, &mut Sink::default())
        .unwrap()
        .archive()
        .to_vec();
    (fixture, plan, archive)
}

fn assert_archive_error(plan: &PackagePlan, bytes: &[u8], expected: DistributionErrorId) {
    assert_eq!(verify_package(plan, bytes).unwrap_err().id(), expected);
}

#[test]
#[cfg(unix)]
fn typed_identity_surfaces_bind_only_verified_same_candidate_observations() {
    let (fixture, plan, archive) = candidate();
    let package = verify_package(&plan, &archive).unwrap();
    let runtime_program = fixture.0.join("runtime-probe.sh");
    let host = HostCapabilityDeclaration::isolated(
        &fixture.0.join("home"),
        &fixture.0.join("project"),
        "isolated-contract-v1",
        Some(&runtime_program),
    )
    .unwrap();
    let binding =
        JourneyBinding::new(package.identity().clone(), &host, "local-harness-plugins").unwrap();
    let marketplace_plan = plan_codex_marketplace(
        None,
        None,
        "local-harness-plugins",
        "Local Harness Plugins",
        CodexPlugin::harness_ultragoal(),
        package.identity().clone(),
    )
    .unwrap();
    let marketplace = observe_codex_marketplace(
        marketplace_plan.replacement(),
        &marketplace_plan,
        MarketplaceScope::Personal,
    )
    .unwrap();
    assert_eq!(marketplace.plugin_id(), "harness-ultragoal");
    assert_eq!(marketplace.version(), "0.0.11");

    let install_plan = InstallPlan::new(
        CONTEXT.into(),
        CANDIDATE.into(),
        InstallScope::PersonalFixture,
        "fixture/package.hugpkg".into(),
        package.package_sha256().into(),
        ExpectedPrior::Absent,
    )
    .unwrap();
    let installed = install(&install_plan, &package, &mut Installed::default()).unwrap();

    let cache_bytes = serde_json::to_vec(&json!({
        "schema":"harness-ultragoal.codex-cache-observation.v1",
        "context_id":CONTEXT,
        "candidate_id":CANDIDATE,
        "cache_root_id":host.home_id(),
        "entries":[{
            "marketplace":"local-harness-plugins",
            "plugin_id":"harness-ultragoal",
            "version":"0.0.11",
            "package_tree_sha256":package.identity().tree_sha256()
        }]
    }))
    .unwrap();
    let cache_expectation = CacheExpectation::new(
        CONTEXT.into(),
        CANDIDATE.into(),
        host.home_id().into(),
        "local-harness-plugins".into(),
        "harness-ultragoal".into(),
        "0.0.11".into(),
        package.identity().tree_sha256().into(),
    )
    .unwrap();
    let cache = reconcile_cache_read_only(&cache_bytes, &cache_expectation).unwrap();
    assert_eq!(cache.context_id(), CONTEXT);
    assert_eq!(cache.candidate_id(), CANDIDATE);
    assert_eq!(cache.cache_root_id(), host.home_id());
    assert_eq!(cache.marketplace(), "local-harness-plugins");
    assert_eq!(cache.plugin_id(), "harness-ultragoal");
    assert_eq!(cache.version(), "0.0.11");
    assert_eq!(
        cache.package_tree_sha256(),
        package.identity().tree_sha256()
    );

    let registry = registry_document(&binding, true, true).unwrap();
    let discovery = observe_discovery(Some(&registry), &binding, &host).unwrap();
    let before = snapshot_tree(&fixture.0);
    let runtime_plan = RuntimeProbePlan::new(
        binding.clone(),
        &host,
        &runtime_program,
        Vec::new(),
        Duration::from_secs(5),
    )
    .unwrap();
    let (_, runtime) = runtime_plan.execute_bound().unwrap();

    let surfaces = [
        SurfaceIdentity::from_verified_marketplace(&marketplace, &binding).unwrap(),
        SurfaceIdentity::from_verified_install(installed.snapshot(), &binding).unwrap(),
        SurfaceIdentity::from_verified_cache(&cache, &binding).unwrap(),
        SurfaceIdentity::from_verified_discovery(&discovery, &binding).unwrap(),
        runtime,
    ];
    verify_bound_surface_chain(&surfaces, &binding).unwrap();
    assert_eq!(
        snapshot_tree(&fixture.0),
        before,
        "identity joins are zero-write"
    );

    let substituted = PackageIdentity::new(
        package.identity().source().clone(),
        package.identity().tree_sha256().into(),
        SUBSTITUTE.into(),
    )
    .unwrap();
    let wrong_binding = JourneyBinding::new(substituted, &host, "local-harness-plugins").unwrap();
    for result in [
        SurfaceIdentity::from_verified_marketplace(&marketplace, &wrong_binding),
        SurfaceIdentity::from_verified_install(installed.snapshot(), &wrong_binding),
        SurfaceIdentity::from_verified_discovery(&discovery, &wrong_binding),
    ] {
        assert_eq!(
            result.unwrap_err().id(),
            DistributionErrorId::ProvenanceMismatch
        );
    }
    let substituted_tree = PackageIdentity::new(
        package.identity().source().clone(),
        WRONG_TREE.into(),
        package.identity().archive_sha256().into(),
    )
    .unwrap();
    let wrong_tree_binding =
        JourneyBinding::new(substituted_tree, &host, "local-harness-plugins").unwrap();
    assert_eq!(
        SurfaceIdentity::from_verified_cache(&cache, &wrong_tree_binding)
            .unwrap_err()
            .id(),
        DistributionErrorId::ProvenanceMismatch
    );

    cache_identity_substitution_controls(&package, &binding, &host, &cache_bytes);

    let unavailable = ultragoal::distribution::MarketplaceExpectation::new(
        CONTEXT.into(),
        CANDIDATE.into(),
        MarketplaceScope::Personal,
        "harness-ultragoal".into(),
        "0.0.11".into(),
        "plugins/harness-ultragoal".into(),
        package.package_sha256().into(),
    )
    .and_then(|expectation| unavailable_marketplace(&expectation, "host-unavailable"))
    .unwrap();
    assert_eq!(
        SurfaceIdentity::from_verified_marketplace(&unavailable, &binding)
            .unwrap_err()
            .id(),
        DistributionErrorId::ProvenanceMismatch
    );

    let wrong_version_expectation = ultragoal::distribution::MarketplaceExpectation::new(
        CONTEXT.into(),
        CANDIDATE.into(),
        MarketplaceScope::Personal,
        "harness-ultragoal".into(),
        "9.9.9".into(),
        "plugins/harness-ultragoal".into(),
        package.package_sha256().into(),
    )
    .unwrap();
    let wrong_version_catalog = serde_json::to_vec(&json!({
        "schema":"harness-ultragoal.marketplace-catalog.v1",
        "context_id":CONTEXT,
        "candidate_id":CANDIDATE,
        "scope":"personal",
        "plugins":[{
            "plugin_id":"harness-ultragoal",
            "version":"9.9.9",
            "origin":"plugins/harness-ultragoal",
            "package_sha256":package.package_sha256()
        }]
    }))
    .unwrap();
    let wrong_version = verify_marketplace(&wrong_version_catalog, &wrong_version_expectation)
        .expect("wrong-version catalog is internally exact before package-source binding");
    assert_eq!(wrong_version.version(), "9.9.9");
    assert_eq!(wrong_version.package_sha256(), package.package_sha256());
    assert_eq!(
        SurfaceIdentity::from_verified_marketplace(&wrong_version, &binding)
            .unwrap_err()
            .id(),
        DistributionErrorId::ProvenanceMismatch,
        "same package digest cannot promote a different marketplace version"
    );

    let exact_version_catalog = serde_json::to_vec(&json!({
        "schema":"harness-ultragoal.marketplace-catalog.v1",
        "context_id":CONTEXT,
        "candidate_id":CANDIDATE,
        "scope":"personal",
        "plugins":[{
            "plugin_id":"harness-ultragoal",
            "version":"0.0.11",
            "origin":"plugins/harness-ultragoal",
            "package_sha256":package.package_sha256()
        }]
    }))
    .unwrap();
    assert_eq!(
        verify_marketplace(&exact_version_catalog, &wrong_version_expectation)
            .unwrap_err()
            .id(),
        DistributionErrorId::InstallConflict,
        "caller-authored expectation drift cannot relabel exact catalog bytes"
    );

    let hidden_registry = registry_document(&binding, true, false).unwrap();
    let hidden = observe_discovery(Some(&hidden_registry), &binding, &host).unwrap();
    assert_eq!(
        SurfaceIdentity::from_verified_discovery(&hidden, &binding)
            .unwrap_err()
            .id(),
        DistributionErrorId::ProvenanceMismatch
    );

    let substituted_runtime = RuntimeProbePlan::new(
        binding,
        &host,
        &runtime_program,
        vec!["sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd".into()],
        Duration::from_secs(5),
    )
    .unwrap();
    assert_eq!(
        substituted_runtime.execute_bound().unwrap_err().id(),
        DistributionErrorId::ProvenanceMismatch
    );
}

fn cache_identity_substitution_controls(
    package: &ultragoal::distribution::PackageSnapshot,
    binding: &JourneyBinding,
    host: &HostCapabilityDeclaration,
    exact_bytes: &[u8],
) {
    let tree = package.identity().tree_sha256();
    let observation = |context: &str,
                       candidate: &str,
                       root: &str,
                       marketplace: &str,
                       plugin: &str,
                       version: &str,
                       tree: &str| {
        serde_json::to_vec(&json!({
            "schema":"harness-ultragoal.codex-cache-observation.v1",
            "context_id":context,
            "candidate_id":candidate,
            "cache_root_id":root,
            "entries":[{
                "marketplace":marketplace,
                "plugin_id":plugin,
                "version":version,
                "package_tree_sha256":tree
            }]
        }))
        .unwrap()
    };
    let expectation = |context: &str,
                       candidate: &str,
                       root: &str,
                       marketplace: &str,
                       version: &str,
                       tree: &str| {
        CacheExpectation::new(
            context.into(),
            candidate.into(),
            root.into(),
            marketplace.into(),
            "harness-ultragoal".into(),
            version.into(),
            tree.into(),
        )
        .unwrap()
    };
    let reconciled = |bytes: Vec<u8>, expected: CacheExpectation| {
        reconcile_cache_read_only(&bytes, &expected)
            .expect("internally exact caller-authored cache observation")
    };

    let wrong_version = reconciled(
        observation(
            CONTEXT,
            CANDIDATE,
            host.home_id(),
            "local-harness-plugins",
            "harness-ultragoal",
            "9.9.9",
            tree,
        ),
        expectation(
            CONTEXT,
            CANDIDATE,
            host.home_id(),
            "local-harness-plugins",
            "9.9.9",
            tree,
        ),
    );
    assert_eq!(wrong_version.version(), "9.9.9");
    assert_eq!(
        SurfaceIdentity::from_verified_cache(&wrong_version, binding)
            .unwrap_err()
            .id(),
        DistributionErrorId::ProvenanceMismatch,
        "an exact 0.0.11 package cannot promote a same-tree 9.9.9 cache"
    );

    let wrong_root = reconciled(
        observation(
            CONTEXT,
            CANDIDATE,
            WRONG_HOME,
            "local-harness-plugins",
            "harness-ultragoal",
            "0.0.11",
            tree,
        ),
        expectation(
            CONTEXT,
            CANDIDATE,
            WRONG_HOME,
            "local-harness-plugins",
            "0.0.11",
            tree,
        ),
    );
    assert_eq!(wrong_root.cache_root_id(), WRONG_HOME);
    assert_eq!(
        SurfaceIdentity::from_verified_cache(&wrong_root, binding)
            .unwrap_err()
            .id(),
        DistributionErrorId::ProvenanceMismatch,
        "a caller-authored cache root cannot substitute for the journey home"
    );

    let wrong_tree = reconciled(
        observation(
            CONTEXT,
            CANDIDATE,
            host.home_id(),
            "local-harness-plugins",
            "harness-ultragoal",
            "0.0.11",
            WRONG_TREE,
        ),
        expectation(
            CONTEXT,
            CANDIDATE,
            host.home_id(),
            "local-harness-plugins",
            "0.0.11",
            WRONG_TREE,
        ),
    );
    assert_eq!(
        SurfaceIdentity::from_verified_cache(&wrong_tree, binding)
            .unwrap_err()
            .id(),
        DistributionErrorId::ProvenanceMismatch,
        "same-path cache tree substitution cannot promote"
    );

    for (context, candidate) in [(SUBSTITUTE, CANDIDATE), (CONTEXT, SUBSTITUTE)] {
        let substituted = reconciled(
            observation(
                context,
                candidate,
                host.home_id(),
                "local-harness-plugins",
                "harness-ultragoal",
                "0.0.11",
                tree,
            ),
            expectation(
                context,
                candidate,
                host.home_id(),
                "local-harness-plugins",
                "0.0.11",
                tree,
            ),
        );
        assert_eq!(
            SurfaceIdentity::from_verified_cache(&substituted, binding)
                .unwrap_err()
                .id(),
            DistributionErrorId::ProvenanceMismatch,
            "context and candidate substitution must fail at package promotion"
        );
    }

    let wrong_plugin = observation(
        CONTEXT,
        CANDIDATE,
        host.home_id(),
        "local-harness-plugins",
        "other-plugin",
        "0.0.11",
        tree,
    );
    assert_eq!(
        reconcile_cache_read_only(
            &wrong_plugin,
            &expectation(
                CONTEXT,
                CANDIDATE,
                host.home_id(),
                "local-harness-plugins",
                "0.0.11",
                tree,
            ),
        )
        .unwrap_err()
        .id(),
        DistributionErrorId::InstallConflict
    );

    let wrong_marketplace = observation(
        CONTEXT,
        CANDIDATE,
        host.home_id(),
        "other-marketplace",
        "harness-ultragoal",
        "0.0.11",
        tree,
    );
    assert_eq!(
        reconcile_cache_read_only(
            &wrong_marketplace,
            &expectation(
                CONTEXT,
                CANDIDATE,
                host.home_id(),
                "local-harness-plugins",
                "0.0.11",
                tree,
            ),
        )
        .unwrap_err()
        .id(),
        DistributionErrorId::InstallConflict
    );

    let self_consistent_wrong_marketplace = reconciled(
        observation(
            CONTEXT,
            CANDIDATE,
            host.home_id(),
            "attacker-marketplace",
            "harness-ultragoal",
            "0.0.11",
            tree,
        ),
        expectation(
            CONTEXT,
            CANDIDATE,
            host.home_id(),
            "attacker-marketplace",
            "0.0.11",
            tree,
        ),
    );
    assert_eq!(
        SurfaceIdentity::from_verified_cache(&self_consistent_wrong_marketplace, binding)
            .unwrap_err()
            .id(),
        DistributionErrorId::ProvenanceMismatch,
        "a self-consistent caller marketplace cannot substitute for journey authority"
    );

    let mut ambiguous: serde_json::Value = serde_json::from_slice(exact_bytes).unwrap();
    ambiguous["entries"].as_array_mut().unwrap().push(json!({
        "marketplace":"other-marketplace",
        "plugin_id":"harness-ultragoal",
        "version":"9.9.9",
        "package_tree_sha256":WRONG_TREE
    }));
    assert_eq!(
        reconcile_cache_read_only(
            &serde_json::to_vec(&ambiguous).unwrap(),
            &expectation(
                CONTEXT,
                CANDIDATE,
                host.home_id(),
                "local-harness-plugins",
                "0.0.11",
                tree,
            ),
        )
        .unwrap_err()
        .id(),
        DistributionErrorId::InstallConflict,
        "ambiguous plugin identities fail closed"
    );

    let other_home = Fixture::new("other-home");
    let other_host = HostCapabilityDeclaration::isolated(
        &other_home.0.join("home"),
        &other_home.0.join("project"),
        "isolated-contract-v1",
        None,
    )
    .unwrap();
    let other_binding = JourneyBinding::new(
        package.identity().clone(),
        &other_host,
        "local-harness-plugins",
    )
    .unwrap();
    let exact = reconcile_cache_read_only(
        exact_bytes,
        &expectation(
            CONTEXT,
            CANDIDATE,
            host.home_id(),
            "local-harness-plugins",
            "0.0.11",
            tree,
        ),
    )
    .unwrap();
    assert_eq!(
        SurfaceIdentity::from_verified_cache(&exact, &other_binding)
            .unwrap_err()
            .id(),
        DistributionErrorId::ProvenanceMismatch,
        "journey substitution changes the authoritative home"
    );
}

#[test]
fn independent_decoder_reinventories_without_writes_and_rejects_every_structural_substitution() {
    let (fixture, plan, archive) = candidate();
    let before = snapshot_tree(&fixture.0);
    let verified = verify_package(&plan, &archive).unwrap();
    assert_eq!(verified.archive(), archive);
    assert_eq!(
        snapshot_tree(&fixture.0),
        before,
        "verification is zero-write"
    );
    let index = layout(&archive);

    let mut mutations = Vec::new();
    let mut wrong_magic = archive.clone();
    wrong_magic[0] ^= 1;
    mutations.push(wrong_magic);
    let mut non_utf8 = archive.clone();
    non_utf8[index.metadata[0].start] = 0xff;
    mutations.push(non_utf8);
    let mut wrong_context = archive.clone();
    wrong_context[index.metadata[0].start + 7] = b'b';
    mutations.push(wrong_context);
    let mut timestamp = archive.clone();
    timestamp[index.epoch..index.epoch + 8].copy_from_slice(&1u64.to_be_bytes());
    mutations.push(timestamp);
    let mut zero_count = archive.clone();
    zero_count[index.count..index.count + 4].copy_from_slice(&0u32.to_be_bytes());
    mutations.push(zero_count);
    let mut wrong_mode = archive.clone();
    wrong_mode[index.entries[0].mode..index.entries[0].mode + 4]
        .copy_from_slice(&0o777u32.to_be_bytes());
    mutations.push(wrong_mode);
    let mut unknown_role = archive.clone();
    unknown_role[index.entries[0].role] = 255;
    mutations.push(unknown_role);
    let mut wrong_digest = archive.clone();
    let digest_byte = &mut wrong_digest[index.entries[0].digest.start + 7];
    *digest_byte = if *digest_byte == b'a' { b'b' } else { b'a' };
    mutations.push(wrong_digest);
    let mut wrong_payload = archive.clone();
    wrong_payload[index.entries[0].payload.start] ^= 1;
    mutations.push(wrong_payload);
    let mut reordered = archive.clone();
    reordered[index.entries[2].path.clone()].fill(b'0');
    mutations.push(reordered);
    let mut trailing = archive.clone();
    trailing.push(0);
    mutations.push(trailing);
    let truncated = archive[..archive.len() - 1].to_vec();
    mutations.push(truncated);

    for bytes in mutations {
        assert_archive_error(&plan, &bytes, DistributionErrorId::ArchiveMismatch);
    }

    let mut escape = archive.clone();
    let path = &index.entries[1].path;
    let replacement = format!("../{}", "x".repeat(path.len() - 3));
    escape[path.clone()].copy_from_slice(replacement.as_bytes());
    assert_archive_error(&plan, &escape, DistributionErrorId::InvalidPath);
}

#[test]
fn declared_count_entry_size_and_total_archive_limits_fail_before_unbounded_work() {
    let (_fixture, plan, archive) = candidate();
    let index = layout(&archive);
    let mut count = archive.clone();
    count[index.count..index.count + 4].copy_from_slice(&4097u32.to_be_bytes());
    assert_archive_error(&plan, &count, DistributionErrorId::ObjectTooLarge);

    let mut entry = archive.clone();
    entry[index.entries[0].length..index.entries[0].length + 8]
        .copy_from_slice(&(4u64 * 1024 * 1024 + 1).to_be_bytes());
    assert_archive_error(&plan, &entry, DistributionErrorId::ObjectTooLarge);

    let oversized = vec![0; 65 * 1024 * 1024 + 1];
    assert_archive_error(&plan, &oversized, DistributionErrorId::ObjectTooLarge);
}

#[test]
fn corrective_worker_result_is_typed_lease_bound_and_exact_set_verified() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    assert_eq!(
        digest(&fs::read(root.join(R3_WORK_PACKAGE_PATH)).unwrap()),
        R3_WORK_PACKAGE_SHA256,
        "receipt is bound to the exact root-issued corrective package"
    );
    let bytes = fs::read(root.join(R3_RESULT_PATH)).unwrap();
    let result = WorkerResultV1::parse_json(&bytes).expect("authoritative WorkerResultV1 parser");
    let (package, lease, policy) = corrective_lease();
    package.validate().expect("typed corrective work package");
    policy.validate().expect("typed corrective scope policy");
    lease
        .validate(&policy)
        .expect("typed corrective lease and policy binding");
    let before = result
        .artifacts
        .iter()
        .map(|row| (row.path.clone(), fs::read(root.join(&row.path)).unwrap()))
        .collect::<Vec<_>>();
    let verified = ArtifactWorkspace::new(root)
        .unwrap()
        .verify(&result, &lease, &package)
        .expect("ArtifactWorkspace exact digest and lease verification");
    let result_id = result
        .result_id()
        .expect("canonical WorkerResult result_id");
    assert_eq!(verified.result_id(), result_id);
    assert_eq!(verified.artifact_count(), 18);
    assert!(r3_artifact_set_is_exact(&result));
    assert_eq!(result.context_id, R3_CONTEXT);
    assert_eq!(result.candidate_identity["context_id"], R3_CONTEXT);
    assert_eq!(result.candidate_identity["candidate_id"], R3_CANDIDATE);
    assert_eq!(
        result.candidate_identity["work_package_sha256"],
        R3_WORK_PACKAGE_SHA256
    );
    assert_eq!(
        result.candidate_identity["root_head"],
        "1750d586f6561d9d6ba64887cdafa6a36f23e41e"
    );
    assert_eq!(
        result.candidate_identity["root_tree"],
        "bf2d3e53c4b9410df41fb94ddf9b56ea4cf1e58a"
    );
    let after = result
        .artifacts
        .iter()
        .map(|row| (row.path.clone(), fs::read(root.join(&row.path)).unwrap()))
        .collect::<Vec<_>>();
    assert_eq!(after, before, "typed receipt verification is zero-write");

    let mut missing: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    missing["artifacts"].as_array_mut().unwrap().remove(0);
    let missing = WorkerResultV1::parse_json(&serde_json::to_vec(&missing).unwrap())
        .expect("generic parser accepts a structurally valid but incomplete listed set");
    assert!(
        !r3_artifact_set_is_exact(&missing),
        "root-sealed exact-set validation rejects a missing artifact"
    );

    let mut replaced: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    replaced["artifacts"].as_array_mut().unwrap()[0]["path"] =
        json!("validator/src/distribution/mod.rs");
    let replaced = WorkerResultV1::parse_json(&serde_json::to_vec(&replaced).unwrap())
        .expect("generic parser accepts a structurally valid substituted row");
    assert!(
        !r3_artifact_set_is_exact(&replaced),
        "root-sealed exact-set validation rejects an unknown replacement"
    );
    eprintln!("SUPPORTED_PACKAGE_IDENTITY_R3_RESULT_ID={result_id}");
}

fn r3_artifact_set_is_exact(result: &WorkerResultV1) -> bool {
    let expected = BTreeSet::from([
        "validator/src/distribution/cache.rs",
        "validator/src/distribution/host_capability.rs",
        "validator/src/distribution/host_effect/executor/tests.rs",
        "validator/src/distribution/host_effect/lifecycle/binding.rs",
        "validator/src/distribution/host_effect/lifecycle/tests.rs",
        "validator/src/distribution/identity.rs",
        "validator/src/distribution/marketplace.rs",
        "validator/src/distribution/package/archive.rs",
        "validator/src/distribution/package/snapshot.rs",
        "validator/src/distribution/registry_observation.rs",
        "validator/src/distribution/runtime_probe.rs",
        "validator/src/plugin_product/host_lifecycle/session.rs",
        "validator/tests/distribution_contract/isolated_journey.rs",
        "validator/tests/distribution_contract/journey_adversarial.rs",
        "validator/tests/distribution_contract/observation_races.rs",
        "validator/tests/distribution_contract/runtime_session.rs",
        "validator/tests/plugin_host_lifecycle_contract/negative.rs",
        "validator/tests/supported_package_identity_contract.rs",
    ]);
    result
        .artifacts
        .iter()
        .map(|row| row.path.as_str())
        .collect::<BTreeSet<_>>()
        == expected
}

fn corrective_lease() -> (WorkPackage, LeaseSpec, ScopePolicy) {
    let owned_paths = canonical_paths(&[
        R3_RESULT_PATH,
        "validator/src/distribution/cache.rs",
        "validator/src/distribution/host_capability.rs",
        "validator/src/distribution/host_effect/executor/tests.rs",
        "validator/src/distribution/host_effect/lifecycle/binding.rs",
        "validator/src/distribution/host_effect/lifecycle/tests.rs",
        "validator/src/distribution/identity.rs",
        "validator/src/distribution/marketplace.rs",
        "validator/src/distribution/package/archive.rs",
        "validator/src/distribution/package/snapshot.rs",
        "validator/src/distribution/registry_observation.rs",
        "validator/src/distribution/runtime_probe.rs",
        "validator/src/plugin_product/host_lifecycle/session.rs",
        "validator/tests/distribution_contract/isolated_journey.rs",
        "validator/tests/distribution_contract/journey_adversarial.rs",
        "validator/tests/distribution_contract/observation_races.rs",
        "validator/tests/distribution_contract/runtime_session.rs",
        "validator/tests/plugin_host_lifecycle_contract/negative.rs",
        "validator/tests/supported_package_identity_contract.rs",
    ]);
    let semantic_symbols = BTreeSet::from([
        "distribution::supported-package-cache-identity".into(),
        "distribution::marketplace-bound-journey-identity".into(),
        "plugin_product::supported-host-marketplace-join".into(),
    ]);
    let effects = BTreeSet::from([
        EffectGrant::new(
            EffectClass::WorkspaceWrite,
            "supported-package-identity-source",
        )
        .unwrap(),
        EffectGrant::new(EffectClass::Process, "offline-validation").unwrap(),
    ]);
    let owned_scope = OwnedScope {
        paths: owned_paths.clone(),
        semantic_symbols: semantic_symbols.clone(),
        generated_outputs: BTreeSet::new(),
        fixtures: BTreeSet::new(),
        effects: effects.clone(),
    };
    let package = WorkPackage {
        node_id: "supported-package-identity".into(),
        dependencies: BTreeSet::from([
            "accepted-distribution-package-install-kernel".into(),
            "accepted-supported-host-plugin-lifecycle-coordinator".into(),
        ]),
        required_tools: BTreeSet::from(["cargo-nextest".into(), "rustfmt".into(), "shasum".into()]),
        safety_class: SafetyClass::IsolatedWorkspaceWrite,
        read_paths: BTreeSet::new(),
        owned_scope: owned_scope.clone(),
        prerequisites: BTreeSet::from([
            "preserve-prior-package-identity-candidate".into(),
            "root-retains-manifest-version-and-claim-authority".into(),
        ]),
        outputs: BTreeSet::from([
            "marketplace-bound-distribution-journey".into(),
            "typed-worker-result-receipt".into(),
        ]),
        acceptance: BTreeSet::from([
            "same-tree-different-version-rejected".into(),
            "wrong-cache-root-rejected".into(),
            "wrong-marketplace-rejected".into(),
            "worker-result-exact-artifact-set-verified".into(),
        ]),
        claim_effect: "none".into(),
    };
    let lease = LeaseSpec {
        lease_id: "SUPPORTED-PACKAGE-IDENTITY-074-R3".into(),
        run_id: "ultragoal-successor-live-20260713-package-marketplace-identity-r3-root".into(),
        node_id: package.node_id.clone(),
        principal: Principal::Worker,
        owner: Actor::parse("/root").unwrap(),
        binding: Binding::new(R3_CONTEXT, R3_CANDIDATE).unwrap(),
        safety_class: SafetyClass::IsolatedWorkspaceWrite,
        read_paths: BTreeSet::new(),
        owned_scope,
        prerequisite_evidence: PrerequisiteEvidence::default(),
        issued_tick: 1,
        heartbeat_deadline_tick: 1_000_000,
        max_retries: 2,
    };
    let policy = ScopePolicy {
        allowed_read_paths: package.read_paths.clone(),
        allowed_paths: owned_paths,
        allowed_semantic_prefixes: semantic_symbols,
        allowed_effects: effects,
        ..ScopePolicy::default()
    };
    (package, lease, policy)
}

fn canonical_paths(values: &[&str]) -> BTreeSet<CanonicalPath> {
    values
        .iter()
        .map(|value| CanonicalPath::parse(value).unwrap())
        .collect()
}

fn snapshot_tree(root: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    fn walk(root: &Path, current: &Path, rows: &mut Vec<(PathBuf, Vec<u8>)>) {
        let mut entries = fs::read_dir(current)
            .unwrap()
            .map(|row| row.unwrap())
            .collect::<Vec<_>>();
        entries.sort_by_key(|row| row.file_name());
        for entry in entries {
            let path = entry.path();
            if entry.file_type().unwrap().is_dir() {
                walk(root, &path, rows);
            } else {
                rows.push((
                    path.strip_prefix(root).unwrap().into(),
                    fs::read(path).unwrap(),
                ));
            }
        }
    }
    let mut rows = Vec::new();
    walk(root, root, &mut rows);
    rows
}

fn digest(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    format!("sha256:{:x}", Sha256::digest(bytes))
}
