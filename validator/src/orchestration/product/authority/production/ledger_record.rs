fn validate_record(
    key: &[u8; 32],
    prior: &[LedgerRecord],
    record: &LedgerRecord,
) -> Result<(), ProductError> {
    if record.schema_version != RECORD_SCHEMA
        || record.sequence != prior.len() as u64 + 1
        || record.prior_record_id != prior.last().map(|record| record.record_id.clone())
        || record.record_id != record_id(record)?
        || !super::super::constant_time_equal(
            record.authenticator.as_bytes(),
            authenticate(key, record)?.as_bytes(),
        )
    {
        return Err(ProductError::AuthorityStoreInvalid);
    }
    validate_digest(&record.permit_id)?;
    validate_digest(&record.causal_slot_id)?;
    let current = latest_states(prior).get(&record.permit_id).copied();
    let prior_slot = prior
        .iter()
        .rev()
        .find(|prior_record| prior_record.permit_id == record.permit_id)
        .map(|prior_record| prior_record.causal_slot_id.as_str());
    let legal = matches!(
        (current, record.state),
        (None, LedgerState::Issued)
            | (Some(LedgerState::Issued), LedgerState::Reserved)
            | (
                Some(LedgerState::Reserved),
                LedgerState::Committed | LedgerState::Refused | LedgerState::Ambiguous
            )
    );
    let slot_is_legal = match current {
        None => require_available_causal_slot(prior, &record.causal_slot_id).is_ok(),
        Some(_) => prior_slot == Some(record.causal_slot_id.as_str()),
    };
    if !legal || !slot_is_legal {
        return Err(ProductError::AuthorityStoreInvalid);
    }
    Ok(())
}

fn record_id(record: &LedgerRecord) -> Result<String, ProductError> {
    #[derive(Serialize)]
    struct Identity<'a> {
        schema_version: &'a str,
        sequence: u64,
        permit_id: &'a str,
        causal_slot_id: &'a str,
        state: LedgerState,
        prior_record_id: &'a Option<String>,
    }
    let bytes = serde_json::to_vec(&Identity {
        schema_version: &record.schema_version,
        sequence: record.sequence,
        permit_id: &record.permit_id,
        causal_slot_id: &record.causal_slot_id,
        state: record.state,
        prior_record_id: &record.prior_record_id,
    })
    .map_err(|_| ProductError::AuthorityStoreInvalid)?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

fn authenticate(key: &[u8; 32], record: &LedgerRecord) -> Result<String, ProductError> {
    let mut mac =
        Hmac::<Sha256>::new_from_slice(key).map_err(|_| ProductError::AuthorityStoreInvalid)?;
    mac.update(RECORD_DOMAIN);
    mac.update(record.record_id.as_bytes());
    Ok(format!("hmac-sha256:{:x}", mac.finalize().into_bytes()))
}

fn validate_digest(value: &str) -> Result<(), ProductError> {
    super::super::validate_digest(value)
}
