#[cfg(not(target_os = "macos"))]
use super::*;

#[cfg(any(target_os = "linux", target_os = "android"))]
pub(crate) const fn no_follow_nonblock_flags() -> i32 {
    0x20000 | 0x0800
}

#[cfg(all(
    unix,
    not(any(target_os = "macos", target_os = "linux", target_os = "android"))
))]
compile_error!(
    "repository-fit template source staging requires audited O_NOFOLLOW and O_NONBLOCK flags"
);
