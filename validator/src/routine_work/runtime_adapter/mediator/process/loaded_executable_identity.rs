use super::*;
use std::ffi::CStr;
use std::os::unix::ffi::OsStrExt;

const PROC_PIDREGIONPATHINFO: i32 = 8;
const MAX_REGIONS: usize = 4096;

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct ProcRegionInfo {
    protection: u32,
    max_protection: u32,
    inheritance: u32,
    flags: u32,
    offset: u64,
    behavior: u32,
    user_wired_count: u32,
    user_tag: u32,
    pages_resident: u32,
    pages_shared_now_private: u32,
    pages_swapped_out: u32,
    pages_dirtied: u32,
    reference_count: u32,
    shadow_depth: u32,
    share_mode: u32,
    private_pages_resident: u32,
    shared_pages_resident: u32,
    object_id: u32,
    depth: u32,
    address: u64,
    size: u64,
}

#[repr(C)]
struct RegionWithPath {
    region: ProcRegionInfo,
    vnode: libc::vnode_info_path,
}

pub(crate) fn validate_loaded_executable(
    child: &BoundChild,
    program: &PinnedExecutable,
) -> Result<(), RoutineError> {
    let mut process_path = [0_i8; libc::PROC_PIDPATHINFO_MAXSIZE as usize];
    // SAFETY: child.pid is a spawned child and process_path is a writable buffer of its declared size.
    let length = unsafe {
        libc::proc_pidpath(
            child.pid(),
            process_path.as_mut_ptr().cast(),
            process_path.len() as u32,
        )
    };
    if length <= 0 {
        return Err(mediator_error("mediator-loaded-executable-unavailable"));
    }
    // SAFETY: successful proc_pidpath writes a NUL-terminated path into process_path.
    let process_path = unsafe { CStr::from_ptr(process_path.as_ptr()) }.to_bytes();
    if process_path != program.path().as_os_str().as_bytes() {
        return Err(mediator_error("mediator-loaded-executable-path-mismatch"));
    }

    let mut address = 0_u64;
    for _ in 0..MAX_REGIONS {
        // SAFETY: RegionWithPath is a repr(C) kernel output buffer whose all-zero state is valid.
        let mut observation: RegionWithPath = unsafe { std::mem::zeroed() };
        let expected = std::mem::size_of::<RegionWithPath>() as i32;
        // SAFETY: observation is writable for expected bytes and child.pid identifies the child.
        let observed = unsafe {
            libc::proc_pidinfo(
                child.pid(),
                PROC_PIDREGIONPATHINFO,
                address,
                (&mut observation as *mut RegionWithPath).cast(),
                expected,
            )
        };
        if observed == 0 {
            break;
        }
        if observed != expected {
            return Err(mediator_error(
                "mediator-loaded-executable-observation-invalid",
            ));
        }
        let next = observation
            .region
            .address
            .checked_add(observation.region.size)
            .filter(|next| *next > address)
            .ok_or_else(|| mediator_error("mediator-loaded-executable-observation-invalid"))?;
        address = next;
        if observation.region.protection & libc::VM_PROT_EXECUTE as u32 == 0 {
            continue;
        }
        // SAFETY: a complete PROC_PIDREGIONPATHINFO observation contains a NUL-terminated vnode path.
        let path =
            unsafe { CStr::from_ptr(observation.vnode.vip_path.as_ptr().cast::<i8>()) }.to_bytes();
        if path != process_path {
            continue;
        }
        let identity = observation.vnode.vip_vi.vi_stat;
        return program.validate_loaded_vnode(identity.vst_dev as u64, identity.vst_ino);
    }
    Err(mediator_error("mediator-loaded-executable-unobserved"))
}
