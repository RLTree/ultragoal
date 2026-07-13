use std::collections::BTreeSet;
use std::fs;
use std::net::TcpListener;
#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;
#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
#[cfg(target_os = "macos")]
use std::process::Command;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use super::context::{BuildRequest, LiveContext};
use super::routine_work::{
    CheckClass, DirtySnapshot, ImpactGraph, LocalDirtyTree, PathMatcher, PlanMode, PlanRequest,
    PreparedRoutineExecution, RoutineAdapterSpec, RoutineCancellation, RoutineEffectRequest,
    RoutineInvocationSpec, RoutineMediationResult, RoutineMediatorStatus, RoutineNodeDisposition,
    RoutinePlan, RoutineReuseInput, RoutineRootGrant, TestProcessSetupFailure,
    bind_routine_invocation, bind_routine_invocation_with_read_sources,
    mediate_prepared_routine_execution, plan_routine, prepare_routine_execution,
    set_test_mediator_finish_failure, set_test_mediator_post_spawn_hook,
    set_test_mediator_pre_spawn_hook, set_test_output_capture_hook, set_test_process_setup_failure,
    set_test_read_source_capture_hook, test_spawn_count,
};
use super::support::{TempRepo, node, path, route};

struct MediatorFixture {
    repo: TempRepo,
    context: LiveContext,
    graph: ImpactGraph,
    snapshot: DirtySnapshot,
    plan: RoutinePlan,
}

fn mediator_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn mediator_context(repo: &TempRepo, profile: &str) -> LiveContext {
    mediator_context_for_tool(repo, profile, "dash")
}

fn mediator_context_for_tool(repo: &TempRepo, profile: &str, tool: &str) -> LiveContext {
    LiveContext::build(
        BuildRequest::new(repo.root())
            .bind_non_secret_configuration("profile", profile)
            .probe_tool("sandbox-exec")
            .probe_tool(tool),
    )
    .unwrap()
}

fn mediator_graph() -> ImpactGraph {
    mediator_graph_for_tool("dash")
}

fn mediator_graph_for_tool(tool: &str) -> ImpactGraph {
    ImpactGraph::new(
        vec![
            node("syntax", &[], CheckClass::Routine, tool, None),
            node("compile", &["syntax"], CheckClass::Routine, tool, None),
            node("unit", &["compile"], CheckClass::Routine, tool, None),
        ],
        vec![
            route(
                "route-src",
                PathMatcher::Prefix(path("src")),
                &["compile"],
                false,
            ),
            route(
                "route-release",
                PathMatcher::Exact(path("release.json")),
                &["unit"],
                true,
            ),
        ],
        Vec::new(),
    )
    .unwrap()
}

fn strict_fixture(label: &str) -> MediatorFixture {
    let repo = TempRepo::new(label);
    repo.write(".git/info/exclude", b"target/\n");
    for node_id in ["syntax", "compile", "unit"] {
        fs::create_dir_all(repo.root().join(format!("target/routine/{node_id}"))).unwrap();
    }
    repo.write("release.json", b"{\"strict\":true}\n");
    let context = mediator_context(&repo, "routine-mediator-strict");
    let snapshot = LocalDirtyTree::capture(&context).unwrap();
    let graph = mediator_graph();
    let plan = plan_routine(&context, &graph, &snapshot, PlanRequest::routine()).unwrap();
    assert_eq!(plan.affected_set().mode(), PlanMode::Strict);
    MediatorFixture {
        repo,
        context,
        graph,
        snapshot,
        plan,
    }
}

fn fallback_fixture(label: &str) -> MediatorFixture {
    let repo = TempRepo::new(label);
    repo.write(".git/info/exclude", b"target/\n");
    fs::create_dir_all(repo.root().join("target/routine/compile")).unwrap();
    repo.write("src/lib.rs", b"pub fn value() -> u8 { 61 }\n");
    let context = LiveContext::build(
        BuildRequest::new(repo.root())
            .bind_non_secret_configuration("profile", "routine-mediator-fallback")
            .probe_tool("sandbox-exec")
            .probe_tool("definitely-missing-routine-mediator")
            .probe_tool("dash"),
    )
    .unwrap();
    let snapshot = LocalDirtyTree::capture(&context).unwrap();
    let graph = ImpactGraph::new(
        vec![node(
            "compile",
            &[],
            CheckClass::Routine,
            "definitely-missing-routine-mediator",
            Some("dash"),
        )],
        vec![route(
            "route-src",
            PathMatcher::Prefix(path("src")),
            &["compile"],
            false,
        )],
        Vec::new(),
    )
    .unwrap();
    let plan = plan_routine(&context, &graph, &snapshot, PlanRequest::routine()).unwrap();
    assert_eq!(plan.checks()[0].selected_tool(), "dash");
    MediatorFixture {
        repo,
        context,
        graph,
        snapshot,
        plan,
    }
}

fn fixture(label: &str, dirty: bool) -> MediatorFixture {
    fixture_for_tool(label, dirty, "dash")
}

fn fixture_for_tool(label: &str, dirty: bool, tool: &str) -> MediatorFixture {
    let repo = TempRepo::new(label);
    repo.write(".git/info/exclude", b"target/\n");
    for node_id in ["syntax", "compile", "unit"] {
        std::fs::create_dir_all(repo.root().join(format!("target/routine/{node_id}"))).unwrap();
    }
    if dirty {
        repo.write("src/lib.rs", b"pub fn value() -> u8 { 59 }\n");
    }
    let context = mediator_context_for_tool(&repo, "routine-mediator", tool);
    let snapshot = LocalDirtyTree::capture(&context).unwrap();
    let graph = mediator_graph_for_tool(tool);
    let plan = plan_routine(&context, &graph, &snapshot, PlanRequest::routine()).unwrap();
    MediatorFixture {
        repo,
        context,
        graph,
        snapshot,
        plan,
    }
}

fn command_script(node_id: &str) -> String {
    format!(
        "printf '%s' '{node_id}' > 'target/routine/{node_id}/result.txt'; printf '{{\"schema_version\":\"RoutineCommandReport-v1\",\"request_id\":\"%s\",\"protocol_id\":\"%s\",\"intent_id\":\"%s\",\"node_id\":\"%s\",\"outcome\":\"passed\",\"behavior_observed\":true}}' \"$HUL_ROUTINE_REQUEST_ID\" \"$HUL_ROUTINE_PROTOCOL_ID\" \"$HUL_ROUTINE_INTENT_ID\" \"$HUL_ROUTINE_NODE_ID\""
    )
}

fn command_file_script() -> Vec<u8> {
    format!(
        "node=$HUL_ROUTINE_NODE_ID; printf '%s' \"$node\" > \"target/routine/$node/result.txt\"; {}",
        report_script()
    )
    .into_bytes()
}

#[cfg(target_os = "macos")]
fn compile_containment_adversary(fixture: &MediatorFixture) -> PathBuf {
    let build = fixture
        .repo
        .root()
        .join("target/routine/containment-adversary");
    fs::create_dir_all(&build).unwrap();
    let source = build.join("containment-adversary.c");
    let executable = build.join("containment-adversary");
    fs::write(
        &source,
        br#"#define _POSIX_C_SOURCE 200809L
#include <errno.h>
#include <fcntl.h>
#include <limits.h>
#include <signal.h>
#include <spawn.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/syscall.h>
#include <sys/types.h>
#include <time.h>
#include <unistd.h>

static void joined(char *buffer, const char *scope, const char *name) {
    int length = snprintf(buffer, PATH_MAX, "%s/%s", scope, name);
    if (length < 0 || length >= PATH_MAX) _exit(80);
}

static void write_text(const char *scope, const char *name, const char *value) {
    char path[PATH_MAX];
    joined(path, scope, name);
    int descriptor = open(path, O_WRONLY | O_CREAT | O_TRUNC, 0600);
    if (descriptor < 0) _exit(81);
    size_t length = strlen(value);
    if (write(descriptor, value, length) != (ssize_t)length) _exit(82);
    if (close(descriptor) != 0) _exit(83);
}

static void write_number(const char *scope, const char *name, long value) {
    char text[64];
    int length = snprintf(text, sizeof(text), "%ld", value);
    if (length < 0 || length >= (int)sizeof(text)) _exit(84);
    write_text(scope, name, text);
}

static void record_parent(const char *scope) {
    write_number(scope, "parent.pid", (long)getpid());
    write_number(scope, "parent.pgid", (long)getpgrp());
    write_number(scope, "parent.sid", (long)getsid(0));
    write_text(scope, "activity", "a");
    write_text(scope, "ready", "ready");
}

static void record_child(const char *scope) {
    write_number(scope, "child.pid", (long)getpid());
    write_number(scope, "child.pgid", (long)getpgrp());
    write_number(scope, "child.sid", (long)getsid(0));
}

static const char *required_environment(const char *name) {
    const char *value = getenv(name);
    if (value == NULL || value[0] == '\0') _exit(85);
    return value;
}

static void emit_report(void) {
    int length = dprintf(
        STDOUT_FILENO,
        "{\"schema_version\":\"RoutineCommandReport-v1\",\"request_id\":\"%s\",\"protocol_id\":\"%s\",\"intent_id\":\"%s\",\"node_id\":\"%s\",\"outcome\":\"passed\",\"behavior_observed\":true}",
        required_environment("HUL_ROUTINE_REQUEST_ID"),
        required_environment("HUL_ROUTINE_PROTOCOL_ID"),
        required_environment("HUL_ROUTINE_INTENT_ID"),
        required_environment("HUL_ROUTINE_NODE_ID")
    );
    if (length <= 0) _exit(86);
}

static void append_forever(const char *scope) {
    char path[PATH_MAX];
    joined(path, scope, "activity");
    int descriptor = open(path, O_WRONLY | O_APPEND);
    if (descriptor < 0) _exit(87);
    const struct timespec pause = {.tv_sec = 0, .tv_nsec = 2000000};
    for (;;) {
        if (write(descriptor, "a", 1) != 1) _exit(88);
        nanosleep(&pause, NULL);
    }
}

static void append_bounded(const char *scope) {
    char path[PATH_MAX];
    joined(path, scope, "activity");
    int descriptor = open(path, O_WRONLY | O_APPEND);
    if (descriptor < 0) _exit(87);
    const struct timespec pause = {.tv_sec = 0, .tv_nsec = 2000000};
    for (int index = 0; index < 1500; ++index) {
        if (write(descriptor, "c", 1) != 1) _exit(88);
        nanosleep(&pause, NULL);
    }
    _exit(0);
}

extern char **environ;
extern pid_t vfork(void);
extern int syscall(int, ...);

int main(int argc, char **argv) {
    if (argc < 3 || argc > 4) return 79;
    const char *mode = argv[1];
    const char *scope = argv[2];
    if (strcmp(mode, "child-loop") == 0) {
        if (signal(SIGTERM, SIG_IGN) == SIG_ERR) return 78;
        record_child(scope);
        append_bounded(scope);
    }
    record_parent(scope);
    if (signal(SIGTERM, SIG_IGN) == SIG_ERR) return 78;

    if (strcmp(mode, "complete") == 0) {
        write_text(scope, "substitute.effect", "unexpected");
        emit_report();
        return 0;
    }
    if (strcmp(mode, "loop") == 0) {
        close(STDOUT_FILENO);
        close(STDERR_FILENO);
        append_forever(scope);
    }
    if (strcmp(mode, "overflow") == 0) {
        char output[4096];
        memset(output, 'x', sizeof(output));
        close(STDERR_FILENO);
        for (;;) write(STDOUT_FILENO, output, sizeof(output));
    }
    if (strcmp(mode, "join-existing-pgid") == 0) {
        if (argc != 4) return 72;
        char *end = NULL;
        long target = strtol(argv[3], &end, 10);
        if (end == argv[3] || *end != '\0' || target <= 0 || target > INT_MAX) return 71;
        write_number(scope, "join.target-pgid", target);
        close(STDOUT_FILENO);
        close(STDERR_FILENO);
        if (setpgid(0, (pid_t)target) != 0) {
            write_number(scope, "join.errno", (long)errno);
        } else {
            write_number(scope, "join.actual-pgid", (long)getpgrp());
        }
        append_forever(scope);
    }
    if (strcmp(mode, "fork-new-pgid") != 0 &&
        strcmp(mode, "fork-new-session") != 0 &&
        strcmp(mode, "posix-spawn") != 0 &&
        strcmp(mode, "vfork") != 0 &&
        strcmp(mode, "raw-fork") != 0) {
        return 77;
    }

    write_text(scope, "report.emitted", "emitted");
    emit_report();
    close(STDOUT_FILENO);
    close(STDERR_FILENO);
    pid_t child = -1;
    int attempt_error = 0;
    if (strcmp(mode, "posix-spawn") == 0) {
        posix_spawn_file_actions_t actions;
        if (posix_spawn_file_actions_init(&actions) != 0) _exit(70);
        posix_spawn_file_actions_addclose(&actions, STDOUT_FILENO);
        posix_spawn_file_actions_addclose(&actions, STDERR_FILENO);
        char *child_argv[] = {argv[0], "child-loop", (char *)scope, NULL};
        attempt_error = posix_spawn(&child, argv[0], &actions, NULL, child_argv, environ);
        posix_spawn_file_actions_destroy(&actions);
    } else if (strcmp(mode, "vfork") == 0) {
        child = vfork();
        if (child == 0) {
            execl(argv[0], argv[0], "child-loop", scope, (char *)NULL);
            _exit(69);
        }
    } else if (strcmp(mode, "raw-fork") == 0) {
        child = (pid_t)syscall(SYS_fork);
    } else {
        child = fork();
    }
    if (child < 0) {
        write_number(scope, "attempt.returned", (long)(attempt_error != 0 ? attempt_error : errno));
        _exit(76);
    }
    if (child > 0) {
        write_text(scope, "process-creation.succeeded", "unexpected");
        write_number(scope, "created-child.pid", (long)child);
        _exit(0);
    }

    if (signal(SIGTERM, SIG_IGN) == SIG_ERR) _exit(75);
    if (strcmp(mode, "fork-new-session") == 0) {
        if (setsid() < 0) _exit(74);
    } else if (setpgid(0, 0) != 0) {
        _exit(73);
    }
    record_child(scope);
    append_bounded(scope);
}
"#,
    )
    .unwrap();
    let output = Command::new("/usr/bin/clang")
        .args(["-std=c11", "-Wall", "-Wextra", "-Werror"])
        .arg(&source)
        .arg("-o")
        .arg(&executable)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "containment adversary compilation failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    executable
}

