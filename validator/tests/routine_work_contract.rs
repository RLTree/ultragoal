pub use ultragoal::{capture, context, routine_work};

#[path = "routine_work_contract/contract.rs"]
mod contract;
#[path = "routine_work_contract/issuer/api_compilation.rs"]
mod issuer_api_compilation;
#[path = "routine_work_contract/issuer/api_visibility.rs"]
mod issuer_api_visibility;
#[path = "routine_work_contract/issuer/hidden_surface.rs"]
mod issuer_hidden_surface;
#[path = "routine_work_contract/public_compile_scratch.rs"]
mod owned_compile_scratch;
#[path = "routine_work_contract/provenance.rs"]
mod provenance;
#[path = "routine_work_contract/reservation_authority_visibility.rs"]
mod reservation_authority_visibility;
#[path = "routine_work_contract/retired/adapter_routes.rs"]
mod retired_adapter_routes;
#[path = "routine_work_contract/retired/authority_routes.rs"]
mod retired_authority_routes;
#[path = "routine_work_contract/retired/behavior_routes.rs"]
mod retired_behavior_routes;
#[path = "routine_work_contract/retired/process_lifecycle_routes.rs"]
mod retired_process_lifecycle_routes;
#[path = "routine_work_contract/retired/reuse_output_routes.rs"]
mod retired_reuse_output_routes;
