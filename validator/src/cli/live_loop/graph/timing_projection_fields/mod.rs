mod baseline;
mod defaults;
mod result;
mod work;

use super::super::nodes::timing::NodeTiming;
use super::super::surfaces::LoopValidationSurface;
use serde_json::{Map, Value};

pub(super) fn insert(
    object: &mut Map<String, Value>,
    surface: LoopValidationSurface,
    node_timing: Option<&NodeTiming>,
    graph_ms: u64,
) {
    work::insert(object, surface, node_timing, graph_ms);
    result::insert(object, surface, node_timing);
    baseline::insert(object, surface, node_timing);
}
