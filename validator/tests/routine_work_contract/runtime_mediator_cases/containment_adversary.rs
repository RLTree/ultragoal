use super::*;

#[cfg(target_os = "macos")]
pub(crate) fn compile_containment_adversary(fixture: &MediatorFixture) -> PathBuf {
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

static void emit_untrusted_stdout(void) {
    int length = dprintf(STDOUT_FILENO, "child-authored-pass-ignored");
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
        emit_untrusted_stdout();
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
    emit_untrusted_stdout();
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
