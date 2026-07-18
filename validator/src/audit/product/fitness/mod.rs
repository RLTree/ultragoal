pub(crate) mod evidence;
pub(crate) mod receipt;
pub(crate) mod substitutions;
use serde_json::Value;
use std::path::Path;

const RECEIPT: &str = "validation_artifacts/harness/product-fitness-receipt.json";
const TEMPLATE: &str = "templates/PRODUCT_FITNESS_RECEIPT.json";
const LAW: &str = "docs/product-fitness-and-quality-in-use.md";
const SCHEMA: &str = "schemas/product-fitness-receipt.schema.json";

pub fn package_failures(root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    for path in [LAW, TEMPLATE, SCHEMA, RECEIPT] {
        if !root.join(path).is_file() {
            out.push(format!("product_fitness_surface_missing:{path}"));
        }
    }
    out.extend(strict_language_failures(root));
    let receipt = match crate::json_boundary::read_json(&root.join(RECEIPT)) {
        Ok(value) => value,
        Err(err) => {
            out.push(format!("product_fitness_receipt_missing:{err}"));
            return out;
        }
    };
    out.extend(canonical_package_receipt_value_failures(root, &receipt));
    out
}

pub fn canonical_package_receipt_value_failures(root: &Path, receipt: &Value) -> Vec<String> {
    crate::audit::product::fitness::receipt::canonical_package_failures(root, receipt)
}

fn strict_language_failures(root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    for path in [LAW, "templates/PRODUCT_FITNESS.md"] {
        let text = std::fs::read_to_string(root.join(path)).unwrap_or_default();
        if text.is_empty() {
            out.push(format!("product_fitness_law_missing:{path}"));
            continue;
        }
        for banned in [" should ", " may ", " can usually ", " recommended "] {
            if text.to_ascii_lowercase().contains(banned) {
                out.push(format!("product_fitness_discretionary_language:{path}"));
                break;
            }
        }
    }
    out
}
