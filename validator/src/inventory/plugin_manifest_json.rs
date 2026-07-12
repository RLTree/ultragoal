pub(super) fn unique_keys(bytes: &[u8]) -> bool {
    crate::plugin_manifest::parse_value(bytes, bytes.len()).is_ok()
}

#[cfg(test)]
mod tests {
    #[test]
    fn duplicate_keys_fail_at_any_depth() {
        assert!(super::unique_keys(br#"{"a":[{"b":1}]}"#));
        assert!(!super::unique_keys(br#"{"a":{"b":1,"b":2}}"#));
    }
}
