use super::replay::Projection;
use super::{
    AcceptanceProposal, Actor, Binding, EffectSink, EventKind, EventLog, FileJournal, JournalHead,
    LeaseSpec, OrchestrationError, ReviewRecord, ScopePolicy, WorkGraph,
};

pub struct Orchestrator<S: EffectSink> {
    pub(crate) graph: WorkGraph,
    pub(crate) policy: ScopePolicy,
    pub(crate) binding: Binding,
    pub(crate) root: Actor,
    pub(crate) log: EventLog,
    pub(crate) projection: Projection,
    pub(crate) sink: S,
    pub(crate) journal: Option<FileJournal>,
    pub(crate) journal_head: Option<JournalHead>,
}

impl<S: EffectSink> Orchestrator<S> {
    pub fn grant_lease(&mut self, tick: u64, lease: LeaseSpec) -> Result<(), OrchestrationError> {
        self.append(self.root.clone(), tick, EventKind::LeaseGranted { lease })
    }

    pub fn start(&mut self, tick: u64, lease_id: &str) -> Result<(), OrchestrationError> {
        let owner = self.owner(lease_id)?;
        self.append(
            owner,
            tick,
            EventKind::WorkStarted {
                lease_id: lease_id.to_owned(),
            },
        )
    }

    pub fn heartbeat(&mut self, tick: u64, lease_id: &str) -> Result<(), OrchestrationError> {
        let owner = self.owner(lease_id)?;
        self.append(
            owner,
            tick,
            EventKind::Heartbeat {
                lease_id: lease_id.to_owned(),
            },
        )
    }

    pub fn assign_review(
        &mut self,
        tick: u64,
        lease_id: &str,
        reviewer: Actor,
    ) -> Result<(), OrchestrationError> {
        self.append(
            self.root.clone(),
            tick,
            EventKind::ReviewAssigned {
                lease_id: lease_id.to_owned(),
                reviewer,
            },
        )
    }

    pub fn record_review(
        &mut self,
        tick: u64,
        lease_id: &str,
        review: ReviewRecord,
    ) -> Result<(), OrchestrationError> {
        let actor = Actor::parse(&review.reviewer)?;
        let result_commitment_id = review.result_commitment_id.clone();
        self.append(
            actor,
            tick,
            EventKind::ReviewRecorded {
                lease_id: lease_id.to_owned(),
                review,
                result_commitment_id,
            },
        )
    }

    pub fn schedule_retry(
        &mut self,
        tick: u64,
        lease_id: &str,
        new_deadline_tick: u64,
    ) -> Result<(), OrchestrationError> {
        self.append(
            self.root.clone(),
            tick,
            EventKind::RetryScheduled {
                lease_id: lease_id.to_owned(),
                new_deadline_tick,
            },
        )
    }

    pub fn cancel(&mut self, tick: u64, lease_id: &str) -> Result<(), OrchestrationError> {
        self.append(
            self.root.clone(),
            tick,
            EventKind::LeaseCancelled {
                lease_id: lease_id.to_owned(),
            },
        )
    }

    pub fn interrupt_root(&mut self, tick: u64) -> Result<(), OrchestrationError> {
        self.append(self.root.clone(), tick, EventKind::RootInterrupted)
    }

    pub fn recover_root(&mut self, tick: u64) -> Result<(), OrchestrationError> {
        self.append(self.root.clone(), tick, EventKind::RootRecovered)
    }

    pub fn accept(
        &mut self,
        tick: u64,
        proposal: &AcceptanceProposal,
    ) -> Result<(), OrchestrationError> {
        proposal.validate()?;
        if proposal.binding != self.binding {
            return Err(OrchestrationError::StaleBinding);
        }
        let runtime = self
            .projection
            .leases
            .get(&proposal.lease_id)
            .ok_or(OrchestrationError::InvalidLease)?;
        let super::replay::LeasePhase::ReviewedPass {
            commitment,
            result_commitment_id,
            review_id,
        } = &runtime.phase
        else {
            return Err(OrchestrationError::InvalidTransition);
        };
        if proposal.commitment()? != *commitment
            || &proposal.result_commitment_id != result_commitment_id
            || &proposal.review_id != review_id
            || !proposal.root_decision_required
        {
            return Err(OrchestrationError::InvalidReview);
        }
        self.append(
            self.root.clone(),
            tick,
            EventKind::RootAccepted {
                proposal: proposal.clone(),
            },
        )
    }

    fn owner(&self, lease_id: &str) -> Result<Actor, OrchestrationError> {
        self.projection
            .leases
            .get(lease_id)
            .map(|runtime| runtime.spec.owner.clone())
            .ok_or(OrchestrationError::InvalidLease)
    }

    pub(crate) fn append(
        &mut self,
        actor: Actor,
        tick: u64,
        event: EventKind,
    ) -> Result<(), OrchestrationError> {
        let item = self.log.next(&self.binding, actor, tick, event)?;
        let projection = self.log.apply_next(
            &self.projection,
            &item,
            &self.binding,
            &self.root,
            &self.graph,
            &self.policy,
        )?;
        let next_binding = projection
            .current_binding
            .clone()
            .ok_or(OrchestrationError::ReplayMismatch)?;
        let next_head = if let (Some(journal), Some(head)) =
            (self.journal.as_ref(), self.journal_head.as_ref())
        {
            let mut next_log = self.log.clone();
            next_log.0.push(item.clone());
            Some(journal.append(head, &next_binding, &next_log)?)
        } else if self.journal.is_some() || self.journal_head.is_some() {
            return Err(OrchestrationError::JournalCorrupt);
        } else {
            None
        };
        self.log.0.push(item);
        self.binding = next_binding;
        self.projection = projection;
        if let Some(head) = next_head {
            self.journal_head = Some(head);
        }
        Ok(())
    }
}
