use super::fixture_workspace::{
    assert_contains, cleanup, failures, inventory, temp_root, write_canonical_bin, write_file,
};

#[test]
fn duplicate_cli_binary_without_alias_contract_fails() {
    let root = temp_root("duplicate-cli-binary");
    write_file(
        &root,
        "validator/Cargo.toml",
        r#"
[[bin]]
name = "ultragoal-validator"
path = "src/bin/ultragoal-validator.rs"
[[bin]]
name = "ultragoal"
path = "src/bin/ultragoal.rs"
"#,
    );
    write_file(&root, "validator/src/bin/ultragoal.rs", "fn main() {}\n");
    write_file(
        &root,
        "validator/src/bin/ultragoal-validator.rs",
        "fn main() {}\n",
    );

    let failures = failures(
        &root,
        inventory(&[
            "validator/src/bin/ultragoal.rs",
            "validator/src/bin/ultragoal-validator.rs",
        ]),
    );
    assert_contains(&failures, "surface=binary:ultragoal-validator");
    cleanup(root);
}

#[test]
fn duplicate_nonlegacy_cargo_binary_fails() {
    let root = temp_root("duplicate-nonlegacy-cli-binary");
    write_file(
        &root,
        "validator/Cargo.toml",
        r#"
[[bin]]
name = "ultragoal"
path = "src/bin/ultragoal.rs"

[[bin]]
name = "ultragoal-shadow"
path = "src/bin/ultragoal-shadow.rs"
"#,
    );
    write_file(&root, "validator/src/bin/ultragoal.rs", "fn main() {}\n");
    write_file(
        &root,
        "validator/src/bin/ultragoal-shadow.rs",
        "fn main() {}\n",
    );

    let failures = failures(
        &root,
        inventory(&[
            "validator/src/bin/ultragoal.rs",
            "validator/src/bin/ultragoal-shadow.rs",
        ]),
    );
    assert_contains(&failures, "surface=binary:cargo-bin-duplicates");
    cleanup(root);
}

#[test]
fn cargo_binary_before_next_section_remains_registered() {
    let root = temp_root("cargo-bin-before-next-section");
    write_file(
        &root,
        "validator/Cargo.toml",
        r#"
[[bin]]
name = "ultragoal"
path = "src/bin/ultragoal.rs"

[dependencies]
serde = "1"
"#,
    );
    write_file(&root, "validator/src/bin/ultragoal.rs", "fn main() {}\n");

    let inventory =
        crate::audit::law::authority_surfaces::package_surface_inventory_artifact(&root);
    super::fixture_workspace::assert_row(
        &inventory,
        "validator/src/bin/ultragoal.rs",
        "binary",
        "canonical",
    );
    cleanup(root);
}

#[test]
fn unsupported_legacy_command_alias_fails() {
    let root = temp_root("legacy-command-alias");
    write_canonical_bin(&root);
    write_file(
        &root,
        "validator/src/argument_parser.rs",
        r#"match command { "package-digest" => Command::PackageDigest, _ => Command::Help }"#,
    );

    let failures = failures(
        &root,
        inventory(&[
            "validator/src/bin/ultragoal.rs",
            "validator/src/argument_parser.rs",
        ]),
    );
    assert_contains(&failures, "surface=command-alias:package-digest");
    cleanup(root);
}

#[test]
fn usage_text_cannot_advertise_legacy_validator_binary() {
    let root = temp_root("usage-legacy-validator");
    write_canonical_bin(&root);
    write_file(
        &root,
        "validator/src/cli/usage.rs",
        "pub fn text() -> &'static str { \"ultragoal-validator\" }\n",
    );

    let failures = failures(
        &root,
        inventory(&[
            "validator/src/bin/ultragoal.rs",
            "validator/src/cli/usage.rs",
        ]),
    );
    assert_contains(&failures, "surface=usage:ultragoal-validator");
    cleanup(root);
}
