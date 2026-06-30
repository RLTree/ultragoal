use serde_json::Value;

pub(super) fn check(value: &Value, out: &mut Vec<String>) {
    let Some(board) = value
        .get("fitting_control_board")
        .and_then(Value::as_object)
    else {
        out.push("observability_fitting_control_board_missing".to_string());
        return;
    };
    let mut incomplete = Vec::new();
    for (family, inventory_key, required_ids) in families() {
        let counts = Counts::from_inventory(value, inventory_key);
        counts.require_board_family(board, family, out);
        if let Some(first) = first_incomplete(value, inventory_key, required_ids) {
            incomplete.push((family, first));
        }
    }
    let actual_status = if incomplete.is_empty() {
        "fitted"
    } else {
        "blocked"
    };
    if board.get("status").and_then(Value::as_str) != Some(actual_status) {
        out.push(format!(
            "observability_fitting_control_board_status_mismatch:{actual_status}"
        ));
    }
    require_first_incomplete(board, &incomplete, out);
    if !non_empty_string(board.get("claim_impact")) {
        out.push("observability_fitting_control_board_claim_impact_missing".to_string());
    }
}

fn families() -> [(&'static str, &'static str, &'static [&'static str]); 4] {
    [
        (
            "commands",
            "fitting_inventory",
            super::fitting::REQUIRED_COMMANDS,
        ),
        (
            "surfaces",
            "surface_inventory",
            super::surfaces::REQUIRED_SURFACES,
        ),
        (
            "operating_loop",
            "operating_loop_inventory",
            super::operating::REQUIRED_LOOP_STAGES,
        ),
        (
            "signals",
            "signal_inventory",
            super::operating::REQUIRED_SIGNAL_CLASSES,
        ),
    ]
}

fn require_first_incomplete(
    board: &serde_json::Map<String, Value>,
    incomplete: &[(&str, IncompleteRow)],
    out: &mut Vec<String>,
) {
    let first = board.get("first_incomplete").and_then(Value::as_object);
    match (incomplete.first(), first) {
        (None, None) => {}
        (None, Some(_)) => {
            out.push("observability_fitting_control_board_stale_first_incomplete".to_string());
        }
        (Some((family, expected)), Some(actual)) => {
            if actual.get("family").and_then(Value::as_str) != Some(*family)
                || actual.get("id").and_then(Value::as_str) != Some(expected.id.as_str())
                || actual.get("fitting_status").and_then(Value::as_str)
                    != Some(expected.status.as_str())
                || actual.get("next_unfitted_surface").and_then(Value::as_str)
                    != Some(expected.next_unfitted_surface.as_str())
            {
                out.push(format!(
                    "observability_fitting_control_board_first_incomplete_mismatch:{family}:{}",
                    expected.id
                ));
            }
        }
        (Some((family, expected)), None) => {
            out.push(format!(
                "observability_fitting_control_board_first_incomplete_missing:{family}:{}",
                expected.id
            ));
        }
    }
}

#[derive(Default)]
struct Counts {
    total: usize,
    fitted: usize,
    partially_fitted: usize,
    unfitted: usize,
}

impl Counts {
    fn from_inventory(value: &Value, inventory_key: &str) -> Self {
        let mut counts = Counts::default();
        let Some(rows) = value.get(inventory_key).and_then(Value::as_object) else {
            return counts;
        };
        for row in rows.values() {
            counts.total += 1;
            match row.get("fitting_status").and_then(Value::as_str) {
                Some("fitted") => counts.fitted += 1,
                Some("partially_fitted") => counts.partially_fitted += 1,
                Some("unfitted") => counts.unfitted += 1,
                _ => {}
            }
        }
        counts
    }

    fn require_board_family(
        &self,
        board: &serde_json::Map<String, Value>,
        family: &str,
        out: &mut Vec<String>,
    ) {
        let Some(row) = board
            .get("families")
            .and_then(Value::as_object)
            .and_then(|families| families.get(family))
            .and_then(Value::as_object)
        else {
            out.push(format!(
                "observability_fitting_control_board_family_missing:{family}"
            ));
            return;
        };
        for (key, expected) in [
            ("total", self.total),
            ("fitted", self.fitted),
            ("partially_fitted", self.partially_fitted),
            ("unfitted", self.unfitted),
        ] {
            if row.get(key).and_then(Value::as_u64) != Some(expected as u64) {
                out.push(format!(
                    "observability_fitting_control_board_count_mismatch:{family}:{key}"
                ));
            }
        }
    }
}

struct IncompleteRow {
    id: String,
    status: String,
    next_unfitted_surface: String,
}

fn first_incomplete(
    value: &Value,
    inventory_key: &str,
    required_ids: &[&str],
) -> Option<IncompleteRow> {
    let rows = value.get(inventory_key).and_then(Value::as_object)?;
    required_ids.iter().find_map(|id| {
        rows.get(*id).and_then(|row| {
            let status = row.get("fitting_status").and_then(Value::as_str)?;
            if status == "fitted" {
                return None;
            }
            Some(IncompleteRow {
                id: (*id).to_string(),
                status: status.to_string(),
                next_unfitted_surface: row
                    .get("next_unfitted_surface")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
            })
        })
    })
}

fn non_empty_string(value: Option<&Value>) -> bool {
    value
        .and_then(Value::as_str)
        .is_some_and(|text| !text.trim().is_empty())
}
