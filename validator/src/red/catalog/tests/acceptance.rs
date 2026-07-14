use super::fixture::CatalogRoot;
use crate::red::catalog::RedCatalogProjectionRequest;

#[test]
fn exact_projection_is_sorted_digest_bound_and_zero_write() {
    let root = CatalogRoot::new("accepted");
    root.packet("zeta", "zeta");
    root.packet("alpha", "alpha");
    let request = RedCatalogProjectionRequest { root: root.path() };
    let projection = crate::red::catalog::render(request).expect("render");
    assert_eq!(projection.rows[0].id, "alpha");
    assert_eq!(projection.rows[1].id, "zeta");
    assert!(
        projection.rows.iter().all(|row| {
            row.packet_digest.starts_with("sha256:") && row.packet_digest.len() == 71
        })
    );
    root.write("templates/RED_FIXTURES.json", &projection.bytes);
    let before = tree(root.path());
    let checked = crate::red::catalog::check(request).expect("current");
    assert_eq!(checked, projection);
    assert_eq!(tree(root.path()), before);
}

fn tree(root: &std::path::Path) -> Vec<(String, Vec<u8>)> {
    let mut rows = Vec::new();
    walk(root, root, &mut rows);
    rows.sort();
    rows
}

fn walk(root: &std::path::Path, directory: &std::path::Path, rows: &mut Vec<(String, Vec<u8>)>) {
    let mut paths = std::fs::read_dir(directory)
        .expect("read directory")
        .map(|entry| entry.expect("entry").path())
        .collect::<Vec<_>>();
    paths.sort();
    for path in paths {
        if path.is_dir() {
            walk(root, &path, rows);
        } else {
            rows.push((
                path.strip_prefix(root)
                    .expect("relative")
                    .to_string_lossy()
                    .into_owned(),
                std::fs::read(path).expect("bytes"),
            ));
        }
    }
}

#[test]
fn live_red_catalog_is_the_exact_projection_of_every_packet() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("repository root");
    let projection = crate::red::catalog::check(RedCatalogProjectionRequest { root })
        .expect("live catalog current");
    assert_eq!(projection.rows.len(), 1280);
}
