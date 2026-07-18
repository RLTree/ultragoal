#[path = "authority/cases.rs"]
mod authority_cases;
#[path = "authority/descendants.rs"]
mod authority_descendants;
#[path = "authority/mutants.rs"]
mod authority_mutants;
mod compile_cases;
mod direct_routes;
#[path = "reservation/cases.rs"]
mod reservation_cases;
#[path = "reservation/mutants.rs"]
mod reservation_mutants;
#[path = "reservation/tokens.rs"]
mod reservation_tokens;

use super::fixture::TestRoot;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

const PRODUCTION: &str = "orchestration/product/authority/production/mod.rs";
const TRANSACTION: &str = "orchestration/product/authority/production/execution_transaction.rs";

#[test]
fn same_crate_boundaries_bind_each_exact_expression_to_a_green_exposure() {
    let hidden = MutantCrate::new("hidden");
    let members = MutantCrate::new("members");
    let opened = MutantCrate::new("opened");

    install_hidden_probes(&hidden);
    expose_types(&members);
    install_member_probes(&members);
    expose_types(&opened);
    expose_members(&opened);
    install_hidden_probes(&opened);
    install_member_probes(&opened);
    direct_routes::expose(&opened);

    let hidden_process = hidden.start_check();
    let member_process = members.start_check();
    let opened_process = opened.start_check();
    let hidden_output = hidden_process.wait_with_output().unwrap();
    let member_output = member_process.wait_with_output().unwrap();
    let opened_output = opened_process.wait_with_output().unwrap();

    compile_cases::assert_rejected(&hidden, &opened, &hidden_output, authority_cases::HIDDEN);
    compile_cases::assert_rejected(&hidden, &opened, &hidden_output, reservation_cases::HIDDEN);
    compile_cases::assert_rejected(&hidden, &opened, &hidden_output, direct_routes::CASES);
    compile_cases::assert_rejected(&members, &opened, &member_output, authority_cases::MEMBERS);
    compile_cases::assert_rejected(
        &members,
        &opened,
        &member_output,
        reservation_cases::MEMBERS,
    );
    assert!(
        opened_output.status.success(),
        "paired green exposure failed: {}",
        String::from_utf8_lossy(&opened_output.stderr)
    );
}

fn install_hidden_probes(fixture: &MutantCrate) {
    fixture.append(PRODUCTION, authority_mutants::HIDDEN);
    fixture.append(PRODUCTION, direct_routes::MUTANTS);
    fixture.append(TRANSACTION, authority_mutants::HIDDEN_EFFECTS);
    fixture.append(TRANSACTION, reservation_mutants::HIDDEN);
}

fn install_member_probes(fixture: &MutantCrate) {
    fixture.append(PRODUCTION, authority_mutants::MEMBERS);
    fixture.append(TRANSACTION, authority_mutants::MEMBER_EFFECTS);
    fixture.append(TRANSACTION, reservation_mutants::MEMBERS);
}

fn expose_types(fixture: &MutantCrate) {
    authority_descendants::expose_types(fixture);
    reservation_tokens::expose_types(fixture);
}

fn expose_members(fixture: &MutantCrate) {
    authority_descendants::expose_members(fixture);
    reservation_tokens::expose_members(fixture);
}

pub(super) struct MutantCrate {
    _scratch: TestRoot,
    crate_root: PathBuf,
    target_root: PathBuf,
}

impl MutantCrate {
    fn new(label: &str) -> Self {
        let scratch = TestRoot::new(&format!("same-crate-boundary-{label}"), 0o700);
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
        copy_manifest_inputs(repository_root, &crate_root, scratch.path());
        let target_root = crate_root.join("target");
        Self {
            _scratch: scratch,
            crate_root,
            target_root,
        }
    }

    pub(super) fn append(&self, relative: &str, addition: &str) {
        let path = self.source(relative);
        let source = fs::read_to_string(&path).unwrap();
        fs::write(path, format!("{source}\n{addition}\n")).unwrap();
    }

    pub(super) fn replace(&self, relative: &str, before: &str, after: &str) {
        let path = self.source(relative);
        let source = fs::read_to_string(&path).unwrap();
        let count = source.matches(before).count();
        assert!(
            count > 0,
            "missing exposure anchor {before:?} in {relative}"
        );
        fs::write(path, source.replace(before, after)).unwrap();
    }

    pub(super) fn unique_marker(&self, relative: &str, marker: &str) -> (usize, String) {
        let source = fs::read_to_string(self.source(relative)).unwrap();
        let matches: Vec<_> = source
            .lines()
            .enumerate()
            .filter(|(_, line)| line.contains(marker))
            .collect();
        assert_eq!(
            matches.len(),
            1,
            "marker {marker} must identify one expression"
        );
        (matches[0].0 + 1, matches[0].1.trim().to_owned())
    }

    fn source(&self, relative: &str) -> PathBuf {
        self.crate_root.join("src").join(relative)
    }

    fn start_check(&self) -> Child {
        Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
            .args(["check", "--offline", "--lib", "--message-format=json"])
            .current_dir(&self.crate_root)
            .env("CARGO_TARGET_DIR", &self.target_root)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap()
    }
}

fn copy_manifest_inputs(repository_root: &Path, crate_root: &Path, scratch: &Path) {
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
