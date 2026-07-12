use crate::orchestration::*;
use crate::support::{
    CountingSink, LIVE_LIB_BYTES, LIVE_MANIFEST_BYTES, LIVE_PRIOR_BYTES, binding, bootstrap,
    content_digest, digest, graph_one, policy, root,
};
use std::cell::Cell;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_JOURNAL: AtomicU64 = AtomicU64::new(1);

pub struct JournalRoot(PathBuf);

impl JournalRoot {
    pub fn new(label: &str) -> Self {
        let serial = NEXT_JOURNAL.fetch_add(1, Ordering::Relaxed);
        let path = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!(
            "orchestration-journal-{label}-{}-{serial}",
            std::process::id()
        ));
        if path.exists() {
            std::fs::remove_dir_all(&path).unwrap();
        }
        Self(path)
    }

    pub fn path(&self) -> &Path {
        &self.0
    }
}

pub struct LiveWorkspace {
    root: JournalRoot,
    observer: RootWorkspace,
}

impl LiveWorkspace {
    pub fn observer(&self) -> &RootWorkspace {
        &self.observer
    }

    pub fn path(&self) -> &Path {
        self.root.path()
    }
}

impl Drop for JournalRoot {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

pub fn integration_for(proposal: &AcceptanceProposal, tick: u64) -> RootIntegrationReceipt {
    RootIntegrationReceipt {
        schema_version: "RootIntegrationReceipt-v1".to_owned(),
        base_binding: binding(),
        integrated_binding: Binding::new(&digest('2'), &digest('3')).unwrap(),
        integrated_bootstrap: BootstrapEvidence {
            observed_tick: tick,
            completed_nodes: BTreeMap::from([(
                proposal.node_id.clone(),
                proposal.proposal_id().unwrap(),
            )]),
            available_tools: bootstrap().available_tools,
            satisfied_prerequisites: bootstrap().satisfied_prerequisites,
        },
        root_actor: "ultra-root".to_owned(),
        accepted_leases: BTreeMap::from([(
            proposal.lease_id.clone(),
            AcceptedLeaseIntegration {
                node_id: proposal.node_id.clone(),
                proposal_id: proposal.proposal_id().unwrap(),
                requested_root_changes_digest: proposal.requested_root_changes_digest.clone(),
                requested_root_change_count: proposal.requested_root_change_count,
            },
        )]),
        reconciled_request_digests: [proposal.requested_root_changes_digest.clone()]
            .into_iter()
            .collect(),
        invalidated_lease_ids: Default::default(),
        applied_changes: proposal.expected_root_changes.clone(),
        validation_receipts: BTreeMap::from([("root-reproduction".to_owned(), digest('5'))]),
    }
}

pub fn integration_intent_for(proposal: &AcceptanceProposal) -> RootIntegrationIntent {
    RootIntegrationIntent {
        schema_version: "RootIntegrationIntent-v1".to_owned(),
        base_binding: binding(),
        root_actor: "ultra-root".to_owned(),
        accepted_proposals: BTreeMap::from([(
            proposal.lease_id.clone(),
            proposal.proposal_id().unwrap(),
        )]),
        prior_digests: proposal
            .expected_root_changes
            .keys()
            .map(|path| (path.clone(), Some(content_digest(LIVE_PRIOR_BYTES))))
            .collect(),
        expected_digests: proposal.expected_root_changes.clone(),
    }
}

pub fn live_workspace_for_intent(intent: &RootIntegrationIntent) -> LiveWorkspace {
    let files = intent
        .expected_digests
        .iter()
        .map(|(path, expected)| {
            let bytes = match path.as_str() {
                "validator/src/lib.rs" => LIVE_LIB_BYTES,
                "plugin-manifest-draft.json" => LIVE_MANIFEST_BYTES,
                other => panic!("no live fixture bytes declared for {other}"),
            };
            assert_eq!(&content_digest(bytes), expected);
            (path.as_str(), bytes)
        })
        .collect::<Vec<_>>();
    live_workspace_with_files(&files)
}

pub fn live_workspace_with_files(files: &[(&str, &[u8])]) -> LiveWorkspace {
    let root = JournalRoot::new("live-integration");
    fs::create_dir(root.path()).unwrap();
    for (path, bytes) in files {
        let target = root.path().join(path);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::write(target, bytes).unwrap();
    }
    let observer = RootWorkspace::open(root.path()).unwrap();
    LiveWorkspace { root, observer }
}

pub fn prepare_full_integration(
    engine: &mut Orchestrator<CountingSink>,
    proposal: &AcceptanceProposal,
    start_tick: u64,
) -> LiveWorkspace {
    let intent = integration_intent_for(proposal);
    let workspace = live_workspace_for_intent(&intent);
    engine.begin_integration(start_tick, intent).unwrap();
    assert_eq!(
        engine
            .observe_integration(start_tick + 1, workspace.observer())
            .unwrap(),
        IntegrationDisposition::Full
    );
    workspace
}

pub fn durable_engine(label: &str) -> (Orchestrator<CountingSink>, Rc<Cell<usize>>, JournalRoot) {
    let journal = JournalRoot::new(label);
    let (sink, calls) = CountingSink::new();
    let engine = Orchestrator::new_durable(
        graph_one(),
        policy(),
        binding(),
        root(),
        bootstrap(),
        journal.path(),
        sink,
    )
    .expect("durable engine");
    (engine, calls, journal)
}
