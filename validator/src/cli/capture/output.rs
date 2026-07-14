use super::environment::InvocationSensitivity;
use sha2::{Digest, Sha256};
use std::io::Read;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
enum OutputDisposition {
    Public,
    WithheldSecretBearingInvocation,
}

impl OutputDisposition {
    const fn from_sensitivity(sensitivity: InvocationSensitivity) -> Self {
        match sensitivity {
            InvocationSensitivity::Public => Self::Public,
            InvocationSensitivity::SecretBearing => Self::WithheldSecretBearingInvocation,
        }
    }

    const fn is_withheld(self) -> bool {
        matches!(self, Self::WithheldSecretBearingInvocation)
    }
}

#[derive(Debug, serde::Serialize)]
pub(crate) struct CapturedOutput {
    encoding: &'static str,
    content_disposition: OutputDisposition,
    retained_bytes: Vec<u8>,
    captured_byte_length: u64,
    captured_sha256: String,
    truncated: bool,
    observation_limit_exceeded: bool,
    captured_byte_length_is_lower_bound: bool,
}

impl CapturedOutput {
    pub fn retained(&self) -> &[u8] {
        &self.retained_bytes
    }

    #[cfg(test)]
    pub fn captured_byte_length(&self) -> u64 {
        self.captured_byte_length
    }
}

pub(super) struct OutputBudget {
    limit: u64,
    disposition: OutputDisposition,
    observed: AtomicU64,
    exceeded: AtomicBool,
}

pub(super) struct PendingOutput {
    collector: Collector,
}

pub(super) struct StableOutputs {
    pub first: CapturedOutput,
    pub second: CapturedOutput,
    pub output_limit_exceeded: bool,
}

impl OutputBudget {
    #[cfg(test)]
    pub fn new(limit: usize) -> Self {
        Self::classified(limit, OutputDisposition::Public)
    }

    pub fn for_sensitivity(limit: usize, sensitivity: InvocationSensitivity) -> Self {
        Self::classified(limit, OutputDisposition::from_sensitivity(sensitivity))
    }

    #[cfg(test)]
    pub fn for_bound_secrets(limit: usize, secrets: &[Vec<u8>]) -> Self {
        Self::for_sensitivity(limit, InvocationSensitivity::from_bound_secrets(secrets))
    }

    fn classified(limit: usize, disposition: OutputDisposition) -> Self {
        Self {
            limit: limit as u64,
            disposition,
            observed: AtomicU64::new(0),
            exceeded: AtomicBool::new(false),
        }
    }

    fn claim(&self, requested: usize) -> usize {
        loop {
            let observed = self.observed.load(Ordering::SeqCst);
            let remaining = self.limit.saturating_sub(observed);
            if remaining == 0 {
                self.exceeded.store(true, Ordering::SeqCst);
                return 0;
            }
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

    pub fn exceeded(&self) -> bool {
        self.exceeded.load(Ordering::SeqCst)
    }

    #[cfg(test)]
    pub fn observed(&self) -> u64 {
        self.observed.load(Ordering::SeqCst)
    }

    pub fn empty_output(&self) -> CapturedOutput {
        Collector::new(0, self.disposition).finish(false)
    }

    pub fn finalize_streams(&self, first: PendingOutput, second: PendingOutput) -> StableOutputs {
        let output_limit_exceeded = self.exceeded();
        StableOutputs {
            first: first.finish(output_limit_exceeded),
            second: second.finish(output_limit_exceeded),
            output_limit_exceeded,
        }
    }
}

struct Collector {
    disposition: OutputDisposition,
    retained: Vec<u8>,
    captured: u64,
    hasher: Sha256,
    limit: usize,
}

impl Collector {
    fn new(limit: usize, disposition: OutputDisposition) -> Self {
        Self {
            disposition,
            retained: Vec::with_capacity(limit.min(64 * 1024)),
            captured: 0,
            hasher: Sha256::new(),
            limit,
        }
    }

    fn absorb(&mut self, bytes: &[u8]) {
        if self.disposition.is_withheld() {
            return;
        }
        self.hasher.update(bytes);
        self.captured = self.captured.saturating_add(bytes.len() as u64);
        let available = self.limit.saturating_sub(self.retained.len());
        self.retained
            .extend_from_slice(&bytes[..available.min(bytes.len())]);
    }

    fn finish(self, observation_limit_exceeded: bool) -> CapturedOutput {
        let withheld = self.disposition.is_withheld();
        let public_observation_limit_exceeded = !withheld && observation_limit_exceeded;
        CapturedOutput {
            encoding: if withheld {
                "withheld-secret-bearing-invocation-v1"
            } else {
                "raw-byte-array"
            },
            content_disposition: self.disposition,
            truncated: withheld
                || public_observation_limit_exceeded
                || self.captured > self.retained.len() as u64,
            retained_bytes: self.retained,
            captured_byte_length: self.captured,
            captured_sha256: format!("sha256:{:x}", self.hasher.finalize()),
            observation_limit_exceeded: public_observation_limit_exceeded,
            captured_byte_length_is_lower_bound: public_observation_limit_exceeded,
        }
    }
}

impl PendingOutput {
    fn finish(self, observation_limit_exceeded: bool) -> CapturedOutput {
        self.collector.finish(observation_limit_exceeded)
    }
}

pub(super) fn observe(
    mut reader: impl Read,
    limit: usize,
    budget: &OutputBudget,
) -> Result<PendingOutput, String> {
    let mut collector = Collector::new(limit, budget.disposition);
    let mut buffer = [0_u8; 16 * 1024];
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|_| "captured output stream read failed".to_owned())?;
        if read == 0 {
            break;
        }
        let allowed = budget.claim(read);
        collector.absorb(&buffer[..allowed]);
        if allowed < read {
            break;
        }
    }
    Ok(PendingOutput { collector })
}

#[cfg(test)]
#[path = "../../../tests/cli_contract/capture/shared_budget.rs"]
mod tests;

#[cfg(test)]
#[path = "../../../tests/cli_contract/capture/output_secret_safety.rs"]
mod secret_safety_tests;
