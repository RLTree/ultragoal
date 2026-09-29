use super::*;

fn rust_tree(root: &Path) -> String {
    fn collect(directory: &Path, paths: &mut Vec<PathBuf>) {
        for entry in fs::read_dir(directory).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if entry.file_type().unwrap().is_dir() {
                collect(&path, paths);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                paths.push(path);
            }
        }
    }

    let mut paths = Vec::new();
    collect(root, &mut paths);
    paths.sort();
    paths
        .into_iter()
        .map(|path| fs::read_to_string(path).unwrap())
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
pub(crate) fn production_mutation_grant_has_one_private_mint_in_the_sealed_authority() {
    let source_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/repository_fit");
    let authority = rust_tree(&source_root.join("product_adapter/authority"));
    let adapter = fs::read_to_string(source_root.join("product_adapter/mod.rs")).unwrap();
    let local = fs::read_to_string(source_root.join("local/mod.rs")).unwrap();
    let effects = rust_tree(&source_root.join("local/effects"));

    assert_eq!(authority.matches("struct LocalMutationGrant").count(), 1);
    assert_eq!(authority.matches("const fn issue() -> Self").count(), 1);
    assert_eq!(authority.matches("LocalMutationGrant::issue()").count(), 1);
    assert_eq!(authority.matches("pub(super) const fn issue").count(), 1);
    assert!(!authority.contains("pub(crate) const fn issue"));
    assert!(!authority.contains("pub(in crate::repository_fit) const fn issue"));
    assert!(!authority.contains("pub(crate) _private"));
    assert!(adapter.contains("pub(in crate::repository_fit) use authority::LocalMutationGrant;"));
    assert!(!local.contains("LocalMutationGrant"));
    assert!(!local.contains("issue_local_mutation_grant"));
    assert_eq!(
        effects
            .matches("use crate::repository_fit::product_adapter::LocalMutationGrant;")
            .count(),
        2
    );
    assert_eq!(effects.matches("_grant: LocalMutationGrant").count(), 2);
}
