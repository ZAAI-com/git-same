//! Setup wizard screen renderers.

pub mod auth;
pub mod complete;
pub mod confirm;
pub mod orgs;
pub mod path;
pub mod provider;
pub mod requirements;

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
