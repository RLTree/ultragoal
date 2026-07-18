use crate::host_fixture::{Fixture, Reader, installed, lifecycle, request};
use crate::host_lifecycle::{
    HostLifecycleErrorId, HostObservationTransactionRequest, HostScopeAuthority, HostSurfaceReader,
    HostSurfaceTransaction, HostSurfaceTransactionError,
};
use crate::plugin_product::lifecycle::{LifecycleIntent, LifecycleState};

include!("sealed_plan_replay_and_sibling_sessions_authorize_exactly_one_transition.rs");

include!("link_hardlink_fifo_and_socket_install_objects_refuse_without_outside_write.rs");

include!("test_surface_source.rs");

include!("counting_surface_transaction_provenance_sha256.rs");

include!("stale_scope_rejected_session_wrong_state_and_effect_replay_never_reach_adapter.rs");

include!("scope_mutation_after_preflight_is_rejected_before_transaction_metadata_or_reads.rs");

include!("marketplace_and_cache_substitution_remain_zero_write_and_non_echoing.rs");

include!("every_host_surface_and_reader_provenance_is_reobserved_after_capture.rs");

include!("installed_drift_reader_provenance_sha256.rs");

include!(
    "final_generation_revalidation_catches_after_last_layer_and_restore_attacks_without_close.rs"
);

include!("transaction_failure_reader_with_transaction.rs");

include!("reader_owned_transaction_serializes_concurrent_writer_through_session_close.rs");

include!("external_callers_cannot_capture_construct_clone_or_deserialize_observation_frames.rs");
