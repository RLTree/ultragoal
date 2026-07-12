use super::*;
use std::cell::RefCell;
use std::ffi::CString;
use std::os::unix::fs::MetadataExt;
use std::time::{SystemTime, UNIX_EPOCH};
const ORIGINAL: &[u8] = b"original";
const FOREIGN: &[u8] = b"foreign!";
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ReadEvent {
    BeforeAncestorOpen,
    AfterAncestorOpen,
    AfterFirstLeafRead,
    AfterAttachmentRevalidated,
    BeforeFinalRootCheck,
    BeforeFinalPathBinding,
    AfterFinalPathBinding,
}

type Observer = Box<dyn FnMut(ReadEvent)>;

thread_local! {
    static OBSERVER: RefCell<Option<Observer>> = RefCell::new(None);
}

pub(super) fn observe(event: ReadEvent) {
    OBSERVER.with(|slot| {
        if let Some(observer) = slot.borrow_mut().as_mut() {
            observer(event);
        }
    });
}

fn with_observer<T>(observer: impl FnMut(ReadEvent) + 'static, action: impl FnOnce() -> T) -> T {
    OBSERVER.with(|slot| {
        assert!(slot.replace(Some(Box::new(observer))).is_none());
    });
    let result = action();
    OBSERVER.with(|slot| slot.borrow_mut().take());
    result
}

struct Fixture {
    container: PathBuf,
    root: PathBuf,
    alternate: PathBuf,
}

impl Fixture {
    fn new(label: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let container = PathBuf::from(format!(
            "/tmp/hul-fit-ancestor-{label}-{}-{nonce}",
            std::process::id()
        ));
        let root = container.join("repo");
        let alternate = container.join("alternate");
        fs::create_dir_all(&root).unwrap();
        fs::create_dir_all(&alternate).unwrap();
        Self {
            container,
            root,
            alternate,
        }
    }

    fn write_root(&self, relative: &str, bytes: &[u8]) {
        write(&self.root.join(relative), bytes);
    }

    fn write_alternate(&self, relative: &str, bytes: &[u8]) {
        write(&self.alternate.join(relative), bytes);
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.container);
    }
}

fn write(path: &Path, bytes: &[u8]) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
}

fn assert_fail_closed(result: Result<Option<Vec<u8>>, FitError>) {
    let id = result.expect_err("detached bytes returned").id();
    assert!(
        matches!(id, FitErrorId::StaleBinding | FitErrorId::UnsafeObject),
        "{id:?}"
    );
}

fn observed_substitution(
    fixture: &Fixture,
    relative: &str,
    event: ReadEvent,
    occurrence: usize,
    active_relative: &str,
    alternate_relative: &str,
) {
    let workspace = Workspace::open(&fixture.root).unwrap();
    let active = fixture.root.join(active_relative);
    let alternate = fixture.alternate.join(alternate_relative);
    let detached = fixture.container.join("detached");
    let mut observed_occurrences = 0;
    let result = with_observer(
        move |observed_event| {
            if observed_event == event {
                if observed_occurrences == occurrence {
                    fs::rename(&active, &detached).unwrap();
                    fs::rename(&alternate, &active).unwrap();
                }
                observed_occurrences += 1;
            }
        },
        || workspace.read_file(&CanonicalPath::parse(relative).unwrap(), 1024),
    );
    assert_fail_closed(result);
    assert_eq!(fs::read(fixture.root.join(relative)).unwrap(), FOREIGN);
}

fn case(
    label: &str,
    relative: &str,
    event: ReadEvent,
    occurrence: usize,
    active: &str,
    alternate_file: &str,
    alternate: &str,
) {
    let fixture = Fixture::new(label);
    fixture.write_root(relative, ORIGINAL);
    fixture.write_alternate(alternate_file, FOREIGN);
    observed_substitution(&fixture, relative, event, occurrence, active, alternate);
}

#[test]
fn every_ordered_pre_binding_substitution_fails_closed() {
    use ReadEvent::*;
    let (bo, ao, lr) = (BeforeAncestorOpen, AfterAncestorOpen, AfterFirstLeafRead);
    let (ar, rr, fb) = (
        AfterAttachmentRevalidated,
        BeforeFinalRootCheck,
        BeforeFinalPathBinding,
    );
    case("before-open", "a/b/FILE", bo, 0, "a", "b/FILE", "");
    case("traversal", "a/b/FILE", ao, 0, "a", "b/FILE", "");
    case("leaf", "a/b/FILE", lr, 0, "a/b/FILE", "FILE", "FILE");
    case("deepest", "a/b/c/FILE", ar, 0, "a/b/c", "FILE", "");
    case("intermediate", "a/b/c/FILE", ar, 1, "a/b", "c/FILE", "");
    case("root-most", "a/b/c/FILE", ar, 2, "a", "b/c/FILE", "");
    case("root", "a/b/FILE", rr, 0, "", "a/b/FILE", "");
    case("depth-one-bind", "a/FILE", fb, 0, "a", "FILE", "");
    case("deep-bind", "a/b/c/FILE", fb, 0, "a", "b/c/FILE", "");
}

