use super::*;

#[cfg(target_os = "macos")]
pub(crate) fn compile_mapping_adversary(fixture: &MediatorFixture) -> PathBuf {
    let build = fixture.repo.root().join("target/routine/mapping-adversary");
    fs::create_dir_all(&build).unwrap();
    let source = build.join("mapping-adversary.c");
    let library = build.join("mapping-adversary.dylib");
    fs::write(
        &source,
        br#"#include <fcntl.h>
#include <string.h>
#include <unistd.h>

int write_effect(const char *path) {
    int descriptor = open(path, O_WRONLY | O_CREAT | O_TRUNC, 0600);
    if (descriptor < 0) return 91;
    const char *value = "unexpected";
    size_t length = strlen(value);
    if (write(descriptor, value, length) != (ssize_t)length) return 92;
    if (close(descriptor) != 0) return 93;
    return 0;
}
"#,
    )
    .unwrap();
    let output = Command::new("/usr/bin/clang")
        .args(["-std=c11", "-Wall", "-Wextra", "-Werror", "-dynamiclib"])
        .arg(&source)
        .arg("-o")
        .arg(&library)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "mapping adversary compilation failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    library
}

#[cfg(target_os = "macos")]
pub(crate) fn shell_literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

#[cfg(target_os = "macos")]
pub(crate) fn runner_record_script(node_id: &str) -> String {
    let scope = format!("target/routine/{node_id}");
    format!(
        "printf '%s' \"$$\" > {scope}/parent.pid; printf '%s' \"$$\" > {scope}/parent.pgid; printf '%s' \"$$\" > {scope}/parent.sid; printf a > {scope}/activity; printf ready > {scope}/ready",
    )
}

#[cfg(target_os = "macos")]
pub(crate) fn adversary_script(node_id: &str, mode: &str) -> String {
    let scope = format!("target/routine/{node_id}");
    let record = runner_record_script(node_id);
    match mode {
        "complete" => format!("{record}; {}", successful_exit_script()),
        "loop" => format!("{record}; trap '' TERM; while :; do :; done"),
        "overflow" => format!("{record}; trap '' TERM; while :; do printf '%04096d' 0; done"),
        "fork-new-pgid" | "fork-new-session" | "posix-spawn" | "vfork" | "raw-fork" => {
            format!(
                "{record}; printf emitted > {scope}/report.emitted; {}; exec 1>&- 2>&-; (:); printf returned > {scope}/attempt.returned",
                successful_exit_script()
            )
        }
        _ => panic!("unsupported adversary mode {mode}"),
    }
}

#[cfg(target_os = "macos")]
pub(crate) fn setup_failure_script(node_id: &str) -> String {
    let record = runner_record_script(node_id);
    format!(
        "{record}; i=0; while test \"$i\" -lt 500000; do i=$((i + 1)); done; {}",
        successful_exit_script()
    )
}

#[cfg(target_os = "macos")]
pub(crate) fn ruby_join_existing_pgid_arguments(node_id: &str, target_pgid: i32) -> Vec<String> {
    let scope = format!("target/routine/{node_id}");
    let script = format!(
        "scope={scope:?}; File.write(\"#{{scope}}/parent.pid\", Process.pid.to_s); File.write(\"#{{scope}}/parent.pgid\", Process.getpgrp.to_s); File.write(\"#{{scope}}/parent.sid\", Process.pid.to_s); File.write(\"#{{scope}}/activity\", \"a\"); Process.setpgid(0, {target_pgid}); File.write(\"#{{scope}}/join.actual-pgid\", Process.getpgrp.to_s); File.write(\"#{{scope}}/ready\", \"ready\"); Signal.trap(\"TERM\", \"IGNORE\"); loop {{ }}"
    );
    vec!["--disable-gems".to_owned(), "-e".to_owned(), script]
}

#[cfg(target_os = "macos")]
pub(crate) fn ruby_mapping_adversary_arguments(
    node_id: &str,
    library: &Path,
    root: &Path,
) -> Vec<String> {
    let scope = root.join(format!("target/routine/{node_id}"));
    let effect = scope.join("mapping.effect");
    let script = format!(
        "scope={scope:?}; File.write(File.join(scope, \"parent.pid\"), Process.pid.to_s); File.write(File.join(scope, \"parent.pgid\"), Process.getpgrp.to_s); File.write(File.join(scope, \"parent.sid\"), Process.pid.to_s); File.write(File.join(scope, \"activity\"), \"a\"); File.write(File.join(scope, \"ready\"), \"ready\"); require \"fiddle\"; handle=Fiddle.dlopen({library:?}); function=Fiddle::Function.new(handle[\"write_effect\"], [Fiddle::TYPE_VOIDP], Fiddle::TYPE_INT); raise \"effect failed\" unless function.call(Fiddle::Pointer[{effect:?}]) == 0; STDOUT.write(\"child-authored-pass-ignored\")",
        scope = scope.to_string_lossy(),
        library = library.to_string_lossy(),
        effect = effect.to_string_lossy(),
    );
    vec!["--disable-gems".to_owned(), "-e".to_owned(), script]
}

