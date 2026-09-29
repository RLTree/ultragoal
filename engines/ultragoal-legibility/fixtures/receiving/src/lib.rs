/// A tiny receiving project that has no privileged operations.
pub fn add(left: u32, right: u32) -> u32 {
    left.saturating_add(right)
}
