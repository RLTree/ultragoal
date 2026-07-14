#[cfg(test)]
pub(crate) fn issue_action_permit_for_test(
    authority: &RootAuthority,
    request: RootActionPermitIssuance<'_>,
) -> Result<RootPermit, ProductError> {
    authority.issue_action(request)
}

#[cfg(test)]
pub(crate) fn issue_reconcile_permit_for_test(
    authority: &RootAuthority,
    request: RootReconcilePermitIssuance<'_>,
) -> Result<RootPermit, ProductError> {
    authority.issue_reconcile(request)
}
