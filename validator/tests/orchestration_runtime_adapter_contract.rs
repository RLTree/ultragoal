#[path = "../src/orchestration/mod.rs"]
mod orchestration;
#[path = "../src/orchestration/product/runtime_adapter/mod.rs"]
mod runtime_adapter;

mod orchestration_runtime_adapter_contract {
    mod fixture_contract;
    mod runtime_fixture;
    mod security;
}
