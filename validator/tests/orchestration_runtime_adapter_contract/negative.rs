use super::runtime_fixture::*;
use crate::orchestration::product::*;
use crate::orchestration::*;
use crate::runtime_adapter::*;
use std::collections::BTreeSet;

include!(
    "negative/current_source_action_request_and_permit_substitutions_refuse_before_mutation.rs"
);

include!(
    "negative/reconciliation_lease_operation_result_and_source_variant_substitutions_refuse.rs"
);
