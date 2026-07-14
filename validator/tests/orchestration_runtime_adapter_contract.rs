#[path = "../src/orchestration/mod.rs"]
mod orchestration;
#[path = "../src/orchestration/product/runtime_adapter/mod.rs"]
mod runtime_adapter;

mod orchestration_runtime_adapter_contract {
    mod decision_binding;
    mod fixture_contract;
    mod fresh_process;
    mod fresh_process_child;
    mod negative;
    mod positive;
    mod race;
    mod runtime_fixture;
    mod security;
}
