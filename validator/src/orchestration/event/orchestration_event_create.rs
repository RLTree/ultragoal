impl OrchestrationEvent {
    pub fn create(
        sequence: u64,
        prior_event_id: Option<String>,
        binding: Binding,
        actor: Actor,
        logical_tick: u64,
        event: EventKind,
    ) -> Result<Self, OrchestrationError> {
        binding.validate()?;
        validate_actor_identifier(actor.as_str())?;
        if let Some(prior) = &prior_event_id {
            validate_digest(prior)?;
        }
        let envelope = EventEnvelope {
            sequence,
            prior_event_id: &prior_event_id,
            binding: &binding,
            actor: &actor,
            logical_tick,
            event: &event,
        };
        let bytes = serde_json::to_vec(&envelope).map_err(|_| OrchestrationError::InvalidEvent)?;
        let event_id = format!("sha256:{:x}", Sha256::digest(bytes));
        Ok(Self {
            sequence,
            prior_event_id,
            event_id,
            binding,
            actor,
            logical_tick,
            event,
        })
    }

    pub(crate) fn verify_identity(&self) -> Result<(), OrchestrationError> {
        let rebuilt = Self::create(
            self.sequence,
            self.prior_event_id.clone(),
            self.binding.clone(),
            self.actor.clone(),
            self.logical_tick,
            self.event.clone(),
        )?;
        if rebuilt.event_id != self.event_id {
            return Err(OrchestrationError::InvalidEvent);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EventLog(pub Vec<OrchestrationEvent>);

impl EventLog {
    pub fn events(&self) -> &[OrchestrationEvent] {
        &self.0
    }

    pub(crate) fn next(
        &self,
        binding: &Binding,
        actor: Actor,
        tick: u64,
        event: EventKind,
    ) -> Result<OrchestrationEvent, OrchestrationError> {
        if self.0.len() >= MAX_EVENTS {
            return Err(OrchestrationError::ResourceLimit);
        }
        OrchestrationEvent::create(
            self.0.len() as u64,
            self.0.last().map(|prior| prior.event_id.clone()),
            binding.clone(),
            actor,
            tick,
            event,
        )
    }

}
