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

#[test]
fn wrappers_aliases_and_indirect_transitions_are_rejected() {
    let authority = include_str!(
        "../../../src/routine_work/runtime_adapter/mediator/reservation_state/authority.rs"
    );
    let staged = include_str!(
        "../../../src/routine_work/runtime_adapter/mediator/reservation_state/staged_custody.rs"
    );
    for replacement in [
        "(self).started.set(true);",
        "let first = self; let second = &first; second.started.set(true);",
        "let pair = (self,); pair.0.started.set(true);",
        "match self { alias => alias.started.set(true), }",
        "let alias = || self.started.set(true); alias();",
        "let alias = const { |target: &Self| target.started.set(true) }; alias(self);",
        "let future = async { self.started.set(true) }; drop(future);",
    ] {
        assert_eq!(
            super::validate(
                &authority.replacen("self.started.set(true);", replacement, 1),
                staged
            ),
            Err("authority-custody-receiver-alias")
        );
    }
    for replacement in [
        "(self).0.borrow().is_empty()",
        "{ let alias = &self; alias.0.borrow().is_empty() }",
        "{ let pair = (self,); pair.0.0.borrow().is_empty() }",
        "match self { alias => alias.0.borrow().is_empty(), }",
        "{ let alias = || self.0.borrow().is_empty(); alias() }",
        "{ let alias = const { |target: &Self| target.0.borrow().is_empty() }; alias(self) }",
        "{ let future = async { self.0.borrow().is_empty() }; drop(future); false }",
    ] {
        assert_eq!(
            super::validate(
                authority,
                &staged.replacen("self.0.borrow().is_empty()", replacement, 1)
            ),
            Err("authority-custody-receiver-alias")
        );
    }
    for replacement in [
        "Self::finish_terminal(self, true);",
        "let finish = Self::finish_terminal; finish(self, true);",
    ] {
        assert_eq!(
            super::validate(
                &authority.replacen("self.finish_terminal(true);", replacement, 1),
                staged
            ),
            Err("authority-custody-transition-alias")
        );
    }
    assert_eq!(
        super::validate(
            authority,
            &staged.replacen("self.is_empty()", "Self::is_empty(self)", 1)
        ),
        Err("authority-custody-transition-alias")
    );
    assert_eq!(
        super::validate(
            &format!(
                "use self::AttemptReservation as Owner;\n{}",
                authority.replacen(
                    "self.started.set(true);",
                    "let Owner { started, .. } = self; started.set(true);",
                    1
                )
            ),
            staged,
        ),
        Err("authority-custody-receiver-alias")
    );
    assert_eq!(
        super::validate(
            authority,
            &format!(
                "use self::StagedCustody as Owner;\n{}",
                staged.replacen(
                    "self.0.borrow().is_empty()",
                    "{ let Owner(items) = self; items.borrow().is_empty() }",
                    1
                )
            ),
        ),
        Err("authority-custody-receiver-alias")
    );
    let constructor = authority.replacen(
        "    Ok(AttemptReservation {",
        "    let attempt = AttemptReservation {",
        1,
    );
    let constructor = constructor.replacen(
        "        staged: StagedCustody::new(),\n    })\n}\n\nimpl AttemptReservation",
        "        staged: StagedCustody::new(),\n    };\n    attempt.started.set(true);\n    Ok(attempt)\n}\n\nimpl AttemptReservation",
        1,
    );
    assert_eq!(
        super::validate(&constructor, staged),
        Err("authority-custody-receiver-alias")
    );
    let staged_constructor = staged.replacen(
        "    pub(super) fn new() -> Self {\n        Self(RefCell::new(Vec::new()))\n    }",
        "    pub(super) fn new() -> Self {\n        let staged = Self(RefCell::new(Vec::new()));\n        staged.0.borrow_mut().clear();\n        staged\n    }",
        1,
    );
    assert_eq!(
        super::validate(authority, &staged_constructor),
        Err("authority-custody-receiver-alias")
    );
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
        "impl AttemptReservation { fn f(&self) { let super::AttemptReservation { started, .. } = self; started.set(true); } }",
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
        "impl StagedCustody { fn f(&self) { let super::StagedCustody(items) = self; items.borrow_mut().clear(); } }",
        "impl StagedCustody { fn f(&self) { let StagedCustody(items) = self else { return; }; items.borrow_mut().clear(); } }",
        "impl StagedCustody { fn f(&self) { if let StagedCustody(items) = self { items.borrow_mut().clear(); } } }",
        "impl StagedCustody { fn f(&self) { match self { StagedCustody(items) => items.borrow_mut().clear(), } } }",
        "impl StagedCustody { fn f(&self) { let alias = |StagedCustody(items): &StagedCustody| items.borrow_mut().clear(); alias(self); } }",
        "impl StagedCustody { fn f(StagedCustody(items): &StagedCustody) { items.borrow_mut().clear(); } }",
        "impl StagedCustody { fn f(&self) { let &StagedCustody(ref items) = self; items.borrow_mut().clear(); } }",
    ]
}
