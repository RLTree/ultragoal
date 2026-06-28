use std::path::PathBuf;

fn args(root: PathBuf, raw: &[&str]) -> crate::Args {
    crate::Args {
        root,
        command: crate::parse_command(&raw.iter().map(|s| s.to_string()).collect::<Vec<_>>())
            .expect("parse command"),
    }
}

#[test]
fn command_dispatch_routes_product_receipt_minting() {
    let root = crate::self_tests::boundaries::support::repo_root();
    let rel = format!("target/ultragoal-command-product-{}", std::process::id());
    let out = root.join(&rel);
    let _ = std::fs::remove_dir_all(&out);
    let code = crate::command_run::run_with_exit_code(args(
        root.clone(),
        &["product", "prove-fitness", "--receipt-dir", &rel],
    ))
    .expect("product command");
    assert_eq!(code, 0);
    assert!(out.join("fit-repo-receipt.json").is_file());
    assert!(out.join("product-fitness-receipt.json").is_file());
    assert!(out.join("plugin-product-journey-receipt.json").is_file());
    std::fs::remove_dir_all(out).expect("cleanup product command dispatch");
}

#[test]
fn command_parser_rejects_incomplete_product_command_without_fallback_authority() {
    let error = crate::parse_command(&["product".to_string()]).expect_err("usage error");
    assert!(error.contains("usage:"), "{error}");
}
