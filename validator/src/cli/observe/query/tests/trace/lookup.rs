#[test]
fn trace_backend_id_uses_exporter_trace_id_projection() {
    let backend_id = super::super::super::transport::trace_backend_id_for_test("trace-query-bound");
    let expected = crate::digest::bytes("trace-query-bound".as_bytes())
        .trim_start_matches("sha256:")
        .chars()
        .take(32)
        .collect::<String>();

    assert_eq!(backend_id, expected);
    assert_eq!(backend_id.len(), 32);
}

#[test]
fn trace_id_lookup_404_is_reported_as_bounded_readiness_lag() {
    let mapped = super::super::super::transport::trace_lookup_error_for_test(
        "trace-target",
        "curl query failed with exit code 22: 404".to_string(),
    );
    let unchanged = super::super::super::transport::trace_lookup_error_for_test(
        "trace-target",
        "connection refused".to_string(),
    );

    assert_eq!(
        mapped,
        "victoriatraces trace lookup returned 404 before the span tree was queryable: trace_id=trace-target"
    );
    assert_eq!(unchanged, "connection refused");
}
