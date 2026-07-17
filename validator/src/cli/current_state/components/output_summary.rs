use super::*;

pub(crate) fn print_summary(state: &Value, receipt: Option<&Path>) {
    let blocker = state.get("first_blocker").unwrap_or(&Value::Null);
    println!(
        "ultragoal-current-state {} candidate={} receipt={} first_blocker={} why={} next_repair={} narrow_rerun='{}' claim_ceiling='{}'",
        text(state, "status", "fail"),
        text(state, "candidate_digest", "<missing>"),
        receipt
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "none-read-only".to_string()),
        text(blocker, "id", "unknown"),
        text(blocker, "why_failed", "unknown"),
        text(state, "next_repair", "unknown"),
        text(state, "narrow_rerun", "unknown"),
        text(state, "claim_ceiling", "unknown")
    );
}

pub(crate) fn text<'a>(value: &'a Value, field: &str, default: &'a str) -> &'a str {
    value.get(field).and_then(Value::as_str).unwrap_or(default)
}

#[cfg(test)]
pub(crate) fn opt_path(args: &[String], key: &str) -> Option<PathBuf> {
    args.iter()
        .position(|arg| arg == key)
        .and_then(|index| args.get(index + 1))
        .map(PathBuf::from)
}
