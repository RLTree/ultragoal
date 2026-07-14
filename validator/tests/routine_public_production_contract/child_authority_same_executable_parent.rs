use super::*;
use std::fs;
use std::process::Command;

const ATTACK_DYLIB_SOURCE: &str = r#"
#include <fcntl.h>
#include <stdlib.h>
#include <sys/socket.h>
#include <sys/wait.h>
#include <unistd.h>

__attribute__((constructor)) static void same_executable_parent_attack(void) {
    const char *binary = getenv("HUL_ATTACK_BINARY");
    if (binary == NULL) return;
    unsetenv("DYLD_INSERT_LIBRARIES");
    unsetenv("DYLD_FORCE_FLAT_NAMESPACE");
    int channel[2];
    if (socketpair(AF_UNIX, SOCK_STREAM, 0, channel) != 0) _exit(120);
    const char forged[] = "prebuffered-attacker-capability";
    if (write(channel[0], forged, sizeof(forged)) < 0) _exit(121);
    pid_t child = fork();
    if (child < 0) _exit(122);
    if (child == 0) {
        close(channel[0]);
        if (dup2(channel[1], 198) < 0) _exit(123);
        fcntl(198, F_SETFD, 0);
        setenv("HUL_ROUTINE_CHILD_FD", "198", 1);
        execl(binary, binary, "--json", "check", "routine", (char *)0);
        _exit(124);
    }
    close(channel[0]);
    close(channel[1]);
    int status = 0;
    if (waitpid(child, &status, 0) != child) _exit(125);
    if (WIFEXITED(status)) _exit(WEXITSTATUS(status));
    _exit(126);
}
"#;

#[test]
fn live_same_executable_parent_fork_exec_and_prebuffer_refuse_without_writes() {
    let fixture = dirty_fixture("same-executable-parent");
    let substrate = fixture.home.join("same-executable-parent-substrate");
    fs::create_dir_all(&substrate).unwrap();
    let source = substrate.join("attack.c");
    let dylib = substrate.join("attack.dylib");
    fs::write(&source, ATTACK_DYLIB_SOURCE).unwrap();
    let compiled = Command::new("/usr/bin/clang")
        .args(["-dynamiclib", "-o"])
        .arg(&dylib)
        .arg(&source)
        .status()
        .unwrap();
    assert!(
        compiled.success(),
        "same-executable attack substrate failed"
    );

    let before_root = tree(&fixture.root);
    let before_home = tree(&fixture.home);
    let mut command = fixture.base_command();
    let binary = command.get_program().to_os_string();
    let output = command
        .args(["--json", "check", "routine"])
        .env("HUL_ATTACK_BINARY", &binary)
        .env("DYLD_INSERT_LIBRARIES", &dylib)
        .output()
        .unwrap();
    assert_child_refused(&output);
    assert_eq!(tree(&fixture.root), before_root);
    assert_eq!(tree(&fixture.home), before_home);
}
