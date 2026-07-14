use sha2::{Digest, Sha256};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use super::issuer_compile;

#[test]
fn sealed_issuer_and_grant_entrypoints_are_not_externally_callable() {
    let owned = issuer_compile::OwnedScratch::claim("routine-issuer-visibility");
    let scratch = owned.path();
    let probes = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/routine_work_contract/probes");
    issuer_compile::prepare(&scratch, &probes);

    let control = issuer_compile::check(&scratch, "routine_public_api_control");
    assert!(control.status.success(), "{}", diagnostic(&control));

    for (probe, code) in [
        ("production_issuer_consumer.rs", "E0603"),
        ("production_grant_consumer.rs", "E0432"),
        ("production_grant_entrypoint_consumer.rs", "E0432"),
        ("production_private_module_consumer.rs", "E0603"),
        ("production_private_grant_consumer.rs", "E0603"),
    ] {
        let output = issuer_compile::check(&scratch, probe.trim_end_matches(".rs"));
        assert_private_failure(&output, code, probe);
    }

    let docs = issuer_compile::document(&scratch);
    let inventory = public_inventory(&docs);
    let digest = format!("{:x}", Sha256::digest(&inventory));
    assert_eq!(
        digest,
        "b132f14ce3ffbc2871dc6fd05f55bec497d32d582ad487682193ffa2ed550d3e"
    );
    let routine_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/routine_work");
    assert!(!tree_has_hidden_public_api(&routine_root));

    let mut source = OpenOptions::new()
        .append(true)
        .open(scratch.join("public_surface.rs"))
        .unwrap();
    source
        .write_all(b"\n#[doc(hidden)]\npub fn hidden_grant_entrypoint() {}\n")
        .unwrap();
    source.flush().unwrap();
    assert!(file_has_hidden_public_api(
        &fs::read_to_string(scratch.join("public_surface.rs")).unwrap()
    ));
    source
        .write_all(b"\nimpl ApiSentinel { pub fn grant(&self) {} pub const GRANT: () = (); }\n")
        .unwrap();
    source.flush().unwrap();
    let mutated = public_inventory(&issuer_compile::document(&scratch));
    assert_ne!(Sha256::digest(&mutated), Sha256::digest(&inventory));
}

#[test]
fn concurrent_issuer_controls_use_disjoint_authorized_scratch() {
    let owned = issuer_compile::OwnedScratch::claim("routine-issuer-concurrency");
    let scratch = owned.path().join("scratch");
    let tmp = owned.path().join("tmp");
    fs::create_dir(&scratch).unwrap();
    fs::create_dir(&tmp).unwrap();
    let executable = std::env::current_exe().unwrap();
    let mut children = (0..2)
        .map(|_| {
            Command::new(&executable)
                .args([
                    "issuer_visibility::sealed_issuer_and_grant_entrypoints_are_not_externally_callable",
                    "--exact",
                ])
                .env("CODEX_WORKTREE_SCRATCH", &scratch)
                .env("CODEX_WORKTREE_TMP", &tmp)
                .spawn()
                .unwrap()
        })
        .collect::<Vec<_>>();
    for child in &mut children {
        assert!(child.wait().unwrap().success());
    }
    assert_eq!(fs::read_dir(&scratch).unwrap().count(), 0);
    assert_eq!(fs::read_dir(&tmp).unwrap().count(), 0);
}

fn public_inventory(docs: &Path) -> Vec<u8> {
    fn visit(root: &Path, current: &Path, rows: &mut Vec<(PathBuf, Vec<u8>)>) {
        let mut entries = fs::read_dir(current)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect::<Vec<_>>();
        entries.sort();
        for path in entries {
            if path.is_dir() {
                visit(root, &path, rows);
            } else {
                rows.push((
                    path.strip_prefix(root).unwrap().to_path_buf(),
                    fs::read(path).unwrap(),
                ));
            }
        }
    }
    let mut rows = Vec::new();
    visit(docs, docs, &mut rows);
    let mut bytes = Vec::new();
    for (path, data) in rows {
        bytes.extend(path.as_os_str().as_encoded_bytes());
        bytes.push(0);
        bytes.extend(data);
    }
    bytes
}