#[cfg(target_os = "macos")]
fn compile_mapping_adversary(fixture: &MediatorFixture) -> PathBuf {
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
fn shell_literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

#[cfg(target_os = "macos")]
fn runner_record_script(node_id: &str) -> String {
    let scope = format!("target/routine/{node_id}");
    format!(
        "printf '%s' \"$$\" > {scope}/parent.pid; printf '%s' \"$$\" > {scope}/parent.pgid; printf '%s' \"$$\" > {scope}/parent.sid; printf a > {scope}/activity; printf ready > {scope}/ready",
    )
}

#[cfg(target_os = "macos")]
fn adversary_script(node_id: &str, mode: &str) -> String {
    let scope = format!("target/routine/{node_id}");
    let record = runner_record_script(node_id);
    match mode {
        "complete" => format!("{record}; {}", report_script()),
        "loop" => format!("{record}; trap '' TERM; while :; do :; done"),
        "overflow" => format!("{record}; trap '' TERM; while :; do printf '%04096d' 0; done"),
        "fork-new-pgid" | "fork-new-session" | "posix-spawn" | "vfork" | "raw-fork" => {
            format!(
                "{record}; printf emitted > {scope}/report.emitted; {}; exec 1>&- 2>&-; (:); printf returned > {scope}/attempt.returned",
                report_script()
            )
        }
        _ => panic!("unsupported adversary mode {mode}"),
    }
}

#[cfg(target_os = "macos")]
fn setup_failure_script(node_id: &str) -> String {
    let record = runner_record_script(node_id);
    format!(
        "{record}; i=0; while test \"$i\" -lt 500000; do i=$((i + 1)); done; {}",
        report_script()
    )
}

#[cfg(target_os = "macos")]
fn ruby_join_existing_pgid_arguments(node_id: &str, target_pgid: i32) -> Vec<String> {
    let scope = format!("target/routine/{node_id}");
    let script = format!(
        "scope={scope:?}; File.write(\"#{{scope}}/parent.pid\", Process.pid.to_s); File.write(\"#{{scope}}/parent.pgid\", Process.getpgrp.to_s); File.write(\"#{{scope}}/parent.sid\", Process.pid.to_s); File.write(\"#{{scope}}/activity\", \"a\"); Process.setpgid(0, {target_pgid}); File.write(\"#{{scope}}/join.actual-pgid\", Process.getpgrp.to_s); File.write(\"#{{scope}}/ready\", \"ready\"); Signal.trap(\"TERM\", \"IGNORE\"); loop {{ }}"
    );
    vec!["--disable-gems".to_owned(), "-e".to_owned(), script]
}

#[cfg(target_os = "macos")]
fn ruby_mapping_adversary_arguments(node_id: &str, library: &Path, root: &Path) -> Vec<String> {
    let scope = root.join(format!("target/routine/{node_id}"));
    let effect = scope.join("mapping.effect");
    let script = format!(
        "scope={scope:?}; File.write(File.join(scope, \"parent.pid\"), Process.pid.to_s); File.write(File.join(scope, \"parent.pgid\"), Process.getpgrp.to_s); File.write(File.join(scope, \"parent.sid\"), Process.pid.to_s); File.write(File.join(scope, \"activity\"), \"a\"); File.write(File.join(scope, \"ready\"), \"ready\"); require \"fiddle\"; handle=Fiddle.dlopen({library:?}); function=Fiddle::Function.new(handle[\"write_effect\"], [Fiddle::TYPE_VOIDP], Fiddle::TYPE_INT); raise \"effect failed\" unless function.call(Fiddle::Pointer[{effect:?}]) == 0; STDOUT.write(%Q({{\"schema_version\":\"RoutineCommandReport-v1\",\"request_id\":\"#{{ENV.fetch('HUL_ROUTINE_REQUEST_ID')}}\",\"protocol_id\":\"#{{ENV.fetch('HUL_ROUTINE_PROTOCOL_ID')}}\",\"intent_id\":\"#{{ENV.fetch('HUL_ROUTINE_INTENT_ID')}}\",\"node_id\":\"#{{ENV.fetch('HUL_ROUTINE_NODE_ID')}}\",\"outcome\":\"passed\",\"behavior_observed\":true}}))",
        scope = scope.to_string_lossy(),
        library = library.to_string_lossy(),
        effect = effect.to_string_lossy(),
    );
    vec!["--disable-gems".to_owned(), "-e".to_owned(), script]
}

#[cfg(target_os = "macos")]
fn ruby_process_creation_arguments(node_id: &str, mode: &str) -> Vec<String> {
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
        "scope={scope:?}; File.write(File.join(scope, \"parent.pid\"), Process.pid.to_s); File.write(File.join(scope, \"parent.pgid\"), Process.getpgrp.to_s); File.write(File.join(scope, \"parent.sid\"), Process.pid.to_s); File.write(File.join(scope, \"activity\"), \"a\"); File.write(File.join(scope, \"ready\"), \"ready\"); File.write(File.join(scope, \"report.emitted\"), \"emitted\"); STDOUT.write(%Q({{\"schema_version\":\"RoutineCommandReport-v1\",\"request_id\":\"#{{ENV.fetch('HUL_ROUTINE_REQUEST_ID')}}\",\"protocol_id\":\"#{{ENV.fetch('HUL_ROUTINE_PROTOCOL_ID')}}\",\"intent_id\":\"#{{ENV.fetch('HUL_ROUTINE_INTENT_ID')}}\",\"node_id\":\"#{{ENV.fetch('HUL_ROUTINE_NODE_ID')}}\",\"outcome\":\"passed\",\"behavior_observed\":true}})); STDOUT.close; STDERR.close; Signal.trap(\"TERM\", \"IGNORE\"); {operation}; File.write(File.join(scope, \"attempt.returned\"), child.to_s); exit! 0",
    );
    vec!["--disable-gems".to_owned(), "-e".to_owned(), script]
}

#[cfg(target_os = "macos")]
fn executable_substitution_script(node_id: &str, command: &str) -> String {
    format!("{}; {command}", runner_record_script(node_id))
}

#[cfg(unix)]
#[derive(Debug)]
struct ReportedProcess {
    pid: i32,
    pgid: i32,
    sid: i32,
    activity: PathBuf,
}

