use crate::orchestration::*;
use crate::orchestration_fixture::*;

include!("leases/disjoint_leases_can_run_together.rs");

include!("leases/reads_are_allowlisted_and_host_protected_reads_fail_closed.rs");
