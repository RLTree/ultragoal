use super::*;

impl RoutineNoOpProjection {
    pub(crate) fn context_id(&self) -> &str {
        &self.context_id
    }
    pub(crate) fn candidate_id(&self) -> &str {
        &self.candidate_id
    }
    pub(crate) fn plan_id(&self) -> &str {
        &self.plan_id
    }
    pub(crate) fn selected(&self) -> &[String] {
        &self.selected
    }
    pub(crate) fn effect_intent_count(&self) -> usize {
        self.effect_intent_count
    }
}

pub(crate) enum PreparedRoutineExecution {
    NoOp(RoutineNoOpProjection),
    Effect(RoutineEffectRequest),
}

impl PreparedRoutineExecution {
    /// Stable identity for the exact execution shape that a host checkpoint may
    /// replay.  A checkpoint must never be selected by the coarser live
    /// context alone: the bound runner and read authority are part of an
    /// effect protocol, while a no-op has its own immutable projection.
    pub(crate) fn checkpoint_execution_id(&self) -> &str {
        match self {
            Self::NoOp(projection) => &projection.projection_id,
            Self::Effect(request) => request.protocol_id(),
        }
    }
}

pub(crate) const REQUEST_STAGE_PREPARED: u8 = 0;
pub(crate) const REQUEST_STAGE_MEDIATING: u8 = 1;
pub(crate) const REQUEST_STAGE_RECONCILED: u8 = 2;

pub(crate) struct RequestSeal {
    pub(crate) issuance: u64,
    pub(crate) seal_id: String,
    pub(crate) stage: AtomicU8,
    pub(crate) next_order: AtomicUsize,
}

impl RequestSeal {
    pub(crate) fn new(issuance: u64, seal_id: String) -> Self {
        Self {
            issuance,
            seal_id,
            stage: AtomicU8::new(REQUEST_STAGE_PREPARED),
            next_order: AtomicUsize::new(0),
        }
    }

    pub(crate) fn issuance(&self) -> u64 {
        self.issuance
    }

    pub(crate) fn matches(&self, expected: &str) -> bool {
        self.seal_id == expected
    }

    pub(crate) fn begin_mediation(&self) -> Result<(), RoutineError> {
        self.stage
            .compare_exchange(
                REQUEST_STAGE_PREPARED,
                REQUEST_STAGE_MEDIATING,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .map(|_| ())
            .map_err(|_| request_error("adapter-effect-request-replayed"))
    }

    pub(crate) fn require_order(&self, order: usize) -> Result<(), RoutineError> {
        if self.stage.load(Ordering::Acquire) != REQUEST_STAGE_MEDIATING {
            return Err(request_error("adapter-effect-request-not-mediating"));
        }
        if self.next_order.load(Ordering::Acquire) != order {
            return Err(request_error("adapter-intent-transition-order-invalid"));
        }
        Ok(())
    }

    pub(crate) fn advance(&self, order: usize) -> Result<(), RoutineError> {
        self.require_order(order)?;
        self.next_order
            .compare_exchange(order, order + 1, Ordering::AcqRel, Ordering::Acquire)
            .map(|_| ())
            .map_err(|_| request_error("adapter-intent-transition-replayed"))
    }

    pub(crate) fn require_complete(&self, expected: usize) -> Result<(), RoutineError> {
        if self.stage.load(Ordering::Acquire) != REQUEST_STAGE_MEDIATING
            || self.next_order.load(Ordering::Acquire) != expected
        {
            return Err(request_error("adapter-mediation-transition-incomplete"));
        }
        Ok(())
    }

    pub(crate) fn finish(&self, expected: usize) -> Result<(), RoutineError> {
        self.require_complete(expected)?;
        self.stage
            .compare_exchange(
                REQUEST_STAGE_MEDIATING,
                REQUEST_STAGE_RECONCILED,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .map(|_| ())
            .map_err(|_| request_error("adapter-mediation-transition-replayed"))
    }
}

/// Opaque one-use request. It intentionally implements neither Clone, Copy,
/// Serialize, nor Deserialize and can only be consumed by reconciliation.
#[must_use = "an effect request must be mediated once or explicitly discarded"]
pub(crate) struct RoutineEffectRequest {
    pub(crate) request_id: String,
    pub(crate) protocol_id: String,
    pub(crate) binding: RoutineBinding,
    pub(crate) graph_id: String,
    pub(crate) snapshot_id: String,
    pub(crate) plan_id: String,
    pub(crate) result_scope: String,
    pub(crate) intents: Vec<RoutineEffectIntent>,
    pub(crate) seal: Arc<RequestSeal>,
}
