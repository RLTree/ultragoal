use super::{GUARD_BYTES, GUARD_PATH, MAX_SOURCE_BYTES};
use crate::context::ReadSession;
use crate::inventory::agent_reader_guard_digests::all_readers;
use crate::inventory::fs::{read_bounded, relative};
use crate::inventory::walk::repository_files;
use std::path::Path;

mod literals;
mod unicode;

use literals::fragments_construct_agent_path;
use unicode::java_unicode_source;

fn active_reader_surface(path: &str) -> bool {
    if path.starts_with("validator/src/") {
        return !path.starts_with("validator/src/self_tests/")
            && !path.contains("/tests/")
            && !path.ends_with("/tests.rs")
            && !path.ends_with("_tests.rs");
    }
    if !(path.starts_with("scripts/")
        || path.starts_with("hooks/")
        || path.starts_with(".harness/"))
    {
        return false;
    }
    !matches!(
        Path::new(path).extension().and_then(|value| value.to_str()),
        Some(
            "json"
                | "jsonl"
                | "md"
                | "txt"
                | "toml"
                | "yaml"
                | "yml"
                | "csv"
                | "lock"
                | "svg"
                | "png"
                | "jpg"
                | "jpeg"
                | "gif"
                | "pdf"
        )
    )
}

fn fold(bytes: &[u8]) -> Vec<u8> {
    bytes
        .iter()
        .filter_map(|byte| {
            let byte = byte.to_ascii_lowercase();
            (byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'.' | b'_' | b'-'))
                .then_some(byte)
        })
        .collect()
}

fn compact(bytes: &[u8]) -> Vec<u8> {
    bytes
        .iter()
        .filter_map(|byte| {
            let byte = byte.to_ascii_lowercase();
            byte.is_ascii_alphanumeric().then_some(byte)
        })
        .collect()
}

fn constructed_agent_path(bytes: &[u8], java_prelexical: bool) -> bool {
    fragments_construct_agent_path(bytes)
        || (java_prelexical
            && java_unicode_source(bytes)
                .is_some_and(|decoded| fragments_construct_agent_path(&decoded)))
}

fn contains(folded: &[u8], token: &[u8]) -> bool {
    folded.windows(token.len()).any(|window| window == token)
}

fn contains_identifier(bytes: &[u8], token: &[u8]) -> bool {
    let lower = bytes.iter().map(u8::to_ascii_lowercase).collect::<Vec<_>>();
    lower
        .windows(token.len())
        .enumerate()
        .any(|(index, window)| {
            if window != token {
                return false;
            }
            let identifier = |byte: u8| byte.is_ascii_alphanumeric() || byte == b'_';
            index
                .checked_sub(1)
                .is_none_or(|prior| !identifier(lower[prior]))
                && lower
                    .get(index + token.len())
                    .is_none_or(|next| !identifier(*next))
        })
}

pub(super) fn contains_legacy_tokens(bytes: &[u8]) -> bool {
    let folded = fold(bytes);
    let compact = compact(bytes);
    [
        b"agent_type".as_slice(),
        b"custom_agent_path".as_slice(),
        b"persona_prompt_path".as_slice(),
    ]
    .iter()
    .any(|token| contains_identifier(bytes, token))
        || [
            b"custom-agents/".as_slice(),
            b"agents/contract-claim-falsifier".as_slice(),
            b"agents/orchestration-recovery-falsifier".as_slice(),
            b"agents/security-trust-boundary-falsifier".as_slice(),
            b"agents/product-simplicity-falsifier".as_slice(),
            b"agents/material-review-scope-gatekeeper".as_slice(),
            b"agents/plugin-scout".as_slice(),
            b"agents/standards-extractor".as_slice(),
        ]
        .iter()
        .any(|token| contains(&folded, token))
        || [
            b"agentscontractclaimfalsifier".as_slice(),
            b"agentsorchestrationrecoveryfalsifier".as_slice(),
            b"agentssecuritytrustboundaryfalsifier".as_slice(),
            b"agentsproductsimplicityfalsifier".as_slice(),
            b"agentsmaterialreviewscopegatekeeper".as_slice(),
            b"agentspluginscout".as_slice(),
            b"agentsstandardsextractor".as_slice(),
        ]
        .iter()
        .any(|token| contains(&compact, token))
}

fn looks_like_agent_reader_with_java(bytes: &[u8], java_prelexical: bool) -> bool {
    let folded = fold(bytes);
    contains_legacy_tokens(bytes)
        || constructed_agent_path(bytes, java_prelexical)
        || contains(&folded, b".codex/agents/")
        || (contains(&folded, b".codex/") && contains(&folded, b"agents/"))
        || (contains(&folded, b"custom-") && contains(&folded, b"agents/"))
        || [
            b"agent_manifest".as_slice(),
            b"agentmanifest".as_slice(),
            b"canonical_agent_roles".as_slice(),
            b"discover_agents".as_slice(),
            b"required_agent".as_slice(),
            b"agent_types".as_slice(),
        ]
        .iter()
        .any(|token| contains(&folded, token))
        || (contains(&folded, b"agents")
            && contains(&folded, b"manifest")
            && [
                b"path".as_slice(),
                b"resource".as_slice(),
                b"serde".as_slice(),
            ]
            .iter()
            .any(|token| contains(&folded, token)))
}

#[cfg(test)]
pub(super) fn looks_like_agent_reader(bytes: &[u8]) -> bool {
    looks_like_agent_reader_with_java(bytes, true)
}

pub(super) fn looks_like_agent_reader_at(path: &str, bytes: &[u8]) -> bool {
    let extension = Path::new(path).extension().and_then(|value| value.to_str());
    let java_prelexical = extension == Some("java") || extension.is_none();
    looks_like_agent_reader_with_java(bytes, java_prelexical)
}

pub(super) fn unbound_reader_paths(reads: &ReadSession, root: &Path) -> Option<Vec<String>> {
    let Ok(paths) = repository_files(reads, root) else {
        return None;
    };
    let mut unbound = Vec::new();
    for path in paths {
        let rel = relative(root, &path).ok()?;
        if !active_reader_surface(&rel) {
            continue;
        }
        let bytes = read_bounded(reads, &path, MAX_SOURCE_BYTES).ok()?;
        if !looks_like_agent_reader_at(&rel, &bytes) {
            continue;
        }
        if rel == GUARD_PATH {
            if bytes != GUARD_BYTES {
                unbound.push(rel);
            }
            continue;
        }
        let bound = all_readers().any(|spec| {
            spec.path == rel
                && spec.bytes == bytes
                && (!contains_legacy_tokens(&bytes) || spec.legacy_tokens_are_negative_only)
        });
        if !bound {
            unbound.push(rel);
        }
    }
    Some(unbound)
}

pub(super) fn no_unbound_reader(reads: &ReadSession, root: &Path) -> bool {
    unbound_reader_paths(reads, root).is_some_and(|paths| paths.is_empty())
}
