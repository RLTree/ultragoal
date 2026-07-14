use crate::distribution::error::{DistributionError, DistributionErrorId, error};
use crate::distribution::host::MarketplaceTransaction;
use crate::distribution::json;
use crate::distribution::marketplace::{
    MarketplaceExpectation, MarketplacePlan, MarketplaceSnapshot, MarketplaceVerdict, snapshot,
};
use crate::distribution::reader::sha256;
use crate::distribution::reader::validate_relative_path;
use crate::distribution::spec::digest;
use crate::plugin_manifest::Version;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

include!("marketplace_transaction.rs");

include!("marketplace_catalog.rs");

include!("expectation.rs");

include!("snapshot.rs");
