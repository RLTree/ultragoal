#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
pub(super) enum RawAuthorityMarker {
    RawJson,
    RawMap,
    RawObservation,
    RawPath,
    RawString,
}

impl RawAuthorityMarker {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::RawJson => "raw_json",
            Self::RawMap => "raw_map",
            Self::RawObservation => "raw_observation",
            Self::RawPath => "raw_path",
            Self::RawString => "raw_string",
        }
    }
}

pub(super) fn raw_authority_markers(text: &str) -> Vec<RawAuthorityMarker> {
    let mut out = Vec::new();
    if text.contains("serde_json::Map") || contains_identifier(text, "raw_map") {
        out.push(RawAuthorityMarker::RawMap);
    }
    if contains_raw_path_authority(text) {
        out.push(RawAuthorityMarker::RawPath);
    }
    if contains_identifier(text, "raw_string") {
        out.push(RawAuthorityMarker::RawString);
    }
    if super::contains_json_value_binding(text)
        || text.contains(": &Value")
        || text.contains(": &[Value]")
    {
        out.push(RawAuthorityMarker::RawJson);
    }
    if text.contains("\"raw_") || contains_identifier(text, "raw_observation") {
        out.push(RawAuthorityMarker::RawObservation);
    }
    out.sort_unstable();
    out.dedup();
    out
}

fn contains_raw_path_authority(text: &str) -> bool {
    let compact = text.replace(['\n', '\t'], " ");
    contains_identifier(&compact, "raw_path")
        || compact.contains("PathBuf::from(")
        || exported_record_carries_pathbuf(text)
}

fn exported_record_carries_pathbuf(text: &str) -> bool {
    let mut in_record = false;
    let mut brace_depth = 0usize;
    for line in text.lines() {
        let trimmed = line.trim_start();
        if !in_record
            && (trimmed.starts_with("pub(crate) struct ")
                || trimmed.starts_with("pub(crate) enum "))
        {
            in_record = true;
            brace_depth = 0;
        }
        if in_record && line.contains("PathBuf") {
            return true;
        }
        if in_record {
            for ch in line.chars() {
                match ch {
                    '{' => brace_depth += 1,
                    '}' => brace_depth = brace_depth.saturating_sub(1),
                    _ => {}
                }
            }
            if brace_depth == 0 && (line.contains('}') || line.trim_end().ends_with(';')) {
                in_record = false;
            }
        }
    }
    false
}

fn contains_identifier(text: &str, needle: &str) -> bool {
    text.split(|ch: char| !(ch == '_' || ch.is_ascii_alphanumeric()))
        .any(|token| token == needle)
}
