use super::model::AuthorityKind;
use std::collections::{BTreeMap, BTreeSet};
use syn::Token;
use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::{GenericArgument, PathArguments, ReturnType, Type};

pub(super) fn authority_for_type(
    node: &Type,
    aliases: &BTreeMap<String, AuthorityKind>,
) -> BTreeSet<AuthorityKind> {
    let mut out = BTreeSet::new();
    let Type::Path(path) = node else {
        return out;
    };
    let segments = path
        .path
        .segments
        .iter()
        .map(|part| part.ident.to_string())
        .collect::<Vec<_>>();
    if let Some(kind) = authority_for_segments(&segments)
        .or_else(|| segments.last().and_then(|name| aliases.get(name).copied()))
    {
        out.insert(kind);
    }
    if segments
        .last()
        .is_some_and(|name| name == "HashMap" || name == "BTreeMap")
        && type_contains_value(
            path.path.segments.last().map(|part| &part.arguments),
            aliases,
        )
    {
        out.insert(AuthorityKind::StringValueMap);
    }
    out
}

fn type_contains_value(
    arguments: Option<&PathArguments>,
    aliases: &BTreeMap<String, AuthorityKind>,
) -> bool {
    let Some(PathArguments::AngleBracketed(values)) = arguments else {
        return false;
    };
    values.args.iter().any(|argument| match argument {
        GenericArgument::Type(value) => authority_for_type(value, aliases)
            .iter()
            .any(|kind| matches!(kind, AuthorityKind::JsonValue | AuthorityKind::TomlValue)),
        _ => false,
    })
}

pub(super) fn authority_for_segments(segments: &[String]) -> Option<AuthorityKind> {
    match segments.join("::").as_str() {
        "String" | "std::string::String" => Some(AuthorityKind::RawString),
        "Path" | "std::path::Path" => Some(AuthorityKind::RawPath),
        "PathBuf" | "std::path::PathBuf" => Some(AuthorityKind::RawPathBuffer),
        "serde_json::Value" => Some(AuthorityKind::JsonValue),
        "serde_json::Map" => Some(AuthorityKind::JsonMap),
        "toml::Value" => Some(AuthorityKind::TomlValue),
        "std::process::Command" => Some(AuthorityKind::Process),
        "std::env"
        | "std::env::var"
        | "std::env::var_os"
        | "std::env::vars"
        | "std::env::current_dir"
        | "std::env::temp_dir" => Some(AuthorityKind::Environment),
        _ => None,
    }
}

pub(super) fn closed_result(output: &ReturnType) -> bool {
    let ReturnType::Type(_, output_type) = output else {
        return false;
    };
    let Type::Path(path) = output_type.as_ref() else {
        return false;
    };
    let Some(last) = path.path.segments.last() else {
        return false;
    };
    if last.ident != "Result" {
        return false;
    }
    let PathArguments::AngleBracketed(arguments) = &last.arguments else {
        return false;
    };
    let types = arguments
        .args
        .iter()
        .filter_map(|argument| match argument {
            GenericArgument::Type(value) => Some(value),
            _ => None,
        })
        .collect::<Vec<_>>();
    types.len() == 2 && types.into_iter().all(closed_named_type)
}

fn closed_named_type(value: &Type) -> bool {
    let Type::Path(path) = value else {
        return false;
    };
    let Some(last) = path.path.segments.last() else {
        return false;
    };
    !matches!(
        last.ident.to_string().as_str(),
        "String" | "Path" | "PathBuf" | "Value" | "Map" | "HashMap" | "BTreeMap" | "Vec" | "Option"
    )
}

pub(super) fn visible(visibility: &syn::Visibility) -> bool {
    !matches!(visibility, syn::Visibility::Inherited)
}

pub(super) fn cfg_test(attributes: &[syn::Attribute]) -> bool {
    attributes
        .iter()
        .filter(|attribute| attribute.path().is_ident("cfg"))
        .any(|attribute| {
            attribute.meta.require_list().ok().is_some_and(|list| {
                let parser = Punctuated::<syn::Meta, Token![,]>::parse_terminated;
                parser
                    .parse2(list.tokens.clone())
                    .ok()
                    .is_some_and(|items| items.iter().any(meta_has_test))
            })
        })
}

fn meta_has_test(meta: &syn::Meta) -> bool {
    match meta {
        syn::Meta::Path(path) => path.is_ident("test"),
        syn::Meta::List(list) => {
            let parser = Punctuated::<syn::Meta, Token![,]>::parse_terminated;
            parser
                .parse2(list.tokens.clone())
                .ok()
                .is_some_and(|items| items.iter().any(meta_has_test))
        }
        syn::Meta::NameValue(_) => false,
    }
}

pub(super) fn semantic_boundary_symbol(symbol: &str) -> bool {
    symbol.split('_').any(|token| {
        matches!(
            token,
            "parse" | "decode" | "read" | "load" | "ingest" | "capture"
        )
    })
}

pub(super) fn structured_authority(kind: AuthorityKind) -> bool {
    matches!(
        kind,
        AuthorityKind::JsonMap
            | AuthorityKind::JsonValue
            | AuthorityKind::StringValueMap
            | AuthorityKind::TomlValue
    )
}

pub(super) fn structured_parser_path(segments: &[String]) -> bool {
    matches!(
        segments.join("::").as_str(),
        "serde_json::from_str"
            | "serde_json::from_slice"
            | "serde_json::from_value"
            | "toml::from_str"
            | "toml::from_slice"
    )
}

pub(super) fn process_path(segments: &[String]) -> bool {
    segments
        .windows(2)
        .any(|pair| pair == ["process", "Command"])
}

pub(super) fn environment_path(segments: &[String]) -> bool {
    segments.windows(2).any(|pair| {
        pair[0] == "env"
            && matches!(
                pair[1].as_str(),
                "var" | "var_os" | "vars" | "current_dir" | "temp_dir"
            )
    })
}