#[cfg(unix)]
fn read_reported_process(scope: &Path) -> Option<ReportedProcess> {
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
fn wait_for_live_reported_process(scope: &Path) -> ReportedProcess {
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
fn wait_for_joined_process(scope: &Path, target_pgid: i32) -> ReportedProcess {
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

fn report_script() -> &'static str {
    "printf '{\"schema_version\":\"RoutineCommandReport-v1\",\"request_id\":\"%s\",\"protocol_id\":\"%s\",\"intent_id\":\"%s\",\"node_id\":\"%s\",\"outcome\":\"passed\",\"behavior_observed\":true}' \"$HUL_ROUTINE_REQUEST_ID\" \"$HUL_ROUTINE_PROTOCOL_ID\" \"$HUL_ROUTINE_INTENT_ID\" \"$HUL_ROUTINE_NODE_ID\""
}

fn invocations(
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

fn invocations_with_arguments(
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

fn invocations_with_read_source(
    fixture: &MediatorFixture,
    arguments: impl Fn(&str) -> Vec<String>,
    read_source: &str,
    timeout_ms: u64,
    output_budget_bytes: u64,
) -> Vec<RoutineInvocationSpec> {
    fixture
        .plan
        .checks()
        .iter()
        .map(|check| {
            bind_routine_invocation_with_read_sources(
                &fixture.context,
                &fixture.plan,
                check.node_id(),
                arguments(check.node_id()),
                vec![path(read_source)],
                timeout_ms,
                output_budget_bytes,
                vec![path(&format!("target/routine/{}", check.node_id()))],
            )
            .unwrap()
        })
        .collect()
}

fn prepared_with(
    fixture: &MediatorFixture,
    script: impl Fn(&str) -> String,
    timeout_ms: u64,
    output_budget_bytes: u64,
) -> PreparedRoutineExecution {
    prepared_with_arguments(
        fixture,
        |node_id| vec!["-c".to_owned(), script(node_id)],
        timeout_ms,
        output_budget_bytes,
    )
}

fn prepared_with_arguments(
    fixture: &MediatorFixture,
    arguments: impl Fn(&str) -> Vec<String>,
    timeout_ms: u64,
    output_budget_bytes: u64,
) -> PreparedRoutineExecution {
    prepare_routine_execution(
        &fixture.context,
        &fixture.graph,
        &fixture.snapshot,
        &fixture.plan,
        RoutineAdapterSpec::new(
            "routine",
            invocations_with_arguments(fixture, arguments, timeout_ms, output_budget_bytes),
        ),
    )
    .unwrap()
}

fn prepared_with_read_source(
    fixture: &MediatorFixture,
    arguments: impl Fn(&str) -> Vec<String>,
    read_source: &str,
    timeout_ms: u64,
    output_budget_bytes: u64,
) -> PreparedRoutineExecution {
    prepare_routine_execution(
        &fixture.context,
        &fixture.graph,
        &fixture.snapshot,
        &fixture.plan,
        RoutineAdapterSpec::new(
            "routine",
            invocations_with_read_source(
                fixture,
                arguments,
                read_source,
                timeout_ms,
                output_budget_bytes,
            ),
        ),
    )
    .unwrap()
}

fn prepared(fixture: &MediatorFixture) -> PreparedRoutineExecution {
    prepared_with(fixture, command_script, 10_000, 1024 * 1024)
}

fn effect_request(prepared: &PreparedRoutineExecution) -> &RoutineEffectRequest {
    match prepared {
        PreparedRoutineExecution::Effect(request) => request,
        PreparedRoutineExecution::NoOp(_) => panic!("dirty fixture emitted no-op"),
    }
}

fn issue_grant(
    prepared: &PreparedRoutineExecution,
    session_id: &str,
    recovery_for: Option<String>,
) -> RoutineRootGrant {
    RoutineRootGrant::test_issue(effect_request(prepared), session_id, recovery_for)
}

fn mediate(
    fixture: &MediatorFixture,
    prepared: PreparedRoutineExecution,
    grant: Option<RoutineRootGrant>,
    cancellation: RoutineCancellation,
    reuse: Vec<Vec<u8>>,
) -> RoutineMediationResult {
    mediate_prepared_routine_execution(
        &fixture.context,
        &fixture.plan,
        prepared,
        grant,
        cancellation,
        RoutineReuseInput::new(reuse),
    )
    .unwrap()
}

#[test]
fn mediator_fixture_catalog_is_exact_and_claimless() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/routine-public-mediator/cases.json"
    ))
    .unwrap();
    assert_eq!(fixture["schema_version"], "RoutinePublicMediatorCases-v1");
    assert_eq!(fixture["claim_effect"], "none");
    assert_eq!(fixture["production_grant_issuer"], "absent");
    let cases = fixture["cases"].as_array().unwrap();
    let ids = cases
        .iter()
        .map(|case| case["id"].as_str().unwrap())
        .collect::<BTreeSet<_>>();
    assert_eq!(ids.len(), cases.len());
    assert_eq!(ids.len(), 46);
}

#[test]
fn clean_noop_consumes_no_authority_spawns_nothing_and_writes_nothing() {
    let _serial = mediator_lock();
    let fixture = fixture("mediator-clean-noop", false);
    let before_tree = fixture.repo.tree();
    let before_status = fixture.repo.status();
    let before_spawns = test_spawn_count();
    let prepared = prepare_routine_execution(
        &fixture.context,
        &fixture.graph,
        &fixture.snapshot,
        &fixture.plan,
        RoutineAdapterSpec::new("routine", Vec::new()),
    )
    .unwrap();
    let result = mediate(
        &fixture,
        prepared,
        None,
        RoutineCancellation::new(),
        Vec::new(),
    );
    assert_eq!(result.status(), RoutineMediatorStatus::CompleteNoOp);
    assert!(result.request_id().is_none());
    assert!(result.nodes().is_empty());
    assert!(result.reuse_artifacts().is_empty());
    assert!(result.recovery_marker().is_none());
    assert!(result.support_limit().contains("public dispatch"));
    assert_eq!(test_spawn_count(), before_spawns);
    assert_eq!(fixture.repo.tree(), before_tree);
    assert_eq!(fixture.repo.status(), before_status);
}

#[test]
#[cfg(target_os = "macos")]
fn ordered_execution_emits_correlated_results_and_exact_reuse_skips_all_spawns() {
    let _serial = mediator_lock();
    let fixture = fixture("mediator-ordered-reuse", true);
    let original_status = fixture.repo.status();
    let before_spawns = test_spawn_count();
    let first_prepared = prepared(&fixture);
    let first_grant = issue_grant(&first_prepared, "ordered-session-1", None);
    let first = mediate(
        &fixture,
        first_prepared,
        Some(first_grant),
        RoutineCancellation::new(),
        Vec::new(),
    );
    assert_eq!(
        first.status(),
        RoutineMediatorStatus::CompleteExecution,
        "nodes={:?}",
        first.nodes()
    );
    assert_eq!(first.nodes().len(), 3);
    assert_eq!(
        first
            .nodes()
            .iter()
            .map(|node| node.node_id())
            .collect::<Vec<_>>(),
        ["syntax", "compile", "unit"]
    );
    assert!(first.nodes().iter().all(|node| {
        node.disposition() == RoutineNodeDisposition::Executed
            && node.result_artifact_sha256().is_some()
            && node.failure_code().is_none()
    }));
    assert_eq!(first.reuse_artifacts().len(), 3);
    assert_eq!(test_spawn_count(), before_spawns + 3);

    let second_prepared = prepared(&fixture);
    let second_grant = issue_grant(&second_prepared, "ordered-session-2", None);
    let second = mediate(
        &fixture,
        second_prepared,
        Some(second_grant),
        RoutineCancellation::new(),
        first.reuse_artifacts().to_vec(),
    );
    assert_eq!(second.status(), RoutineMediatorStatus::CompleteExecution);
    assert!(
        second
            .nodes()
            .iter()
            .all(|node| node.disposition() == RoutineNodeDisposition::Reused)
    );
    assert_eq!(second.reuse_artifacts(), first.reuse_artifacts());
    assert_eq!(test_spawn_count(), before_spawns + 3);
    assert_eq!(fixture.repo.status(), original_status);
}

#[test]
#[cfg(target_os = "macos")]
fn external_dash_script_replacement_cannot_execute_or_seed_reuse() {
    let _serial = mediator_lock();
    let fixture = fixture("mediator-external-dash-source", true);
    let owner = TempRepo::new("mediator-external-dash-source-owner");
    let command = owner.root().join("command.sh");
    fs::write(&command, b"exit 0\n").unwrap();
    let command_argument = command.to_string_lossy().into_owned();
    let first_prepared = prepared_with_arguments(
        &fixture,
        |_| vec![command_argument.clone()],
        10_000,
        1024 * 1024,
    );
    let first_grant = issue_grant(&first_prepared, "external-source-1", None);
    let replacement = owner.root().join("replacement.sh");
    fs::write(&replacement, command_file_script()).unwrap();
    fs::rename(&replacement, &command).unwrap();
    let before_spawns = test_spawn_count();
    let first = mediate(
        &fixture,
        first_prepared,
        Some(first_grant),
        RoutineCancellation::new(),
        Vec::new(),
    );
    assert_eq!(first.status(), RoutineMediatorStatus::IncompleteExecution);
    assert_eq!(
        first.nodes()[0].failure_code(),
        Some("MEDIATOR-CHECK-FAILED")
    );
    assert!(
        first
            .nodes()
            .iter()
            .all(|node| node.result_artifact_sha256().is_none())
    );
    assert!(first.reuse_artifacts().is_empty());
    assert_eq!(test_spawn_count(), before_spawns + 1);
    for node_id in ["syntax", "compile", "unit"] {
        assert!(
            !fixture
                .repo
                .root()
                .join(format!("target/routine/{node_id}/result.txt"))
                .exists(),
            "undeclared external source reached effect for {node_id}"
        );
    }

    fs::write(&command, b"exit 71\n").unwrap();
    let recovery_marker = first.recovery_marker().unwrap().to_owned();
    let retry_prepared = prepared_with_arguments(
        &fixture,
        |_| vec![command_argument.clone()],
        10_000,
        1024 * 1024,
    );
    let retry_grant = issue_grant(&retry_prepared, "external-source-2", Some(recovery_marker));
    let retry = mediate(
        &fixture,
        retry_prepared,
        Some(retry_grant),
        RoutineCancellation::new(),
        first.reuse_artifacts().to_vec(),
    );
    assert_eq!(retry.status(), RoutineMediatorStatus::IncompleteExecution);
    assert!(
        retry
            .nodes()
            .iter()
            .all(|node| node.disposition() != RoutineNodeDisposition::Reused)
    );
    assert!(retry.reuse_artifacts().is_empty());
    assert_eq!(test_spawn_count(), before_spawns + 2);
}

#[test]
#[cfg(target_os = "macos")]
fn bound_dash_source_executes_reuses_exactly_and_mutation_invalidates_reuse() {
    let _serial = mediator_lock();
    let fixture = fixture("mediator-bound-dash-source", true);
    let relative_source = "target/routine/command.sh";
    let source = fixture.repo.root().join(relative_source);
    fs::write(&source, command_file_script()).unwrap();
    let before_spawns = test_spawn_count();
    let first_prepared = prepared_with_read_source(
        &fixture,
        |_| vec![relative_source.to_owned()],
        relative_source,
        10_000,
        1024 * 1024,
    );
    let first_grant = issue_grant(&first_prepared, "bound-source-1", None);
    let first = mediate(
        &fixture,
        first_prepared,
        Some(first_grant),
        RoutineCancellation::new(),
        Vec::new(),
    );
    assert_eq!(first.status(), RoutineMediatorStatus::CompleteExecution);
    assert_eq!(first.reuse_artifacts().len(), 3);
    assert_eq!(test_spawn_count(), before_spawns + 3);

    let reuse_prepared = prepared_with_read_source(
        &fixture,
        |_| vec![relative_source.to_owned()],
        relative_source,
        10_000,
        1024 * 1024,
    );
    let reuse_grant = issue_grant(&reuse_prepared, "bound-source-2", None);
    let reused = mediate(
        &fixture,
        reuse_prepared,
        Some(reuse_grant),
        RoutineCancellation::new(),
        first.reuse_artifacts().to_vec(),
    );
    assert_eq!(reused.status(), RoutineMediatorStatus::CompleteExecution);
    assert!(
        reused
            .nodes()
            .iter()
            .all(|node| node.disposition() == RoutineNodeDisposition::Reused)
    );
    assert_eq!(test_spawn_count(), before_spawns + 3);

    fs::write(&source, b"exit 71\n").unwrap();
    let changed_prepared = prepared_with_read_source(
        &fixture,
        |_| vec![relative_source.to_owned()],
        relative_source,
        10_000,
        1024 * 1024,
    );
    let changed_grant = issue_grant(&changed_prepared, "bound-source-3", None);
    let changed = mediate(
        &fixture,
        changed_prepared,
        Some(changed_grant),
        RoutineCancellation::new(),
        first.reuse_artifacts().to_vec(),
    );
    assert_eq!(changed.status(), RoutineMediatorStatus::IncompleteExecution);
    assert_eq!(
        changed.nodes()[0].failure_code(),
        Some("MEDIATOR-CHECK-FAILED")
    );
    assert!(
        changed
            .nodes()
            .iter()
            .all(|node| node.disposition() != RoutineNodeDisposition::Reused)
    );
    assert!(changed.reuse_artifacts().is_empty());
    assert_eq!(test_spawn_count(), before_spawns + 4);
}

#[test]
#[cfg(target_os = "macos")]
fn post_spawn_context_mutation_requires_exact_recovery_before_retry() {
    let _serial = mediator_lock();
    let fixture = fixture("mediator-post-spawn-context-recovery", true);
    let first_prepared = prepared(&fixture);
    let first_grant = issue_grant(&first_prepared, "post-spawn-context-1", None);
    let marker = first_grant.test_recovery_marker();
    let root = fixture.repo.root().to_owned();
    set_test_mediator_post_spawn_hook(move || {
        fs::write(root.join("src/lib.rs"), b"pub fn value() -> u8 { 60 }\n").unwrap();
    });
    let before_spawns = test_spawn_count();
    let refused = mediate_prepared_routine_execution(
        &fixture.context,
        &fixture.plan,
        first_prepared,
        Some(first_grant),
        RoutineCancellation::new(),
        RoutineReuseInput::default(),
    )
    .expect_err("post-spawn context mutation must refuse final reconciliation");
    assert_eq!(refused.cause(), "mediator-result-context-stale");
    assert_eq!(test_spawn_count(), before_spawns + 1);
    assert_eq!(
        fs::read(fixture.repo.root().join("target/routine/syntax/result.txt")).unwrap(),
        b"syntax"
    );
    assert!(
        !fixture
            .repo
            .root()
            .join("target/routine/compile/result.txt")
            .exists()
    );

    fixture
        .repo
        .write("src/lib.rs", b"pub fn value() -> u8 { 59 }\n");
    fixture.context.revalidate().unwrap();

    let retry_prepared = prepared(&fixture);
    let retry_grant = issue_grant(&retry_prepared, "post-spawn-context-2", None);
    let retry = mediate_prepared_routine_execution(
        &fixture.context,
        &fixture.plan,
        retry_prepared,
        Some(retry_grant),
        RoutineCancellation::new(),
        RoutineReuseInput::default(),
    )
    .expect_err("fresh non-recovery authority must not silently retry started work");
    assert_eq!(retry.cause(), "mediator-recovery-authority-required");
    assert_eq!(test_spawn_count(), before_spawns + 1);

    let wrong_prepared = prepared(&fixture);
    let wrong_grant = issue_grant(
        &wrong_prepared,
        "post-spawn-context-3",
        Some(format!("sha256:{}", "f".repeat(64))),
    );
    let wrong = mediate_prepared_routine_execution(
        &fixture.context,
        &fixture.plan,
        wrong_prepared,
        Some(wrong_grant),
        RoutineCancellation::new(),
        RoutineReuseInput::default(),
    )
    .expect_err("substituted recovery marker must not clear ambiguity");
    assert_eq!(wrong.cause(), "mediator-recovery-authority-required");
    assert_eq!(test_spawn_count(), before_spawns + 1);

    let recovery_prepared = prepared(&fixture);
    let recovery_grant = issue_grant(&recovery_prepared, "post-spawn-context-4", Some(marker));
    let recovered = mediate(
        &fixture,
        recovery_prepared,
        Some(recovery_grant),
        RoutineCancellation::new(),
        Vec::new(),
    );
    assert_eq!(recovered.status(), RoutineMediatorStatus::CompleteExecution);
    assert!(recovered.recovery_marker().is_none());
    assert!(recovered.nodes().iter().all(|node| {
        node.disposition() == RoutineNodeDisposition::Executed
            && node.result_artifact_sha256().is_some()
    }));
    assert_eq!(recovered.reuse_artifacts().len(), 3);
    assert_eq!(test_spawn_count(), before_spawns + 4);
}

#[test]
#[cfg(target_os = "macos")]
fn post_spawn_finish_failure_keeps_the_exact_recovery_barrier() {
    let _serial = mediator_lock();
    let fixture = fixture("mediator-post-spawn-finish-recovery", true);
    let first_prepared = prepared(&fixture);
    let first_grant = issue_grant(&first_prepared, "post-spawn-finish-1", None);
    let marker = first_grant.test_recovery_marker();
    set_test_mediator_finish_failure();
    let before_spawns = test_spawn_count();
    let refused = mediate_prepared_routine_execution(
        &fixture.context,
        &fixture.plan,
        first_prepared,
        Some(first_grant),
        RoutineCancellation::new(),
        RoutineReuseInput::default(),
    )
    .expect_err("finish failure after execution must not expose success");
    assert_eq!(refused.cause(), "adapter-mediation-transition-incomplete");
    assert_eq!(test_spawn_count(), before_spawns + 3);

    let retry_prepared = prepared(&fixture);
    let retry_grant = issue_grant(&retry_prepared, "post-spawn-finish-2", None);
    let retry = mediate_prepared_routine_execution(
        &fixture.context,
        &fixture.plan,
        retry_prepared,
        Some(retry_grant),
        RoutineCancellation::new(),
        RoutineReuseInput::default(),
    )
    .expect_err("finish failure must leave the protocol recovery-blocked");
    assert_eq!(retry.cause(), "mediator-recovery-authority-required");
    assert_eq!(test_spawn_count(), before_spawns + 3);

    let recovery_prepared = prepared(&fixture);
    let recovery_grant = issue_grant(&recovery_prepared, "post-spawn-finish-3", Some(marker));
    let recovered = mediate(
        &fixture,
        recovery_prepared,
        Some(recovery_grant),
        RoutineCancellation::new(),
        Vec::new(),
    );
    assert_eq!(recovered.status(), RoutineMediatorStatus::CompleteExecution);
    assert!(recovered.recovery_marker().is_none());
    assert_eq!(test_spawn_count(), before_spawns + 6);
}

#[test]
#[cfg(target_os = "macos")]
fn post_spawn_setup_failures_cleanup_every_resource_and_require_exact_recovery() {
    let _serial = mediator_lock();
    let cases = [
        (
            TestProcessSetupFailure::ProcessGroup,
            "process-group",
            "MEDIATOR-PROCESS-GROUP-SETUP-INJECTED",
        ),
        (
            TestProcessSetupFailure::StdoutNonblocking,
            "stdout-nonblocking",
            "MEDIATOR-STDOUT-NONBLOCKING-INJECTED",
        ),
        (
            TestProcessSetupFailure::StderrNonblocking,
            "stderr-nonblocking",
            "MEDIATOR-STDERR-NONBLOCKING-INJECTED",
        ),
        (
            TestProcessSetupFailure::StdoutReaderStart,
            "stdout-reader-start",
            "MEDIATOR-STDOUT-READER-START-INJECTED",
        ),
        (
            TestProcessSetupFailure::StderrReaderStart,
            "stderr-reader-start",
            "MEDIATOR-STDERR-READER-START-INJECTED",
        ),
    ];

    for (point, label, expected_failure) in cases {
        let fixture = fixture(&format!("mediator-setup-{label}"), true);
        let first_prepared = prepared_with(&fixture, setup_failure_script, 10_000, 1024 * 1024);
        let first_grant = issue_grant(&first_prepared, &format!("setup-{label}-1"), None);
        let marker = first_grant.test_recovery_marker();
        let syntax = fixture.repo.root().join("target/routine/syntax");
        let hook_syntax = syntax.clone();
        set_test_process_setup_failure(point, move || {
            let process = wait_for_live_reported_process(&hook_syntax);
            assert_eq!(
                unsafe { libc::getpgid(process.pid) },
                process.pgid,
                "case={label} runner left the reported process group before injection"
            );
        });

        let before_spawns = test_spawn_count();
        let first = mediate(
            &fixture,
            first_prepared,
            Some(first_grant),
            RoutineCancellation::new(),
            Vec::new(),
        );
        assert_eq!(
            first.status(),
            RoutineMediatorStatus::IncompleteExecution,
            "case={label} nodes={:?}",
            first.nodes()
        );
        assert_eq!(first.nodes()[0].failure_code(), Some(expected_failure));
        assert!(
            first
                .nodes()
                .iter()
                .all(|node| node.result_artifact_sha256().is_none()),
            "case={label} exposed a success artifact"
        );
        assert!(first.reuse_artifacts().is_empty());
        assert_eq!(first.recovery_marker(), Some(marker.as_str()));
        assert_eq!(test_spawn_count(), before_spawns + 1);

        let process = read_reported_process(&syntax).unwrap();
        assert_process_absent(process.pid);
        assert_process_group_absent(process.pgid);
        assert_output_stopped(&process.activity, label);

        let retry_prepared = prepared_with(&fixture, setup_failure_script, 10_000, 1024 * 1024);
        let retry_grant = issue_grant(&retry_prepared, &format!("setup-{label}-2"), None);
        let retry = mediate_prepared_routine_execution(
            &fixture.context,
            &fixture.plan,
            retry_prepared,
            Some(retry_grant),
            RoutineCancellation::new(),
            RoutineReuseInput::default(),
        )
        .expect_err("fresh non-recovery authority must not retry setup-ambiguous work");
        assert_eq!(retry.cause(), "mediator-recovery-authority-required");
        assert_eq!(test_spawn_count(), before_spawns + 1);

        let wrong_prepared = prepared_with(&fixture, setup_failure_script, 10_000, 1024 * 1024);
        let wrong_grant = issue_grant(
            &wrong_prepared,
            &format!("setup-{label}-3"),
            Some(format!("sha256:{}", "f".repeat(64))),
        );
        let wrong = mediate_prepared_routine_execution(
            &fixture.context,
            &fixture.plan,
            wrong_prepared,
            Some(wrong_grant),
            RoutineCancellation::new(),
            RoutineReuseInput::default(),
        )
        .expect_err("substituted setup recovery marker must not proceed");
        assert_eq!(wrong.cause(), "mediator-recovery-authority-required");
        assert_eq!(test_spawn_count(), before_spawns + 1);

        let recovery_prepared = prepared_with(&fixture, setup_failure_script, 10_000, 1024 * 1024);
        let recovery_grant = issue_grant(
            &recovery_prepared,
            &format!("setup-{label}-4"),
            Some(marker),
        );
        let recovered = mediate(
            &fixture,
            recovery_prepared,
            Some(recovery_grant),
            RoutineCancellation::new(),
            Vec::new(),
        );
        assert_eq!(
            recovered.status(),
            RoutineMediatorStatus::CompleteExecution,
            "case={label} nodes={:?}",
            recovered.nodes()
        );
        assert!(recovered.recovery_marker().is_none());
        assert!(recovered.nodes().iter().all(|node| {
            node.disposition() == RoutineNodeDisposition::Executed
                && node.result_artifact_sha256().is_some()
        }));
        assert_eq!(recovered.reuse_artifacts().len(), 3);
        assert_eq!(test_spawn_count(), before_spawns + 4);
    }
}

#[test]
#[cfg(target_os = "macos")]
fn missing_forged_and_replayed_root_authority_fail_closed() {
    let _serial = mediator_lock();
    let fixture = fixture("mediator-authority", true);
    let before_spawns = test_spawn_count();

    let missing = mediate_prepared_routine_execution(
        &fixture.context,
        &fixture.plan,
        prepared(&fixture),
        None,
        RoutineCancellation::new(),
        RoutineReuseInput::default(),
    )
    .unwrap_err();
    assert_eq!(missing.cause(), "mediator-root-grant-missing");

    let forged_prepared = prepared(&fixture);
    let forged = issue_grant(&forged_prepared, "forged-session", None)
        .test_with_seal("sha256:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff");
    let error = mediate_prepared_routine_execution(
        &fixture.context,
        &fixture.plan,
        forged_prepared,
        Some(forged),
        RoutineCancellation::new(),
        RoutineReuseInput::default(),
    )
    .unwrap_err();
    assert_eq!(error.cause(), "mediator-root-grant-binding-invalid");

    for (label, grant) in [
        {
            let value = prepared(&fixture);
            let grant = issue_grant(&value, "cross-session", None)
                .test_with_session_id("substituted-session");
            ("session", (value, grant))
        },
        {
            let value = prepared(&fixture);
            let grant = issue_grant(&value, "cross-context", None)
                .test_with_context_id(format!("sha256:{}", "a".repeat(64)));
            ("context", (value, grant))
        },
        {
            let value = prepared(&fixture);
            let grant = issue_grant(&value, "cross-plan", None)
                .test_with_plan_id(format!("sha256:{}", "b".repeat(64)));
            ("plan", (value, grant))
        },
        {
            let value = prepared(&fixture);
            let grant =
                issue_grant(&value, "expanded-scope", None).test_with_scopes(vec![path("target")]);
            ("scope", (value, grant))
        },
    ] {
        let (value, grant) = grant;
        let error = mediate_prepared_routine_execution(
            &fixture.context,
            &fixture.plan,
            value,
            Some(grant),
            RoutineCancellation::new(),
            RoutineReuseInput::default(),
        )
        .expect_err("substituted root grant must fail");
        assert_eq!(
            error.cause(),
            "mediator-root-grant-binding-invalid",
            "variant={label}"
        );
    }

    let replay_prepared = prepared(&fixture);
    let duplicate_request = effect_request(&replay_prepared).test_duplicate();
    let replay_grant = issue_grant(&replay_prepared, "replay-session", None);
    let duplicate_grant = replay_grant.test_duplicate();
    let completed = mediate(
        &fixture,
        replay_prepared,
        Some(replay_grant),
        RoutineCancellation::new(),
        Vec::new(),
    );
    assert_eq!(completed.status(), RoutineMediatorStatus::CompleteExecution);
    let replay = mediate_prepared_routine_execution(
        &fixture.context,
        &fixture.plan,
        PreparedRoutineExecution::Effect(duplicate_request),
        Some(duplicate_grant),
        RoutineCancellation::new(),
        RoutineReuseInput::default(),
    )
    .unwrap_err();
    assert_eq!(replay.cause(), "mediator-root-grant-replayed");
    assert_eq!(test_spawn_count(), before_spawns + 3);
}

#[test]
#[cfg(target_os = "macos")]
fn fabricated_or_mutated_reuse_never_becomes_a_cache_hit() {
    let _serial = mediator_lock();
    let fixture = fixture("mediator-reuse-forgery", true);
    let first_prepared = prepared(&fixture);
    let first_grant = issue_grant(&first_prepared, "reuse-forgery-1", None);
    let first = mediate(
        &fixture,
        first_prepared,
        Some(first_grant),
        RoutineCancellation::new(),
        Vec::new(),
    );
    assert_eq!(first.status(), RoutineMediatorStatus::CompleteExecution);
    let mut started: serde_json::Value =
        serde_json::from_slice(&first.reuse_artifacts()[0]).unwrap();
    started["state"] = serde_json::Value::String("started".to_owned());
    let started_prepared = prepared(&fixture);
    let started_grant = issue_grant(&started_prepared, "reuse-started", None);
    let before_started = test_spawn_count();
    let refusal = mediate_prepared_routine_execution(
        &fixture.context,
        &fixture.plan,
        started_prepared,
        Some(started_grant),
        RoutineCancellation::new(),
        RoutineReuseInput::new(vec![serde_json::to_vec(&started).unwrap()]),
    )
    .expect_err("started reuse evidence must never be retried silently");
    assert_eq!(refusal.cause(), "mediator-ambiguous-started-artifact");
    assert_eq!(test_spawn_count(), before_started);

    let mut artifacts = first.reuse_artifacts().to_vec();
    let offset = artifacts[0]
        .iter()
        .position(|byte| *byte == b'c')
        .expect("artifact has mutable payload byte");
    artifacts[0][offset] = b'd';
    artifacts.push(
        br#"{"schema_version":"RoutineMediatedReuseArtifact-v1","state":"complete"}"#.to_vec(),
    );
    let before_second = test_spawn_count();
    let second_prepared = prepared(&fixture);
    let second_grant = issue_grant(&second_prepared, "reuse-forgery-2", None);
    let second = mediate(
        &fixture,
        second_prepared,
        Some(second_grant),
        RoutineCancellation::new(),
        artifacts,
    );
    assert_eq!(second.status(), RoutineMediatorStatus::CompleteExecution);
    assert!(
        second
            .nodes()
            .iter()
            .all(|node| node.disposition() == RoutineNodeDisposition::Executed)
    );
    assert_eq!(test_spawn_count(), before_second + 3);

    let mut nested = first.reuse_artifacts().to_vec();
    let mut nested_value: serde_json::Value = serde_json::from_slice(&nested[1]).unwrap();
    nested_value["result_artifact"]["behavior_sha256"] =
        serde_json::Value::String(format!("sha256:{}", "e".repeat(64)));
    nested[1] = serde_json::to_vec(&nested_value).unwrap();
    let nested_prepared = prepared(&fixture);
    let nested_grant = issue_grant(&nested_prepared, "reuse-forgery-3", None);
    let before_nested = test_spawn_count();
    let nested_result = mediate(
        &fixture,
        nested_prepared,
        Some(nested_grant),
        RoutineCancellation::new(),
        nested,
    );
    assert_eq!(
        nested_result.status(),
        RoutineMediatorStatus::CompleteExecution
    );
    assert_eq!(
        nested_result
            .nodes()
            .iter()
            .map(|node| node.disposition())
            .collect::<Vec<_>>(),
        [
            RoutineNodeDisposition::Reused,
            RoutineNodeDisposition::Executed,
            RoutineNodeDisposition::Executed,
        ]
    );
    assert_eq!(test_spawn_count(), before_nested + 2);
}

#[test]
#[cfg(target_os = "macos")]
fn strict_expansion_and_capability_fallback_execute_the_selected_plan_exactly() {
    let _serial = mediator_lock();
    let strict = strict_fixture("mediator-strict");
    assert_eq!(
        strict
            .plan
            .checks()
            .iter()
            .map(|check| check.node_id())
            .collect::<Vec<_>>(),
        ["syntax", "compile", "unit"]
    );
    let strict_prepared = prepared(&strict);
    let strict_grant = issue_grant(&strict_prepared, "strict-session", None);
    let strict_result = mediate(
        &strict,
        strict_prepared,
        Some(strict_grant),
        RoutineCancellation::new(),
        Vec::new(),
    );
    assert_eq!(
        strict_result.status(),
        RoutineMediatorStatus::CompleteExecution
    );
    assert_eq!(strict_result.nodes().len(), 3);

    let fallback = fallback_fixture("mediator-fallback");
    let fallback_prepared = prepared(&fallback);
    assert_eq!(
        effect_request(&fallback_prepared).intents()[0].selected_tool(),
        "dash"
    );
    let fallback_grant = issue_grant(&fallback_prepared, "fallback-session", None);
    let fallback_result = mediate(
        &fallback,
        fallback_prepared,
        Some(fallback_grant),
        RoutineCancellation::new(),
        Vec::new(),
    );
    assert_eq!(
        fallback_result.status(),
        RoutineMediatorStatus::CompleteExecution
    );
    assert_eq!(fallback_result.nodes().len(), 1);
    assert_eq!(fallback_result.nodes()[0].node_id(), "compile");
}

#[test]
#[cfg(target_os = "macos")]
fn current_user_owned_0555_runner_is_rejected_before_spawn_or_effect() {
    const CHILD_ENV: &str = "HUL_ROUTINE_OWNED_EXECUTABLE_CHILD";
    const TEST_NAME: &str =
        "runtime_mediator::current_user_owned_0555_runner_is_rejected_before_spawn_or_effect";

    let _serial = mediator_lock();
    if let Some(expected) = std::env::var_os(CHILD_ENV) {
        let expected = PathBuf::from(expected).canonicalize().unwrap();
        let fixture = fixture("mediator-owned-0555-executable", true);
        let selected = fixture
            .context
            .capabilities()
            .tool("dash")
            .and_then(|tool| tool.executable.as_deref())
            .map(PathBuf::from)
            .unwrap();
        assert_eq!(selected, expected);
        let metadata = fs::symlink_metadata(&selected).unwrap();
        assert_eq!(metadata.uid(), unsafe { libc::geteuid() });
        assert_eq!(metadata.mode() & 0o777, 0o555);

        let prepared = prepared(&fixture);
        let grant = issue_grant(&prepared, "owned-0555-runner", None);
        let before_spawns = test_spawn_count();
        let error = mediate_prepared_routine_execution(
            &fixture.context,
            &fixture.plan,
            prepared,
            Some(grant),
            RoutineCancellation::new(),
            RoutineReuseInput::default(),
        )
        .expect_err("a current-user-owned 0555 runner remains chmod-mutable");
        assert_eq!(error.cause(), "mediator-executable-path-mutable");
        assert_eq!(test_spawn_count(), before_spawns);
        for node_id in ["syntax", "compile", "unit"] {
            assert!(
                !fixture
                    .repo
                    .root()
                    .join(format!("target/routine/{node_id}/result.txt"))
                    .exists(),
                "owned runner produced an effect for {node_id}"
            );
        }
        return;
    }

    let owner = TempRepo::new("mediator-owned-runner-parent");
    let bin = owner.root().join("owned-bin");
    let executable = bin.join("dash");
    fs::create_dir_all(&bin).unwrap();
    fs::copy("/bin/dash", &executable).unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o555)).unwrap();
    fs::set_permissions(&bin, fs::Permissions::from_mode(0o555)).unwrap();
    let path =
        std::env::join_paths([bin.as_path(), Path::new("/usr/bin"), Path::new("/bin")]).unwrap();
    let output = Command::new(std::env::current_exe().unwrap())
        .args([TEST_NAME, "--exact", "--nocapture", "--test-threads=1"])
        .env(CHILD_ENV, &executable)
        .env("PATH", path)
        .output()
        .unwrap();
    fs::set_permissions(&bin, fs::Permissions::from_mode(0o755)).unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o755)).unwrap();
    assert!(
        output.status.success(),
        "owned-runner child failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
#[cfg(target_os = "macos")]
fn root_owned_system_shell_remains_eligible_for_non_root_execution() {
    let _serial = mediator_lock();
    let fixture = fixture("mediator-system-shell", true);
    let selected = fixture
        .context
        .capabilities()
        .tool("dash")
        .and_then(|tool| tool.executable.as_deref())
        .unwrap();
    assert_eq!(selected, "/bin/dash");
    let metadata = fs::symlink_metadata(selected).unwrap();
    assert_eq!(metadata.uid(), 0);
    assert_eq!(metadata.mode() & 0o022, 0);

    let prepared = prepared(&fixture);
    let grant = issue_grant(&prepared, "system-shell", None);
    let before_spawns = test_spawn_count();
    if unsafe { libc::geteuid() } == 0 {
        let error = mediate_prepared_routine_execution(
            &fixture.context,
            &fixture.plan,
            prepared,
            Some(grant),
            RoutineCancellation::new(),
            RoutineReuseInput::default(),
        )
        .expect_err("effective root must fail closed for path-based spawn");
        assert_eq!(error.cause(), "mediator-executable-path-mutable");
        assert_eq!(test_spawn_count(), before_spawns);
        return;
    }

    let result = mediate(
        &fixture,
        prepared,
        Some(grant),
        RoutineCancellation::new(),
        Vec::new(),
    );
    assert_eq!(result.status(), RoutineMediatorStatus::CompleteExecution);
    assert_eq!(test_spawn_count(), before_spawns + 3);
}

#[test]
#[cfg(target_os = "macos")]
fn concurrent_duplicate_grant_and_request_have_exactly_one_winner() {
    let _serial = mediator_lock();
    let fixture = fixture("mediator-concurrent-authority", true);
    let first = prepared(&fixture);
    let second_request = effect_request(&first).test_duplicate();
    let first_grant = issue_grant(&first, "concurrent-session", None);
    let second_grant = first_grant.test_duplicate();
    let before_spawns = test_spawn_count();
    let (left, right) = std::thread::scope(|scope| {
        let left = scope.spawn(|| {
            mediate_prepared_routine_execution(
                &fixture.context,
                &fixture.plan,
                first,
                Some(first_grant),
                RoutineCancellation::new(),
                RoutineReuseInput::default(),
            )
        });
        let right = scope.spawn(|| {
            mediate_prepared_routine_execution(
                &fixture.context,
                &fixture.plan,
                PreparedRoutineExecution::Effect(second_request),
                Some(second_grant),
                RoutineCancellation::new(),
                RoutineReuseInput::default(),
            )
        });
        (left.join().unwrap(), right.join().unwrap())
    });
    let outcomes = [left, right];
    assert_eq!(outcomes.iter().filter(|value| value.is_ok()).count(), 1);
    assert_eq!(outcomes.iter().filter(|value| value.is_err()).count(), 1);
    let winner = outcomes
        .iter()
        .find_map(|value| value.as_ref().ok())
        .unwrap();
    assert_eq!(winner.status(), RoutineMediatorStatus::CompleteExecution);
    let loser = outcomes
        .iter()
        .find_map(|value| value.as_ref().err())
        .unwrap();
    assert!(matches!(
        loser.cause(),
        "mediator-root-grant-replayed" | "adapter-mediation-transition-replayed"
    ));
    assert_eq!(test_spawn_count(), before_spawns + 3);
}

#[test]
#[cfg(target_os = "macos")]
fn active_protocol_attempt_blocks_a_distinct_grant_before_second_spawn() {
    let _serial = mediator_lock();
    let fixture = fixture("mediator-concurrent-distinct-authority", true);
    let first_prepared = prepared(&fixture);
    let first_grant = issue_grant(&first_prepared, "concurrent-distinct-1", None);
    let second_prepared = prepared(&fixture);
    let second_grant = issue_grant(&second_prepared, "concurrent-distinct-2", None);
    let (started_tx, started_rx) = std::sync::mpsc::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    set_test_mediator_post_spawn_hook(move || {
        started_tx.send(()).unwrap();
        release_rx.recv().unwrap();
    });
    let before_spawns = test_spawn_count();
    let (winner, loser) = std::thread::scope(|scope| {
        let winner = scope.spawn(|| {
            mediate_prepared_routine_execution(
                &fixture.context,
                &fixture.plan,
                first_prepared,
                Some(first_grant),
                RoutineCancellation::new(),
                RoutineReuseInput::default(),
            )
        });
        started_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("first child never crossed the successful-spawn boundary");
        let loser = mediate_prepared_routine_execution(
            &fixture.context,
            &fixture.plan,
            second_prepared,
            Some(second_grant),
            RoutineCancellation::new(),
            RoutineReuseInput::default(),
        );
        release_tx.send(()).unwrap();
        (winner.join().unwrap(), loser)
    });
    assert_eq!(
        loser.unwrap_err().cause(),
        "mediator-protocol-attempt-active"
    );
    assert_eq!(
        winner.unwrap().status(),
        RoutineMediatorStatus::CompleteExecution
    );
    assert_eq!(test_spawn_count(), before_spawns + 3);
}

#[test]
fn stale_dirty_bytes_and_output_scope_swap_are_refused_before_spawn() {
    let _serial = mediator_lock();
    let source_fixture = fixture("mediator-stale-source", true);
    let source_prepared = prepared(&source_fixture);
    let grant = issue_grant(&source_prepared, "stale-source-session", None);
    let source = source_fixture.repo.root().join("src/lib.rs");
    set_test_mediator_pre_spawn_hook(move || {
        fs::write(source, b"pub fn value() -> u8 { 60 }\n").unwrap();
    });
    let before_spawns = test_spawn_count();
    let stale = mediate_prepared_routine_execution(
        &source_fixture.context,
        &source_fixture.plan,
        source_prepared,
        Some(grant),
        RoutineCancellation::new(),
        RoutineReuseInput::default(),
    )
    .expect_err("mutated dirty bytes must invalidate the prepared snapshot");
    assert!(matches!(
        stale.cause(),
        "mediator-context-preflight-stale" | "mediator-dirty-snapshot-stale"
    ));
    assert_eq!(test_spawn_count(), before_spawns);

    let scope_fixture = fixture("mediator-scope-swap", true);
    let scope_prepared = prepared(&scope_fixture);
    let grant = issue_grant(&scope_prepared, "scope-swap-session", None);
    let scope = scope_fixture.repo.root().join("target/routine/syntax");
    let moved = scope_fixture
        .repo
        .root()
        .join("target/routine/syntax-moved");
    set_test_mediator_pre_spawn_hook(move || {
        fs::rename(&scope, moved).unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink("compile", scope).unwrap();
    });
    let swapped = mediate_prepared_routine_execution(
        &scope_fixture.context,
        &scope_fixture.plan,
        scope_prepared,
        Some(grant),
        RoutineCancellation::new(),
        RoutineReuseInput::default(),
    )
    .expect_err("scope replacement must fail the second preflight");
    assert_eq!(swapped.cause(), "mediator-output-scope-not-directory");
    assert_eq!(test_spawn_count(), before_spawns);

    #[cfg(unix)]
    {
        let root_fixture = fixture("mediator-root-swap", true);
        let root_prepared = prepared(&root_fixture);
        let root_grant = issue_grant(&root_prepared, "root-swap-session", None);
        let root = root_fixture.repo.root().to_path_buf();
        let moved = root.with_extension("moved");
        let hook_root = root.clone();
        let hook_moved = moved.clone();
        set_test_mediator_pre_spawn_hook(move || {
            fs::rename(&hook_root, &hook_moved).unwrap();
            std::os::unix::fs::symlink(&hook_moved, &hook_root).unwrap();
        });
        let replaced = mediate_prepared_routine_execution(
            &root_fixture.context,
            &root_fixture.plan,
            root_prepared,
            Some(root_grant),
            RoutineCancellation::new(),
            RoutineReuseInput::default(),
        )
        .expect_err("working-directory replacement must fail closed");
        fs::remove_file(&root).unwrap();
        fs::rename(&moved, &root).unwrap();
        assert!(matches!(
            replaced.cause(),
            "mediator-context-preflight-stale"
                | "mediator-working-directory-identity-invalid"
                | "mediator-working-directory-not-canonical"
        ));
        assert_eq!(test_spawn_count(), before_spawns);
    }
}

#[test]
#[cfg(unix)]
fn read_source_binding_refuses_symlinks_hardlinks_special_files_and_capture_races() {
    let _serial = mediator_lock();

    let symlink_fixture = fixture("mediator-read-source-symlink", true);
    symlink_fixture
        .repo
        .write("target/routine/real.sh", b"exit 0\n");
    std::os::unix::fs::symlink(
        "real.sh",
        symlink_fixture.repo.root().join("target/routine/source.sh"),
    )
    .unwrap();
    let symlink = bind_routine_invocation_with_read_sources(
        &symlink_fixture.context,
        &symlink_fixture.plan,
        "syntax",
        vec!["target/routine/source.sh".to_owned()],
        vec![path("target/routine/source.sh")],
        1_000,
        1_024,
        vec![path("target/routine/syntax")],
    )
    .unwrap_err();
    assert_eq!(symlink.cause(), "mediator-read-source-object-unsafe");

    let hardlink_fixture = fixture("mediator-read-source-hardlink", true);
    hardlink_fixture
        .repo
        .write("target/routine/backing.sh", b"exit 0\n");
    fs::hard_link(
        hardlink_fixture
            .repo
            .root()
            .join("target/routine/backing.sh"),
        hardlink_fixture
            .repo
            .root()
            .join("target/routine/source.sh"),
    )
    .unwrap();
    let hardlink = bind_routine_invocation_with_read_sources(
        &hardlink_fixture.context,
        &hardlink_fixture.plan,
        "syntax",
        vec!["target/routine/source.sh".to_owned()],
        vec![path("target/routine/source.sh")],
        1_000,
        1_024,
        vec![path("target/routine/syntax")],
    )
    .unwrap_err();
    assert_eq!(hardlink.cause(), "mediator-read-source-object-unsafe");

    let fifo_fixture = fixture("mediator-read-source-fifo", true);
    let fifo = fifo_fixture.repo.root().join("target/routine/source.fifo");
    let fifo_bytes = std::ffi::CString::new(fifo.as_os_str().as_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(fifo_bytes.as_ptr(), 0o600) }, 0);
    let special = bind_routine_invocation_with_read_sources(
        &fifo_fixture.context,
        &fifo_fixture.plan,
        "syntax",
        vec!["target/routine/source.fifo".to_owned()],
        vec![path("target/routine/source.fifo")],
        1_000,
        1_024,
        vec![path("target/routine/syntax")],
    )
    .unwrap_err();
    assert_eq!(special.cause(), "mediator-read-source-object-unsafe");

    let race_fixture = fixture("mediator-read-source-capture-race", true);
    let source = race_fixture.repo.root().join("target/routine/source.sh");
    let moved = race_fixture
        .repo
        .root()
        .join("target/routine/source-moved.sh");
    fs::write(&source, b"exit 0\n").unwrap();
    let hook_source = source.clone();
    let hook_moved = moved.clone();
    set_test_read_source_capture_hook(move || {
        fs::rename(&hook_source, &hook_moved).unwrap();
        std::os::unix::fs::symlink("source-moved.sh", &hook_source).unwrap();
    });
    let raced = bind_routine_invocation_with_read_sources(
        &race_fixture.context,
        &race_fixture.plan,
        "syntax",
        vec!["target/routine/source.sh".to_owned()],
        vec![path("target/routine/source.sh")],
        1_000,
        1_024,
        vec![path("target/routine/syntax")],
    )
    .unwrap_err();
    assert!(matches!(
        raced.cause(),
        "mediator-read-source-mutated-during-capture"
            | "mediator-read-source-replaced-during-capture"
    ));
    fs::remove_file(&source).unwrap();
    fs::rename(&moved, &source).unwrap();
}

