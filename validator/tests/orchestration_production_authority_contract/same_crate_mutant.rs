mod authority_descendants;
mod direct_routes;
mod reservation_tokens;

use super::fixture::TestRoot;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

#[test]
fn same_crate_boundaries_have_independent_exact_path_controls() {
    let rejected_fixture = MutantCrate::new("rejected");
    let exposed_fixture = MutantCrate::new("exposed");
    let production_path = "orchestration/product/authority/production/mod.rs";
    let transaction_path = "orchestration/product/authority/production/execution_transaction.rs";
    let production = rejected_fixture.original(production_path);
    let transaction = rejected_fixture.original(transaction_path);
    rejected_fixture.write_with(
        production_path,
        &production,
        &format!(
            "{}\n{}",
            authority_descendants::PRODUCTION_MUTANT,
            direct_routes::MUTANTS
        ),
    );
    rejected_fixture.write_with(
        transaction_path,
        &transaction,
        &format!(
            "{}\n{}",
            authority_descendants::EFFECT_MUTANT,
            reservation_tokens::MUTANTS
        ),
    );
    reservation_tokens::install_setup(&rejected_fixture);

    exposed_fixture.write_with(production_path, &production, direct_routes::MUTANTS);
    exposed_fixture.write_with(transaction_path, &transaction, reservation_tokens::MUTANTS);
    reservation_tokens::install_setup(&exposed_fixture);
    reservation_tokens::expose_types(&exposed_fixture);
    direct_routes::expose(&exposed_fixture);
    reservation_tokens::expose_members(&exposed_fixture);

    let rejected_process = rejected_fixture.start_check();
    let exposed_process = exposed_fixture.start_check();
    let rejected = rejected_process.wait_with_output().unwrap();
    let exposed = exposed_process.wait_with_output().unwrap();
    let stderr = String::from_utf8_lossy(&rejected.stderr);
    assert!(!rejected.status.success(), "same-crate mutants compiled");
    authority_descendants::assert_rejected(&stderr);
    direct_routes::assert_rejected(&stderr);
    reservation_tokens::assert_types_rejected(&stderr);
    reservation_tokens::assert_methods_rejected(&stderr);

    assert!(
        exposed.status.success(),
        "red fixtures did not expose every exact mutant: {}",
        String::from_utf8_lossy(&exposed.stderr)
    );
}

struct MutantCrate {
    _scratch: TestRoot,
    crate_root: PathBuf,
    target_root: PathBuf,
}

impl MutantCrate {
    fn new(label: &str) -> Self {
        let scratch = TestRoot::new(&format!("same-crate-transaction-mutant-{label}"), 0o700);
        let crate_root = scratch.path().join("validator");
        let repository_root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        fs::create_dir(&crate_root).unwrap();
        copy_tree(
            &repository_root.join("validator/src"),
            &crate_root.join("src"),
        );
        copy_tree(
            &repository_root.join("validator/build_support"),
            &crate_root.join("build_support"),
        );
        for path in [
            "templates",
            ".codex/agents",
            "docs/ultragoal-contract-2026-07-successor-v2",
        ] {
            copy_tree(&repository_root.join(path), &scratch.path().join(path));
        }
        copy_required_manifest_files(repository_root, &crate_root, scratch.path());
        let target_root = crate_root.join("target");
        Self {
            _scratch: scratch,
            crate_root,
            target_root,
        }
    }

    fn source(&self, relative: &str) -> PathBuf {
        self.crate_root.join("src").join(relative)
    }

    fn original(&self, relative: &str) -> String {
        fs::read_to_string(self.source(relative)).unwrap()
    }

    fn write_with(&self, relative: &str, original: &str, addition: &str) {
        fs::write(self.source(relative), format!("{original}\n{addition}\n")).unwrap();
    }

    fn start_check(&self) -> Child {
        Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
            .args(["check", "--offline", "--lib", "--quiet"])
            .current_dir(&self.crate_root)
            .env("CARGO_TARGET_DIR", &self.target_root)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap()
    }
}

fn copy_required_manifest_files(repository_root: &Path, crate_root: &Path, scratch: &Path) {
    fs::create_dir_all(crate_root.join("tests")).unwrap();
    fs::copy(
        repository_root.join("validator/tests/public_api_witness.rs"),
        crate_root.join("tests/public_api_witness.rs"),
    )
    .unwrap();
    fs::create_dir_all(crate_root.join("examples")).unwrap();
    fs::copy(
        repository_root.join("validator/examples/hct_inventory.rs"),
        crate_root.join("examples/hct_inventory.rs"),
    )
    .unwrap();
    fs::copy(
        repository_root.join("validator/build.rs"),
        crate_root.join("build.rs"),
    )
    .unwrap();
    fs::copy(
        repository_root.join("plugin-manifest-draft.json"),
        scratch.join("plugin-manifest-draft.json"),
    )
    .unwrap();
    let manifest = fs::read_to_string(repository_root.join("validator/Cargo.toml"))
        .unwrap()
        .replace("[lints]\nworkspace = true\n\n", "")
        + "\n[workspace]\n";
    fs::write(crate_root.join("Cargo.toml"), manifest).unwrap();
}

fn copy_tree(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).unwrap();
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let target = destination.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).unwrap();
        }
    }
}
