#[derive(Clone, Copy)]
struct CacheObservationFixture<'a> {
    context_id: &'a str,
    candidate_id: &'a str,
    cache_root_id: &'a str,
    marketplace: &'a str,
    plugin_id: &'a str,
    version: &'a str,
    package_tree_sha256: &'a str,
}

impl<'a> CacheObservationFixture<'a> {
    fn exact(host: &'a HostCapabilityDeclaration, package_tree_sha256: &'a str) -> Self {
        Self {
            context_id: CONTEXT,
            candidate_id: CANDIDATE,
            cache_root_id: host.home_id(),
            marketplace: "local-harness-plugins",
            plugin_id: "harness-ultragoal",
            version: "0.0.11",
            package_tree_sha256,
        }
    }

    fn bytes(self) -> Vec<u8> {
        serde_json::to_vec(&json!({
            "schema":"harness-ultragoal.codex-cache-observation.v1",
            "context_id":self.context_id,
            "candidate_id":self.candidate_id,
            "cache_root_id":self.cache_root_id,
            "entries":[{
                "marketplace":self.marketplace,
                "plugin_id":self.plugin_id,
                "version":self.version,
                "package_tree_sha256":self.package_tree_sha256
            }]
        }))
        .unwrap()
    }

    fn expectation(self) -> CacheExpectation {
        CacheExpectation::new(
            self.context_id.into(),
            self.candidate_id.into(),
            self.cache_root_id.into(),
            self.marketplace.into(),
            "harness-ultragoal".into(),
            self.version.into(),
            self.package_tree_sha256.into(),
        )
        .unwrap()
    }

    fn reconcile(self) -> ultragoal::distribution::CacheSnapshot {
        reconcile_cache_read_only(&self.bytes(), &self.expectation())
            .expect("internally exact caller-authored cache observation")
    }
}
