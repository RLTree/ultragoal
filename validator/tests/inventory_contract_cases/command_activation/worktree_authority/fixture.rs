use std::path::Path;
use std::process::Command;

fn establish_fixture_authority(repo: &TestRepo) {
    repo.commit();
    let alternate = repo.root.join(".git/objects/info/alternates");
    fs::create_dir_all(alternate.parent().unwrap()).unwrap();
    fs::write(
        alternate,
        format!("{}\n", live_root().join(".git/objects").display()),
    )
    .unwrap();
    let tree = git_output(repo, &["rev-parse", "HEAD^{tree}"]);
    let base = git_output_path(&live_root(), &["rev-parse", "HEAD"]);
    let authority = git_output(
        repo,
        &["commit-tree", &tree, "-p", &base, "-m", "fixture authority"],
    );
    run_git(repo, &["reset", "--hard", "-q", &authority]);
}

fn git_output_path(root: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .unwrap();
    assert!(output.status.success());
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

fn git_output(repo: &TestRepo, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(&repo.root)
        .output()
        .unwrap();
    assert!(output.status.success());
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

fn run_git(repo: &TestRepo, args: &[&str]) {
    assert!(
        Command::new("git")
            .args(args)
            .current_dir(&repo.root)
            .status()
            .unwrap()
            .success()
    );
}
