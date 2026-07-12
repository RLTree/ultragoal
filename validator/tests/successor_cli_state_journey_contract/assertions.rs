use serde_json::Value;
use std::process::Output;

pub(super) fn assert_payload(output: &Output, exit: i32, schema: &str, private: &str) -> Value {
    assert_eq!(
        output.status.code(),
        Some(exit),
        "unexpected output: {output:?}"
    );
    assert!(
        output.stderr.is_empty(),
        "payload contaminated stderr: {output:?}"
    );
    machine_value(&output.stdout, schema, private)
}

pub(super) fn assert_diagnostic(output: &Output, exit: i32, schema: &str, private: &str) -> Value {
    assert_eq!(
        output.status.code(),
        Some(exit),
        "unexpected output: {output:?}"
    );
    assert!(
        output.stdout.is_empty(),
        "diagnostic contaminated stdout: {output:?}"
    );
    machine_value(&output.stderr, schema, private)
}

pub(super) fn machine_value(bytes: &[u8], schema: &str, private: &str) -> Value {
    assert!(!bytes.is_empty(), "machine stream is empty");
    assert!(!bytes.contains(&0x1b), "machine stream contains ANSI");
    let text = String::from_utf8_lossy(bytes);
    assert!(!text.contains(private), "private input echoed: {private}");
    let value: Value = serde_json::from_slice(bytes).expect("machine stream is one JSON value");
    assert_eq!(value["schema_version"], schema, "unexpected machine schema");
    value
}
