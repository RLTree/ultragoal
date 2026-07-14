//! Refusal-only marker for root-broker child requests.
//!
//! This module intentionally defines no capability value, constructor,
//! decoder, key, ledger adapter, or success transition.  Same-UID source code
//! cannot mint the root-owned authorization that a future broker integration
//! must provide outside this package.

pub(crate) const BROKER_CHILD_REQUEST_ENV: &str = "HUL_ROUTINE_CHILD_FD";
