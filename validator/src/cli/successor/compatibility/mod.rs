mod classification;
mod render;

pub(crate) use classification::classify_legacy_command;
pub(crate) use render::{COMPATIBILITY_EXIT_CODE, render_compatibility_guidance};
