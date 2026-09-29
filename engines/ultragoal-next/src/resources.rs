//! Host headroom for bounded work units. Unknown telemetry permits only a
//! small listing window before an explicit incomplete result.
#[derive(Clone, Copy, Debug)]
pub struct Headroom {
    pub available: u64,
    pub physical: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HeadroomError { Unavailable, Malformed, #[cfg(not(any(target_os = "macos", target_os = "linux")))] Unsupported }

#[cfg(target_os = "macos")]
unsafe extern "C" {
    fn host_page_size(host: libc::host_t, page: *mut libc::vm_size_t) -> libc::kern_return_t;
    fn mach_port_deallocate(task: libc::mach_port_t, name: libc::mach_port_t) -> libc::kern_return_t;
}

#[cfg(target_os = "macos")]
#[allow(deprecated)]
fn measure() -> Result<Headroom, HeadroomError> {
    let host = unsafe { libc::mach_host_self() };
    if host == 0 { return Err(HeadroomError::Unavailable); }
    let mut stats: libc::vm_statistics64 = unsafe { std::mem::zeroed() };
    let mut count = libc::HOST_VM_INFO64_COUNT;
    let mut page: libc::vm_size_t = 0;
    // The kernel writes only the declared struct width. Release the acquired
    // host-port right before returning, including on a failed observation.
    let observed = unsafe {
        libc::host_statistics64(host, libc::HOST_VM_INFO64, (&mut stats as *mut libc::vm_statistics64).cast(), &mut count)
    };
    let sized = unsafe { host_page_size(host, &mut page) };
    let closed = unsafe { mach_port_deallocate(libc::mach_task_self(), host) };
    if observed != libc::KERN_SUCCESS || sized != libc::KERN_SUCCESS || closed != libc::KERN_SUCCESS {
        return Err(HeadroomError::Unavailable);
    }
    if count < libc::HOST_VM_INFO64_COUNT || page == 0 { return Err(HeadroomError::Malformed); }
    let page = page as u64;
    let free = stats.free_count as u64;
    let inactive = stats.inactive_count as u64;
    let speculative = stats.speculative_count as u64;
    let active = stats.active_count as u64;
    let wired = stats.wire_count as u64;
    let compressed = stats.compressor_page_count as u64;
    Ok(Headroom {
        available: free.saturating_add(inactive).saturating_add(speculative).saturating_mul(page),
        physical: free.saturating_add(inactive).saturating_add(speculative)
            .saturating_add(active).saturating_add(wired).saturating_add(compressed).saturating_mul(page),
    })
}

#[cfg(target_os = "linux")]
fn measure() -> Result<Headroom, HeadroomError> {
    let text = std::fs::read_to_string("/proc/meminfo").map_err(|_| HeadroomError::Unavailable)?;
    let kib = |name: &str| -> Option<u64> {
        text.lines().find_map(|line| line.strip_prefix(name))?
            .split_whitespace().next()?.parse::<u64>().ok()
    };
    Ok(Headroom { available: kib("MemAvailable:").ok_or(HeadroomError::Malformed)?.saturating_mul(1024), physical: kib("MemTotal:").ok_or(HeadroomError::Malformed)?.saturating_mul(1024) })
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn measure() -> Result<Headroom, HeadroomError> { Err(HeadroomError::Unsupported) }

pub fn current() -> Option<Headroom> { measure().ok() }

impl Headroom {
    /// Reserve an eighth of physical memory for the host. Bend can amplify a
    /// request by roughly 80x on real source, so use only a fraction of the rest.
    pub fn work_bytes(self) -> u64 {
        self.available.saturating_sub(self.physical / 8)
    }

    pub fn frame_bytes(self, provider_max: usize) -> usize {
        if self.work_bytes() < 160 * 1024 { return 0; }
        usize::try_from(self.work_bytes() / 160).unwrap_or(usize::MAX)
            .clamp(1024, provider_max)
    }

    pub fn listing_can_grow(self, entries: usize) -> bool {
        (entries as u64).saturating_mul(512) < self.work_bytes() / 4
    }

    pub fn index_lanes(self) -> usize {
        if self.work_bytes() >= 4 * 1024 * 1024 * 1024 { 2 } else { 1 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pressure_reduces_work_and_recovery_expands_it() {
        let low = Headroom { physical: 16 << 30, available: 3 << 30 };
        let high = Headroom { physical: 16 << 30, available: 12 << 30 };
        assert!(low.frame_bytes(16 << 20) < high.frame_bytes(16 << 20));
        assert_eq!(Headroom { physical: 16 << 30, available: 1 << 30 }.frame_bytes(16 << 20), 0);
        let recovering = Headroom { physical: 16 << 30, available: (16 << 30) / 8 + (1 << 20) };
        assert!((1024..64 * 1024).contains(&recovering.frame_bytes(16 << 20)));
        assert!(!low.listing_can_grow(1_000_000));
        assert!(high.listing_can_grow(1_000_000));
    }
}