#[test]
#[cfg(target_os = "macos")]
fn read_source_aba_before_spawn_is_refused_without_effect() {
    let _serial = mediator_lock();
    let fixture = fixture("mediator-read-source-aba", true);
    let relative_source = "target/routine/source.sh";
    let source = fixture.repo.root().join(relative_source);
    let moved = fixture
        .repo
        .root()
        .join("target/routine/source-original.sh");
    fs::write(&source, command_file_script()).unwrap();
    let prepared = prepared_with_read_source(
        &fixture,
        |_| vec![relative_source.to_owned()],
        relative_source,
        10_000,
        1024 * 1024,
    );
    let grant = issue_grant(&prepared, "read-source-aba", None);
    let hook_source = source.clone();
    let hook_moved = moved.clone();
    set_test_mediator_pre_spawn_hook(move || {
        fs::rename(&hook_source, &hook_moved).unwrap();
        fs::write(
            &hook_source,
            b"printf false-pass > target/routine/syntax/false-pass\n",
        )
        .unwrap();
        fs::remove_file(&hook_source).unwrap();
        fs::rename(&hook_moved, &hook_source).unwrap();
    });
    let before_spawns = test_spawn_count();
    let error = mediate_prepared_routine_execution(
        &fixture.context,
        &fixture.plan,
        prepared,
        Some(grant),
        RoutineCancellation::new(),
        RoutineReuseInput::default(),
    )
    .expect_err("read-source mutate/restore ambiguity must fail the second preflight");
    assert_eq!(error.cause(), "mediator-read-source-binding-stale");
    assert_eq!(test_spawn_count(), before_spawns);
    assert!(
        !fixture
            .repo
            .root()
            .join("target/routine/syntax/false-pass")
            .exists()
    );
}

