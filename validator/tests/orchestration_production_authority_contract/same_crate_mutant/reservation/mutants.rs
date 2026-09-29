pub(super) const HIDDEN: &str = r#"
mod n10_hidden_reservation_type_probes {
    fn validated(_: Option<super::route::ValidatedExecution<'static>>) { // N10_VALIDATED_TYPE
    }
    fn reserved(_: Option<super::route::ReservedExecution<'static>>) { // N10_RESERVED_TYPE
    }
    fn validated_construct<'a>() {
        let _ = |permit_id: String, request: super::route::ExecutionRequest<'a>| super::route::ValidatedExecution { permit_id, request }; // N10_VALIDATED_CONSTRUCT
    }
    fn reserved_construct<'a>() {
        let _ = |permit_id: String, request: super::route::ExecutionRequest<'a>| super::route::ReservedExecution { permit_id, request }; // N10_RESERVED_CONSTRUCT
    }
}
"#;

pub(super) const MEMBERS: &str = r#"
mod n10_reservation_member_probes {
    type Validated<'a> = super::route::ValidatedExecution<'a>;
    type Reserved<'a> = super::route::ReservedExecution<'a>;

    fn validated_permit_field(token: &Validated<'_>) {
        let _ = &token.permit_id; // N10_VALIDATED_PERMIT_FIELD
    }
    fn validated_request_field(token: &Validated<'_>) {
        let _ = &token.request; // N10_VALIDATED_REQUEST_FIELD
    }
    fn reserved_permit_field(token: &Reserved<'_>) {
        let _ = &token.permit_id; // N10_RESERVED_PERMIT_FIELD
    }
    fn reserved_request_field(token: &Reserved<'_>) {
        let _ = &token.request; // N10_RESERVED_REQUEST_FIELD
    }
    fn validated_permit_method(token: &Validated<'_>) {
        let _ = token.permit_id(); // N10_VALIDATED_PERMIT_METHOD
    }
    fn validated_reserve_method(token: Validated<'_>) {
        let _ = token.reserve(); // N10_VALIDATED_RESERVE_METHOD
    }
    fn reserved_permit_method(token: &Reserved<'_>) {
        let _ = token.permit_id(); // N10_RESERVED_PERMIT_METHOD
    }
    fn reserved_execute_method(token: Reserved<'_>) {
        let _ = token.execute(); // N10_RESERVED_EXECUTE_METHOD
    }
    fn validated_replay(token: Validated<'_>) {
        let _ = token.clone(); // N10_VALIDATED_REPLAY
    }
    fn reserved_replay(token: Reserved<'_>) {
        let _ = token.clone(); // N10_RESERVED_REPLAY
    }
}
"#;
