use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Default)]
pub(super) struct TerminationFaults {
    primary_kill_refusals: usize,
    group_signal_refusals: usize,
    reap_status_refusals: usize,
    pipe_drain_refusals: usize,
    observations: Arc<FaultObservations>,
}

#[derive(Default)]
pub(super) struct FaultObservations {
    primary_kill_refusals: AtomicUsize,
    group_signal_refusals: AtomicUsize,
    reap_status_refusals: AtomicUsize,
    pipe_drain_refusals: AtomicUsize,
}

impl TerminationFaults {
    pub(super) fn injected(
        primary_kill_refusals: usize,
        group_signal_refusals: usize,
        reap_status_refusals: usize,
        pipe_drain_refusals: usize,
    ) -> (Self, Arc<FaultObservations>) {
        let observations = Arc::new(FaultObservations::default());
        (
            Self {
                primary_kill_refusals,
                group_signal_refusals,
                reap_status_refusals,
                pipe_drain_refusals,
                observations: Arc::clone(&observations),
            },
            observations,
        )
    }

    pub(super) fn refuse_primary_kill(&mut self) -> bool {
        refuse(
            &mut self.primary_kill_refusals,
            &self.observations.primary_kill_refusals,
        )
    }

    pub(super) fn refuse_group_signal(&mut self) -> bool {
        refuse(
            &mut self.group_signal_refusals,
            &self.observations.group_signal_refusals,
        )
    }

    pub(super) fn refuse_reap_status(&mut self) -> bool {
        refuse(
            &mut self.reap_status_refusals,
            &self.observations.reap_status_refusals,
        )
    }

    pub(super) fn refuse_pipe_drain(&mut self) -> bool {
        refuse(
            &mut self.pipe_drain_refusals,
            &self.observations.pipe_drain_refusals,
        )
    }
}

impl FaultObservations {
    pub(super) fn primary_kill_refusals(&self) -> usize {
        self.primary_kill_refusals.load(Ordering::SeqCst)
    }

    pub(super) fn group_signal_refusals(&self) -> usize {
        self.group_signal_refusals.load(Ordering::SeqCst)
    }

    pub(super) fn reap_status_refusals(&self) -> usize {
        self.reap_status_refusals.load(Ordering::SeqCst)
    }

    pub(super) fn pipe_drain_refusals(&self) -> usize {
        self.pipe_drain_refusals.load(Ordering::SeqCst)
    }
}

fn refuse(remaining: &mut usize, observed: &AtomicUsize) -> bool {
    if *remaining == 0 {
        return false;
    }
    *remaining -= 1;
    observed.fetch_add(1, Ordering::SeqCst);
    true
}
