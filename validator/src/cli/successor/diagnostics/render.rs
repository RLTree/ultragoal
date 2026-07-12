use super::super::super::OutputMode;
use super::model::{RuntimeOutcome, RuntimeStreams};

const FALLBACK: &[u8] = b"{\"schema_version\":\"HarnessDiagnostic-v1\",\"diagnostic_id\":\"successor_runtime_projection_failed\",\"exit_class\":\"internal_failure\"}\n";

impl RuntimeOutcome {
    pub fn render(&self, mode: OutputMode) -> RuntimeStreams {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        match (
            &self.machine_payload,
            &self.human_payload,
            &self.diagnostic,
            mode,
        ) {
            (Some(machine), _, None, OutputMode::Json) => append_line(&mut stdout, machine),
            (_, Some(human), None, OutputMode::Human) => append_line(&mut stdout, human.as_bytes()),
            (_, _, Some(diagnostic), OutputMode::Json) => match serde_json::to_vec(diagnostic) {
                Ok(bytes) => append_line(&mut stderr, &bytes),
                Err(_) => stderr.extend_from_slice(FALLBACK),
            },
            (_, _, Some(diagnostic), OutputMode::Human) => {
                append_line(&mut stderr, diagnostic.human().as_bytes());
            }
            _ => stderr.extend_from_slice(FALLBACK),
        }
        RuntimeStreams {
            exit_code: self.exit_class.code(),
            stdout,
            stderr,
        }
    }
}

fn append_line(output: &mut Vec<u8>, bytes: &[u8]) {
    output.extend_from_slice(bytes);
    if !bytes.ends_with(b"\n") {
        output.push(b'\n');
    }
}