#[test]
#[cfg(target_os = "macos")]
fn passing_report_cannot_hide_post_spawn_read_source_mutation() {
    let _serial = mediator_lock();
    let fixture = fixture("mediator-read-source-false-pass", true);
    let relative_source = "target/routine/source.sh";
    let source = fixture.repo.root().join(relative_source);
    let original = command_file_script();
    fs::write(&source, &original).unwrap();
    let prepared = prepared_with_read_source(
        &fixture,
        |_| vec![relative_source.to_owned()],
        relative_source,
        10_000,
        1024 * 1024,
    );
    let grant = issue_grant(&prepared, "read-source-false-pass", None);
    let hook_source = source.clone();
    set_test_mediator_post_spawn_hook(move || {
        fs::write(&hook_source, b"exit 71\n").unwrap();
        fs::write(&hook_source, original).unwrap();
    });
    let before_spawns = test_spawn_count();
    let result = mediate(
        &fixture,
        prepared,
        Some(grant),
        RoutineCancellation::new(),
        Vec::new(),
    );
    assert_eq!(result.status(), RoutineMediatorStatus::IncompleteExecution);
    assert_eq!(
        result.nodes()[0].failure_code(),
        Some("MEDIATOR-READ-SOURCE-CHANGED")
    );
    assert!(result.reuse_artifacts().is_empty());
    assert!(result.recovery_marker().is_some());
    assert_eq!(test_spawn_count(), before_spawns + 1);
    assert_eq!(
        fs::read(fixture.repo.root().join("target/routine/syntax/result.txt")).unwrap(),
        b"syntax",
        "the canonical passing child reached its authorized effect before causal validation"
    );
    assert!(
        result
            .nodes()
            .iter()
            .all(|node| node.result_artifact_sha256().is_none())
    );
}

