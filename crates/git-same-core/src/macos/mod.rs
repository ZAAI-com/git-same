//! macOS-only host integration helpers.
//!
//! These wrap Cocoa / xattr operations and TCC probes that the FinderSync
//! extension cannot perform from its sandbox: custom workspace folder icons
//! (painted via `NSWorkspace.setIcon`) and the Full Disk Access probe. On
//! non-macOS targets the submodules expose no-op stubs so callers can stay
//! platform-agnostic.

pub mod folder_icon;
pub mod full_disk_access;
pub mod monitor_agent;
