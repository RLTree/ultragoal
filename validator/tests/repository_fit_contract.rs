#![allow(dead_code, unused_imports)]

#[path = "../src/context/mod.rs"]
mod context;

#[path = "../src/repository_fit/mod.rs"]
mod repository_fit;

#[path = "repository_fit_contract/apply_failures.rs"]
mod apply_failures;
#[path = "repository_fit_contract/engine.rs"]
mod engine;
#[path = "repository_fit_contract/enumeration.rs"]
mod enumeration;
#[path = "repository_fit_contract/local_read.rs"]
mod local_read;
#[path = "repository_fit_contract/spec.rs"]
mod spec;
#[path = "repository_fit_contract/support.rs"]
mod support;
