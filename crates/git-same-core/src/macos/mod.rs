//! macOS-only host integration helpers.
//!
//! These wrap Cocoa / xattr operations that the FinderSync extension cannot
//! perform from its sandbox — currently only custom workspace folder icons
//! (painted via `NSWorkspace.setIcon`). On non-macOS targets the submodules
//! expose no-op stubs so callers can stay platform-agnostic.

pub mod folder_icon;
