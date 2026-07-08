use std::path::Path;

mod syntax;
pub(super) use syntax::test_cfg_attribute;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SourceSymbol {
    pub(super) kind: &'static str,
    pub(super) name: String,
    pub(super) test_only: bool,
}

pub(super) fn actual_paths(root: &Path) -> Result<Vec<String>, String> {
    Ok(actual_package_paths(root)?
        .into_iter()
        .filter(|rel| {
            (rel.starts_with("validator/src/") || rel.starts_with("validator/tests/"))
                && rel.ends_with(".rs")
        })
        .collect())
}

pub(super) fn actual_package_paths(root: &Path) -> Result<Vec<String>, String> {
    crate::package::inventory::closure::actual_files(root)
}

pub(super) fn package_resource_kind(rel: &str) -> Option<&'static str> {
    if rel.starts_with("schemas/") {
        Some("schema")
    } else if rel.starts_with("fixtures/") {
        Some("fixture")
    } else if rel.starts_with("scripts/") || rel.starts_with("templates/scripts/") {
        Some("script")
    } else if rel.starts_with("docs/generated/") || rel.starts_with("examples/generated/") {
        Some("generated_artifact")
    } else if rel.contains("final-packet") {
        Some("final_packet_blocker")
    } else if rel.contains("update-goal") {
        Some("update_goal_blocker")
    } else {
        None
    }
}

pub(super) fn symbols(root: &Path, rel: &str) -> Result<Vec<SourceSymbol>, String> {
    let text = std::fs::read_to_string(root.join(rel))
        .map_err(|err| format!("unable to read {rel}: {err}"))?;
    let mut out = Vec::new();
    let mut next_function_is_test = false;
    let mut next_item_is_cfg_test = false;
    let mut test_module_depth = None;
    let mut enum_body_depth = None;
    let mut enum_name = String::new();
    let mut enum_test_only = false;
    let mut module_stack = Vec::<syntax::ModuleContext>::new();
    let mut impl_stack = Vec::<syntax::ImplContext>::new();
    let mut brace_depth = 0usize;
    for line in text.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("#[test]") {
            next_function_is_test = true;
            continue;
        }
        if syntax::test_cfg_attribute(trimmed) {
            next_item_is_cfg_test = true;
            continue;
        }
        let public_trimmed = syntax::strip_declaration_modifiers(syntax::strip_visibility(trimmed));
        let start_depth = brace_depth;
        let in_test_module = test_module_depth.is_some()
            || module_stack.last().is_some_and(|context| context.test_only);
        let cfg_test_applies_to_line = next_item_is_cfg_test;
        let cfg_test_consumed_by_line =
            cfg_test_applies_to_line && !trimmed.is_empty() && !trimmed.starts_with("#[");
        let item_is_test_only = next_function_is_test || cfg_test_applies_to_line || in_test_module;
        if let Some(owner) = syntax::impl_owner(public_trimmed)
            && public_trimmed.contains('{')
        {
            impl_stack.push(syntax::ImplContext {
                body_depth: start_depth + 1,
                owner: syntax::scoped_name(&module_stack, &owner),
                test_only: item_is_test_only,
            });
        }
        if let Some(name) = syntax::inline_module_name(public_trimmed)
            && public_trimmed.contains('{')
        {
            let scoped_name = syntax::scoped_name(&module_stack, name);
            out.push(SourceSymbol {
                kind: "inline_module",
                name: scoped_name.clone(),
                test_only: item_is_test_only,
            });
            if cfg_test_applies_to_line {
                test_module_depth = Some(start_depth + 1);
            }
            module_stack.push(syntax::ModuleContext {
                body_depth: start_depth + 1,
                owner: scoped_name,
                test_only: item_is_test_only,
            });
            if cfg_test_consumed_by_line {
                next_item_is_cfg_test = false;
            }
            syntax::update_brace_depth(&mut brace_depth, trimmed);
            syntax::close_scopes(
                brace_depth,
                &mut test_module_depth,
                &mut enum_body_depth,
                &mut enum_name,
                &mut enum_test_only,
                &mut impl_stack,
                &mut module_stack,
            );
            continue;
        }
        if enum_body_depth == Some(brace_depth)
            && let Some(variant) = syntax::enum_variant_name(public_trimmed)
        {
            out.push(SourceSymbol {
                kind: "enum_variant",
                name: format!("{enum_name}::{variant}"),
                test_only: enum_test_only || item_is_test_only,
            });
        }
        if let Some(rest) = public_trimmed.strip_prefix("fn ") {
            if let Some(name) = syntax::take_identifier(rest) {
                let scoped_name = impl_stack
                    .last()
                    .map(|context| format!("{}::{name}", context.owner))
                    .unwrap_or_else(|| syntax::scoped_name(&module_stack, name));
                out.push(SourceSymbol {
                    kind: if next_function_is_test {
                        "test_function"
                    } else {
                        "function"
                    },
                    name: scoped_name,
                    test_only: item_is_test_only
                        || impl_stack.last().is_some_and(|context| context.test_only),
                });
            }
            next_function_is_test = false;
            if cfg_test_consumed_by_line {
                next_item_is_cfg_test = false;
            }
            syntax::update_brace_depth(&mut brace_depth, trimmed);
            syntax::close_scopes(
                brace_depth,
                &mut test_module_depth,
                &mut enum_body_depth,
                &mut enum_name,
                &mut enum_test_only,
                &mut impl_stack,
                &mut module_stack,
            );
            continue;
        }
        next_function_is_test = false;
        for (prefix, kind) in [
            ("struct ", "type"),
            ("enum ", "type"),
            ("trait ", "type"),
            ("type ", "type"),
            ("const ", "constant"),
            ("static ", "constant"),
        ] {
            if let Some(rest) = public_trimmed.strip_prefix(prefix)
                && let Some(name) = syntax::take_identifier(rest)
            {
                let scoped_name = syntax::scoped_name(&module_stack, name);
                out.push(SourceSymbol {
                    kind,
                    name: scoped_name.clone(),
                    test_only: item_is_test_only,
                });
                if prefix == "enum " && public_trimmed.contains('{') {
                    enum_body_depth = Some(start_depth + 1);
                    enum_name = scoped_name;
                    enum_test_only = item_is_test_only;
                }
                if cfg_test_consumed_by_line {
                    next_item_is_cfg_test = false;
                }
            }
        }
        if cfg_test_consumed_by_line {
            next_item_is_cfg_test = false;
        }
        syntax::update_brace_depth(&mut brace_depth, trimmed);
        syntax::close_scopes(
            brace_depth,
            &mut test_module_depth,
            &mut enum_body_depth,
            &mut enum_name,
            &mut enum_test_only,
            &mut impl_stack,
            &mut module_stack,
        );
    }
    Ok(out)
}

pub(super) fn contains(root: &Path, rel: &str, needle: &str) -> bool {
    std::fs::read_to_string(root.join(rel)).is_ok_and(|text| text.contains(needle))
}
