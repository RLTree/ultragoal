use serde_json::json;

#[test]
fn loop_status_projects_current_blocker_state() {
    let none = json!({"id": "none"});
    assert_eq!(
        super::blockers::status_for_blockers(&none, &json!({"id": "none"})),
        "pass"
    );
    assert_eq!(
        super::blockers::status_for_blockers(
            &json!({"id": "fmt_check"}),
            &json!({"id": "fmt_check"})
        ),
        "fail"
    );
    assert_eq!(
        super::blockers::status_for_blockers(&none, &json!({"id": "fmt_check"})),
        "partial"
    );
}

#[test]
fn loop_blocker_separates_hot_product_and_control_board_blockers() {
    let current_state = json!({
        "first_blocker": {
            "id": "coverage_prove",
            "why_failed": "coverage receipt is stale"
        }
    });
    let product = super::blockers::first_product_blocker(&[]);
    let control_board = super::blockers::first_control_board_blocker(&current_state);
    let blocker = super::blockers::first_loop_blocker(
        &product,
        &super::blockers::none(),
        &super::blockers::none(),
        &control_board,
    );
    assert_eq!(product["id"], "none");
    assert_eq!(control_board["id"], "coverage_prove");
    assert_eq!(blocker["id"], "coverage_prove");
    assert_eq!(blocker["why_failed"], "coverage receipt is stale");
}
