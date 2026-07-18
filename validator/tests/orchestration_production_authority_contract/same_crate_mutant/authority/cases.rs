use super::compile_cases::CaseSpec;

const PRODUCTION: &str = "orchestration/product/authority/production/mod.rs";
const TRANSACTION: &str = "orchestration/product/authority/production/execution_transaction.rs";

pub(super) const HIDDEN: &[CaseSpec] = &[
    hidden(
        "N10_ROOT_AUTHORITY_TYPE",
        "module `root_authority` is private",
        PRODUCTION,
    ),
    hidden(
        "N10_ROOT_CONSTRUCT",
        "module `root_authority` is private",
        PRODUCTION,
    ),
    hidden("N10_STORE_TYPE", "module `store` is private", PRODUCTION),
    hidden("N10_LEDGER_TYPE", "module `ledger` is private", PRODUCTION),
    hidden(
        "N10_RAW_RESUME_MODULE",
        "module `resume` is private",
        TRANSACTION,
    ),
    hidden(
        "N10_RAW_RECOVER_MODULE",
        "module `recover` is private",
        TRANSACTION,
    ),
    hidden(
        "N10_RAW_RECONCILE_MODULE",
        "module `reconcile` is private",
        TRANSACTION,
    ),
];

pub(super) const MEMBERS: &[CaseSpec] = &[
    member(
        "N10_ROOT_ACTOR_EXTRACT",
        "E0616",
        "field `root_actor`",
        PRODUCTION,
    ),
    member("N10_ROOT_KEY_EXTRACT", "E0616", "field `key`", PRODUCTION),
    member(
        "N10_ROOT_CLONE",
        "E0599",
        "no method named `clone`",
        PRODUCTION,
    ),
    member(
        "N10_ROOT_ISSUE",
        "E0624",
        "method `issue` is private",
        PRODUCTION,
    ),
    member(
        "N10_ROOT_VERIFY",
        "E0624",
        "method `verify_action` is private",
        PRODUCTION,
    ),
    member(
        "N10_PRODUCTION_AUTHORITY_EXTRACT",
        "E0616",
        "field `authority`",
        PRODUCTION,
    ),
    member(
        "N10_PRODUCTION_LEDGER_EXTRACT",
        "E0616",
        "field `ledger`",
        PRODUCTION,
    ),
    member(
        "N10_PRODUCTION_CLONE",
        "E0599",
        "no method named `clone`",
        PRODUCTION,
    ),
    member(
        "N10_PRODUCTION_NEW",
        "E0624",
        "function `new` is private",
        PRODUCTION,
    ),
    member(
        "N10_STORE_OPEN",
        "E0624",
        "function `open_or_initialize` is private",
        PRODUCTION,
    ),
    member(
        "N10_LEDGER_OPEN",
        "E0624",
        "function `open` is private",
        PRODUCTION,
    ),
    member(
        "N10_LEDGER_ISSUE",
        "E0624",
        "method `issue` is private",
        PRODUCTION,
    ),
    member(
        "N10_LEDGER_RESERVE",
        "E0624",
        "method `reserve` is private",
        PRODUCTION,
    ),
    member(
        "N10_RAW_RESUME_FUNCTION",
        "E0603",
        "function `execute` is private",
        TRANSACTION,
    ),
    member(
        "N10_RAW_RECOVER_FUNCTION",
        "E0603",
        "function `execute` is private",
        TRANSACTION,
    ),
    member(
        "N10_RAW_RECONCILE_FUNCTION",
        "E0603",
        "function `execute` is private",
        TRANSACTION,
    ),
];

const fn hidden(marker: &'static str, message: &'static str, file: &'static str) -> CaseSpec {
    CaseSpec {
        marker,
        file,
        code: "E0603",
        message,
        additional_message: None,
    }
}

const fn member(
    marker: &'static str,
    code: &'static str,
    message: &'static str,
    file: &'static str,
) -> CaseSpec {
    CaseSpec {
        marker,
        file,
        code,
        message,
        additional_message: None,
    }
}
