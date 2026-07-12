use crate::distribution::{
    CacheExpectation, CacheReader, DistributionErrorId as ErrorId, HostCapabilityDeclaration,
    JourneyBinding, RegistryReader, observe_registry_file, reconcile_cache_file, registry_document,
};
use crate::journey_support::JourneyFixture;
use crate::support::{PLUGIN_ID, VERSION};
use serde_json::json;

struct MutatingCache {
    rows: Vec<Vec<u8>>,
    reads: usize,
}

impl CacheReader for MutatingCache {
    fn read_cache(&mut self, _: usize) -> Result<Option<Vec<u8>>, ()> {
        let value = self
            .rows
            .get(self.reads)
            .cloned()
            .or_else(|| self.rows.last().cloned());
        self.reads += 1;
        Ok(value)
    }
}

struct MutatingRegistry {
    rows: Vec<Vec<u8>>,
    reads: usize,
}

impl RegistryReader for MutatingRegistry {
    fn read_registry(&mut self, _: usize) -> Result<Option<Vec<u8>>, ()> {
        let value = self
            .rows
            .get(self.reads)
            .cloned()
            .or_else(|| self.rows.last().cloned());
        self.reads += 1;
        Ok(value)
    }
}

#[test]
fn cache_and_registry_substitution_during_final_revalidation_fail_closed() {
    let fixture = JourneyFixture::new("observation-races");
    let package = fixture.build("packages/current.hugpkg");
    let executable = crate::runtime_session::program();
    let host = HostCapabilityDeclaration::isolated(
        &fixture.root,
        &fixture.project,
        "isolated-host-v1",
        Some(&executable),
    )
    .unwrap();
    let binding = JourneyBinding::new(package.identity().clone(), &host).unwrap();
    let cache = |tree: &str| {
        serde_json::to_vec(&json!({
            "schema":"harness-ultragoal.codex-cache-observation.v1",
            "context_id":package.context_id(), "candidate_id":package.candidate_id(),
            "cache_root_id":host.home_id(),
            "entries":[{"marketplace":"local-harness-plugins","plugin_id":PLUGIN_ID,
                "version":VERSION,"package_tree_sha256":tree}]
        }))
        .unwrap()
    };
    let expected = CacheExpectation::new(
        package.context_id().into(),
        package.candidate_id().into(),
        host.home_id().into(),
        "local-harness-plugins".into(),
        PLUGIN_ID.into(),
        VERSION.into(),
        package.identity().tree_sha256().into(),
    )
    .unwrap();
    let mut cache_reader = MutatingCache {
        rows: vec![
            cache(package.identity().tree_sha256()),
            cache("sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd"),
        ],
        reads: 0,
    };
    assert_eq!(
        reconcile_cache_file(&mut cache_reader, &expected)
            .unwrap_err()
            .id(),
        ErrorId::ObjectChanged,
    );

    let mut registry_reader = MutatingRegistry {
        rows: vec![
            registry_document(&binding, true, true).unwrap(),
            registry_document(&binding, true, false).unwrap(),
        ],
        reads: 0,
    };
    assert_eq!(
        observe_registry_file(&mut registry_reader, &binding, &host)
            .unwrap_err()
            .id(),
        ErrorId::ObjectChanged,
    );
}