#[test]
fn unsafe_output_objects_are_rejected_before_authority_is_consumed() {
    let _serial = mediator_lock();
    let before_spawns = test_spawn_count();

    let symlink_fixture = fixture("mediator-output-symlink", true);
    let symlink_prepared = prepared(&symlink_fixture);
    let grant = issue_grant(&symlink_prepared, "output-symlink-session", None);
    #[cfg(unix)]
    std::os::unix::fs::symlink(
        "../compile",
        symlink_fixture
            .repo
            .root()
            .join("target/routine/syntax/escape"),
    )
    .unwrap();
    let symlink = mediate_prepared_routine_execution(
        &symlink_fixture.context,
        &symlink_fixture.plan,
        symlink_prepared,
        Some(grant),
        RoutineCancellation::new(),
        RoutineReuseInput::default(),
    )
    .expect_err("symlink output must be rejected");
    assert_eq!(symlink.cause(), "mediator-output-object-unsafe");

    let hardlink_fixture = fixture("mediator-output-hardlink", true);
    hardlink_fixture
        .repo
        .write("target/routine/backing", b"same inode\n");
    fs::hard_link(
        hardlink_fixture.repo.root().join("target/routine/backing"),
        hardlink_fixture
            .repo
            .root()
            .join("target/routine/syntax/hardlink"),
    )
    .unwrap();
    let hardlink_prepared = prepared(&hardlink_fixture);
    let grant = issue_grant(&hardlink_prepared, "output-hardlink-session", None);
    let hardlink = mediate_prepared_routine_execution(
        &hardlink_fixture.context,
        &hardlink_fixture.plan,
        hardlink_prepared,
        Some(grant),
        RoutineCancellation::new(),
        RoutineReuseInput::default(),
    )
    .expect_err("hardlinked output must be rejected");
    assert_eq!(hardlink.cause(), "mediator-output-hardlink-refused");

    #[cfg(unix)]
    {
        use std::ffi::CString;
        use std::os::unix::ffi::OsStrExt;
        let fifo_fixture = fixture("mediator-output-fifo", true);
        let fifo = fifo_fixture.repo.root().join("target/routine/syntax/fifo");
        let fifo_bytes = CString::new(fifo.as_os_str().as_bytes()).unwrap();
        assert_eq!(unsafe { libc::mkfifo(fifo_bytes.as_ptr(), 0o600) }, 0);
        let prepared = prepared(&fifo_fixture);
        let grant = issue_grant(&prepared, "output-fifo-session", None);
        let special = mediate_prepared_routine_execution(
            &fifo_fixture.context,
            &fifo_fixture.plan,
            prepared,
            Some(grant),
            RoutineCancellation::new(),
            RoutineReuseInput::default(),
        )
        .expect_err("special output object must be rejected");
        assert_eq!(special.cause(), "mediator-output-object-unsafe");
    }
    assert_eq!(test_spawn_count(), before_spawns);
}

#[test]
fn descriptor_walk_refuses_a_nested_directory_swap_during_capture() {
    let _serial = mediator_lock();
    let fixture = fixture("mediator-nested-capture-race", true);
    let nested = fixture.repo.root().join("target/routine/syntax/nested");
    fs::create_dir_all(&nested).unwrap();
    fixture
        .repo
        .write("target/routine/syntax/nested/value", b"captured\n");
    let moved = fixture
        .repo
        .root()
        .join("target/routine/syntax/nested-moved");
    let hook_nested = nested.clone();
    set_test_output_capture_hook(move || {
        fs::rename(&hook_nested, moved).unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink("../compile", hook_nested).unwrap();
    });
    let prepared = prepared(&fixture);
    let grant = issue_grant(&prepared, "nested-capture-race", None);
    let before_spawns = test_spawn_count();
    let error = mediate_prepared_routine_execution(
        &fixture.context,
        &fixture.plan,
        prepared,
        Some(grant),
        RoutineCancellation::new(),
        RoutineReuseInput::default(),
    )
    .expect_err("descriptor-relative capture must reject nested path replacement");
    assert_eq!(error.cause(), "mediator-output-object-unsafe");
    assert_eq!(test_spawn_count(), before_spawns);
}

#[test]
#[cfg(target_os = "macos")]
fn cleared_environment_and_correlated_child_report_are_required() {
    let _serial = mediator_lock();
    let environment_fixture = fixture("mediator-environment-report", true);
    let guarded = |node_id: &str| {
        format!(
            "test \"$LANG\" = C && test \"$LC_ALL\" = C && test \"$PATH\" = /bin && test -z \"${{SSH_AUTH_SOCK+x}}\" || exit 70; printf '%s' '{node_id}' > 'target/routine/{node_id}/result.txt'; {}",
            report_script()
        )
    };
    let prepared = prepared_with(&environment_fixture, guarded, 10_000, 1024 * 1024);
    let grant = issue_grant(&prepared, "environment-session", None);
    let result = mediate(
        &environment_fixture,
        prepared,
        Some(grant),
        RoutineCancellation::new(),
        Vec::new(),
    );
    assert_eq!(result.status(), RoutineMediatorStatus::CompleteExecution);

    let missing_report_fixture = fixture("mediator-missing-report", true);
    let prepared = prepared_with(
        &missing_report_fixture,
        |_| "true".to_owned(),
        10_000,
        1024 * 1024,
    );
    let grant = issue_grant(&prepared, "missing-report-session", None);
    let result = mediate(
        &missing_report_fixture,
        prepared,
        Some(grant),
        RoutineCancellation::new(),
        Vec::new(),
    );
    assert_eq!(result.status(), RoutineMediatorStatus::IncompleteExecution);
    assert_eq!(result.reuse_artifacts().len(), 0);
    assert_eq!(
        result.nodes()[0].failure_code(),
        Some("MEDIATOR-COMMAND-REPORT-INVALID")
    );
    assert!(result.recovery_marker().is_some());
}

