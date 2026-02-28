//! Provider-specific configuration.
//!
//! This module is kept minimal — provider configuration is now handled
//! directly by `WorkspaceProvider` in the workspace config. The `AuthMethod`
//! enum has been removed since gh-cli is the only supported auth method and
//! is hardcoded in the auth module.

// This module is intentionally kept as a placeholder. All provider
// configuration is now in workspace.rs (WorkspaceProvider).

#[cfg(test)]
#[path = "provider_config_tests.rs"]
mod tests;
