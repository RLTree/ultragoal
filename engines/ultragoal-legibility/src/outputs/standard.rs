pub(super) fn known_pure(call: &str) -> bool {
    if matches!(
        call,
        "usize::from"
            | "u64::from"
            | "u32::from"
            | "i32::from"
            | "usize::try_from"
            | "u64::try_from"
            | "u32::try_from"
            | "i32::try_from"
    ) {
        return true;
    }
    let call = call.trim_start_matches("::");
    if matches!(call, "Ok" | "Err" | "Some") {
        return true;
    }
    if matches!(
        call,
        "serde_json::to_string"
            | "serde_json::to_string_pretty"
            | "serde_json::to_vec"
            | "serde_json::to_vec_pretty"
            | "serde_json::to_value"
    ) {
        return true;
    }
    // Exact allowlist: std also contains filesystem, environment and process authority.
    let tail = call
        .strip_prefix("std::")
        .or_else(|| call.strip_prefix("core::"))
        .or_else(|| call.strip_prefix("alloc::"));
    matches!(
        tail,
        Some(
            "string::String::new"
                | "string::String::from"
                | "string::String::from_utf8"
                | "string::String::from_utf8_lossy"
                | "string::String::with_capacity"
                | "vec::Vec::with_capacity"
                | "collections::BTreeMap::new"
                | "collections::BTreeMap::from"
                | "collections::BTreeSet::new"
                | "collections::BTreeSet::from"
                | "path::Path::new"
                | "path::PathBuf::new"
                | "path::PathBuf::from"
                | "path::Component::Normal"
                | "str::from_utf8"
                | "slice::from_ref"
                | "ffi::CString::new"
                | "time::Duration::from_secs"
                | "time::Duration::from_millis"
                | "time::Duration::from_micros"
                | "time::Duration::from_nanos"
                | "iter::once"
                | "iter::empty"
                | "mem::take"
                | "mem::replace"
                | "mem::drop"
                | "mem::MaybeUninit::zeroed"
                | "io::Error::new"
                | "io::Error::from"
                | "vec::Vec::new"
                | "boxed::Box::new"
                | "option::Option::Some"
                | "result::Result::Ok"
                | "result::Result::Err"
                | "cmp::min"
                | "cmp::max"
                | "mem::size_of"
                | "mem::align_of"
        )
    )
}

// Exact observations and local waiting; not deterministic purity or latency guarantees.
pub(super) fn local_observation_or_timing(call: &str) -> bool {
    matches!(
        call,
        "std::time::Instant::now"
            | "std::time::SystemTime::now"
            | "std::process::id"
            | "std::env::current_exe"
            | "std::thread::sleep"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn observation_and_waiting_do_not_authorize_environment_or_thread_effects() {
        assert!(local_observation_or_timing("std::env::current_exe"));
        assert!(local_observation_or_timing("std::thread::sleep"));
        assert!(!local_observation_or_timing("std::env::var"));
        assert!(!local_observation_or_timing("std::thread::spawn"));
        assert!(!known_pure("std::thread::sleep"));
    }
}
