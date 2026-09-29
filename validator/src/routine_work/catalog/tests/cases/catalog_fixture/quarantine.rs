use super::cleanup_hook::{set_before_entry_removal, set_before_final_removal};
use super::rebind::{owner_bound_scans, set_rebind_refusals};
use super::scope::{ClaimedFixtureScope, FixtureScopeError};
use super::*;
use std::ffi::CString;
use std::sync::{Arc, Mutex};

#[test]
pub(crate) fn nested_quarantine_revalidation_preserves_a_replacement() {
    let guard = super::lock_fixture_root();
    let parent = fixture_parent();
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
    drop(guard);
}

#[test]
pub(crate) fn final_quarantine_substitution_is_retained_without_deleting_the_replacement() {
    let guard = super::lock_fixture_root();
    let parent = fixture_parent();
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
    drop(guard);
}

#[test]
pub(crate) fn detached_scope_rebinds_only_the_held_directory_after_repeated_refusals() {
    let guard = super::lock_fixture_root();
    let parent = fixture_parent();
    let before = inventory(&parent);
    let name = format!(
        "detached-rebind-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    );
    let mut scope = ClaimedFixtureScope::claim(&parent, &name).unwrap();
    let observed = Arc::new(Mutex::new(None::<PathBuf>));
    let replacement = Arc::clone(&observed);
    let held_name = name.clone();
    set_before_final_removal(Some(Box::new({
        let parent = parent.clone();
        move |quarantine| {
            let quarantine = parent.join(quarantine.to_string_lossy().as_ref());
            fs::rename(&quarantine, parent.join(format!("{held_name}-held"))).unwrap();
            fs::create_dir(&quarantine).unwrap();
            fs::write(
                quarantine.join("foreign"),
                b"preserve detached replacement\n",
            )
            .unwrap();
            *replacement.lock().unwrap() = Some(quarantine);
        }
    })));
    assert!(matches!(
        scope.rollback(),
        Err(FixtureScopeError::Retained(_))
    ));
    assert!(!scope.has_name_binding());
    set_rebind_refusals(2);
    let first_refusal = scope.rollback().unwrap_err();
    assert!(matches!(
        first_refusal,
        FixtureScopeError::Retained(ref reason) if reason.contains("after owner-bound scan")
    ));
    assert!(parent.join(format!("{name}-held")).is_dir());
    let second_refusal = scope.rollback().unwrap_err();
    assert!(
        matches!(
            second_refusal,
            FixtureScopeError::Retained(ref reason) if reason.contains("after owner-bound scan")
        ),
        "{second_refusal:?}"
    );
    assert!(parent.join(format!("{name}-held")).is_dir());
    assert_eq!(owner_bound_scans(), 2);
    scope.rollback().unwrap();
    let replacement = observed.lock().unwrap().take().unwrap();
    assert_eq!(
        fs::read(replacement.join("foreign")).unwrap(),
        b"preserve detached replacement\n"
    );
    fs::remove_dir_all(replacement).unwrap();
    assert_eq!(inventory(&parent), before);
    drop(guard);
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

fn fixture_parent() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("target/routine-production-catalog-fixtures")
}

fn inventory(parent: &Path) -> Vec<PathBuf> {
    let mut entries = fs::read_dir(parent)
        .map(|rows| {
            rows.filter_map(Result::ok)
                .map(|entry| entry.path())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    entries.sort();
    entries
}
