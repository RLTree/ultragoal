/// Verifies promotion-ledger genesis and current snapshot consistency.
fn require_promotion_genesis(
    anchor: &File,
    key: &[u8; 32],
    key_id: &str,
    lock_identity: FileIdentity,
    binding: &PromotionLedgerBinding,
) -> Result<AuthenticatedReviewAnchorRecord, PromotionLedgerError> {
    let authority = safe_file_identity(anchor).map_err(map_storage)?.authority();
    let scan = scan_anchor_journal(
        &read_anchor_bytes(anchor)?,
        key,
        key_id,
        lock_identity,
        authority,
        binding,
    )?;
    let [only] = scan.records.as_slice() else {
        return Err(PromotionLedgerError::new(
            "promotion-ledger-initialization-ambiguous",
        ));
    };
    if scan.partial_tail
        || scan.complete_length != safe_file_identity(anchor).map_err(map_storage)?.length
        || only.record.payload.core
            != promotion_genesis_core(key_id, lock_identity, authority, binding.clone())
    {
        return Err(PromotionLedgerError::new(
            "promotion-ledger-initialization-ambiguous",
        ));
    }
    Ok(only.record.clone())
}

fn require_promotion_state(
    file: &File,
    expected: &AuthenticatedReviewSnapshot,
    key: &[u8; 32],
    lock_identity: FileIdentity,
    anchor_authority: FileAuthorityIdentity,
    binding: &PromotionLedgerBinding,
) -> Result<(), PromotionLedgerError> {
    let observed: AuthenticatedReviewSnapshot = read_promotion_initialization_json(file)?;
    let authenticated = authenticate_snapshot(observed.payload.clone(), key)?;
    if observed.mac_sha256 != authenticated.mac_sha256
        || observed.head_sha256 != authenticated.head_sha256
        || observed.payload.core.key_id != sha256(key)
        || observed.payload.core.lock_identity != lock_identity
        || observed.payload.core.anchor_authority != anchor_authority
        || observed.payload.core.binding != *binding
        || observed.head_sha256 != expected.head_sha256
    {
        return Err(PromotionLedgerError::new(
            "promotion-ledger-initialization-ambiguous",
        ));
    }
    Ok(())
}
