//! Cache and history persistence.

mod discovery;
#[cfg(feature = "tui")]
mod sync_history;

pub use discovery::{CacheManager, DiscoveryCache, CACHE_VERSION};
#[cfg(feature = "tui")]
pub use sync_history::SyncHistoryManager;
