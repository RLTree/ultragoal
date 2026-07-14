use super::*;

pub fn submit(ledger: &mut DecisionLedger, observations: Vec<Observation>) -> Vec<String> {
    observations
        .into_iter()
        .map(|observation| {
            let id = observation.evidence_id().to_owned();
            ledger.submit(observation).expect("submit observation");
            id
        })
        .collect()
}

pub fn pass_claim(
    ledger: &mut DecisionLedger,
    definitions: &ClaimDefinitions,
    claim_id: &str,
    tag: &str,
) {
    let ids = submit(ledger, observations_for(definitions, claim_id, tag));
    let decision = ledger.decide(
        definitions,
        claim_id,
        context_id(),
        candidate_id(),
        now(),
        &reviewer(definitions, claim_id),
        &ids,
    );
    assert_eq!(
        decision.status,
        DecisionStatus::Passed,
        "{claim_id}:{:?}",
        decision.reasons
    );
}

pub fn pass_before(
    ledger: &mut DecisionLedger,
    definitions: &ClaimDefinitions,
    target: &str,
    tag: &str,
) {
    for claim_id in definitions.order() {
        if claim_id == target {
            break;
        }
        pass_claim(ledger, definitions, claim_id, tag);
    }
}

pub fn digest(value: &str) -> String {
    format!("sha256:{:x}", Sha256::digest(value.as_bytes()))
}

pub fn raw_digest(value: &str) -> String {
    format!("{:x}", Sha256::digest(value.as_bytes()))
}

pub(crate) fn digest_bytes(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}
