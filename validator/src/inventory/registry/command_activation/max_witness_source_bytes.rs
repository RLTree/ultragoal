const MAX_WITNESS_SOURCE_BYTES: u64 = 4 * 1024 * 1024;
const API_GENERATOR: &str = "HCT-INVENTORY:compiled-api-witness";
const COMMAND_GENERATOR: &str = "HCT-INVENTORY:compiled-command-catalog-witness";

#[derive(Clone, Copy, Debug)]
struct WitnessSource {
    relative: &'static str,
    embedded: &'static [u8],
    responsibility: &'static str,
}
