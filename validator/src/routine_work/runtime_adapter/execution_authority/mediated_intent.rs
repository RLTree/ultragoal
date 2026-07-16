use super::*;

impl RoutineMediatedIntent {
    pub(crate) fn new(
        request_id: String,
        protocol_id: String,
        intent: RoutineEffectIntent,
        seal: Arc<RequestSeal>,
    ) -> Self {
        Self {
            request_id,
            protocol_id,
            intent,
            seal,
        }
    }

    pub(crate) fn request_id(&self) -> &str {
        &self.request_id
    }

    pub(crate) fn protocol_id(&self) -> &str {
        &self.protocol_id
    }

    pub(crate) fn intent(&self) -> &RoutineEffectIntent {
        &self.intent
    }

    pub(crate) fn require_current(&self) -> Result<(), RoutineError> {
        self.seal.require_order(self.intent.plan_order())
    }

    pub(crate) fn advance(&self) -> Result<(), RoutineError> {
        self.seal.advance(self.intent.plan_order())
    }
}

pub(crate) fn mediation_rows(intents: &[RoutineEffectIntent]) -> Vec<MediatedExpectedRow> {
    intents
        .iter()
        .map(|intent| MediatedExpectedRow {
            intent_id: intent.intent_id.clone(),
            plan_order: intent.plan_order,
            node_id: intent.node_id.clone(),
        })
        .collect()
}
