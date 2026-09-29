use super::{declarations, imports, visitor::Scanner};
use quote::ToTokens;
use syn::parse::discouraged::Speculative;
use syn::parse::{Parse, ParseStream, Parser};
use syn::visit::Visit;

impl Scanner {
    pub(super) fn inspect_macro(&mut self, item: &syn::Macro) {
        if item.path.is_ident("include") {
            self.report
                .failures
                .push("unexpanded_rust_include:use_explicit_governed_modules".into());
        }
        if item.path.is_ident("include_str") || item.path.is_ident("include_bytes") {
            match syn::parse2::<syn::LitStr>(item.tokens.clone()) {
                Ok(path) => self
                    .report
                    .includes
                    .push((item.path.to_token_stream().to_string(), path.value())),
                Err(_) => self
                    .report
                    .failures
                    .push("unresolved_compile_time_data_include:require_literal_owned_path".into()),
            }
        }
        if item.path.is_ident("env") || item.path.is_ident("option_env") {
            self.authority("environment");
        }
        if composes_path(item.tokens.clone()) {
            self.report
                .failures
                .push("macro_composes_unresolved_authority_path".into());
        }
        let definition = item.path.is_ident("macro_rules");
        if !definition {
            for symbol in imports::resolve(&item.path, &self.scopes) {
                self.call(symbol.clone());
                self.dependency(symbol);
            }
        }
        if item.path.is_ident("object") {
            self.object(item);
            return;
        }
        if !definition
            && let Ok(expressions) =
                syn::punctuated::Punctuated::<syn::Expr, syn::Token![,]>::parse_terminated
                    .parse2(item.tokens.clone())
        {
            for expression in &expressions {
                self.visit_expr(expression);
            }
            return;
        }
        // Conservative token-tree hardening added to the recovered AST visitor.
        // This inspects Rust paths, not textual substrings or string contents.
        match syn::parse2::<MacroPaths>(item.tokens.clone()) {
            Ok(paths) => {
                for path in &paths.0 {
                    self.visit_path(path);
                }
            }
            Err(error) => self
                .report
                .limitations
                .push(format!("Macro token inspection failed: {error}")),
        }
        self.report.limitations.push(format!(
            "Unexpanded macro: {}; token paths inspected conservatively",
            item.path.to_token_stream()
        ));
    }

    fn object(&mut self, item: &syn::Macro) {
        match syn::parse2::<declarations::Object>(item.tokens.clone()) {
            Ok(object) => {
                let contract = self
                    .object_contract
                    .iter()
                    .rev()
                    .find_map(|x| *x)
                    .unwrap_or(false);
                self.record_type(
                    &object.name,
                    &[],
                    object.fields.iter().collect(),
                    "object_macro",
                    contract,
                );
                if !contract {
                    self.report.limitations.push(
                        "object! without recognized local or inherited definition is not closed"
                            .into(),
                    );
                }
            }
            Err(error) => self
                .report
                .limitations
                .push(format!("Unparsed object!: {error}")),
        }
    }
}

fn composes_path(tokens: proc_macro2::TokenStream) -> bool {
    use proc_macro2::TokenTree;
    let tokens: Vec<_> = tokens.into_iter().collect();
    let punct = |token: &TokenTree, ch| matches!(token,TokenTree::Punct(p) if p.as_char()==ch);
    for (index, token) in tokens.iter().enumerate() {
        if let TokenTree::Group(group) = token
            && composes_path(group.stream())
        {
            return true;
        }
        if tokens
            .get(index..index + 3)
            .is_some_and(|s| punct(&s[0], ':') && punct(&s[1], ':') && punct(&s[2], '$'))
        {
            return true;
        }
        if tokens.get(index..index + 4).is_some_and(|s| {
            punct(&s[0], '$')
                && !matches!(&s[1],TokenTree::Ident(i) if i=="crate")
                && punct(&s[2], ':')
                && punct(&s[3], ':')
        }) {
            return true;
        }
    }
    false
}

struct MacroPaths(Vec<syn::Path>);

impl Parse for MacroPaths {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut paths = Vec::new();
        while !input.is_empty() {
            let nested;
            if input.peek(syn::token::Brace) {
                syn::braced!(nested in input);
                paths.extend(nested.parse::<MacroPaths>()?.0);
            } else if input.peek(syn::token::Paren) {
                syn::parenthesized!(nested in input);
                paths.extend(nested.parse::<MacroPaths>()?.0);
            } else if input.peek(syn::token::Bracket) {
                syn::bracketed!(nested in input);
                paths.extend(nested.parse::<MacroPaths>()?.0);
            } else {
                let fork = input.fork();
                if let Ok(path) = fork.parse::<syn::ExprPath>() {
                    paths.push(path.path);
                    input.advance_to(&fork);
                } else {
                    input.step(|cursor| {
                        cursor
                            .token_tree()
                            .map(|(_, rest)| ((), rest))
                            .ok_or_else(|| cursor.error("expected macro token"))
                    })?;
                }
            }
        }
        Ok(Self(paths))
    }
}
