use syn::visit::Visit;

use super::operations::BodyShape;

#[test]
fn reservation_and_staged_pattern_aliases_are_rejected() {
    for source in authority_pattern_forms() {
        assert_rejected(source, ["started"], ["AttemptReservation", "Self"]);
    }
    for source in staged_pattern_forms() {
        assert_rejected(source, ["0"], ["StagedCustody", "Self"]);
    }
}

fn assert_rejected<const N: usize, const M: usize>(
    source: &str,
    fields: [&str; N],
    patterns: [&str; M],
) {
    let file = syn::parse_file(source).unwrap();
    let mut shape = BodyShape::new(fields, patterns);
    shape.visit_file(&file);
    assert_eq!(
        shape.require_no_custody_patterns(),
        Err("authority-custody-pattern-alias")
    );
}

fn authority_pattern_forms() -> [&'static str; 7] {
    [
        "impl AttemptReservation { fn f(&self) { let AttemptReservation { started, .. } = self; started.set(true); } }",
        "impl AttemptReservation { fn f(&self) { let AttemptReservation { started, .. } = self else { return; }; started.set(true); } }",
        "impl AttemptReservation { fn f(&self) { if let AttemptReservation { started, .. } = self { started.set(true); } } }",
        "impl AttemptReservation { fn f(&self) { match self { AttemptReservation { started, .. } => started.set(true), } } }",
        "impl AttemptReservation { fn f(&self) { let alias = |AttemptReservation { started, .. }: &AttemptReservation| started.set(true); alias(self); } }",
        "impl AttemptReservation { fn f(AttemptReservation { started, .. }: &AttemptReservation) { started.set(true); } }",
        "impl AttemptReservation { fn f(&self) { let &AttemptReservation { ref started, .. } = self; started.set(true); } }",
    ]
}

fn staged_pattern_forms() -> [&'static str; 7] {
    [
        "impl StagedCustody { fn f(&self) { let StagedCustody(items) = self; items.borrow_mut().clear(); } }",
        "impl StagedCustody { fn f(&self) { let StagedCustody(items) = self else { return; }; items.borrow_mut().clear(); } }",
        "impl StagedCustody { fn f(&self) { if let StagedCustody(items) = self { items.borrow_mut().clear(); } } }",
        "impl StagedCustody { fn f(&self) { match self { StagedCustody(items) => items.borrow_mut().clear(), } } }",
        "impl StagedCustody { fn f(&self) { let alias = |StagedCustody(items): &StagedCustody| items.borrow_mut().clear(); alias(self); } }",
        "impl StagedCustody { fn f(StagedCustody(items): &StagedCustody) { items.borrow_mut().clear(); } }",
        "impl StagedCustody { fn f(&self) { let &StagedCustody(ref items) = self; items.borrow_mut().clear(); } }",
    ]
}
