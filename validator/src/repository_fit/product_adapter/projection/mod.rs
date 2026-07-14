use serde::{Deserialize, Serialize};

use super::AdapterErrorId;

#[path = "inspection_projection.rs"]
mod inspection_projection;
#[path = "plan_projection.rs"]
mod plan_projection;

pub(crate) use inspection_projection::*;
pub(crate) use plan_projection::*;
