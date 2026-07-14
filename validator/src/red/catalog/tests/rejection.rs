use super::fixture::{CatalogRoot, packet};
use crate::red::catalog::RedCatalogProjectionRequest;

fn error(root: &CatalogRoot) -> String {
    crate::red::catalog::render(RedCatalogProjectionRequest { root: root.path() })
        .expect_err("rejected")
        .stable_text()
}

#[test]
fn malformed_unknown_and_identity_mutations_fail_closed() {
    let root = CatalogRoot::new("mutations");
    root.write("fixtures/red/bad.json", b"{bad json\n");
    assert!(error(&root).contains("packet_contract_invalid"));

    root.write(
        "fixtures/red/bad.json",
        packet("bad").replace("\n}", ",\n  \"unknown\": true\n}"),
    );
    assert!(error(&root).contains("packet_contract_invalid"));

    root.write("fixtures/red/bad.json", packet("different"));
    assert!(error(&root).contains("identity_path_mismatch"));

    let root = CatalogRoot::new("nul-file-contents");
    root.write(
        "fixtures/red/bad.json",
        packet("bad").replace(
            "\n}",
            ",\n  \"filesystem_fixtures\": [{\"kind\":\"file\",\"path\":\"bad.rs\",\"contents\":\"bad\\u0000source\"}]\n}",
        ),
    );
    assert!(error(&root).contains("packet_values_invalid"));
}

#[test]
fn duplicate_ids_unknown_paths_and_stale_catalog_fail_closed() {
    let root = CatalogRoot::new("duplicates");
    root.packet("alpha", "alpha");
    root.packet("beta", "alpha");
    assert!(error(&root).contains("packet_id_duplicate"));

    let root = CatalogRoot::new("unknown-path");
    root.packet("alpha", "alpha");
    root.write("fixtures/red/readme.txt", b"not a packet\n");
    assert!(error(&root).contains("fixture_path_unknown"));

    let root = CatalogRoot::new("stale");
    root.packet("alpha", "alpha");
    root.write("templates/RED_FIXTURES.json", b"[]\n");
    let failure = crate::red::catalog::check(RedCatalogProjectionRequest { root: root.path() })
        .expect_err("stale")
        .stable_text();
    assert_eq!(failure, "red_catalog_projection_drift");
}

#[cfg(unix)]
#[test]
fn symlink_and_special_packet_entries_fail_closed() {
    use std::os::unix::fs::symlink;
    use std::process::Command;

    let root = CatalogRoot::new("special");
    root.packet("alpha", "alpha");
    symlink(
        root.path().join("fixtures/red/alpha.json"),
        root.path().join("fixtures/red/alias.json"),
    )
    .expect("symlink");
    assert!(error(&root).contains("regular_file_required"));
    std::fs::remove_file(root.path().join("fixtures/red/alias.json")).expect("remove alias");
    assert!(
        Command::new("mkfifo")
            .arg(root.path().join("fixtures/red/event.json"))
            .status()
            .expect("mkfifo")
            .success()
    );
    assert!(error(&root).contains("regular_file_required"));
}
