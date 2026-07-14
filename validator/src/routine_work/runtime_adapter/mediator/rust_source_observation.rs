use super::*;
use process::{ProcessObservation, ProcessTermination};

pub(crate) fn validate_rust_source_observation(
    frame: Option<&[u8]>,
    observation: &ProcessObservation,
) -> Result<(), RoutineError> {
    let exit_code = match observation.termination {
        ProcessTermination::Exited(code) => code,
        _ => return Err(mediator_error("mediator-behavior-exit-invalid")),
    };
    if !trusted_rust_source_execution_observed(
        frame,
        exit_code,
        &observation.stdout,
        observation.stderr_sha256 == sha256(&[]),
    ) {
        return Err(mediator_error("mediator-rust-source-observation-invalid"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::routine_work::{
        RustSourceFrameInput, RustSourceSyntaxOutcome, encode_rust_source_syntax_frame,
        evaluate_rust_source_syntax_frame, rust_source_syntax_observation_json,
    };

    fn frame() -> Vec<u8> {
        let bytes = b"pub fn value() -> u8 { 1 }\n";
        let digest = sha256(bytes);
        encode_rust_source_syntax_frame(&[RustSourceFrameInput::new(
            "src/lib.rs",
            &digest,
            bytes.len() as u64,
            bytes,
        )])
        .unwrap()
    }

    fn observation(frame: &[u8]) -> ProcessObservation {
        let RustSourceSyntaxOutcome::Passed(parsed) = evaluate_rust_source_syntax_frame(frame)
        else {
            panic!("canonical frame refused");
        };
        let mut stdout = rust_source_syntax_observation_json(&parsed);
        stdout.push(b'\n');
        ProcessObservation {
            termination: ProcessTermination::Exited(0),
            stdout,
            stderr_sha256: sha256(&[]),
            output_byte_length: 0,
            started: true,
        }
    }

    #[test]
    fn exact_framed_observation_is_required_for_success() {
        let frame = frame();
        assert!(validate_rust_source_observation(Some(&frame), &observation(&frame)).is_ok());
    }

    #[test]
    fn exit_zero_without_a_frame_is_refused() {
        let frame = frame();
        assert!(validate_rust_source_observation(None, &observation(&frame)).is_err());
    }

    #[test]
    fn exit_zero_with_wrong_stdout_is_refused() {
        let frame = frame();
        let mut observed = observation(&frame);
        observed.stdout = b"{}\n".to_vec();
        assert!(validate_rust_source_observation(Some(&frame), &observed).is_err());
    }

    #[test]
    fn exit_zero_with_nonempty_stderr_is_refused() {
        let frame = frame();
        let mut observed = observation(&frame);
        observed.stderr_sha256 = sha256(b"authored success");
        assert!(validate_rust_source_observation(Some(&frame), &observed).is_err());
    }
}
