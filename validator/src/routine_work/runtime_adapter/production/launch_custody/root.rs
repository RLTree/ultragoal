use crate::routine_work::digest::sha256;

pub(super) fn safe_token_name(value: &str) -> String {
    sha256(value.as_bytes())
        .trim_start_matches("sha256:")
        .to_owned()
}