fn tree_has_hidden_public_api(root: &Path) -> bool {
    let mut entries = fs::read_dir(root)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect::<Vec<_>>();
    entries.sort();
    entries.into_iter().any(|path| {
        if path.is_dir() {
            tree_has_hidden_public_api(&path)
        } else {
            path.extension().is_some_and(|value| value == "rs")
                && file_has_hidden_public_api(&fs::read_to_string(path).unwrap())
        }
    })
}

fn file_has_hidden_public_api(source: &str) -> bool {
    let file = syn::parse_file(source).expect("issuer visibility source parses");
    file.items.iter().any(item_has_hidden_public_api)
}

fn item_has_hidden_public_api(item: &syn::Item) -> bool {
    use syn::Item;
    let direct = match item {
        Item::Const(value) => public(&value.vis) && hidden(&value.attrs),
        Item::Enum(value) => public(&value.vis) && hidden(&value.attrs),
        Item::ExternCrate(value) => public(&value.vis) && hidden(&value.attrs),
        Item::Fn(value) => public(&value.vis) && hidden(&value.attrs),
        Item::Mod(value) => public(&value.vis) && hidden(&value.attrs),
        Item::Static(value) => public(&value.vis) && hidden(&value.attrs),
        Item::Struct(value) => public(&value.vis) && hidden(&value.attrs),
        Item::Trait(value) => public(&value.vis) && hidden(&value.attrs),
        Item::TraitAlias(value) => public(&value.vis) && hidden(&value.attrs),
        Item::Type(value) => public(&value.vis) && hidden(&value.attrs),
        Item::Union(value) => public(&value.vis) && hidden(&value.attrs),
        Item::Use(value) => public(&value.vis) && hidden(&value.attrs),
        _ => false,
    };
    if direct {
        return true;
    }
    match item {
        Item::Impl(value) => value.items.iter().any(|item| match item {
            syn::ImplItem::Const(value) => public(&value.vis) && hidden(&value.attrs),
            syn::ImplItem::Fn(value) => public(&value.vis) && hidden(&value.attrs),
            syn::ImplItem::Type(value) => public(&value.vis) && hidden(&value.attrs),
            _ => false,
        }),
        Item::Mod(value) => value
            .content
            .as_ref()
            .is_some_and(|(_, items)| items.iter().any(item_has_hidden_public_api)),
        Item::Trait(value) if public(&value.vis) => value.items.iter().any(|item| match item {
            syn::TraitItem::Const(value) => hidden(&value.attrs),
            syn::TraitItem::Fn(value) => hidden(&value.attrs),
            syn::TraitItem::Macro(value) => hidden(&value.attrs),
            syn::TraitItem::Type(value) => hidden(&value.attrs),
            _ => false,
        }),
        _ => false,
    }
}

fn public(visibility: &syn::Visibility) -> bool {
    matches!(visibility, syn::Visibility::Public(_))
}

fn hidden(attributes: &[syn::Attribute]) -> bool {
    attributes.iter().any(|attribute| {
        let syn::Meta::List(meta) = &attribute.meta else {
            return false;
        };
        meta.path.is_ident("doc") && meta.tokens.to_string() == "hidden"
    })
}

fn assert_private_failure(output: &Output, code: &str, probe: &str) {
    let text = diagnostic(output);
    assert!(!output.status.success(), "{probe} became callable");
    assert!(text.contains(code), "unexpected {probe} failure: {text}");
    for incidental in ["can't find crate", "file not found", "couldn't read"] {
        assert!(!text.contains(incidental), "incidental {probe}: {text}");
    }
}

fn diagnostic(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
