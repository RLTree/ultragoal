use super::{BoundInput, EvaluationError};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::time::{SystemTime, UNIX_EPOCH};

include!("max_research_sources.rs");

include!("experimental_hypothesis_new.rs");

include!("source/record.rs");

include!("source/authority_binding.rs");

include!("source/bound_record.rs");

include!("source/substitution_test.rs");

include!("proposal_analyses_new.rs");

include!("audit/execution.rs");

include!("audit/configuration.rs");
