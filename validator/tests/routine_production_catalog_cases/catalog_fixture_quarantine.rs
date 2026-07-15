use super::*;
use crate::catalog_fixture_cleanup_hook::{set_before_entry_removal, set_before_final_removal};
use crate::catalog_fixture_scope::{ClaimedFixtureScope, FixtureScopeError};
use std::sync::{Arc, Mutex};

#[test]
pub(crate) fn nested_quarantine_revalidation_preserves_a_replacement() {
    let parent = catalog_fixture_parent();
    let name = format!(
        "nested-substitution-{}",
        NEXT.fetch_add(1, Ordering::Relaxed)
    );
    let mut scope = ClaimedFixtureScope::claim(&parent, &name).unwrap();
    fs::write(scope.path().join("owned"), b"owned bytes\n").unwrap();
    let replacement = Arc::new(Mutex::new(None::<CString>));
    let observed = Arc::clone(&replacement);
    set_before_entry_removal(Some(Box::new(move |directory, quarantine| {
        replace_quarantined_entry(directory, quarantine, &observed);
    })));
    assert!(matches!(
        scope.rollback(),
        Err(FixtureScopeError::Retained(_))
    ));
    set_before_entry_removal(None);
    let replacement = replacement.lock().unwrap().take().unwrap();
    assert_eq!(
        fs::read(scope.path().join(replacement.to_string_lossy().as_ref())).unwrap(),
        b"foreign replacement\n"
    );
    fs::remove_dir_all(scope.path()).unwrap();
}

#[test]
pub(crate) fn final_quarantine_substitution_is_retained_without_deleting_the_replacement() {
    let parent = catalog_fixture_parent();
    let name = format!(
        "final-substitution-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    );
    let mut scope = ClaimedFixtureScope::claim(&parent, &name).unwrap();
    fs::write(scope.path().join("owned"), b"owned bytes\n").unwrap();
    let retained = parent.join(format!("{name}-quarantine-held"));
    let replacement = Arc::new(Mutex::new(None::<PathBuf>));
    let observed = Arc::clone(&replacement);
    set_before_final_removal(Some(Box::new({
        let parent = parent.clone();
        let retained = retained.clone();
        move |quarantine| {
            let quarantine = parent.join(quarantine.to_string_lossy().as_ref());
            fs::rename(&quarantine, &retained).unwrap();
            fs::create_dir(&quarantine).unwrap();
            fs::write(quarantine.join("foreign"), b"preserve replacement\n").unwrap();
            *observed.lock().unwrap() = Some(quarantine);
        }
    })));
    assert!(matches!(
        scope.rollback(),
        Err(FixtureScopeError::Retained(_))
    ));
    set_before_final_removal(None);
    assert!(fs::read_dir(&retained).unwrap().next().is_none());
    let replacement = replacement.lock().unwrap().take().unwrap();
    assert_eq!(
        fs::read(replacement.join("foreign")).unwrap(),
        b"preserve replacement\n"
    );
    fs::remove_dir_all(retained).unwrap();
    fs::remove_dir_all(replacement).unwrap();
}

fn replace_quarantined_entry(
    directory: libc::c_int,
    quarantine: &std::ffi::CStr,
    observed: &Mutex<Option<CString>>,
) {
    let held = CString::new("owned-held").unwrap();
    assert_eq!(
        unsafe {
            libc::renameatx_np(
                directory,
                quarantine.as_ptr(),
                directory,
                held.as_ptr(),
                libc::RENAME_EXCL,
            )
        },
        0
    );
    let replacement = unsafe {
        libc::openat(
            directory,
            quarantine.as_ptr(),
            libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_CLOEXEC,
            0o600,
        )
    };
    assert!(replacement >= 0);
    assert_eq!(
        unsafe { libc::write(replacement, b"foreign replacement\n".as_ptr().cast(), 20) },
        20
    );
    assert_eq!(unsafe { libc::close(replacement) }, 0);
    *observed.lock().unwrap() = Some(CString::new(quarantine.to_bytes()).unwrap());
}

fn catalog_fixture_parent() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("target/routine-production-catalog-fixtures")
}
