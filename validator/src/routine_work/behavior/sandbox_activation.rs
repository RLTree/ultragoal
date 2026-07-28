use std::ffi::{CStr, CString, c_char};
use std::io::Read;

const MAGIC: &[u8] = b"HUL-RoutineSandbox-v1\0";
const MAX_PROFILE_BYTES: usize = 1024 * 1024;

pub(crate) fn frame_sandboxed_input(
    profile: &str,
    framed_input: Vec<u8>,
) -> Result<Vec<u8>, &'static str> {
    if profile.is_empty() || profile.len() > MAX_PROFILE_BYTES || profile.as_bytes().contains(&0) {
        return Err("mediator-sandbox-profile-invalid");
    }
    let length = u32::try_from(profile.len()).map_err(|_| "mediator-sandbox-profile-invalid")?;
    let mut input = Vec::with_capacity(MAGIC.len() + 4 + profile.len() + framed_input.len());
    input.extend_from_slice(MAGIC);
    input.extend_from_slice(&length.to_be_bytes());
    input.extend_from_slice(profile.as_bytes());
    input.extend_from_slice(&framed_input);
    Ok(input)
}

pub(crate) fn activate_and_read_frame(
    mut input: impl Read,
    max_frame_bytes: u64,
) -> Result<Vec<u8>, ()> {
    let mut magic = vec![0_u8; MAGIC.len()];
    input.read_exact(&mut magic).map_err(|_| ())?;
    if magic != MAGIC {
        return Err(());
    }
    let mut encoded_length = [0_u8; 4];
    input.read_exact(&mut encoded_length).map_err(|_| ())?;
    let profile_length = u32::from_be_bytes(encoded_length) as usize;
    if profile_length == 0 || profile_length > MAX_PROFILE_BYTES {
        return Err(());
    }
    let mut profile = vec![0_u8; profile_length];
    input.read_exact(&mut profile).map_err(|_| ())?;
    let profile = CString::new(profile).map_err(|_| ())?;
    activate(&profile)?;

    let mut frame = Vec::new();
    input
        .take(max_frame_bytes + 1)
        .read_to_end(&mut frame)
        .map_err(|_| ())?;
    (frame.len() as u64 <= max_frame_bytes)
        .then_some(frame)
        .ok_or(())
}

#[cfg(target_os = "macos")]
fn activate(profile: &CStr) -> Result<(), ()> {
    let mut error = std::ptr::null_mut();
    // SAFETY: `profile` is a NUL-terminated `CString`, and `error` is a valid
    // writable out-pointer for the sandbox API call.
    let result = unsafe { sandbox_init(profile.as_ptr(), 0, &mut error) };
    if !error.is_null() {
        // SAFETY: a non-null error pointer is allocated by `sandbox_init` and
        // must be released with the matching sandbox API function.
        unsafe { sandbox_free_error(error) };
    }
    (result == 0).then_some(()).ok_or(())
}

#[cfg(not(target_os = "macos"))]
fn activate(_profile: &CStr) -> Result<(), ()> {
    Err(())
}

#[cfg(target_os = "macos")]
#[link(name = "sandbox")]
unsafe extern "C" {
    fn sandbox_init(profile: *const c_char, flags: u64, error: *mut *mut c_char) -> i32;
    fn sandbox_free_error(error: *mut c_char);
}
