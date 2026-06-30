use super::*;

#[test]
fn live_curl_error_mappers_record_config_write_and_wait_failures() {
    struct FailingWriter;

    impl std::io::Write for FailingWriter {
        fn write(&mut self, _buf: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::other("closed stdin"))
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    let mut config_failures = Vec::new();
    let mut writer = FailingWriter;
    write_curl_config(
        &mut writer,
        "url = \"https://api.openai.com/v1/responses\"",
        &mut config_failures,
    );
    assert!(config_failures.contains(&"openai_live_curl_config_write_failed".to_string()));

    let mut wait_failures = Vec::new();
    assert!(
        wait_with_output(
            Err(std::io::Error::other("wait failed")),
            &mut wait_failures,
        )
        .is_none()
    );
    assert!(wait_failures.contains(&"openai_live_curl_wait_failed".to_string()));
}
