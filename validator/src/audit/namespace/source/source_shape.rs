pub(crate) fn embedded_rust_source(literal: &str) -> bool {
    let trimmed = literal.trim();
    syn::parse_file(trimmed).is_ok()
        || parseable_rust_declaration(trimmed)
        || parseable_structured_rust_expression(trimmed)
}

fn parseable_rust_declaration(source: &str) -> bool {
    source.ends_with(';')
        && matches!(
            source.split_whitespace().next(),
            Some("let" | "const" | "static" | "use" | "extern")
        )
        && syn::parse_str::<syn::Stmt>(source).is_ok()
}

fn parseable_structured_rust_expression(source: &str) -> bool {
    source.contains(['(', '{', '[', '!', '='])
        && !super::path_labels::authority_like_literal(source)
        && syn::parse_str::<syn::Expr>(source).is_ok()
}

pub(crate) fn raw_literal_is_direct_authority_label(literal: &str) -> bool {
    let trimmed = literal.trim();
    !literal.contains('\n')
        && !matches!(trimmed.chars().next(), Some('{' | '['))
        && super::path_labels::authority_like_literal(trimmed)
}
