use crate::distribution::{HostVerdict, JoinVerdict, Layer, LayerVerdict, verify};
use crate::distribution_fixture::{CANDIDATE_ID, CONTEXT_ID, Fixture, tree};

#[test]
fn complete_ladder_is_candidate_bound_deterministic_and_zero_write() {
    let fixture = Fixture::complete("complete");
    let before = tree(&fixture.root);

    let first = verify(&fixture.root, &fixture.bytes()).expect("distribution ladder");
    let second = verify(&fixture.root, &fixture.bytes()).expect("repeat ladder");

    assert!(first.accepted());
    assert_eq!(first.context_id(), CONTEXT_ID);
    assert_eq!(first.candidate_id(), CANDIDATE_ID);
    assert_eq!(first.host_verdict(), HostVerdict::Supported);
    assert_eq!(first.layers().len(), Layer::ALL.len());
    for layer in Layer::ALL {
        assert_eq!(first.layer(layer).verdict(), LayerVerdict::Verified);
    }
    assert_eq!(first.joins().len(), Layer::ALL.len() - 1);
    assert!(
        first
            .joins()
            .iter()
            .all(|join| join.verdict() == JoinVerdict::Match)
    );
    assert_eq!(first.ladder_sha256(), second.ladder_sha256());
    assert_eq!(tree(&fixture.root), before);
}
