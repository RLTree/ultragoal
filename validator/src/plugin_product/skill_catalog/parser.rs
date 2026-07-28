use std::collections::BTreeMap;

use super::yaml_syntax::{scalar, split_mapping, strip_comment};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ParsedSkillMetadata {
    pub(crate) display_name: String,
    pub(crate) short_description: String,
    pub(crate) default_prompt: String,
    pub(crate) allow_implicit_invocation: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum YamlError {
    Empty,
    InvalidLine(usize),
    DuplicateKey(String),
    UnsupportedField(String),
    UnsupportedSequence(usize),
    InvalidScalar(String),
    MissingField(&'static str),
}

pub(crate) fn parse_skill_metadata(source: &str) -> Result<ParsedSkillMetadata, YamlError> {
    let values = parse_mapping(source)?;
    for key in values.keys() {
        if !matches!(
            key.as_str(),
            "interface.display_name"
                | "interface.short_description"
                | "interface.default_prompt"
                | "policy.allow_implicit_invocation"
        ) {
            return Err(YamlError::UnsupportedField(key.clone()));
        }
    }
    let display_name = required(&values, "interface.display_name", "display_name")?;
    let short_description = required(&values, "interface.short_description", "short_description")?;
    let default_prompt = required(&values, "interface.default_prompt", "default_prompt")?;
    let policy = values
        .get("policy.allow_implicit_invocation")
        .ok_or(YamlError::MissingField("policy.allow_implicit_invocation"))?;
    let allow_implicit_invocation = match policy.as_str() {
        "true" => true,
        "false" => false,
        _ => {
            return Err(YamlError::InvalidScalar(
                "allow_implicit_invocation".to_owned(),
            ));
        }
    };
    if display_name.trim().is_empty()
        || short_description.trim().is_empty()
        || default_prompt.trim().is_empty()
    {
        return Err(YamlError::InvalidScalar("empty interface value".to_owned()));
    }
    Ok(ParsedSkillMetadata {
        display_name,
        short_description,
        default_prompt,
        allow_implicit_invocation,
    })
}

fn required(
    values: &BTreeMap<String, String>,
    key: &'static str,
    label: &'static str,
) -> Result<String, YamlError> {
    values
        .get(key)
        .cloned()
        .ok_or(YamlError::MissingField(label))
}

fn parse_mapping(source: &str) -> Result<BTreeMap<String, String>, YamlError> {
    let lines = source.lines().collect::<Vec<_>>();
    let mut values = BTreeMap::new();
    let mut seen_paths = BTreeMap::new();
    let mut parents: Vec<(usize, String)> = Vec::new();
    let mut index = 0;
    let mut saw_mapping = false;
    let mut saw_document_start = false;
    let mut saw_document_end = false;
    while index < lines.len() {
        let raw = lines[index];
        index += 1;
        if raw.trim().is_empty() || raw.trim_start().starts_with('#') {
            continue;
        }
        if saw_document_end {
            return Err(YamlError::InvalidLine(index));
        }
        if raw.contains('\t') {
            return Err(YamlError::InvalidLine(index));
        }
        let indent = raw.len() - raw.trim_start().len();
        let content = strip_comment(raw[indent..].trim_end());
        if content == "---" {
            if saw_document_start || saw_mapping {
                return Err(YamlError::InvalidLine(index));
            }
            saw_document_start = true;
            continue;
        }
        if content == "..." {
            if !saw_mapping || saw_document_end {
                return Err(YamlError::InvalidLine(index));
            }
            saw_document_end = true;
            continue;
        }
        if content.starts_with('-') {
            return Err(YamlError::UnsupportedSequence(index));
        }
        let (key, value) = split_mapping(content).ok_or(YamlError::InvalidLine(index))?;
        let key = scalar(key.trim()).map_err(|_| YamlError::InvalidLine(index))?;
        while parents
            .last()
            .is_some_and(|(parent_indent, _)| *parent_indent >= indent)
        {
            parents.pop();
        }
        let path = parents
            .iter()
            .map(|(_, parent)| parent.as_str())
            .chain(std::iter::once(key.as_str()))
            .collect::<Vec<_>>()
            .join(".");
        if seen_paths.insert(path.clone(), index).is_some() {
            return Err(YamlError::DuplicateKey(path));
        }
        let value = value.trim();
        let parsed = if is_block_scalar(value) {
            let (block, next_index) = parse_block(&lines, index, indent, value)?;
            index = next_index;
            block
        } else if value.is_empty() {
            parents.push((indent, key.clone()));
            saw_mapping = true;
            continue;
        } else {
            scalar(value).map_err(|_| YamlError::InvalidLine(index))?
        };
        if values.insert(path.clone(), parsed).is_some() {
            return Err(YamlError::DuplicateKey(path));
        }
        saw_mapping = true;
    }
    if !saw_mapping {
        return Err(YamlError::Empty);
    }
    Ok(values)
}

fn parse_block(
    lines: &[&str],
    start: usize,
    parent_indent: usize,
    marker: &str,
) -> Result<(String, usize), YamlError> {
    if !matches!(marker, "|" | "|-" | "|+" | ">" | ">-" | ">+") {
        return Err(YamlError::InvalidScalar(marker.to_owned()));
    }
    let folded = marker.starts_with('>');
    let keep_trailing = marker.ends_with('+');
    let strip_trailing = marker.ends_with('-');
    let mut cursor = start;
    let mut block_lines = Vec::new();
    let mut content_indent = None;
    while cursor < lines.len() {
        let line = lines[cursor];
        let indent = line.len() - line.trim_start().len();
        if !line.trim().is_empty() && indent <= parent_indent {
            break;
        }
        if !line.trim().is_empty() {
            if let Some(current) = content_indent {
                if indent != current {
                    return Err(YamlError::InvalidLine(cursor + 1));
                }
            } else {
                content_indent = Some(indent);
            }
        }
        block_lines.push(line);
        cursor += 1;
    }
    let base = content_indent.unwrap_or(parent_indent + 1);
    let mut rendered = if folded {
        fold_lines(&block_lines, base)
    } else {
        block_lines
            .iter()
            .map(|line| {
                if line.trim().is_empty() {
                    String::new()
                } else {
                    line.get(base..).unwrap_or("").to_owned()
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    if !strip_trailing && (keep_trailing || cursor > start) {
        rendered.push('\n');
    }
    if strip_trailing {
        while rendered.ends_with('\n') {
            rendered.pop();
        }
    }
    Ok((rendered, cursor))
}

fn fold_lines(lines: &[&str], base: usize) -> String {
    let mut rendered = String::new();
    for (index, line) in lines.iter().enumerate() {
        let blank = line.trim().is_empty();
        if blank {
            rendered.push('\n');
        } else {
            if index > 0 && !rendered.ends_with(['\n', ' ']) {
                rendered.push(' ');
            }
            rendered.push_str(line.get(base..).unwrap_or(""));
        }
    }
    rendered
}

fn is_block_scalar(value: &str) -> bool {
    matches!(value.as_bytes().first(), Some(b'|' | b'>'))
}