#[test]
#[cfg(target_os = "macos")]
fn sandbox_denies_undeclared_writes_and_network_connections() {
    let _serial = mediator_lock();
    let write_fixture = fixture("mediator-undeclared-write", true);
    let forbidden = write_fixture.repo.root().join("forbidden.txt");
    let prepared = prepared_with(
        &write_fixture,
        |_| "set -e; printf denied > forbidden.txt".to_owned(),
        10_000,
        1024 * 1024,
    );
    let grant = issue_grant(&prepared, "undeclared-write-session", None);
    let result = mediate(
        &write_fixture,
        prepared,
        Some(grant),
        RoutineCancellation::new(),
        Vec::new(),
    );
    assert_eq!(result.status(), RoutineMediatorStatus::IncompleteExecution);
    assert!(!forbidden.exists());
    assert!(result.reuse_artifacts().is_empty());

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let port = listener.local_addr().unwrap().port();
    let network_fixture = fixture("mediator-network-denied", true);
    let prepared = prepared_with(
        &network_fixture,
        |_| format!("exec /usr/bin/nc -w 1 127.0.0.1 {port}"),
        10_000,
        1024 * 1024,
    );
    let grant = issue_grant(&prepared, "network-denied-session", None);
    let result = mediate(
        &network_fixture,
        prepared,
        Some(grant),
        RoutineCancellation::new(),
        Vec::new(),
    );
    assert_eq!(result.status(), RoutineMediatorStatus::IncompleteExecution);
    assert!(result.reuse_artifacts().is_empty());
    assert!(
        listener.accept().is_err(),
        "sandboxed child reached listener"
    );
}

#[test]
#[cfg(target_os = "macos")]
fn cancellation_kills_the_process_group_and_emits_no_reuse() {
    let _serial = mediator_lock();
    let cancellation_fixture = fixture("mediator-cancel", true);
    let prepared = prepared_with(
        &cancellation_fixture,
        |node_id| adversary_script(node_id, "loop"),
        10_000,
        1024 * 1024,
    );
    let grant = issue_grant(&prepared, "cancel-session", None);
    let cancellation = RoutineCancellation::new();
    let trigger = cancellation.clone();
    let syntax = cancellation_fixture
        .repo
        .root()
        .join("target/routine/syntax");
    let verifier_syntax = syntax.clone();
    let canceller = std::thread::spawn(move || {
        let process = wait_for_live_reported_process(&verifier_syntax);
        trigger.cancel();
        process
    });
    let result = mediate(
        &cancellation_fixture,
        prepared,
        Some(grant),
        cancellation,
        Vec::new(),
    );
    let process = canceller.join().unwrap();
    assert_eq!(result.status(), RoutineMediatorStatus::Cancelled);
    assert!(result.reuse_artifacts().is_empty());
    assert!(result.recovery_marker().is_some());
    assert_process_absent(process.pid);
    assert_process_group_absent(process.pgid);
    assert_output_stopped(&process.activity, "cancellation");

    let partial_fixture = fixture("mediator-cancel-after-complete-node", true);
    let partial_prepared = prepared_with(
        &partial_fixture,
        |node_id| {
            if node_id == "syntax" {
                command_script(node_id)
            } else {
                adversary_script(node_id, "loop")
            }
        },
        10_000,
        1024 * 1024,
    );
    let partial_grant = issue_grant(&partial_prepared, "cancel-after-complete", None);
    let partial_cancellation = RoutineCancellation::new();
    let trigger = partial_cancellation.clone();
    let compile_scope = partial_fixture.repo.root().join("target/routine/compile");
    let canceller = std::thread::spawn(move || {
        let process = wait_for_live_reported_process(&compile_scope);
        trigger.cancel();
        process
    });
    let partial = mediate(
        &partial_fixture,
        partial_prepared,
        Some(partial_grant),
        partial_cancellation,
        Vec::new(),
    );
    let partial_process = canceller.join().unwrap();
    assert_eq!(partial.status(), RoutineMediatorStatus::Cancelled);
    assert_eq!(
        partial.nodes()[0].disposition(),
        RoutineNodeDisposition::Executed
    );
    assert_eq!(
        partial.nodes()[1].disposition(),
        RoutineNodeDisposition::Cancelled
    );
    assert!(partial.reuse_artifacts().is_empty());
    assert!(
        partial.recovery_marker().is_some(),
        "started completed work still requires explicit whole-batch recovery"
    );
    assert_process_absent(partial_process.pid);
    assert_process_group_absent(partial_process.pgid);
    assert_output_stopped(
        &partial_process.activity,
        "cancellation-after-complete-node",
    );
}

#[test]
#[cfg(target_os = "macos")]
fn timeout_requires_exact_recovery_authority_and_leaves_no_child() {
    let _serial = mediator_lock();
    let fixture = fixture("mediator-timeout-recovery", true);
    let slow = |node_id: &str| adversary_script(node_id, "loop");
    let syntax = fixture.repo.root().join("target/routine/syntax");
    let verifier_syntax = syntax.clone();
    let verifier = std::thread::spawn(move || wait_for_live_reported_process(&verifier_syntax));
    let first_prepared = prepared_with(&fixture, &slow, 500, 1024 * 1024);
    let first_grant = issue_grant(&first_prepared, "timeout-session-1", None);
    let first = mediate(
        &fixture,
        first_prepared,
        Some(first_grant),
        RoutineCancellation::new(),
        Vec::new(),
    );
    let first_process = verifier.join().unwrap();
    assert_eq!(first.status(), RoutineMediatorStatus::IncompleteExecution);
    assert_eq!(first.nodes()[0].failure_code(), Some("MEDIATOR-TIMEOUT"));
    assert!(first.reuse_artifacts().is_empty());
    let marker = first.recovery_marker().unwrap().to_owned();
    assert_process_absent(first_process.pid);
    assert_process_group_absent(first_process.pgid);
    assert_output_stopped(&first_process.activity, "timeout");

    let refused_prepared = prepared_with(&fixture, &slow, 500, 1024 * 1024);
    let refused_grant = issue_grant(&refused_prepared, "timeout-session-2", None);
    let before_refusal = test_spawn_count();
    let refusal = mediate_prepared_routine_execution(
        &fixture.context,
        &fixture.plan,
        refused_prepared,
        Some(refused_grant),
        RoutineCancellation::new(),
        RoutineReuseInput::default(),
    )
    .expect_err("ambiguous started work must require recovery authority");
    assert_eq!(refusal.cause(), "mediator-recovery-authority-required");
    assert_eq!(test_spawn_count(), before_refusal);

    let recovery_syntax = syntax.clone();
    let recovery_verifier =
        std::thread::spawn(move || wait_for_live_reported_process(&recovery_syntax));
    let recovery_prepared = prepared_with(&fixture, &slow, 500, 1024 * 1024);
    let recovery_grant = issue_grant(&recovery_prepared, "timeout-session-3", Some(marker));
    let recovered_attempt = mediate(
        &fixture,
        recovery_prepared,
        Some(recovery_grant),
        RoutineCancellation::new(),
        Vec::new(),
    );
    let recovery_process = recovery_verifier.join().unwrap();
    assert_eq!(
        recovered_attempt.status(),
        RoutineMediatorStatus::IncompleteExecution
    );
    assert!(recovered_attempt.recovery_marker().is_some());
    assert_process_absent(recovery_process.pid);
    assert_process_group_absent(recovery_process.pgid);
    assert_output_stopped(&recovery_process.activity, "timeout-recovery");
}

#[test]
#[cfg(target_os = "macos")]
fn compiled_same_process_executable_substitutions_are_denied_without_success_or_reuse() {
    let _serial = mediator_lock();
    for mode in [
        "user-owned-binary",
        "different-root-binary",
        "different-object-alias",
        "dev-fd-binary",
        "shebang-script",
    ] {
        let fixture = fixture(&format!("mediator-exec-substitution-{mode}"), true);
        let scope = fixture.repo.root().join("target/routine/syntax");
        let relative_scope = "target/routine/syntax";
        let mut alias_owner = None;
        let command = match mode {
            "user-owned-binary" => {
                let adversary = compile_containment_adversary(&fixture);
                format!(
                    "exec {} complete {}",
                    shell_literal(&adversary.to_string_lossy()),
                    shell_literal(relative_scope)
                )
            }
            "different-root-binary" => format!(
                "exec /bin/bash -c {}",
                shell_literal(&format!(
                    "printf unexpected > {relative_scope}/substitute.effect; {}",
                    report_script()
                ))
            ),
            "different-object-alias" => {
                let owner = TempRepo::new("mediator-different-object-alias-owner");
                let alias = owner.root().join("different-object-alias");
                std::os::unix::fs::symlink("/bin/bash", &alias).unwrap();
                let command = format!(
                    "exec {} -c {}",
                    shell_literal(&alias.to_string_lossy()),
                    shell_literal(&format!(
                        "printf unexpected > {relative_scope}/substitute.effect; {}",
                        report_script()
                    ))
                );
                alias_owner = Some(owner);
                command
            }
            "dev-fd-binary" => {
                let adversary = compile_containment_adversary(&fixture);
                format!(
                    "exec 9<{}; exec /dev/fd/9 complete {}",
                    shell_literal(&adversary.to_string_lossy()),
                    shell_literal(relative_scope)
                )
            }
            "shebang-script" => {
                let script = fixture
                    .repo
                    .root()
                    .join("target/routine/shebang-substitute");
                fs::write(
                    &script,
                    format!(
                        "#!/bin/dash\nprintf unexpected > \"$1/substitute.effect\"\n{}\n",
                        report_script()
                    ),
                )
                .unwrap();
                fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
                format!(
                    "exec {} {}",
                    shell_literal(&script.to_string_lossy()),
                    shell_literal(relative_scope)
                )
            }
            _ => unreachable!(),
        };
        let _alias_owner = &alias_owner;
        let prepared = prepared_with(
            &fixture,
            |node_id| executable_substitution_script(node_id, &command),
            10_000,
            1024 * 1024,
        );
        let grant = issue_grant(&prepared, &format!("exec-substitution-{mode}"), None);
        let result = mediate(
            &fixture,
            prepared,
            Some(grant),
            RoutineCancellation::new(),
            Vec::new(),
        );
        assert_eq!(
            result.status(),
            RoutineMediatorStatus::IncompleteExecution,
            "mode={mode} nodes={:?}",
            result.nodes()
        );
        assert_eq!(
            result.nodes()[0].failure_code(),
            Some("MEDIATOR-CHECK-FAILED"),
            "mode={mode} nodes={:?}",
            result.nodes()
        );
        assert!(
            result
                .nodes()
                .iter()
                .all(|node| node.result_artifact_sha256().is_none()),
            "mode={mode} exposed a result artifact"
        );
        assert!(result.reuse_artifacts().is_empty(), "mode={mode}");
        assert!(result.recovery_marker().is_some(), "mode={mode}");
        assert!(
            !scope.join("substitute.effect").exists(),
            "mode={mode} reached the substitute effect"
        );
        let process = read_reported_process(&scope).unwrap();
        assert_process_absent(process.pid);
        assert_process_group_absent(process.pgid);
        assert_output_stopped(&process.activity, mode);
    }
}

#[test]
#[cfg(target_os = "macos")]
fn exact_same_executable_reexec_remains_single_process_and_can_complete() {
    let _serial = mediator_lock();
    let fixture = fixture("mediator-exact-reexec", true);
    let prepared = prepared_with(
        &fixture,
        |node_id| {
            format!(
                "{}; exec /bin/dash -c {}",
                runner_record_script(node_id),
                shell_literal(&command_script(node_id))
            )
        },
        10_000,
        1024 * 1024,
    );
    let grant = issue_grant(&prepared, "exact-reexec-session", None);
    let result = mediate(
        &fixture,
        prepared,
        Some(grant),
        RoutineCancellation::new(),
        Vec::new(),
    );
    assert_eq!(result.status(), RoutineMediatorStatus::CompleteExecution);
    assert_eq!(result.reuse_artifacts().len(), 3);
    for node_id in ["syntax", "compile", "unit"] {
        let scope = fixture
            .repo
            .root()
            .join(format!("target/routine/{node_id}"));
        let process = read_reported_process(&scope).unwrap();
        assert_process_absent(process.pid);
        assert_process_group_absent(process.pgid);
    }
}

