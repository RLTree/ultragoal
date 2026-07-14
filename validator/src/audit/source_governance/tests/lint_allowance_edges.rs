use super::fixture::SourceRoot;

fn failures(label: &str, path: &str, source: &str) -> Vec<String> {
    let root = SourceRoot::new(label);
    root.write(path, source);
    crate::audit::source_governance::audit(root.path()).failures
}

fn assert_forbidden(label: &str, lint: &str, source: &str) {
    let path = format!("validator/src/{label}.rs");
    let failures = failures(label, &path, source);
    assert!(
        failures.iter().any(|failure| failure
            .contains(&format!("forbidden_lint_allowance:path={path};lint={lint}"))),
        "{lint}: {failures:?}"
    );
}

#[test]
fn broad_dead_unused_and_unreachable_suppressions_fail_every_rust_scope() {
    for (lint, ordinal) in ["dead_code", "unused_imports", "unreachable_code"]
        .into_iter()
        .zip(0..)
    {
        let path = format!("validator/src/self_tests/lint_{ordinal}.rs");
        let failures = failures(
            &format!("lint-broad-{ordinal}"),
            &path,
            &format!("#![allow({lint})]\nfn behavior() {{}}\n"),
        );
        assert!(
            failures.iter().any(|failure| failure
                .contains(&format!("forbidden_lint_allowance:path={path};lint={lint}"))),
            "{failures:?}"
        );
    }
}

#[test]
fn production_boundary_shape_suppressions_fail_but_test_helpers_remain_scoped() {
    let path = "validator/src/product_boundary.rs";
    let failures = failures(
        "lint-production-boundary",
        path,
        "#[allow(clippy::too_many_arguments)]\nfn parse(a: u8, b: u8) -> Result<Parsed, Error> { let _ = (a, b); Err(Error) }\nstruct Parsed;\nstruct Error;\n",
    );
    assert!(
        failures.iter().any(|failure| failure.contains(
            "forbidden_lint_allowance:path=validator/src/product_boundary.rs;lint=clippy::too_many_arguments"
        )),
        "{failures:?}"
    );

    let test_root = SourceRoot::new("lint-test-helper-shape");
    test_root.write(
        "validator/src/self_tests/fixture_builder.rs",
        "#[allow(clippy::too_many_arguments)]\nfn fixture(a: u8, b: u8) { let _ = (a, b); }\n",
    );
    assert!(
        crate::audit::source_governance::audit(test_root.path())
            .failures
            .iter()
            .all(|failure| !failure.contains("forbidden_lint_allowance"))
    );
}

#[test]
fn cfg_attr_allowances_are_recursive_across_rust_attribute_scopes() {
    let cases = [
        ("crate_scope", "#![cfg_attr(any(), allow(dead_code))]\n"),
        (
            "module_scope",
            "#[cfg_attr(any(), allow(dead_code))] mod product {}\n",
        ),
        (
            "item_scope",
            "#[cfg_attr(any(), allow(dead_code))] fn product() {}\n",
        ),
        (
            "impl_scope",
            "struct Product; impl Product { #[cfg_attr(any(), allow(dead_code))] fn run() {} }\n",
        ),
        (
            "trait_scope",
            "trait Product { #[cfg_attr(any(), allow(dead_code))] fn run(); }\n",
        ),
        (
            "field_scope",
            "struct Product { #[cfg_attr(any(), allow(dead_code))] value: u8 }\n",
        ),
        (
            "nested_cfg_attr",
            "#![cfg_attr(any(), cfg_attr(all(), allow(dead_code)))]\n",
        ),
    ];
    for (label, source) in cases {
        assert_forbidden(label, "dead_code", source);
    }
}

#[test]
fn every_prohibited_lint_class_is_found_inside_cfg_attr() {
    for (ordinal, lint) in [
        "dead_code",
        "warnings",
        "unused",
        "unused_imports",
        "unreachable_code",
        "clippy::too_many_arguments",
        "clippy::type_complexity",
        "clippy::all",
        "clippy::cargo",
        "clippy::complexity",
        "clippy::correctness",
        "clippy::nursery",
        "clippy::pedantic",
        "clippy::perf",
        "clippy::restriction",
        "clippy::style",
        "clippy::suspicious",
    ]
    .into_iter()
    .enumerate()
    {
        let label = format!("cfg_attr_lint_{ordinal}");
        assert_forbidden(
            &label,
            lint,
            &format!("#![cfg_attr(any(), allow({lint}))]\n"),
        );
    }
}

#[test]
fn malformed_or_unconsumed_lint_metadata_fails_closed() {
    for (ordinal, source) in [
        "#[allow(dead_code = \"invalid\")] fn product() {}\n",
        "#[allow(dead_code(extra))] fn product() {}\n",
        "#[allow()] fn product() {}\n",
        "#[allow(reason = \"\")] fn product() {}\n",
        "#[allow(reason = \"missing lint\")] fn product() {}\n",
        "#[allow(dead_code, reason = \"first\", reason = \"second\")] fn product() {}\n",
        "#[cfg_attr(any())] fn product() {}\n",
        "#[cfg_attr(any(), allow(dead_code trailing))] fn product() {}\n",
    ]
    .into_iter()
    .enumerate()
    {
        let path = format!("validator/src/malformed_lint_{ordinal}.rs");
        let failures = failures(&format!("malformed-lint-{ordinal}"), &path, source);
        assert!(
            failures
                .iter()
                .any(|failure| failure.contains("lint_metadata_invalid")),
            "{source}: {failures:?}"
        );
    }
}

#[test]
fn cfg_attr_condition_cannot_hide_nested_allowance_metadata() {
    assert_forbidden(
        "cfg_attr_condition_allowance",
        "dead_code",
        "#[cfg_attr(any(allow(dead_code)), inline)] fn product() {}\n",
    );
}

#[test]
fn scoped_specific_lint_with_reason_remains_permitted() {
    let path = "validator/src/permitted_lint.rs";
    let failures = failures(
        "permitted-lint",
        path,
        "#[cfg_attr(test, allow(clippy::single_match, reason = \"test-only style\"))]\nfn product() {}\n",
    );
    assert!(
        failures.iter().all(|failure| {
            !failure.contains("forbidden_lint_allowance")
                && !failure.contains("lint_metadata_invalid")
        }),
        "{failures:?}"
    );
}
