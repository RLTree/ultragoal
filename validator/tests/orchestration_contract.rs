#![allow(dead_code, unused_imports)]

#[path = "../src/orchestration/mod.rs"]
mod orchestration;

#[path = "orchestration_contract/artifact_aliases.rs"]
mod artifact_aliases;
#[path = "orchestration_contract/artifact_evidence.rs"]
mod artifact_evidence;
#[path = "orchestration_contract/artifact_live.rs"]
mod artifact_live;
#[path = "orchestration_contract/blockers.rs"]
mod blockers;
#[path = "orchestration_contract/candidate_batch.rs"]
mod candidate_batch;
#[path = "orchestration_contract/candidate_rebind.rs"]
mod candidate_rebind;
#[path = "orchestration_contract/durable_support.rs"]
mod durable_support;
#[path = "orchestration_contract/effect_recovery.rs"]
mod effect_recovery;
#[path = "orchestration_contract/effects.rs"]
mod effects;
#[path = "orchestration_contract/frozen_parent.rs"]
mod frozen_parent;
#[path = "orchestration_contract/graph.rs"]
mod graph;
#[path = "orchestration_contract/integration_recovery.rs"]
mod integration_recovery;
#[path = "orchestration_contract/journal_exact_names.rs"]
mod journal_exact_names;
#[path = "orchestration_contract/journal_publication_recovery.rs"]
mod journal_publication_recovery;
#[path = "orchestration_contract/journal_recovery.rs"]
mod journal_recovery;
#[path = "orchestration_contract/journal_security.rs"]
mod journal_security;
#[path = "orchestration_contract/leases.rs"]
mod leases;
#[path = "orchestration_contract/live_integration.rs"]
mod live_integration;
#[path = "orchestration_contract/path_aliases.rs"]
mod path_aliases;
#[path = "orchestration_contract/read_write_authority.rs"]
mod read_write_authority;
#[path = "orchestration_contract/reconciliation.rs"]
mod reconciliation;
#[path = "orchestration_contract/reconciliation_recovery.rs"]
mod reconciliation_recovery;
#[path = "orchestration_contract/recovery.rs"]
mod recovery;
#[path = "orchestration_contract/root_acceptance.rs"]
mod root_acceptance;
#[path = "orchestration_contract/root_issued_envelope.rs"]
mod root_issued_envelope;
#[path = "orchestration_contract/security.rs"]
mod security;
#[path = "orchestration_contract/support.rs"]
mod support;
#[path = "orchestration_contract/worker_results.rs"]
mod worker_results;
#[path = "orchestration_contract/workspace_observer.rs"]
mod workspace_observer;
