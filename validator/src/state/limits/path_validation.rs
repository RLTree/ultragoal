use super::*;

pub(crate) fn valid_relative_path(value: &str) -> bool {
    valid_text(value, false)
        && !unsafe_path_value(value)
        && Path::new(value)
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
}

pub(crate) fn valid_argv(argv: &[String]) -> bool {
    const GROUPS: &[&str] = &[
        "inspect", "next", "fit", "check", "diagnose", "prove", "observe", "package", "eval",
        "migrate",
    ];
    let group_index = if argv.get(1).is_some_and(|argument| argument == "--json") {
        2
    } else {
        1
    };
    if argv.len() <= group_index
        || argv.len() > MAX_ARGV
        || argv[0] != "ultragoal"
        || !GROUPS.contains(&argv[group_index].as_str())
    {
        return false;
    }
    argv.iter().enumerate().all(|(index, argument)| {
        let path_value = argument
            .split_once('=')
            .map_or(argument.as_str(), |(_, value)| value);
        valid_text(argument, false)
            && (argument != "--json" || index == 1)
            && !unsafe_path_value(path_value)
    })
}

pub(crate) fn proof_shaped_target(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower.contains("currentstate")
        || lower.contains("statuslabel")
        || lower
            .split(|character: char| !character.is_ascii_alphanumeric())
            .any(|token| {
                matches!(
                    token,
                    "status"
                        | "label"
                        | "view"
                        | "projection"
                        | "score"
                        | "pass"
                        | "proof"
                        | "evidence"
                        | "claim"
                        | "attestation"
                        | "witness"
                        | "result"
                        | "report"
                        | "artifact"
                ) || token.starts_with("receipt")
                    || token.starts_with("telemetry")
                    || token.starts_with("generated")
            })
}

pub(crate) fn bounded_projection(bytes: Vec<u8>) -> Result<Vec<u8>, StateError> {
    if bytes.len() > MAX_PROJECTION_BYTES {
        return Err(StateError::ResourceLimit(format!(
            "projection bytes {} exceeds {}",
            bytes.len(),
            MAX_PROJECTION_BYTES
        )));
    }
    Ok(bytes)
}

pub(crate) fn forbidden_char(value: char) -> bool {
    value.is_control()
        || matches!(
                value,
                '\u{202a}'
                    ..='\u{202e}'
                        | '\u{2066}'
                            ..='\u{2069}'
                                | '\u{200e}'
                                | '\u{200f}'
                                | '\u{061c}'
                                | '\u{feff}'
        )
}

pub(crate) fn looks_like_windows_absolute(value: &str) -> bool {
    value.starts_with('\\')
        || (value.as_bytes().get(1) == Some(&b':')
            && value
                .as_bytes()
                .first()
                .is_some_and(u8::is_ascii_alphabetic))
}

pub(crate) fn unsafe_path_value(value: &str) -> bool {
    Path::new(value).is_absolute()
        || value.starts_with('~')
        || value.contains("../")
        || value.contains("..\\")
        || value == ".."
        || looks_like_windows_absolute(value)
}
