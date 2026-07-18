//! Source-local plugin lifecycle and Product Fitness authority candidates.
//!
//! This module deliberately does not mutate Codex host state. It supplies the
//! typed planning, adapter boundary, verification, recovery, and build-closure
//! logic that root-owned distribution code may wire after independent review.

pub mod journey_matrix;
pub mod lifecycle;
pub mod product_fitness;
pub mod source_closure;

// The product contract imports this compiled library surface; it does not
// duplicate production modules inside an integration-test crate.
pub mod distribution_adapter;

// The registry control plane consumes the sealed read-only agent authority
// transaction. Host discovery and route eligibility remain separate surfaces.
pub(crate) mod agent_discovery;
