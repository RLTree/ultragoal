//! Local-first semantic events, bounded persistence, causal diagnosis, and opt-in export.
//!
//! Query and explanation paths only read the configured event file. Appends,
//! recovery, deletion, and export are separate explicit calls. Events are
//! diagnostic observations and have no API that can mutate claim state.

mod binding;
mod event;
mod explain;
mod export;
mod filesystem;
mod format;
mod identity;
mod lifecycle;
mod limits;
mod privacy;
mod query;
mod store;

pub use event::SemanticEvent;
pub use explain::CausalExplanation;
pub use export::ExportAdapter;
pub use query::EventQuery;
pub use store::EventStore;
