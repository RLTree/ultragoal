mod fixture;
mod tamper_cases;

use super::validate_current;

#[test]
fn current_log_yields_one_exact_current_amendment() {
    let binding = fixture::binding();
    let validated = validate_current(fixture::CURRENT_LOG, binding).expect("current amendment");
    assert_eq!(validated.amendment_id(), fixture::CURRENT_ID);
    assert_eq!(validated.amendment_hash(), fixture::CURRENT_HASH);
}
