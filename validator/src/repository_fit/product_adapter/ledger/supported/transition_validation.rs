use super::*;

pub(crate) const fn invalid_transition() -> LedgerError {
    LedgerError::new(LedgerErrorId::InvalidTransition)
}

pub(crate) const fn ledger_io() -> LedgerError {
    LedgerError::new(LedgerErrorId::Io)
}
