#![cfg(unix)]

#[path = "successor_cli_state_journey_contract/assertions.rs"]
mod assertions;
#[path = "successor_cli_state_journey_contract/diagnosis_stability.rs"]
mod diagnosis_stability;
#[path = "successor_cli_state_journey_contract/fixture.rs"]
mod fixture;
#[path = "successor_cli_state_journey_contract/repository_classification.rs"]
mod repository_classification;
#[path = "successor_cli_state_journey_contract/snapshot.rs"]
mod snapshot;

const PRIVATE_CANARY: &str = "SUCCESSOR_READ_PRIVATE_CANARY_047";
