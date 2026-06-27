use crate::claim_semantics::str_field;
use serde_json::Value;

pub(crate) fn claim_text_product_applicable(claim: &Value) -> bool {
    let title = str_field(claim, "title");
    let desc = str_field(claim, "description");
    let text = crate::claim::text::normalized_text(&[&title, &desc]);
    let title_text = crate::claim::text::normalized_text(&[&title]);
    let tokens = crate::claim::text::tokens(&text);
    let title_tokens = crate::claim::text::tokens(&title_text);
    crate::claim::language::product_surface_claim(&text, &tokens)
        || crate::claim::language::app_surface_completion(&title_tokens)
        || crate::claim::language::app_surface_completion(&tokens)
}
