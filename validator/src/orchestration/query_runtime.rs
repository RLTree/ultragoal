use super::{
    Actor, Binding, BootstrapEvidence, EffectSink, EventKind, EventLog, FileJournal, JournalHead,
    JournalSnapshot, OrchestrationError, OrchestrationEvent, Orchestrator, Plan, ScopePolicy,
    WorkGraph, WorkProgress,
};
use std::path::Path;

impl<S: EffectSink> Orchestrator<S> {
    pub fn new(
        graph: WorkGraph,
        policy: ScopePolicy,
        binding: Binding,
        root: Actor,
        bootstrap: BootstrapEvidence,
        sink: S,
    ) -> Result<Self, OrchestrationError> {
        binding.validate()?;
        policy.validate()?;
        bootstrap.validate()?;
        let first = OrchestrationEvent::create(
            0,
            None,
            binding.clone(),
            root.clone(),
            bootstrap.observed_tick,
            EventKind::Bootstrapped {
                evidence: bootstrap,
            },
        )?;
        let log = EventLog(vec![first]);
        let projection = log.replay(&binding, &root, &graph, &policy)?;
        Ok(Self {
            graph,
            policy,
            binding,
            root,
            log,
            projection,
            sink,
            journal: None,
            journal_head: None,
        })
    }

    pub fn new_durable(
        graph: WorkGraph,
        policy: ScopePolicy,
        binding: Binding,
        root: Actor,
        bootstrap: BootstrapEvidence,
        journal_root: impl AsRef<Path>,
        sink: S,
    ) -> Result<Self, OrchestrationError> {
        let mut engine = Self::new(graph, policy, binding, root, bootstrap, sink)?;
        let (journal, head) = FileJournal::create(journal_root, &engine.binding, &engine.log)?;
        engine.journal = Some(journal);
        engine.journal_head = Some(head);
        Ok(engine)
    }

    pub fn restart(
        graph: WorkGraph,
        policy: ScopePolicy,
        binding: Binding,
        root: Actor,
        log: EventLog,
        sink: S,
    ) -> Result<Self, OrchestrationError> {
        binding.validate()?;
        policy.validate()?;
        let projection = log.replay(&binding, &root, &graph, &policy)?;
        Ok(Self {
            graph,
            policy,
            binding,
            root,
            log,
            projection,
            sink,
            journal: None,
            journal_head: None,
        })
    }

    pub fn restart_durable(
        graph: WorkGraph,
        policy: ScopePolicy,
        expected_head: JournalHead,
        root: Actor,
        journal_root: impl AsRef<Path>,
        sink: S,
    ) -> Result<Self, OrchestrationError> {
        expected_head.binding.validate()?;
        policy.validate()?;
        let journal = FileJournal::open(journal_root)?;
        let snapshot = journal.inspect()?;
        if snapshot.head != expected_head {
            return Err(OrchestrationError::JournalConflict);
        }
        let binding = expected_head.binding.clone();
        let projection = snapshot
            .log
            .replay_persisted(&binding, &root, &graph, &policy)?;
        Ok(Self {
            graph,
            policy,
            binding,
            root,
            log: snapshot.log,
            projection,
            sink,
            journal: Some(journal),
            journal_head: Some(snapshot.head),
        })
    }

    /// Replays a journal snapshot that was already authenticated by the
    /// durable journal reader without attaching any write capability.
    pub(crate) fn restart_verified_snapshot(
        graph: WorkGraph,
        policy: ScopePolicy,
        root: Actor,
        snapshot: JournalSnapshot,
        sink: S,
    ) -> Result<Self, OrchestrationError> {
        snapshot.head.binding.validate()?;
        policy.validate()?;
        let binding = snapshot.head.binding.clone();
        let projection = snapshot
            .log
            .replay_persisted(&binding, &root, &graph, &policy)?;
        Ok(Self {
            graph,
            policy,
            binding,
            root,
            log: snapshot.log,
            projection,
            sink,
            journal: None,
            journal_head: Some(snapshot.head),
        })
    }

    /// Pure query. It neither receives nor calls the effect sink.
    pub fn plan(&self) -> Result<Plan, OrchestrationError> {
        self.graph.plan(&WorkProgress {
            completed_nodes: self.projection.completed_nodes.clone(),
            active_nodes: self.projection.active_nodes(),
            available_tools: self.projection.available_tools.keys().cloned().collect(),
        })
    }

    /// Pure query over immutable in-memory events.
    pub fn events(&self) -> &[super::OrchestrationEvent] {
        self.log.events()
    }

    pub fn event_log(&self) -> EventLog {
        self.log.clone()
    }

    pub fn binding(&self) -> &Binding {
        &self.binding
    }

    pub fn journal_head(&self) -> Option<&JournalHead> {
        self.journal_head.as_ref()
    }
}
