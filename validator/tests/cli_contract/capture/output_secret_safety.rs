use super::{CapturedOutput, OutputBudget, observe};
use sha2::{Digest, Sha256};
use std::io::Cursor;

fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

fn capture(
    input: &[u8],
    retention_limit: usize,
    observation_limit: usize,
    secrets: &[Vec<u8>],
) -> CapturedOutput {
    let budget = OutputBudget::for_bound_secrets(observation_limit, secrets);
    let output = observe(Cursor::new(input), retention_limit, &budget).unwrap();
    let empty = observe(Cursor::new([]), retention_limit, &budget).unwrap();
    budget.finalize_streams(output, empty).first
}

fn assert_absent(output: &CapturedOutput, needles: &[&[u8]]) {
    let json = serde_json::to_vec(output).unwrap();
    let debug = format!("{output:?}");
    let text = String::from_utf8_lossy(&json);
    for needle in needles.iter().copied().filter(|needle| !needle.is_empty()) {
        assert!(!json.windows(needle.len()).any(|seen| seen == needle));
        let raw_digest = digest(needle);
        let bare_digest = raw_digest.strip_prefix("sha256:").unwrap();
        assert!(!text.contains(&raw_digest) && !text.contains(bare_digest));
        assert!(!debug.contains(&raw_digest) && !debug.contains(bare_digest));
        assert_ne!(output.captured_sha256, raw_digest);
        let numeric = format!(
            "[{}]",
            needle
                .iter()
                .map(u8::to_string)
                .collect::<Vec<_>>()
                .join(",")
        );
        assert!(!text.contains(&numeric) && !debug.contains(&numeric));
    }
}

fn assert_withheld(output: &CapturedOutput, needles: &[&[u8]]) {
    assert!(output.retained().is_empty());
    assert_eq!(output.captured_byte_length, 0);
    assert_eq!(output.captured_sha256, digest(&[]));
    assert!(output.truncated);
    assert!(!output.observation_limit_exceeded);
    assert!(!output.captured_byte_length_is_lower_bound);
    let value = serde_json::to_value(output).unwrap();
    assert_eq!(
        value["content_disposition"],
        "withheld-secret-bearing-invocation"
    );
    assert_eq!(value["encoding"], "withheld-secret-bearing-invocation-v1");
    assert_absent(output, needles);
}

#[test]
fn secret_bearing_invocation_withholds_every_output_transformation() {
    let secret = b"123456".to_vec();
    let representations = [
        b"123456".as_slice(),
        b"1e240",
        b"000123456",
        b"ABCDEF",
        b"abcdef",
        b"\\x31\\x32\\x33\\x34\\x35\\x36",
        b"prefix-1e240-suffix",
        b"MTIzNDU2",
        &[0xff, 0xfe, 0x01, 0x02],
    ];
    for transformed in representations {
        let public = capture(transformed, 8192, 8192, &[]);
        assert_eq!(public.retained(), transformed);
        assert_eq!(public.captured_sha256, digest(transformed));
        assert!(!public.truncated && !public.observation_limit_exceeded);

        let output = capture(transformed, 8192, 8192, std::slice::from_ref(&secret));
        assert!(!output.observation_limit_exceeded);
        assert_withheld(&output, &[&secret, transformed]);
    }
}

#[test]
fn true_eof_and_non_utf8_secret_prefixes_are_fully_withheld() {
    let utf8 = b"zzq7V5-true-eof-output-secret".to_vec();
    for cut in 1..utf8.len() {
        let input = [b"public-leading-".as_slice(), &utf8[..cut]].concat();
        let output = capture(&input, 8192, 8192, std::slice::from_ref(&utf8));
        assert_withheld(&output, &[&utf8, &utf8[..cut], &input]);
    }
    let non_utf8 = vec![0xff, 0xfe, b'z', b'q', b'7', b'V', b'5'];
    let output = capture(
        &[0xff, 0xfe, b'z'],
        8192,
        8192,
        std::slice::from_ref(&non_utf8),
    );
    assert_withheld(&output, &[&non_utf8, &[0xff, 0xfe, b'z']]);
}

#[test]
fn empty_secret_is_not_a_secret_channel_and_public_output_remains_exact() {
    let public = b"ordinary-public-output";
    let output = capture(public, 8192, 8192, &[Vec::new()]);
    assert_eq!(output.retained(), public);
    assert_eq!(output.captured_sha256, digest(public));
    assert!(!output.truncated && !output.observation_limit_exceeded);
    let value = serde_json::to_value(&output).unwrap();
    assert_eq!(value["content_disposition"], "public");
}

fn pair(
    first: &[u8],
    second: &[u8],
    secrets: &[Vec<u8>],
    reverse: bool,
    observation_limit: usize,
) -> (CapturedOutput, CapturedOutput, bool) {
    let budget = OutputBudget::for_bound_secrets(observation_limit, secrets);
    let first_pending = observe(Cursor::new(first), 4, &budget).unwrap();
    let second_pending = observe(Cursor::new(second), 4, &budget).unwrap();
    let outputs = if reverse {
        let outputs = budget.finalize_streams(second_pending, first_pending);
        (outputs.second, outputs.first, outputs.output_limit_exceeded)
    } else {
        let outputs = budget.finalize_streams(first_pending, second_pending);
        (outputs.first, outputs.second, outputs.output_limit_exceeded)
    };
    outputs
}

#[test]
fn two_streams_overlaps_truncation_and_finalization_order_remain_withheld() {
    let short = b"zzq7".to_vec();
    let long = b"zzq7V5-overlapping-output-secret".to_vec();
    let first = b"1e240";
    let second = b"PREFIX-ZZQ7V5-SUFFIX";
    let ordinary = pair(first, second, &[short.clone(), long.clone()], false, 4096);
    let reversed = pair(first, second, &[long.clone(), short.clone()], true, 4096);
    assert_eq!(
        serde_json::to_vec(&ordinary.0).unwrap(),
        serde_json::to_vec(&reversed.0).unwrap()
    );
    assert_eq!(
        serde_json::to_vec(&ordinary.1).unwrap(),
        serde_json::to_vec(&reversed.1).unwrap()
    );
    for output in [&ordinary.0, &ordinary.1, &reversed.0, &reversed.1] {
        assert_withheld(output, &[&short, &long, first, second]);
    }

    let overflow = pair(first, second, std::slice::from_ref(&long), false, 3);
    assert!(overflow.2);
    for output in [&overflow.0, &overflow.1] {
        assert!(!output.observation_limit_exceeded);
        assert_withheld(output, &[&long, first, second]);
    }
}
