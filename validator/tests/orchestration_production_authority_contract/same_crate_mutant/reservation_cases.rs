use super::compile_cases::CaseSpec;

const TRANSACTION: &str = "orchestration/product/authority/production/execution_transaction.rs";

pub(super) const HIDDEN: &[CaseSpec] = &[
    hidden(
        "N10_VALIDATED_TYPE",
        "struct `ValidatedExecution` is private",
        None,
    ),
    hidden(
        "N10_RESERVED_TYPE",
        "struct `ReservedExecution` is private",
        None,
    ),
    hidden(
        "N10_VALIDATED_CONSTRUCT",
        "struct `ValidatedExecution` is private",
        Some("enum `ExecutionRequest` is private"),
    ),
    hidden(
        "N10_RESERVED_CONSTRUCT",
        "struct `ReservedExecution` is private",
        Some("enum `ExecutionRequest` is private"),
    ),
];

pub(super) const MEMBERS: &[CaseSpec] = &[
    member("N10_VALIDATED_PERMIT_FIELD", "E0616", "field `permit_id`"),
    member("N10_VALIDATED_REQUEST_FIELD", "E0616", "field `request`"),
    member("N10_RESERVED_PERMIT_FIELD", "E0616", "field `permit_id`"),
    member("N10_RESERVED_REQUEST_FIELD", "E0616", "field `request`"),
    member(
        "N10_VALIDATED_PERMIT_METHOD",
        "E0624",
        "method `permit_id` is private",
    ),
    member(
        "N10_VALIDATED_RESERVE_METHOD",
        "E0624",
        "method `reserve` is private",
    ),
    member(
        "N10_RESERVED_PERMIT_METHOD",
        "E0624",
        "method `permit_id` is private",
    ),
    member(
        "N10_RESERVED_EXECUTE_METHOD",
        "E0624",
        "method `execute` is private",
    ),
    member("N10_VALIDATED_REPLAY", "E0599", "no method named `clone`"),
    member("N10_RESERVED_REPLAY", "E0599", "no method named `clone`"),
];

const fn hidden(
    marker: &'static str,
    message: &'static str,
    additional_message: Option<&'static str>,
) -> CaseSpec {
    CaseSpec {
        marker,
        file: TRANSACTION,
        code: "E0603",
        message,
        additional_message,
    }
}

const fn member(marker: &'static str, code: &'static str, message: &'static str) -> CaseSpec {
    CaseSpec {
        marker,
        file: TRANSACTION,
        code,
        message,
        additional_message: None,
    }
}