#[cfg(target_os = "macos")]
pub(crate) fn ruby_process_creation_arguments(node_id: &str, mode: &str) -> Vec<String> {
    let scope = format!("target/routine/{node_id}");
    let operation = match mode {
        "fork-new-pgid" => "child=fork { Process.setpgid(0, 0); loop { } }".to_owned(),
        "fork-new-session" => "child=fork { Process.setsid; loop { } }".to_owned(),
        "posix-spawn" => {
            "child=Process.spawn(\"/usr/bin/ruby\", \"--disable-gems\", \"-e\", \"loop { }\")"
                .to_owned()
        }
        "vfork" => "require \"fiddle\"; child=Fiddle::Function.new(Fiddle::Handle::DEFAULT[\"vfork\"], [], Fiddle::TYPE_INT).call".to_owned(),
        "raw-fork" => "require \"fiddle\"; child=Fiddle::Function.new(Fiddle::Handle::DEFAULT[\"syscall\"], [Fiddle::TYPE_LONG], Fiddle::TYPE_LONG).call(2)".to_owned(),
        _ => panic!("unsupported process creation mode {mode}"),
    };
    let script = format!(
        "scope={scope:?}; File.write(File.join(scope, \"parent.pid\"), Process.pid.to_s); File.write(File.join(scope, \"parent.pgid\"), Process.getpgrp.to_s); File.write(File.join(scope, \"parent.sid\"), Process.pid.to_s); File.write(File.join(scope, \"activity\"), \"a\"); File.write(File.join(scope, \"ready\"), \"ready\"); File.write(File.join(scope, \"report.emitted\"), \"emitted\"); STDOUT.write(\"child-authored-pass-ignored\"); STDOUT.close; STDERR.close; Signal.trap(\"TERM\", \"IGNORE\"); {operation}; File.write(File.join(scope, \"attempt.returned\"), child.to_s); exit! 0",
    );
    vec!["--disable-gems".to_owned(), "-e".to_owned(), script]
}

#[cfg(target_os = "macos")]
pub(crate) fn executable_substitution_script(node_id: &str, command: &str) -> String {
    format!("{}; {command}", runner_record_script(node_id))
}

#[cfg(unix)]
#[derive(Debug)]
pub(crate) struct ReportedProcess {
    pub(crate) pid: i32,
    pub(crate) pgid: i32,
    pub(crate) sid: i32,
    pub(crate) activity: PathBuf,
}

#[cfg(unix)]
pub(crate) fn read_reported_process(scope: &Path) -> Option<ReportedProcess> {
    let read_number = |name: &str| {
        fs::read_to_string(scope.join(name))
            .ok()
            .and_then(|value| value.trim().parse::<i32>().ok())
    };
    Some(ReportedProcess {
        pid: read_number("parent.pid")?,
        pgid: read_number("parent.pgid")?,
        sid: read_number("parent.sid")?,
        activity: scope.join("activity"),
    })
}

#[cfg(unix)]
pub(crate) fn wait_for_live_reported_process(scope: &Path) -> ReportedProcess {
    for _ in 0..2_000 {
        let Some(process) = read_reported_process(scope) else {
            std::thread::sleep(Duration::from_millis(2));
            continue;
        };
        let current_group = unsafe { libc::getpgid(process.pid) };
        let ready = scope.join("ready").exists();
        let active = fs::metadata(&process.activity).is_ok_and(|metadata| metadata.len() > 0);
        if process.pid == process.pgid
            && current_group == process.pgid
            && process.sid > 0
            && ready
            && active
        {
            return process;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    panic!("runner did not report a live TERM-resistant single process: {scope:?}");
}

#[cfg(unix)]
pub(crate) fn wait_for_joined_process(scope: &Path, target_pgid: i32) -> ReportedProcess {
    for _ in 0..2_000 {
        let Some(process) = read_reported_process(scope) else {
            std::thread::sleep(Duration::from_millis(2));
            continue;
        };
        let recorded = fs::read_to_string(scope.join("join.actual-pgid"))
            .ok()
            .and_then(|value| value.trim().parse::<i32>().ok());
        let current = unsafe { libc::getpgid(process.pid) };
        if recorded == Some(target_pgid)
            && current == target_pgid
            && process.pgid == process.pid
            && fs::metadata(&process.activity).is_ok_and(|metadata| metadata.len() > 0)
        {
            return process;
        }
        if let Ok(error) = fs::read_to_string(scope.join("join.errno")) {
            panic!("runner could not join existing PGID {target_pgid}: errno={error}");
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    panic!("runner did not join existing PGID {target_pgid}: {scope:?}");
}

pub(crate) fn successful_exit_script() -> &'static str {
    ":"
}

pub(crate) fn invocations(
    fixture: &MediatorFixture,
    script: impl Fn(&str) -> String,
    timeout_ms: u64,
    output_budget_bytes: u64,
) -> Vec<RoutineInvocationSpec> {
    invocations_with_arguments(
        fixture,
        |node_id| vec!["-c".to_owned(), script(node_id)],
        timeout_ms,
        output_budget_bytes,
    )
}

pub(crate) fn invocations_with_arguments(
    fixture: &MediatorFixture,
    arguments: impl Fn(&str) -> Vec<String>,
    timeout_ms: u64,
    output_budget_bytes: u64,
) -> Vec<RoutineInvocationSpec> {
    fixture
        .plan
        .checks()
        .iter()
        .map(|check| {
            bind_routine_invocation(
                &fixture.context,
                &fixture.plan,
                check.node_id(),
                arguments(check.node_id()),
                timeout_ms,
                output_budget_bytes,
                vec![path(&format!("target/routine/{}", check.node_id()))],
            )
            .unwrap()
        })
        .collect()
}
