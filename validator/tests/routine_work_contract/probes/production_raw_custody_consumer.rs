use ultragoal::routine_work::runtime_adapter::production::custody::store::{
    DurableCustody, ReservationToken, TerminalRecord,
};

fn main() {
    let _ = std::mem::size_of::<(DurableCustody, ReservationToken, TerminalRecord)>();
}
