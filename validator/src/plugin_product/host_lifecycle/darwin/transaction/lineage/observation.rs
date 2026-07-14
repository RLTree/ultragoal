use super::super::*;

impl DarwinHostTransactionAdapter {
    pub(in super::super) fn read_lineage(
        &self,
    ) -> Result<Option<LineageSnapshot>, DarwinHostError> {
        let Some(raw) = self.read_tree(LINEAGE_PATH, None, 1024 * 1024)? else {
            if self
                .lineage_floor
                .lock()
                .map_err(|_| DarwinHostError::new(DarwinHostErrorId::LineageConflict))?
                .is_some()
            {
                return Err(DarwinHostError::new(DarwinHostErrorId::LineageConflict));
            }
            return Ok(None);
        };
        let record = decode_lineage(&raw.bytes)?;
        let expected_steps = if record.operation == DarwinHostOperation::RepairCache {
            1
        } else {
            DarwinHostSurface::ALL.len()
        };
        let step_surfaces_valid = if record.operation == DarwinHostOperation::RepairCache {
            record.steps.first().map(|row| row.surface) == Some(DarwinHostSurface::Cache)
        } else {
            record
                .steps
                .iter()
                .enumerate()
                .all(|(index, row)| row.surface as usize == index)
        };
        let terminal_surfaces_valid = record
            .terminal
            .iter()
            .enumerate()
            .all(|(index, row)| row.surface as usize == index);
        let step_shape_valid = match record.operation {
            DarwinHostOperation::Install | DarwinHostOperation::Reinstall => {
                record.prior.is_none()
                    && record.steps.iter().all(|row| {
                        row.desired_tree_sha256.is_some() && row.prior_tree_sha256.is_none()
                    })
            }
            DarwinHostOperation::Update | DarwinHostOperation::RepairCache => {
                record.prior.is_some()
                    && record.steps.iter().all(|row| {
                        row.desired_tree_sha256.is_some() && row.prior_tree_sha256.is_some()
                    })
            }
            DarwinHostOperation::Uninstall => {
                record.prior.is_none()
                    && record.steps.iter().all(|row| {
                        row.desired_tree_sha256.is_none() && row.prior_tree_sha256.is_none()
                    })
            }
        };
        let digest_fields_valid = valid_digest(&record.root_id)
            && valid_digest(&record.plan_sha256)
            && record
                .previous_lineage_sha256
                .as_deref()
                .is_none_or(valid_digest)
            && record.steps.iter().all(|row| {
                row.desired_tree_sha256.as_deref().is_none_or(valid_digest)
                    && row.prior_tree_sha256.as_deref().is_none_or(valid_digest)
            })
            && record
                .terminal
                .iter()
                .all(|row| row.tree_sha256.as_deref().is_none_or(valid_digest))
            && valid_target_binding(&record.target)
            && record.prior.as_ref().is_none_or(valid_package_binding);
        let operation_valid =
            validate_operation(record.operation, &record.target, record.prior.as_ref()).is_ok();
        let plan_sha256_valid = plan_sha256_from_parts(
            &record.root_id,
            record.operation,
            &record.target,
            &record.prior,
            &record.marketplace,
            record
                .steps
                .iter()
                .map(|row| {
                    (
                        row.surface,
                        row.desired_tree_sha256.as_deref(),
                        row.prior_tree_sha256.as_deref(),
                    )
                })
                .collect(),
        )
        .is_ok_and(|digest| digest == record.plan_sha256);
        if lineage_bytes(&record).ok().as_deref() != Some(raw.bytes.as_slice())
            || record.schema != "harness-ultragoal.darwin-host-transaction-lineage.v1"
            || record.root_id != self.root_id
            || record.marketplace != SUPPORTED_MARKETPLACE
            || record.generation == 0
            || (record.generation == 1) != record.previous_lineage_sha256.is_none()
            || record.steps.len() != expected_steps
            || record.terminal.len() != DarwinHostSurface::ALL.len()
            || !step_surfaces_valid
            || !terminal_surfaces_valid
            || !step_shape_valid
            || !digest_fields_valid
            || !operation_valid
            || !plan_sha256_valid
        {
            return Err(DarwinHostError::new(DarwinHostErrorId::LineageCorrupt));
        }
        let snapshot = LineageSnapshot {
            record,
            tree_sha256: raw.tree_sha256,
        };
        self.observe_lineage(&snapshot)?;
        Ok(Some(snapshot))
    }

    pub(in super::super) fn read_lineage_anchor(
        &self,
    ) -> Result<Option<LineageAnchorSnapshot>, DarwinHostError> {
        let Some(raw) = self.read_tree(LINEAGE_ANCHOR_PATH, None, 1024 * 1024)? else {
            return Ok(None);
        };
        let record = decode_lineage_anchor(&raw.bytes)?;
        if lineage_anchor_bytes(&record).ok().as_deref() != Some(raw.bytes.as_slice())
            || record.schema != "harness-ultragoal.darwin-host-lineage-anchor.v1"
            || record.root_id != self.root_id
            || record.generation == 0
            || !valid_digest(&record.root_id)
            || !valid_digest(&record.lineage_sha256)
        {
            return Err(DarwinHostError::new(DarwinHostErrorId::LineageCorrupt));
        }
        Ok(Some(LineageAnchorSnapshot {
            record,
            tree_sha256: raw.tree_sha256,
        }))
    }

    pub(in super::super) fn observe_lineage(
        &self,
        lineage: &LineageSnapshot,
    ) -> Result<(), DarwinHostError> {
        let mut floor = self
            .lineage_floor
            .lock()
            .map_err(|_| DarwinHostError::new(DarwinHostErrorId::LineageConflict))?;
        match floor.as_ref() {
            Some(previous) if lineage.record.generation < previous.generation => {
                return Err(DarwinHostError::new(DarwinHostErrorId::LineageConflict));
            }
            Some(previous)
                if lineage.record.generation == previous.generation
                    && lineage.tree_sha256 != previous.tree_sha256 =>
            {
                return Err(DarwinHostError::new(DarwinHostErrorId::LineageConflict));
            }
            Some(previous)
                if lineage.record.generation > previous.generation
                    && lineage.record.previous_lineage_sha256.as_deref()
                        != Some(previous.tree_sha256.as_str()) =>
            {
                return Err(DarwinHostError::new(DarwinHostErrorId::LineageConflict));
            }
            _ => {}
        }
        if floor
            .as_ref()
            .is_none_or(|previous| lineage.record.generation > previous.generation)
        {
            *floor = Some(LineageFloor {
                generation: lineage.record.generation,
                tree_sha256: lineage.tree_sha256.clone(),
            });
        }
        Ok(())
    }
}
