use super::environment::InvocationSensitivity;
use std::io::Read;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

pub(super) struct CapturedOutput(Vec<u8>);

impl CapturedOutput {
    pub(super) fn retained(&self) -> &[u8] {
        &self.0
    }
}

pub(super) struct OutputBudget {
    limit: u64,
    observed: AtomicU64,
    exceeded: AtomicBool,
}

pub(super) struct PendingOutput(Vec<u8>);

pub(super) struct StableOutputs {
    pub(super) first: CapturedOutput,
    pub(super) output_limit_exceeded: bool,
}

impl OutputBudget {
    pub(super) fn for_sensitivity(limit: usize, _: InvocationSensitivity) -> Self {
        Self {
            limit: limit as u64,
            observed: AtomicU64::new(0),
            exceeded: AtomicBool::new(false),
        }
    }

    fn claim(&self, requested: usize) -> usize {
        loop {
            let observed = self.observed.load(Ordering::SeqCst);
            let remaining = self.limit.saturating_sub(observed);
            let claimed = remaining.min(requested as u64);
            if self
                .observed
                .compare_exchange(
                    observed,
                    observed + claimed,
                    Ordering::SeqCst,
                    Ordering::SeqCst,
                )
                .is_ok()
            {
                if claimed < requested as u64 {
                    self.exceeded.store(true, Ordering::SeqCst);
                }
                return claimed as usize;
            }
        }
    }

    pub(super) fn exceeded(&self) -> bool {
        self.exceeded.load(Ordering::SeqCst)
    }

    pub(super) fn finalize_streams(&self, first: PendingOutput, _: PendingOutput) -> StableOutputs {
        StableOutputs {
            first: CapturedOutput(first.0),
            output_limit_exceeded: self.exceeded(),
        }
    }
}

pub(super) fn observe(
    mut reader: impl Read,
    _: usize,
    budget: &OutputBudget,
) -> Result<PendingOutput, String> {
    let mut retained = Vec::new();
    let mut buffer = [0_u8; 4096];
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|_| "fixture output read failed".to_owned())?;
        if read == 0 {
            break;
        }
        let claimed = budget.claim(read);
        retained.extend_from_slice(&buffer[..claimed]);
        if claimed < read {
            break;
        }
    }
    Ok(PendingOutput(retained))
}
