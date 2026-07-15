use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use super::owned_compile_scratch::OwnedCompileScratch;

const CLOSED: &str = r#"
mod mediator {
    pub(crate) mod reservation_state {
        pub(crate) mod authority {
            use std::cell::{Cell, RefCell};
            pub(in crate::mediator) struct AttemptReservation {
                started: Cell<bool>,
                settled: Cell<bool>,
                staged: RefCell<Vec<()>>,
            }
        }
        pub(in crate::mediator) use authority::AttemptReservation;
        pub mod descendant_attack {
            use super::AttemptReservation;
            pub fn forge(a: &AttemptReservation) {
                a.started.set(true);
                a.settled.set(true);
                a.staged.borrow_mut().clear();
            }
        }
    }
    pub mod sibling_attack {
        use super::reservation_state::AttemptReservation;
        pub fn forge(a: &AttemptReservation) {
            a.started.set(true);
            a.settled.set(true);
            a.staged.borrow_mut().clear();
        }
    }
    // OPENED_EXTENSION
}
"#;

#[test]
fn sibling_and_descendant_cannot_mutate_reservation_storage() {
    assert_production_shape();
    let mut owned = OwnedCompileScratch::claim("routine-reservation-privacy");
    prepare(owned.path());

    fs::write(owned.path().join("lib.rs"), opened()).unwrap();
    let control = run(owned.path());
    assert!(control.status.success(), "{}", diagnostic(&control));

    fs::write(owned.path().join("lib.rs"), CLOSED).unwrap();
    let closed = check(owned.path());
    assert!(!closed.status.success(), "closed mutation probe compiled");
    let stderr = String::from_utf8_lossy(&closed.stderr);
    assert_eq!(stderr.matches("error[E0616]").count(), 6, "{stderr}");
    for field in ["started", "settled", "staged"] {
        assert!(
            stderr.matches(&format!("field `{field}`")).count() >= 2,
            "{stderr}"
        );
    }
    owned.teardown_after_assertions();
}

fn assert_production_shape() {
    let graph =
        include_str!("../../src/routine_work/runtime_adapter/mediator/reservation_state/mod.rs");
    assert!(graph.contains("mod authority;"));
    assert!(graph.contains("pub(super) use authority::{AttemptReservation, reserve_grant};"));
    assert!(!graph.contains("reserved_test_attempt"));
    let source = include_str!(
        "../../src/routine_work/runtime_adapter/mediator/reservation_state/authority.rs"
    );
    let file = syn::parse_file(source).unwrap();
    assert!(
        !file
            .items
            .iter()
            .any(|item| matches!(item, syn::Item::Mod(_) | syn::Item::Macro(_)))
    );
    let attempt = file
        .items
        .iter()
        .find_map(|item| match item {
            syn::Item::Struct(item) if item.ident == "AttemptReservation" => Some(item),
            _ => None,
        })
        .expect("opaque reservation owner is declared");
    assert!(attempt.attrs.is_empty());
    let syn::Visibility::Restricted(visibility) = &attempt.vis else {
        panic!("reservation owner visibility widened");
    };
    assert_eq!(
        visibility
            .path
            .segments
            .iter()
            .map(|segment| segment.ident.to_string())
            .collect::<Vec<_>>(),
        ["super", "super"]
    );
    let fields = attempt
        .fields
        .iter()
        .map(|field| {
            assert!(matches!(field.vis, syn::Visibility::Inherited));
            field.ident.as_ref().unwrap().to_string()
        })
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        fields,
        ["binding", "durable", "settled", "staged", "started"]
            .into_iter()
            .map(str::to_owned)
            .collect()
    );
    let free_functions = file
        .items
        .iter()
        .filter_map(|item| match item {
            syn::Item::Fn(item) => Some(item.sig.ident.to_string()),
            _ => None,
        })
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        free_functions,
        ["reserve_grant".to_owned()].into_iter().collect()
    );
    let methods = file
        .items
        .iter()
        .filter_map(|item| match item {
            syn::Item::Impl(item) => Some(item),
            _ => None,
        })
        .flat_map(|item| {
            assert!(item.attrs.is_empty());
            assert!(
                item.items
                    .iter()
                    .all(|member| matches!(member, syn::ImplItem::Fn(_)))
            );
            &item.items
        })
        .filter_map(|item| match item {
            syn::ImplItem::Fn(item) => Some(item.sig.ident.to_string()),
            _ => None,
        })
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(methods, expected_methods());
}

fn opened() -> String {
    let graph = CLOSED
        .replace(
            "started: Cell<bool>",
            "pub(in crate::mediator) started: Cell<bool>",
        )
        .replace(
            "settled: Cell<bool>",
            "pub(in crate::mediator) settled: Cell<bool>",
        )
        .replace(
            "staged: RefCell<Vec<()>>",
            "pub(in crate::mediator) staged: RefCell<Vec<()>>",
        );
    graph.replace(
        "// OPENED_EXTENSION",
        "pub fn exercise() {\n        let attempt = reservation_state::authority::AttemptReservation {\n            started: std::cell::Cell::new(false),\n            settled: std::cell::Cell::new(false),\n            staged: std::cell::RefCell::new(vec![()]),\n        };\n        reservation_state::descendant_attack::forge(&attempt);\n        sibling_attack::forge(&attempt);\n        assert!(attempt.started.get());\n        assert!(attempt.settled.get());\n        assert!(attempt.staged.borrow().is_empty());\n    }",
    ) + "\n#[cfg(test)] mod opened_control {\n    #[test] fn both_attacks_execute() { crate::mediator::exercise(); }\n}\n"
}

fn expected_methods() -> std::collections::BTreeSet<String> {
    [
        "authenticates_artifact",
        "cleanup_staged",
        "failure_evidence",
        "finish_terminal",
        "grant_id",
        "is_started",
        "mark_started",
        "prepare_spawn",
        "protocol_id",
        "record_failure_and_transition",
        "recovery_marker",
        "require_open",
        "retain_non_durable_authentication",
        "reuse_only",
        "settle_incomplete",
        "settle_success",
        "stage_and_use",
        "stage_success",
        "terminal_is_authoritative",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect()
}

fn prepare(scratch: &Path) {
    fs::write(
        scratch.join("Cargo.toml"),
        "[workspace]\n\n[package]\nname = \"reservation-authority-visibility\"\nversion = \"0.0.0\"\nedition = \"2024\"\n\n[lib]\npath = \"lib.rs\"\n",
    )
    .unwrap();
}

fn check(scratch: &Path) -> Output {
    Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
        .args(["check", "--offline", "--quiet"])
        .current_dir(scratch)
        .env("CARGO_TARGET_DIR", scratch.join("target"))
        .output()
        .unwrap()
}

fn run(scratch: &Path) -> Output {
    Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
        .args(["test", "--offline", "--quiet"])
        .current_dir(scratch)
        .env("CARGO_TARGET_DIR", scratch.join("target"))
        .output()
        .unwrap()
}

fn diagnostic(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
