pub(super) fn rename_noreplace(
    old_fd: libc::c_int,
    old: *const libc::c_char,
    new_fd: libc::c_int,
    new: *const libc::c_char,
) -> libc::c_int {
    rename_noreplace_platform(old_fd, old, new_fd, new)
}

pub(super) fn rename_swap(
    left_fd: libc::c_int,
    left: *const libc::c_char,
    right_fd: libc::c_int,
    right: *const libc::c_char,
) -> libc::c_int {
    rename_swap_platform(left_fd, left, right_fd, right)
}

#[cfg(target_os = "macos")]
fn rename_noreplace_platform(
    old_fd: libc::c_int,
    old: *const libc::c_char,
    new_fd: libc::c_int,
    new: *const libc::c_char,
) -> libc::c_int {
    const NOFOLLOW_AND_BENEATH: libc::c_uint = 0x10 | 0x20;
    unsafe {
        libc::renameatx_np(
            old_fd,
            old,
            new_fd,
            new,
            libc::RENAME_EXCL | NOFOLLOW_AND_BENEATH,
        )
    }
}

#[cfg(target_os = "linux")]
fn rename_noreplace_platform(
    old_fd: libc::c_int,
    old: *const libc::c_char,
    new_fd: libc::c_int,
    new: *const libc::c_char,
) -> libc::c_int {
    unsafe { libc::renameat2(old_fd, old, new_fd, new, libc::RENAME_NOREPLACE) }
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn rename_noreplace_platform(
    _old_fd: libc::c_int,
    _old: *const libc::c_char,
    _new_fd: libc::c_int,
    _new: *const libc::c_char,
) -> libc::c_int {
    -1
}

#[cfg(target_os = "macos")]
fn rename_swap_platform(
    left_fd: libc::c_int,
    left: *const libc::c_char,
    right_fd: libc::c_int,
    right: *const libc::c_char,
) -> libc::c_int {
    const NOFOLLOW_AND_BENEATH: libc::c_uint = 0x10 | 0x20;
    unsafe {
        libc::renameatx_np(
            left_fd,
            left,
            right_fd,
            right,
            libc::RENAME_SWAP | NOFOLLOW_AND_BENEATH,
        )
    }
}

#[cfg(target_os = "linux")]
fn rename_swap_platform(
    left_fd: libc::c_int,
    left: *const libc::c_char,
    right_fd: libc::c_int,
    right: *const libc::c_char,
) -> libc::c_int {
    unsafe { libc::renameat2(left_fd, left, right_fd, right, libc::RENAME_EXCHANGE) }
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn rename_swap_platform(
    _left_fd: libc::c_int,
    _left: *const libc::c_char,
    _right_fd: libc::c_int,
    _right: *const libc::c_char,
) -> libc::c_int {
    -1
}
