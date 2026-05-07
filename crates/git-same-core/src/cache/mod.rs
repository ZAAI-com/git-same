//! Cache and history persistence.

mod discovery;
mod sync_history;

pub use discovery::{CacheManager, DiscoveryCache, CACHE_VERSION};
pub use sync_history::SyncHistoryManager;
