impl ConfinedHostEffectTarget {
    pub(super) fn require_clean_publication_name(
        &self,
        target_name: &str,
    ) -> Result<(), HostEffectExecutorFailure> {
        if !valid_component(target_name) || !target_name.ends_with(".json") {
            return Err(unsafe_object());
        }
        self.revalidate_anchor()?;
        let names = self.names()?;
        let temporary_prefix = format!(".{target_name}.");
        for name in names.iter().filter(|name| {
            name.as_str() == target_name
                || (name.starts_with(&temporary_prefix) && name.ends_with(".tmp"))
        }) {
            let observed = self.observe_object(name, 1, false)?;
            if observed.kind != PublicationObjectKind::Regular {
                return Err(unsafe_object());
            }
            return Err(HostEffectExecutorFailure::new(if name == target_name {
                HostEffectExecutorErrorId::Replay
            } else {
                HostEffectExecutorErrorId::TempCollision
            }));
        }
        Ok(())
    }

    pub(super) fn publish(
        &self,
        prepared: PreparedPublication,
        current_ledger_head: &super::super::HostEffectLedgerHead,
    ) -> Result<CommittedPublication, PublicationFailure> {
        match self.publish_inner(&prepared, current_ledger_head) {
            Ok(observation) => Ok(CommittedPublication {
                target_name: prepared.target_name,
                temporary_name: prepared.temporary_name,
                expectation: prepared.expectation,
                observation,
            }),
            Err(id) => {
                let observed = self.observe_inventory(InventoryObservationRequest {
                    target_name: &prepared.target_name,
                    expected_temporary_name: &prepared.temporary_name,
                    expectation: prepared.expectation.clone(),
                    ledger_head: current_ledger_head.clone(),
                    acknowledgement: None,
                    data_synced: false,
                });
                match observed {
                    Ok((observation, _classification)) => Err(PublicationFailure {
                        id,
                        target_name: prepared.target_name,
                        temporary_name: prepared.temporary_name,
                        expectation: prepared.expectation,
                        observation: Some(observation),
                    }),
                    Err(_) => Err(PublicationFailure {
                        id,
                        target_name: prepared.target_name,
                        temporary_name: prepared.temporary_name,
                        expectation: prepared.expectation,
                        observation: None,
                    }),
                }
            }
        }
    }
}
