use super::command_contract::{HostPath, ParsedValue, RelativePath, RepositoryTarget, ValueKind};
use super::error::ParseErrorId;

const MAX_IDENTIFIER_BYTES: usize = 128;
const MAX_PATH_BYTES: usize = 4096;

pub(crate) fn parse_value(kind: ValueKind, value: &str) -> Result<ParsedValue, ParseErrorId> {
    if value == "--help" || value == "-h" {
        return Err(ParseErrorId::HelpValueConfusion);
    }
    match kind {
        ValueKind::Flag => Err(ParseErrorId::UnexpectedOptionValue),
        ValueKind::Identifier => validate_identifier(value).map(ParsedValue::Identifier),
        ValueKind::RepositoryTarget => {
            validate_repository_target(value).map(ParsedValue::RepositoryTarget)
        }
        ValueKind::RelativePath => validate_relative_path(value).map(ParsedValue::RelativePath),
        ValueKind::HostPath => HostPath::parse(value).map(ParsedValue::HostPath),
    }
}

fn validate_repository_target(value: &str) -> Result<RepositoryTarget, ParseErrorId> {
    if value == "." {
        return Ok(RepositoryTarget(value.to_owned()));
    }
    validate_relative_path(value).map(|path| RepositoryTarget(path.0))
}

fn validate_identifier(value: &str) -> Result<String, ParseErrorId> {
    if value.is_empty()
        || value.len() > MAX_IDENTIFIER_BYTES
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
    {
        return Err(ParseErrorId::InvalidIdentifier);
    }
    Ok(value.to_owned())
}

fn validate_relative_path(value: &str) -> Result<RelativePath, ParseErrorId> {
    if value.is_empty()
        || value.len() > MAX_PATH_BYTES
        || value.starts_with('/')
        || value.starts_with('~')
        || value.ends_with('/')
        || value.bytes().any(|byte| {
            byte == 0
                || byte.is_ascii_control()
                || matches!(
                    byte,
                    b'\\' | b':' | b'%' | b'?' | b'*' | b'"' | b'<' | b'>' | b'|'
                )
        })
    {
        return Err(ParseErrorId::InvalidPath);
    }
    if value
        .split('/')
        .any(|component| component.is_empty() || matches!(component, "." | ".."))
    {
        return Err(ParseErrorId::InvalidPath);
    }
    Ok(RelativePath(value.to_owned()))
}

#[cfg(test)]
mod tests {
    use super::{ParsedValue, ValueKind, parse_value};

    #[test]
    fn current_repository_marker_is_specific_to_repository_targets() {
        assert!(matches!(
            parse_value(ValueKind::RepositoryTarget, "."),
            Ok(ParsedValue::RepositoryTarget(target)) if target.as_str() == "."
        ));
        assert!(parse_value(ValueKind::RelativePath, ".").is_err());
    }
}
