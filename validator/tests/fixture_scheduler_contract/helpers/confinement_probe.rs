use std::io::Write;
use std::net::TcpStream;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;

#[cfg(unix)]
use std::os::unix::process::CommandExt;

#[repr(C)]
struct Limit {
    current: u64,
    maximum: u64,
}

unsafe extern "C" {
    fn getrlimit(resource: i32, limit: *mut Limit) -> i32;
    fn signal(signal: i32, handler: usize) -> usize;
}

const SIGTERM: i32 = 15;
const SIG_IGN: usize = 1;
const SIG_ERR: usize = usize::MAX;

fn limit(resource: i32) -> u64 {
    let mut value = Limit {
        current: 0,
        maximum: 0,
    };
    assert_eq!(unsafe { getrlimit(resource, &mut value) }, 0);
    value.current
}

fn ignore_sigterm() {
    assert_ne!(unsafe { signal(SIGTERM, SIG_IGN) }, SIG_ERR);
}

fn spawn_descendants(
    executable: &str,
    record_path: &str,
    count: usize,
    lifetime_millis: u64,
) -> (Vec<u32>, usize) {
    let sink = std::fs::File::create(Path::new(record_path).with_extension("stdio")).unwrap();
    let mut descendants = Vec::new();
    let mut denied = 0;
    for _ in 0..count {
        match Command::new(executable)
            .arg("ignore-term-child")
            .arg(lifetime_millis.to_string())
            .stdin(Stdio::null())
            .stdout(Stdio::from(sink.try_clone().unwrap()))
            .stderr(Stdio::from(sink.try_clone().unwrap()))
            .spawn()
        {
            Ok(child) => descendants.push(child.id()),
            Err(error) if error.raw_os_error() == Some(1) => denied += 1,
            Err(error) => panic!("unexpected descendant spawn error: {}", error),
        }
    }
    (descendants, denied)
}

fn spawn_detached_descendants(
    executable: &str,
    record_path: &str,
    count: usize,
    lifetime_millis: u64,
) -> (Vec<u32>, usize) {
    let sink = std::fs::File::create(Path::new(record_path).with_extension("stdio")).unwrap();
    let mut descendants = Vec::new();
    let mut denied = 0;
    for _ in 0..count {
        let mut command = Command::new(executable);
        command
            .arg("ignore-term-child")
            .arg(lifetime_millis.to_string())
            .stdin(Stdio::null())
            .stdout(Stdio::from(sink.try_clone().unwrap()))
            .stderr(Stdio::from(sink.try_clone().unwrap()));
        unsafe {
            command.pre_exec(|| {
                if libc_setsid() < 0 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
        match command.spawn() {
            Ok(child) => descendants.push(child.id()),
            Err(error) if error.raw_os_error() == Some(1) => denied += 1,
            Err(error) => panic!("unexpected detached descendant spawn error: {}", error),
        }
    }
    (descendants, denied)
}

unsafe extern "C" {
    #[link_name = "setsid"]
    fn libc_setsid() -> i32;
}

fn write_process_record(path: &str, descendants: &[u32], denied: usize) {
    let descendants = descendants
        .iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join(",");
    std::fs::write(
        path,
        format!(
            "group={}\ndescendants={descendants}\ndenied={denied}\n",
            std::process::id()
        ),
    )
    .unwrap();
}

fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    match args.get(1).map(String::as_str) {
        Some("limits") => println!("cpu={} as={} file={}", limit(0), limit(5), limit(1)),
        Some("write") => std::fs::write(&args[2], b"confined").unwrap(),
        Some("connect") => {
            TcpStream::connect(&args[2]).unwrap();
        }
        Some("descendant") => {
            let child = Command::new(&args[0]).arg("sleep-child").spawn().unwrap();
            println!("{}", child.id());
            std::io::stdout().flush().unwrap();
            std::thread::sleep(Duration::from_secs(30));
        }
        Some("sleep-child") => std::thread::sleep(Duration::from_secs(30)),
        Some("term-exit-descendants")
        | Some("all-ignore-descendants")
        | Some("output-limit-descendants") => {
            let mode = args[1].as_str();
            if mode != "term-exit-descendants" {
                ignore_sigterm();
            }
            let count = args[3].parse().unwrap();
            let lifetime_millis = args[4].parse().unwrap();
            let (descendants, denied) =
                spawn_descendants(&args[0], &args[2], count, lifetime_millis);
            write_process_record(&args[2], &descendants, denied);
            if mode == "output-limit-descendants" {
                std::io::stdout().write_all(&vec![b'x'; 64 * 1024]).unwrap();
                std::io::stdout().flush().unwrap();
            }
            std::thread::sleep(Duration::from_secs(30));
        }
        Some("ignore-term-child") => {
            ignore_sigterm();
            std::thread::sleep(Duration::from_millis(args[2].parse().unwrap()));
        }
        Some("detached-direct-exit") => {
            let count = args[3].parse().unwrap();
            let lifetime_millis = args[4].parse().unwrap();
            let (descendants, denied) =
                spawn_detached_descendants(&args[0], &args[2], count, lifetime_millis);
            write_process_record(&args[2], &descendants, denied);
        }
        Some("exit-record") => {
            write_process_record(&args[2], &[], 0);
            std::thread::sleep(Duration::from_millis(30));
        }
        _ => panic!("unknown confinement probe"),
    }
}
