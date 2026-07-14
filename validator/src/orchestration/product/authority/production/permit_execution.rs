pub(crate) struct ValidatedExecution<'a> {
    authority: &'a ProductionRootAuthority,
    operation: RootOperation,
    permit_id: String,
}

pub(crate) struct ReservedExecution<'a> {
    authority: &'a ProductionRootAuthority,
    operation: RootOperation,
    permit_id: String,
}

impl<'a> ValidatedExecution<'a> {
    pub(super) fn into_parts(self) -> (&'a ProductionRootAuthority, RootOperation, String) {
        (self.authority, self.operation, self.permit_id)
    }
}

impl<'a> ReservedExecution<'a> {
    pub(super) fn new(
        authority: &'a ProductionRootAuthority,
        operation: RootOperation,
        permit_id: String,
    ) -> Self {
        Self {
            authority,
            operation,
            permit_id,
        }
    }

    pub(super) fn permit_id(&self) -> &str {
        &self.permit_id
    }
}

impl ProductionRootAuthority {
    pub(crate) fn require_issued_permit(&self, permit: &RootPermit) -> Result<(), ProductError> {
        self.ledger.require_issued(&permit_id(permit)?)
    }

    pub(crate) fn validate_action_execution<'a>(
        &'a self,
        verification: super::RootActionPermitVerification<'_>,
    ) -> Result<ValidatedExecution<'a>, ProductError> {
        let operation = verification.operation;
        let permit_id = permit_id(verification.permit)?;
        self.authority.verify_action(verification)?;
        Ok(ValidatedExecution {
            authority: self,
            operation,
            permit_id,
        })
    }

    pub(crate) fn validate_reconcile_execution<'a>(
        &'a self,
        verification: super::RootReconcilePermitVerification<'_>,
    ) -> Result<ValidatedExecution<'a>, ProductError> {
        let permit_id = permit_id(verification.permit)?;
        self.authority.verify_reconcile(verification)?;
        Ok(ValidatedExecution {
            authority: self,
            operation: RootOperation::Reconcile,
            permit_id,
        })
    }

    pub(crate) fn reserve_validated<'a>(
        &'a self,
        execution: ValidatedExecution<'a>,
    ) -> Result<ReservedExecution<'a>, ProductError> {
        if !std::ptr::eq(self, execution.authority) {
            return Err(ProductError::AuthorityInvalid);
        }
        self.ledger.require_issued(&execution.permit_id)?;
        self.ledger.reserve(execution)
    }

    pub(crate) fn complete_action<T>(
        &self,
        reservation: ReservedExecution<'_>,
        operation: impl FnOnce() -> Result<T, ProductError>,
    ) -> Result<T, ProductError> {
        if !std::ptr::eq(self, reservation.authority)
            || reservation.operation == RootOperation::Reconcile
        {
            return Err(ProductError::AuthorityInvalid);
        }
        self.ledger.complete(reservation, operation)
    }

    pub(crate) fn complete_reconcile<T>(
        &self,
        reservation: ReservedExecution<'_>,
        operation: impl FnOnce() -> Result<T, ProductError>,
    ) -> Result<T, ProductError> {
        if !std::ptr::eq(self, reservation.authority)
            || reservation.operation != RootOperation::Reconcile
        {
            return Err(ProductError::AuthorityInvalid);
        }
        self.ledger.complete(reservation, operation)
    }
}
