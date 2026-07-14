pub fn digest(byte: char) -> String {
    format!("sha256:{}", byte.to_string().repeat(64))
}

pub const LIVE_LIB_BYTES: &[u8] = b"pub mod orchestration;\n";
pub const LIVE_MANIFEST_BYTES: &[u8] = b"{\"name\":\"harness-ultragoal\"}\n";
pub const LIVE_PRIOR_BYTES: &[u8] = b"prior root bytes\n";

pub fn content_digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

pub fn binding() -> Binding {
    Binding::new(&digest('a'), &digest('b')).expect("binding")
}

pub fn root() -> Actor {
    Actor::parse("ultra-root").expect("root actor")
}

pub fn bootstrap() -> BootstrapEvidence {
    BootstrapEvidence {
        observed_tick: 0,
        completed_nodes: BTreeMap::new(),
        available_tools: BTreeMap::from([("hct-context".to_owned(), digest('9'))]),
        satisfied_prerequisites: BTreeMap::from([("current-context".to_owned(), digest('8'))]),
    }
}

pub fn worker(name: &str) -> Actor {
    Actor::parse(name).expect("worker actor")
}

pub fn path(value: &str) -> CanonicalPath {
    CanonicalPath::parse(value).expect("canonical path")
}

pub fn effect(class: EffectClass, target: &str) -> EffectGrant {
    EffectGrant::new(class, target).expect("effect")
}

pub fn policy() -> ScopePolicy {
    ScopePolicy {
        allowed_read_paths: BTreeSet::from([path("docs/ultragoal-successor-live")]),
        allowed_paths: BTreeSet::from([
            path("validator/src/orchestration"),
            path("validator/tests/orchestration_contract"),
        ]),
        allowed_semantic_prefixes: BTreeSet::from(["orchestration".to_owned()]),
        allowed_generated_outputs: BTreeSet::from([path("generated/orchestration")]),
        allowed_fixtures: BTreeSet::from([path("validator/tests/orchestration_contract")]),
        allowed_effects: BTreeSet::from([
            effect(EffectClass::WorkspaceWrite, "leased-source"),
            effect(EffectClass::FixtureWrite, "isolated-test"),
            effect(EffectClass::Process, "focused-test"),
        ]),
        root_only_paths: BTreeSet::from([
            path("Cargo.lock"),
            path("plugin-manifest-draft.json"),
            path("migration/authority-routes.json"),
        ]),
        root_only_semantic_prefixes: BTreeSet::from([
            "root::claims".to_owned(),
            "root::public_commands".to_owned(),
        ]),
        root_only_effect_classes: BTreeSet::from([
            EffectClass::Destructive,
            EffectClass::RootAuthority,
        ]),
    }
}

pub fn scope(stem: &str) -> OwnedScope {
    OwnedScope {
        paths: BTreeSet::from([path(&format!("validator/src/orchestration/{stem}.rs"))]),
        semantic_symbols: BTreeSet::from([format!("orchestration::{stem}")]),
        generated_outputs: BTreeSet::from([path(&format!("generated/orchestration/{stem}.json"))]),
        fixtures: BTreeSet::from([path(&format!(
            "validator/tests/orchestration_contract/{stem}.json"
        ))]),
        effects: BTreeSet::from([effect(
            EffectClass::WorkspaceWrite,
            &format!("leased-source/{stem}"),
        )]),
    }
}

pub fn package(node: &str, dependencies: &[&str], stem: &str) -> WorkPackage {
    WorkPackage {
        node_id: node.to_owned(),
        dependencies: dependencies
            .iter()
            .map(|value| (*value).to_owned())
            .collect(),
        required_tools: BTreeSet::from(["hct-context".to_owned()]),
        safety_class: SafetyClass::IsolatedWorkspaceWrite,
        read_paths: BTreeSet::from([path("docs/ultragoal-successor-live")]),
        owned_scope: scope(stem),
        prerequisites: BTreeSet::from(["current-context".to_owned()]),
        outputs: BTreeSet::from([format!("{node}-candidate")]),
        acceptance: BTreeSet::from([format!("{node}-independent-review")]),
        claim_effect: "observation-only".to_owned(),
    }
}

pub fn graph_one() -> WorkGraph {
    WorkGraph::derive(vec![package("node-a", &[], "node_a")]).expect("graph")
}

pub fn lease_with_scope(
    lease_id: &str,
    node: &str,
    owner: &str,
    owned_scope: OwnedScope,
) -> LeaseSpec {
    LeaseSpec {
        lease_id: lease_id.to_owned(),
        run_id: "run-001".to_owned(),
        node_id: node.to_owned(),
        principal: Principal::Worker,
        owner: worker(owner),
        binding: binding(),
        safety_class: SafetyClass::IsolatedWorkspaceWrite,
        read_paths: BTreeSet::from([path("docs/ultragoal-successor-live")]),
        owned_scope,
        prerequisite_evidence: PrerequisiteEvidence {
            dependency_nodes: BTreeMap::new(),
            required_tools: BTreeMap::from([("hct-context".to_owned(), digest('9'))]),
            prerequisites: BTreeMap::from([("current-context".to_owned(), digest('8'))]),
        },
        issued_tick: 1,
        heartbeat_deadline_tick: 20,
        max_retries: 2,
    }
}

pub fn lease() -> LeaseSpec {
    lease_with_scope("lease-001", "node-a", "worker-a", scope("node_a"))
}

pub fn record(key: &str, value: Value) -> BTreeMap<String, Value> {
    BTreeMap::from([(key.to_owned(), value)])
}

pub fn commitment_id(result: &WorkerResultV1) -> String {
    let node = result.touched_semantics[0]
        .trim_start_matches("orchestration::")
        .replace('_', "-");
    AcceptanceProposal::result_commitment_id_for_parts(&binding(), &node, &result.lease_id, result)
        .unwrap()
}
