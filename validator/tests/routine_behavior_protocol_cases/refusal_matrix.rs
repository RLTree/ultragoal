use super::{PREFIX, SCHEMA, raw_frame, refusal_kind};
use ultragoal::routine_work::{RustSourceSyntaxErrorKind, evaluate_rust_source_syntax_frame};

#[test]
fn parser_rejects_legacy_unknown_trailing_duplicate_and_noncanonical_frames() {
    assert_refused(
        br#"{"schema_version":"RoutineCommandReport-v1","status":"passed"}"#,
        RustSourceSyntaxErrorKind::UnsupportedProtocol,
    );

    let mut unknown = raw_frame(&[(b"src/a.rs", b"fn a() {}\n", None)]);
    let schema_start = PREFIX.len() + 2;
    unknown[schema_start + SCHEMA.len() - 1] = b'2';
    assert_refused(&unknown, RustSourceSyntaxErrorKind::UnsupportedProtocol);

    let mut trailing = raw_frame(&[(b"src/a.rs", b"fn a() {}\n", None)]);
    trailing.push(0);
    assert_refused(&trailing, RustSourceSyntaxErrorKind::TrailingBytes);

    let duplicate = raw_frame(&[
        (b"src/a.rs", b"fn a() {}\n", None),
        (b"src/a.rs", b"fn b() {}\n", None),
    ]);
    assert_refused(&duplicate, RustSourceSyntaxErrorKind::DuplicatePath);

    let unordered = raw_frame(&[
        (b"src/b.rs", b"fn b() {}\n", None),
        (b"src/a.rs", b"fn a() {}\n", None),
    ]);
    assert_refused(&unordered, RustSourceSyntaxErrorKind::NonCanonicalOrder);
}

#[test]
fn parser_rejects_encoding_digest_length_syntax_and_size_substitution() {
    let invalid_path_utf8 = raw_frame(&[(b"src/\xff.rs", b"fn a() {}\n", None)]);
    assert_refused(
        &invalid_path_utf8,
        RustSourceSyntaxErrorKind::InvalidPathUtf8,
    );

    let invalid_source_utf8 = raw_frame(&[(b"src/a.rs", b"fn a() { \xff }\n", None)]);
    assert_refused(
        &invalid_source_utf8,
        RustSourceSyntaxErrorKind::InvalidSourceUtf8,
    );

    let mut digest_swap = raw_frame(&[(b"src/a.rs", b"fn a() {}\n", None)]);
    let source_start = PREFIX.len() + 2 + SCHEMA.len() + 4 + 2 + b"src/a.rs".len() + 32 + 8;
    digest_swap[source_start] ^= 1;
    assert_refused(&digest_swap, RustSourceSyntaxErrorKind::DigestMismatch);

    let length_swap = raw_frame(&[(b"src/a.rs", b"fn a() {}\n", Some(1))]);
    assert_refused(&length_swap, RustSourceSyntaxErrorKind::DigestMismatch);

    let invalid_syntax = raw_frame(&[(b"src/a.rs", b"fn a( {\n", None)]);
    assert_refused(
        &invalid_syntax,
        RustSourceSyntaxErrorKind::InvalidRustSyntax,
    );

    let oversized = vec![0; 16 * 1024 * 1024 + 1];
    assert_refused(&oversized, RustSourceSyntaxErrorKind::FrameTooLarge);
}

fn assert_refused(frame: &[u8], expected: RustSourceSyntaxErrorKind) {
    assert_eq!(
        refusal_kind(evaluate_rust_source_syntax_frame(frame)),
        expected
    );
}
