// Standalone: rustc --edition 2024 --test validator/tests/cli_contract/successor/mod.rs
// Nested: the root-owned cli_contract integration test may include this module.
#[path = "../../../src/cli/successor/mod.rs"]
mod successor;

mod boundaries;
mod catalog;
mod command_line;
mod compatibility;
mod help_boundaries;
mod parsing;
mod runtime_context;
mod security;
