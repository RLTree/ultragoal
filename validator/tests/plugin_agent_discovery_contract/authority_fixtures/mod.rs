use crate::agent_discovery::{
    AgentAuthorityLayer, HostAgentAuthorityReader, HostAgentAuthorityRequest,
    HostAgentAuthorityTransaction, HostAgentAuthorityTransactionError, ReadOnlyEffectEnforcement,
    ReadOnlyEffectRequest, SourceAgentCatalog, SupportedHostAgentRoots,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

include!("next_temp.rs");

include!("fixture_reader_with_transaction.rs");

include!("catalog_bytes.rs");
