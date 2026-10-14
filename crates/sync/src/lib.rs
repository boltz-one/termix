//! termix-sync — the local `DocsStore` (SQLite snapshots + processed-command
//! ledger).
//!
//! - [`DocsStore`]: snapshot persistence (the doc IS the outbox — commands + user entries
//!   flush immediately) and the processed-command ledger with mark-BEFORE-execute semantics.

mod store;
mod sync_jobs;

pub use store::{DocsStore, StoreError};
