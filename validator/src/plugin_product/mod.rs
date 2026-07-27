//! Source-local plugin lifecycle and Product Fitness authority candidates.
//!
//! This module deliberately does not mutate Codex host state. It supplies the
//! typed planning, adapter boundary, verification, recovery, and build-closure
//! logic that root-owned distribution code may wire after independent review.

pub mod engineering_advisory;
pub mod journey_matrix;
pub mod lifecycle;
pub mod product_fitness;
pub mod source_closure;

// The legacy source-closure contract compiles this module directly inside an
// integration-test crate that has no distribution kernel. Production builds,
// including every integration test through the library, expose the adapter.
pub mod distribution_adapter;

// The registry control plane consumes the sealed read-only agent authority
// transaction. Host discovery and route eligibility remain separate surfaces.
pub(crate) mod agent_discovery;
