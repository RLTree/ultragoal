#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ObservationMode {
    FullRoundtrip,
    LoopRunSnapshot,
}
