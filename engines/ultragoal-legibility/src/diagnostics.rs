use crate::{inventory::Inventory, metadata::Metadata, syntax::Report};
use serde_json::{Value, json};
use std::collections::BTreeMap;

pub fn inventory(
    inventory: &Inventory,
    metadata: &Metadata,
    reports: &BTreeMap<String, Report>,
) -> Value {
    json!({
        "files": inventory.files.iter().map(|(path, bytes)| json!({
            "path": path, "bytes": bytes.len(),
            "physical_lines": bytes.iter().filter(|b| **b == b'\n').count() + usize::from(!bytes.is_empty() && bytes.last() != Some(&b'\n'))
        })).collect::<Vec<_>>(),
        "direct_dependencies": metadata.dependencies.iter().map(|d| json!({
            "manifest": d.manifest, "crate": d.crate_name, "version_requirement": d.requirement
        })).collect::<Vec<_>>(),
        "entrypoints": metadata.entrypoints,
        "verified_generated_locks": metadata.verified_locks,
        "rust": reports.iter().map(|(path, r)| json!({
            "path": path,
            "functions": r.functions.iter().map(|f| json!({
                "symbol": f.name, "return_type": f.return_type, "closed_result": f.returns_closed_result,
                "calls": f.calls, "direct_calls": f.direct_calls, "output_identifiers": f.output_identifiers,"resolved_error":f.resolved_error,
                "generic_response_parameters":f.generic.response_parameters,"generic_constraints":f.generic.constraints
            })).collect::<Vec<_>>(),
            "authorities": r.authorities.iter().map(|a| json!({"symbol":a.function,"owner":a.owner,"site_kind":if a.function.is_some(){"function"}else{"declaration_or_module"},"kind":a.kind})).collect::<Vec<_>>(),
            "dependency_uses": r.dependencies.iter().map(|d| json!({"crate":d.crate_name,"symbol":d.symbol,"owner":d.owner})).collect::<Vec<_>>(),
            "types": r.types.iter().map(|t| json!({"name":t.name,"closed":t.closed,"source":t.source,"fields":t.fields,"variants":t.variants})).collect::<Vec<_>>()
        })).collect::<Vec<_>>()
    })
}