#[test]
fn atomic_swap_before_binding_fails_but_post_binding_mutation_is_outside_the_read() {
    for (label, relative, active, alternate_file) in [
        ("atomic-depth-one", "a/FILE", "a", "FILE"),
        ("atomic-deep", "a/b/c/FILE", "a", "b/c/FILE"),
    ] {
        let fixture = Fixture::new(label);
        fixture.write_root(relative, ORIGINAL);
        fixture.write_alternate(alternate_file, FOREIGN);
        let workspace = Workspace::open(&fixture.root).unwrap();
        let left = fixture.root.join(active);
        let right = fixture.alternate.clone();
        let result = with_observer(
            move |event| {
                if event == ReadEvent::BeforeFinalPathBinding {
                    rename_swap(&left, &right);
                }
            },
            || workspace.read_file(&CanonicalPath::parse(relative).unwrap(), 1024),
        );
        assert_fail_closed(result);
        assert_eq!(fs::read(fixture.root.join(relative)).unwrap(), FOREIGN);
    }

    let fixture = Fixture::new("post-binding");
    fixture.write_root("a/FILE", ORIGINAL);
    fixture.write_alternate("FILE", FOREIGN);
    let workspace = Workspace::open(&fixture.root).unwrap();
    let left = fixture.root.join("a");
    let right = fixture.alternate.clone();
    let result = with_observer(
        move |event| {
            if event == ReadEvent::AfterFinalPathBinding {
                rename_swap(&left, &right);
            }
        },
        || workspace.read_file(&CanonicalPath::parse("a/FILE").unwrap(), 1024),
    );
    assert_eq!(result.unwrap(), Some(ORIGINAL.to_vec()));
    assert_eq!(fs::read(fixture.root.join("a/FILE")).unwrap(), FOREIGN);
}

#[test]
fn absence_creation_removal_and_same_path_recreation_fail_closed() {
    let absent = Fixture::new("absent-create");
    fs::create_dir(absent.root.join("a")).unwrap();
    let workspace = Workspace::open(&absent.root).unwrap();
    let created = absent.root.join("a/FILE");
    let result = with_observer(
        move |event| {
            if event == ReadEvent::BeforeFinalPathBinding {
                fs::write(&created, FOREIGN).unwrap();
            }
        },
        || workspace.read_file(&CanonicalPath::parse("a/FILE").unwrap(), 1024),
    );
    assert_fail_closed(result);

    for (label, mutate) in [("remove", false), ("recreate", true)] {
        let fixture = Fixture::new(label);
        fixture.write_root("FILE", ORIGINAL);
        let workspace = Workspace::open(&fixture.root).unwrap();
        let active = fixture.root.join("FILE");
        let result = with_observer(
            move |event| {
                if event == ReadEvent::BeforeFinalPathBinding {
                    fs::remove_file(&active).unwrap();
                    if mutate {
                        fs::write(&active, FOREIGN).unwrap();
                    }
                }
            },
            || workspace.read_file(&CanonicalPath::parse("FILE").unwrap(), 1024),
        );
        assert_fail_closed(result);
    }
}

#[test]
fn same_inode_same_size_mutation_with_restored_mtime_fails_closed() {
    let fixture = Fixture::new("same-metadata");
    fixture.write_root("FILE", ORIGINAL);
    let before = fs::metadata(fixture.root.join("FILE")).unwrap();
    let workspace = Workspace::open(&fixture.root).unwrap();
    let active = fixture.root.join("FILE");
    let result = with_observer(
        move |event| {
            if event == ReadEvent::BeforeFinalPathBinding {
                fs::write(&active, FOREIGN).unwrap();
                restore_times(&active, &before);
                let after = fs::metadata(&active).unwrap();
                assert_eq!((after.ino(), after.len()), (before.ino(), before.len()));
                assert_eq!(
                    (after.mtime(), after.mtime_nsec()),
                    (before.mtime(), before.mtime_nsec())
                );
            }
        },
        || workspace.read_file(&CanonicalPath::parse("FILE").unwrap(), 1024),
    );
    assert_fail_closed(result);
}

fn restore_times(path: &Path, metadata: &fs::Metadata) {
    let raw = CString::new(path.as_os_str().as_bytes()).unwrap();
    let times = [
        libc::timespec {
            tv_sec: metadata.atime(),
            tv_nsec: metadata.atime_nsec(),
        },
        libc::timespec {
            tv_sec: metadata.mtime(),
            tv_nsec: metadata.mtime_nsec(),
        },
    ];
    assert_eq!(
        unsafe { libc::utimensat(libc::AT_FDCWD, raw.as_ptr(), times.as_ptr(), 0) },
        0
    );
}

fn rename_swap(left: &Path, right: &Path) {
    let left = CString::new(left.as_os_str().as_bytes()).unwrap();
    let right = CString::new(right.as_os_str().as_bytes()).unwrap();
    let result = unsafe {
        libc::renameatx_np(
            libc::AT_FDCWD,
            left.as_ptr(),
            libc::AT_FDCWD,
            right.as_ptr(),
            libc::RENAME_SWAP,
        )
    };
    assert_eq!(
        result,
        0,
        "rename swap failed: {}",
        std::io::Error::last_os_error()
    );
}
