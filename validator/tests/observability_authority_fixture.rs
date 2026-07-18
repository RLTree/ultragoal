use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub(crate) fn copy_current_inventory_inputs(live: &Path, root: &Path) {
    crate::authority_inputs::copy_declared_files(live, root);
    copy_file(live, root, "LANE_REGISTRY.json");
    copy_file(live, root, "templates/LANE_REGISTRY.json");
}

pub(crate) fn establish_fixture_authority(live: &Path, root: &Path) {
    let registry: serde_json::Value = serde_json::from_slice(
        &fs::read(root.join("LANE_REGISTRY.json")).expect("read fixture lane registry"),
    )
    .expect("parse fixture lane registry");
    let base = observed_source_base(&registry);
    let objects = live_object_directory(live);
    let alternate = root.join(".git/objects/info/alternates");
    fs::create_dir_all(alternate.parent().expect("alternate parent")).expect("create alternates");
    fs::write(&alternate, format!("{}\n", objects.display())).expect("write fixture alternate");
    let tree = git_text(root, &["rev-parse", "HEAD^{tree}"]);
    let authority = git_text(
        root,
        &["commit-tree", &tree, "-p", &base, "-m", "fixture authority"],
    );
    let _ = git_text(root, &["reset", "--hard", "-q", &authority]);
}

fn observed_source_base(registry: &serde_json::Value) -> String {
    let gates = registry["prelaunch_gates"]
        .as_array()
        .expect("fixture prelaunch gates");
    let mut base = None;
    for id in ["compile", "namespace", "standards"] {
        let gates = gates
            .iter()
            .filter(|gate| gate["id"] == id)
            .collect::<Vec<_>>();
        assert_eq!(
            gates.len(),
            1,
            "required fixture gate missing or duplicated"
        );
        let gate = gates[0];
        assert_eq!(gate["status"], "current", "fixture gate is not current");
        assert_eq!(
            gate["evidence_status"], "current",
            "fixture evidence is not current"
        );
        let commit = gate["observed_source_base"]["commit"]
            .as_str()
            .expect("fixture gate source base commit")
            .to_owned();
        assert!(
            matches!(commit.len(), 40 | 64) && commit.bytes().all(|byte| byte.is_ascii_hexdigit()),
            "fixture gate source base is not an object identifier"
        );
        assert!(
            base.as_ref().is_none_or(|value| value == &commit),
            "fixture gates disagree"
        );
        base = Some(commit);
    }
    base.expect("fixture source base unavailable")
}

fn live_object_directory(live: &Path) -> PathBuf {
    let common = git_text(live, &["rev-parse", "--git-common-dir"]);
    let common = Path::new(&common);
    let common = if common.is_absolute() {
        common.to_path_buf()
    } else {
        live.join(common)
    };
    let common = fs::canonicalize(common).expect("canonical live Git common directory");
    let objects = fs::canonicalize(common.join("objects")).expect("canonical live Git objects");
    assert!(
        fs::metadata(&objects)
            .expect("inspect canonical live Git objects")
            .is_dir(),
        "live Git objects must be a directory"
    );
    objects
}

fn git_text(root: &Path, args: &[&str]) -> String {
    let output = Command::new("/usr/bin/git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .expect("run fixture Git");
    assert!(output.status.success(), "fixture Git {args:?}: {output:?}");
    String::from_utf8(output.stdout)
        .expect("fixture Git UTF-8")
        .trim()
        .to_owned()
}

fn copy_file(live: &Path, root: &Path, relative: &str) {
    let target = root.join(relative);
    fs::create_dir_all(target.parent().expect("fixture authority parent"))
        .expect("create fixture authority parent");
    fs::copy(live.join(relative), target).expect("copy fixture authority input");
}
