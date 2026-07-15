use super::MutantCrate;
use serde_json::Value;
use std::process::Output;

pub(super) struct CaseSpec {
    pub(super) marker: &'static str,
    pub(super) file: &'static str,
    pub(super) code: &'static str,
    pub(super) message: &'static str,
    pub(super) additional_message: Option<&'static str>,
}

pub(super) fn assert_rejected(
    fixture: &MutantCrate,
    opened: &MutantCrate,
    output: &Output,
    cases: &[CaseSpec],
) {
    assert!(
        !output.status.success(),
        "negative compile layer unexpectedly passed"
    );
    let diagnostics = diagnostics(output);
    for case in cases {
        let (line, source) = fixture.unique_marker(case.file, case.marker);
        let (_, opened_source) = opened.unique_marker(case.file, case.marker);
        assert_eq!(source, opened_source, "{} probe bytes changed", case.marker);
        let at_probe: Vec<_> = diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.at(case.file, line))
            .collect();
        assert!(
            !at_probe.is_empty(),
            "{} has no primary diagnostic: {:?}",
            case.marker,
            diagnostics
        );
        let expected_messages: Vec<_> = [Some(case.message), case.additional_message]
            .into_iter()
            .flatten()
            .collect();
        for message in &expected_messages {
            assert!(
                at_probe.iter().any(|diagnostic| {
                    diagnostic.code.as_deref() == Some(case.code)
                        && diagnostic.message.contains(message)
                }),
                "{} did not fail with {} / {:?}: {:?}",
                case.marker,
                case.code,
                message,
                at_probe
            );
        }
        assert!(
            at_probe.iter().all(|diagnostic| {
                diagnostic.code.as_deref() == Some(case.code)
                    && expected_messages
                        .iter()
                        .any(|message| diagnostic.message.contains(message))
            }),
            "{} was masked by another error: {:?}",
            case.marker,
            at_probe
        );
    }
}

#[derive(Debug)]
struct Diagnostic {
    code: Option<String>,
    message: String,
    spans: Vec<Span>,
}

impl Diagnostic {
    fn at(&self, expected_file: &str, expected_line: usize) -> bool {
        self.spans.iter().any(|span| {
            span.primary && span.file.ends_with(expected_file) && span.line == expected_line
        })
    }
}

#[derive(Debug)]
struct Span {
    file: String,
    line: usize,
    primary: bool,
}

fn diagnostics(output: &Output) -> Vec<Diagnostic> {
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .filter(|value| value["reason"] == "compiler-message")
        .filter_map(|value| {
            let message = &value["message"];
            if message["level"] != "error" {
                return None;
            }
            Some(Diagnostic {
                code: message["code"]["code"].as_str().map(str::to_owned),
                message: message["message"].as_str()?.to_owned(),
                spans: message["spans"]
                    .as_array()?
                    .iter()
                    .filter_map(|span| {
                        Some(Span {
                            file: span["file_name"].as_str()?.to_owned(),
                            line: span["line_start"].as_u64()? as usize,
                            primary: span["is_primary"].as_bool()?,
                        })
                    })
                    .collect(),
            })
        })
        .collect()
}