#[test]
#[cfg(target_os = "macos")]
fn compiled_user_owned_dlopen_mapping_is_denied_without_effect_success_or_reuse() {
    let _serial = mediator_lock();
    let fixture = fixture_for_tool("mediator-user-dlopen", true, "ruby");
    let library = compile_mapping_adversary(&fixture);
    let root = fixture.repo.root().to_owned();
    let prepared = prepared_with_arguments(
        &fixture,
        |node_id| ruby_mapping_adversary_arguments(node_id, &library, &root),
        10_000,
        1024 * 1024,
    );
    let grant = issue_grant(&prepared, "user-dlopen-session", None);
    let result = mediate(
        &fixture,
        prepared,
        Some(grant),
        RoutineCancellation::new(),
        Vec::new(),
    );
    assert_eq!(
        result.status(),
        RoutineMediatorStatus::IncompleteExecution,
        "nodes={:?}",
        result.nodes()
    );
    assert_eq!(
        result.nodes()[0].failure_code(),
        Some("MEDIATOR-CHECK-FAILED")
    );
    assert!(
        result
            .nodes()
            .iter()
            .all(|node| node.result_artifact_sha256().is_none())
    );
    assert!(result.reuse_artifacts().is_empty());
    assert!(result.recovery_marker().is_some());
    let scope = fixture.repo.root().join("target/routine/syntax");
    assert!(
        !scope.join("mapping.effect").exists(),
        "user-owned executable mapping reached its effect"
    );
    let process = read_reported_process(&scope).unwrap();
    assert_process_absent(process.pid);
    assert_process_group_absent(process.pgid);
    assert_output_stopped(&process.activity, "user-dlopen");
}

#[test]
#[cfg(target_os = "macos")]
fn pinned_single_process_route_completes_and_is_absent_after_natural_exit() {
    let _serial = mediator_lock();
    let fixture = fixture("mediator-compiled-single-process", true);
    let prepared = prepared_with(
        &fixture,
        |node_id| adversary_script(node_id, "complete"),
        10_000,
        1024 * 1024,
    );
    let grant = issue_grant(&prepared, "compiled-single-process", None);
    let result = mediate(
        &fixture,
        prepared,
        Some(grant),
        RoutineCancellation::new(),
        Vec::new(),
    );
    assert_eq!(result.status(), RoutineMediatorStatus::CompleteExecution);
    assert_eq!(result.reuse_artifacts().len(), 3);
    for node_id in ["syntax", "compile", "unit"] {
        let scope = fixture
            .repo
            .root()
            .join(format!("target/routine/{node_id}"));
        let process = read_reported_process(&scope).unwrap();
        assert_process_absent(process.pid);
        assert_process_group_absent(process.pgid);
        assert_output_stopped(&process.activity, "compiled-natural-exit");
    }
}

#[test]
#[cfg(target_os = "macos")]
fn fork_family_bypasses_are_killed_before_success_or_reuse() {
    let _serial = mediator_lock();
    for mode in [
        "fork-new-pgid",
        "fork-new-session",
        "posix-spawn",
        "vfork",
        "raw-fork",
    ] {
        let fixture = fixture_for_tool(&format!("mediator-{mode}"), true, "ruby");
        let prepared = prepared_with_arguments(
            &fixture,
            |node_id| ruby_process_creation_arguments(node_id, mode),
            10_000,
            1024 * 1024,
        );
        let grant = issue_grant(&prepared, &format!("{mode}-session"), None);
        let result = mediate(
            &fixture,
            prepared,
            Some(grant),
            RoutineCancellation::new(),
            Vec::new(),
        );
        assert_eq!(
            result.status(),
            RoutineMediatorStatus::IncompleteExecution,
            "mode={mode} nodes={:?}",
            result.nodes()
        );
        assert_eq!(
            result.nodes()[0].failure_code(),
            Some("MEDIATOR-CHECK-FAILED")
        );
        assert!(result.reuse_artifacts().is_empty());
        assert!(result.recovery_marker().is_some());

        let scope = fixture.repo.root().join("target/routine/syntax");
        let process = read_reported_process(&scope).unwrap();
        assert!(
            scope.join("report.emitted").exists(),
            "mode={mode} did not reach the valid-report control before fork"
        );
        assert!(
            !scope.join("attempt.returned").exists(),
            "mode={mode} process-creation denial returned instead of killing the adversary"
        );
        assert!(
            !scope.join("process-creation.succeeded").exists(),
            "mode={mode} process creation reached a success effect"
        );
        assert!(
            !scope.join("created-child.pid").exists(),
            "mode={mode} process creation returned a child PID"
        );
        for child_record in ["child.pid", "child.pgid", "child.sid"] {
            assert!(
                !scope.join(child_record).exists(),
                "mode={mode} created descendant record {child_record}"
            );
        }
        assert_process_absent(process.pid);
        assert_process_group_absent(process.pgid);
        assert_output_stopped(&process.activity, mode);
    }
}

#[test]
#[cfg(target_os = "macos")]
fn group_leader_joining_existing_pgid_is_reaped_by_direct_pid() {
    let _serial = mediator_lock();
    let target_pgid = unsafe { libc::getpgrp() };
    assert!(target_pgid > 0);
    let fixture = fixture_for_tool("mediator-join-existing-pgid", true, "ruby");
    let prepared = prepared_with_arguments(
        &fixture,
        |node_id| ruby_join_existing_pgid_arguments(node_id, target_pgid),
        10_000,
        1024 * 1024,
    );
    let grant = issue_grant(&prepared, "join-existing-pgid-session", None);
    let cancellation = RoutineCancellation::new();
    let trigger = cancellation.clone();
    let scope = fixture.repo.root().join("target/routine/syntax");
    let verifier_scope = scope.clone();
    let verifier = std::thread::spawn(move || {
        let process = wait_for_joined_process(&verifier_scope, target_pgid);
        trigger.cancel();
        process
    });
    let result = mediate(&fixture, prepared, Some(grant), cancellation, Vec::new());
    let process = verifier.join().unwrap();
    assert_eq!(result.status(), RoutineMediatorStatus::Cancelled);
    assert_eq!(result.nodes()[0].failure_code(), Some("MEDIATOR-CANCELLED"));
    assert!(
        result
            .nodes()
            .iter()
            .all(|node| node.result_artifact_sha256().is_none())
    );
    assert!(result.reuse_artifacts().is_empty());
    assert!(result.recovery_marker().is_some());
    assert_eq!(
        fs::read_to_string(scope.join("join.actual-pgid"))
            .unwrap()
            .trim()
            .parse::<i32>()
            .unwrap(),
        target_pgid
    );
    assert_ne!(process.pgid, target_pgid);
    assert_process_absent(process.pid);
    assert_process_group_absent(process.pgid);
    assert_output_stopped(&process.activity, "join-existing-pgid");
}

#[test]
#[cfg(target_os = "macos")]
fn term_resistant_output_overflow_is_reaped_without_reuse() {
    let _serial = mediator_lock();
    let overflow_fixture = fixture("mediator-output-overflow", true);
    let prepared = prepared_with(
        &overflow_fixture,
        |node_id| adversary_script(node_id, "overflow"),
        10_000,
        4096,
    );
    let grant = issue_grant(&prepared, "overflow-session", None);
    let overflow = mediate(
        &overflow_fixture,
        prepared,
        Some(grant),
        RoutineCancellation::new(),
        Vec::new(),
    );
    assert_eq!(
        overflow.status(),
        RoutineMediatorStatus::IncompleteExecution
    );
    assert_eq!(
        overflow.nodes()[0].failure_code(),
        Some("MEDIATOR-OUTPUT-LIMIT")
    );
    assert!(overflow.reuse_artifacts().is_empty());
    let scope = overflow_fixture.repo.root().join("target/routine/syntax");
    let process = read_reported_process(&scope).unwrap();
    assert_process_absent(process.pid);
    assert_process_group_absent(process.pgid);
    assert_output_stopped(&process.activity, "output-limit");
}

#[cfg(unix)]
fn assert_process_absent(pid: i32) {
    let status = unsafe { libc::kill(pid, 0) };
    let error = std::io::Error::last_os_error();
    assert_eq!(status, -1, "process {pid} survived cleanup");
    assert_eq!(error.raw_os_error(), Some(libc::ESRCH));
}

#[cfg(unix)]
fn assert_process_group_absent(pgid: i32) {
    assert!(pgid > 0, "reported PGID must be strictly positive: {pgid}");
    let target = pgid
        .checked_neg()
        .expect("strictly positive PGID has a negative signal target");
    let status = unsafe { libc::kill(target, 0) };
    let error = std::io::Error::last_os_error();
    assert_eq!(status, -1, "process group {pgid} survived cleanup");
    assert_eq!(error.raw_os_error(), Some(libc::ESRCH));
}

fn assert_output_stopped(activity: &Path, label: &str) {
    let stopped = fs::read(activity).unwrap();
    std::thread::sleep(Duration::from_millis(200));
    assert_eq!(
        fs::read(activity).unwrap(),
        stopped,
        "case={label} output continued after cleanup"
    );
}

#[test]
fn mediator_source_keeps_issuance_public_dispatch_and_claims_outside_the_boundary() {
    let mediator = include_str!("../../src/routine_work/runtime_adapter/mediator.rs");
    let model = include_str!("../../src/routine_work/runtime_adapter/mediator/model.rs");
    let process = include_str!("../../src/routine_work/runtime_adapter/mediator/process.rs");
    let filesystem = include_str!("../../src/routine_work/runtime_adapter/mediator/filesystem.rs");
    let production_filesystem = filesystem.split("#[cfg(all(test, unix))]").next().unwrap();
    assert!(mediator.contains("grant.ok_or_else"));
    assert!(mediator.contains("reserve_grant"));
    assert!(mediator.contains("validate_snapshot"));
    assert!(model.contains("Production construction is intentionally absent"));
    assert!(model.contains("#[cfg(test)]\nimpl RoutineRootGrant"));
    assert!(!mediator.contains("ClaimDecision"));
    assert!(!mediator.contains("public command"));
    assert!(process.contains("(deny network*)"));
    assert!(process.contains("(deny process-fork (with send-signal SIGKILL))"));
    assert!(!process.contains("(allow process-fork"));
    assert!(process.contains("(deny process-exec)"));
    assert!(process.contains("(allow process-exec (literal"));
    assert!(process.contains("let profile = sandbox_profile("));
    assert!(process.contains("&reads.absolute_sources()"));
    assert!(process.contains("(deny file-map-executable)"));
    assert!(process.contains(r#"file-map-executable (subpath \"/System\")"#));
    assert!(process.contains(r#"file-map-executable (subpath \"/usr/lib\")"#));
    assert!(process.contains("(deny file-read*)"));
    assert!(process.contains("(allow file-read* (literal"));
    assert!(process.contains(r#"file-read* (subpath \"/System\")"#));
    assert!(process.contains(r#"file-read* (subpath \"/usr/lib\")"#));
    assert!(process.contains("(deny file-write*)"));
    assert!(process.contains("setpgid(0, 0)"));
    assert!(process.contains("libc::kill(group.signal_target()?, signal)"));
    assert!(process.contains("libc::kill(group.signal_target()?, 0)"));
    assert!(!process.contains("libc::kill(group, signal)"));
    assert!(!process.contains("libc::kill(group, 0)"));
    assert!(process.contains("SpawnSetupGuard"));
    assert!(process.contains("RunningProcess"));
    assert!(mediator.contains("only the exact pinned executable identity may execute"));
    assert!(mediator.contains("unbound file reads are denied"));
    assert!(mediator.contains("identity/content/ctime revalidated"));
    assert!(mediator.contains("read_authority_sha256"));
    assert!(mediator.contains("user-owned executable mappings"));
    assert!(mediator.contains("multi-process runners are unsupported"));
    assert!(filesystem.contains("O_NOFOLLOW"));
    assert!(filesystem.contains("metadata.nlink() != 1"));
    assert!(production_filesystem.contains("libc::geteuid()"));
    assert!(production_filesystem.contains("metadata.uid() == effective_user_id"));
    assert!(production_filesystem.contains("reject_effective_user_control"));
    assert!(production_filesystem.contains("libc::faccessat"));
    assert!(production_filesystem.contains("libc::AT_EACCESS"));
    assert!(!production_filesystem.contains("libc::access(encoded.as_ptr(), libc::W_OK)"));
}
