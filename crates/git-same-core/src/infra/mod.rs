//! Legacy infrastructure facade.
//!
//! New code should import [`crate::cache`], [`crate::config`], [`crate::git`],
//! and [`crate::provider`] directly. This facade remains public for backwards
//! compatibility and is deprecated at the crate root.

pub mod storage;

pub use crate::git;
pub use crate::provider;
