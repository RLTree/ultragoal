pub(super) fn valid(value: &str) -> bool {
    crate::plugin_manifest::semver(value)
}

#[cfg(test)]
mod tests {
    #[test]
    fn strict_semver_examples() {
        for valid in ["0.0.11", "1.2.3-beta.1+codex.local-1"] {
            assert!(super::valid(valid));
        }
        for invalid in ["1", "01.2.3", "1.2.3-01", "1.2.3+"] {
            assert!(!super::valid(invalid));
        }
    }
}
