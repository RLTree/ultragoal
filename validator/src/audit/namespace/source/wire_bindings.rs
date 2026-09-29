use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::{Attribute, Expr, ExprLit, Fields, Item, Lit, Meta, Token};

const CLAIM_REGISTRY: &str =
    "docs/ultragoal-contract-2026-07-successor-v2/FINAL-CONTRACT/CLAIM_REGISTRY.json";
const CLAIM_SCHEMA: &str = "validator/src/state/adopted_claims/schema.rs";
const LIVE_EVIDENCE_FIELD: &str = "live_evidence_verification";
const LEGACY_WIRE_KEY: &str = concat!("current_live_evidence_", "status");
const EXTERNAL_AUTHORITY: (&str, &str, &str, &str) = (
    CLAIM_REGISTRY,
    CLAIM_SCHEMA,
    LIVE_EVIDENCE_FIELD,
    LEGACY_WIRE_KEY,
);

pub(super) fn approved_literal_lines(path: &str, source: &str) -> Vec<(usize, String)> {
    if path != CLAIM_SCHEMA {
        return Vec::new();
    }
    let Ok(file) = syn::parse_file(source) else {
        return Vec::new();
    };
    file.items
        .iter()
        .filter_map(|item| match item {
            Item::Struct(record) => Some(&record.fields),
            _ => None,
        })
        .filter_map(|fields| match fields {
            Fields::Named(fields) => Some(&fields.named),
            _ => None,
        })
        .flat_map(|fields| fields.iter())
        .filter_map(|field| {
            let identifier = field.ident.as_ref()?.to_string();
            let wire_key = field.attrs.iter().find_map(direct_serde_rename)?;
            declared_binding(path, &identifier, &wire_key).then_some((identifier, wire_key))
        })
        .filter(|(identifier, _)| super::identifiers::semantic_identifier(identifier))
        .filter_map(|(identifier, wire_key)| {
            source_attribute_line(source, &identifier, &wire_key).map(|line| (line, wire_key))
        })
        .collect()
}

fn direct_serde_rename(attribute: &Attribute) -> Option<String> {
    let Meta::List(list) = &attribute.meta else {
        return None;
    };
    if !list.path.is_ident("serde") {
        return None;
    }
    let arguments = Punctuated::<Meta, Token![,]>::parse_terminated
        .parse2(list.tokens.clone())
        .ok()?;
    if arguments.len() != 1 {
        return None;
    }
    let Meta::NameValue(rename) = arguments.first()? else {
        return None;
    };
    if !rename.path.is_ident("rename") {
        return None;
    }
    let Expr::Lit(ExprLit {
        lit: Lit::Str(value),
        ..
    }) = &rename.value
    else {
        return None;
    };
    Some(value.value())
}

fn declared_binding(path: &str, identifier: &str, wire_key: &str) -> bool {
    (CLAIM_REGISTRY, path, identifier, wire_key) == EXTERNAL_AUTHORITY
}

fn source_attribute_line(source: &str, identifier: &str, wire_key: &str) -> Option<usize> {
    let attribute = format!(r#"#[serde(rename = "{wire_key}")]"#);
    let lines = source.lines().collect::<Vec<_>>();
    lines.iter().enumerate().find_map(|(index, line)| {
        (line.trim() == attribute && named_field_follows(&lines, index + 1, identifier))
            .then_some(index + 1)
    })
}

fn named_field_follows(lines: &[&str], start: usize, identifier: &str) -> bool {
    lines[start..]
        .iter()
        .take_while(|line| !line.contains(';'))
        .any(|line| line.contains(&format!("{identifier}:")))
}
