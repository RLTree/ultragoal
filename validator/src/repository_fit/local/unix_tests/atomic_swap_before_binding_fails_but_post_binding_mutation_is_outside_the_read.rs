use super::*;
use std::os::unix::ffi::OsStrExt;

#[test]
pub(crate) fn atomic_swap_before_binding_fails_but_post_binding_mutation_is_outside_the_read() {
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
pub(crate) fn absence_creation_removal_and_same_path_recreation_fail_closed() {
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
pub(crate) fn same_inode_same_size_mutation_with_restored_mtime_fails_closed() {
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

pub(crate) fn restore_times(path: &Path, metadata: &fs::Metadata) {
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

pub(crate) fn rename_swap(left: &Path, right: &Path) {
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
