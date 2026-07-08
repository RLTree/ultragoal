pub(super) fn strip_visibility(line: &str) -> &str {
    line.strip_prefix("pub(crate) ")
        .or_else(|| line.strip_prefix("pub(super) "))
        .or_else(|| line.strip_prefix("pub "))
        .or_else(|| {
            line.strip_prefix("pub(in ")
                .and_then(|rest| rest.find(") ").map(|end| &rest[end + 2..]))
        })
        .unwrap_or(line)
}

pub(super) fn strip_declaration_modifiers(mut line: &str) -> &str {
    loop {
        let next = line
            .strip_prefix("async ")
            .or_else(|| {
                line.strip_prefix("const ")
                    .filter(|rest| rest.starts_with("fn "))
            })
            .or_else(|| line.strip_prefix("unsafe "));
        let Some(stripped) = next else {
            return line;
        };
        line = stripped;
    }
}

pub(super) fn take_identifier(rest: &str) -> Option<&str> {
    let start = rest
        .trim_start()
        .strip_prefix("r#")
        .unwrap_or(rest.trim_start());
    let len = start
        .char_indices()
        .take_while(|(_, ch)| ch.is_ascii_alphanumeric() || *ch == '_')
        .map(|(index, ch)| index + ch.len_utf8())
        .last()?;
    Some(&start[..len])
}

pub(super) fn enum_variant_name(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    if trimmed.is_empty()
        || trimmed.starts_with("#[")
        || trimmed.starts_with("//")
        || trimmed.starts_with('}')
    {
        return None;
    }
    let name = take_identifier(trimmed)?;
    Some(name)
}

pub(super) struct ImplContext {
    pub(super) body_depth: usize,
    pub(super) owner: String,
    pub(super) test_only: bool,
}

pub(super) struct ModuleContext {
    pub(super) body_depth: usize,
    pub(super) owner: String,
    pub(super) test_only: bool,
}

pub(super) fn inline_module_name(line: &str) -> Option<&str> {
    line.strip_prefix("mod ").and_then(take_identifier)
}

pub(super) fn scoped_name(module_stack: &[ModuleContext], name: &str) -> String {
    module_stack
        .last()
        .map(|context| format!("{}::{name}", context.owner))
        .unwrap_or_else(|| name.to_string())
}

pub(super) fn close_scopes(
    brace_depth: usize,
    test_module_depth: &mut Option<usize>,
    enum_body_depth: &mut Option<usize>,
    enum_name: &mut String,
    enum_test_only: &mut bool,
    impl_stack: &mut Vec<ImplContext>,
    module_stack: &mut Vec<ModuleContext>,
) {
    if test_module_depth.is_some_and(|depth| brace_depth < depth) {
        *test_module_depth = None;
    }
    if enum_body_depth.is_some_and(|depth| brace_depth < depth) {
        *enum_body_depth = None;
        enum_name.clear();
        *enum_test_only = false;
    }
    while impl_stack
        .last()
        .is_some_and(|context| brace_depth < context.body_depth)
    {
        impl_stack.pop();
    }
    while module_stack
        .last()
        .is_some_and(|context| brace_depth < context.body_depth)
    {
        module_stack.pop();
    }
}

pub(super) fn impl_owner(line: &str) -> Option<String> {
    let rest = line.strip_prefix("impl")?;
    if !rest.starts_with(char::is_whitespace) && !rest.starts_with('<') {
        return None;
    }
    let rest = rest.trim_start();
    let header = rest.split('{').next()?.trim();
    let owner = if let Some((trait_name, type_name)) = header.rsplit_once(" for ") {
        format!(
            "{}_for_{}",
            clean_owner_token(trait_name),
            clean_owner_token(type_name)
        )
    } else {
        clean_owner_token(strip_impl_generics(header))
    };
    (!owner.is_empty()).then_some(owner)
}

fn strip_impl_generics(header: &str) -> &str {
    let header = header.trim_start();
    if !header.starts_with('<') {
        return header;
    }
    let mut depth = 0usize;
    for (index, ch) in header.char_indices() {
        match ch {
            '<' => depth += 1,
            '>' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return header[index + ch.len_utf8()..].trim_start();
                }
            }
            _ => {}
        }
    }
    header
}

fn clean_owner_token(token: &str) -> String {
    let without_where = token.split(" where ").next().unwrap_or(token).trim();
    let segment = without_where
        .trim_start_matches('&')
        .trim_start()
        .rsplit("::")
        .next()
        .unwrap_or(without_where);
    take_identifier(segment).unwrap_or("").to_string()
}

pub(in crate::audit::law::authority_surfaces::package_surfaces) fn test_cfg_attribute(
    line: &str,
) -> bool {
    let compact = line
        .trim_start()
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect::<String>();
    let Some(expr) = compact
        .strip_prefix("#[cfg(")
        .and_then(|rest| rest.strip_suffix(")]"))
    else {
        return false;
    };
    cfg_expr_requires_test(expr)
}

fn cfg_expr_requires_test(expr: &str) -> bool {
    if expr == "test" {
        return true;
    }
    if let Some(inner) = expr
        .strip_prefix("all(")
        .and_then(|rest| rest.strip_suffix(')'))
    {
        return split_cfg_args(inner)
            .into_iter()
            .any(cfg_expr_requires_test);
    }
    if let Some(inner) = expr
        .strip_prefix("any(")
        .and_then(|rest| rest.strip_suffix(')'))
    {
        let args = split_cfg_args(inner);
        return !args.is_empty() && args.into_iter().all(cfg_expr_requires_test);
    }
    false
}

fn split_cfg_args(inner: &str) -> Vec<&str> {
    let mut args = Vec::new();
    let mut start = 0usize;
    let mut depth = 0usize;
    for (index, ch) in inner.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                args.push(&inner[start..index]);
                start = index + ch.len_utf8();
            }
            _ => {}
        }
    }
    if start <= inner.len() {
        args.push(&inner[start..]);
    }
    args.into_iter()
        .map(str::trim)
        .filter(|arg| !arg.is_empty())
        .collect()
}

pub(super) fn update_brace_depth(depth: &mut usize, line: &str) {
    for ch in line.chars() {
        match ch {
            '{' => *depth += 1,
            '}' => *depth = depth.saturating_sub(1),
            _ => {}
        }
    }
}
