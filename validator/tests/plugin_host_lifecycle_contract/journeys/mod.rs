use crate::host_fixture::{
    Fixture, Reader, handoff_external_effect, installed, lifecycle, request,
};
use crate::host_lifecycle::{HostLayer, HostLayerVerdict, HostLifecyclePhase};
use crate::plugin_product::lifecycle::{ApplyDisposition, LifecycleIntent, LifecycleState};

include!(
    "clean_install_keeps_all_host_layers_separate_and_requests_but_never_executes_host_effects.rs"
);

include!(
    "update_recovery_rollback_reinstall_stale_cache_repeat_and_uninstall_share_one_authority.rs"
);

include!("dirty_repeat_use_is_recursive_zero_write.rs");
