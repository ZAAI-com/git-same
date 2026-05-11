//! Paint a custom folder icon on a workspace root.
//!
//! On macOS this wraps `NSWorkspace.setIcon(_:forFile:options:)`, which writes
//! an `Icon\r` resource file inside the folder and sets the `kHasCustomIcon`
//! flag in `com.apple.FinderInfo`. Finder then renders the icon in every
//! view — sidebar, column, list, icon, and Get Info preview — exactly like
//! Synology Drive paints its "D" logo onto its synced folders.
//!
//! On non-macOS targets every entry point is a no-op so callers don't need
//! their own cfg-gating.

use crate::errors::{AppError, Result};
use std::path::Path;

/// The ICNS payload bundled into the binary. Painted onto every workspace
/// root when [`set`] is called. Regenerate via
/// `bash toolkit/icons/build-workspace-folder-icns.sh`.
pub static WORKSPACE_FOLDER_ICNS: &[u8] = include_bytes!("../../assets/workspace-folder.icns");

/// Returns true when `path` already carries a custom icon. macOS marks this by
/// creating a hidden `Icon\r` (carriage-return) child file; checking for that
/// file is faster and more reliable than parsing the folder's FinderInfo
/// xattr, and matches what Finder itself looks for.
pub fn is_set(path: &Path) -> bool {
    #[cfg(target_os = "macos")]
    {
        path.join("Icon\r").exists()
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = path;
        false
    }
}

/// Paint `icns_bytes` as the custom folder icon for `path`. Idempotent: if
/// the same icon is already set, Finder is a no-op. Returns an error if `path`
/// is not a directory or if the Cocoa call fails.
///
/// On non-macOS targets this is a no-op that returns `Ok(())`.
pub fn set(path: &Path, icns_bytes: &[u8]) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        imp::set_icon(path, Some(icns_bytes))
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (path, icns_bytes);
        Ok(())
    }
}

/// Remove the custom folder icon from `path`. Safe to call when no icon is
/// set. On non-macOS targets this is a no-op that returns `Ok(())`.
pub fn clear(path: &Path) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        imp::set_icon(path, None)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = path;
        Ok(())
    }
}

/// Convenience wrapper: log-and-swallow variant for hot paths (workspace
/// creation, monitor loop) where a painting failure should never break the
/// caller's primary task.
pub fn set_or_log(path: &Path, icns_bytes: &[u8]) {
    if let Err(e) = set(path, icns_bytes) {
        tracing::warn!(
            path = %path.display(),
            error = %e,
            "Failed to paint workspace folder icon; continuing"
        );
    }
}

/// Same idea as [`set_or_log`], for cleanup paths.
pub fn clear_or_log(path: &Path) {
    if let Err(e) = clear(path) {
        tracing::warn!(
            path = %path.display(),
            error = %e,
            "Failed to clear workspace folder icon; continuing"
        );
    }
}

#[cfg(target_os = "macos")]
mod imp {
    use super::{AppError, Path, Result};
    use objc2::ClassType;
    use objc2_app_kit::{NSImage, NSWorkspace, NSWorkspaceIconCreationOptions};
    use objc2_foundation::{NSData, NSString};

    /// Call NSWorkspace.setIcon. Pass `None` to remove the existing icon.
    pub(super) fn set_icon(path: &Path, icns_bytes: Option<&[u8]>) -> Result<()> {
        if !path.is_dir() {
            return Err(AppError::config(format!(
                "set_icon target is not a directory: {}",
                path.display()
            )));
        }

        let path_str = path.to_str().ok_or_else(|| {
            AppError::config(format!(
                "set_icon path is not valid UTF-8: {}",
                path.display()
            ))
        })?;

        // Cocoa calls below mutate filesystem state and use autorelease pools;
        // we run them inside an explicit autoreleasepool so any temporary
        // objects (NSData, NSImage, NSString) drain when the block returns
        // even if the caller is a long-running daemon (e.g. the monitor).
        objc2::rc::autoreleasepool(|_| -> Result<()> {
            let ns_path = NSString::from_str(path_str);

            let image = match icns_bytes {
                Some(bytes) => {
                    let data = NSData::with_bytes(bytes);
                    // NSImage.initWithData: returns nil if the data isn't a
                    // recognized image format. We treat nil as a soft failure.
                    let img = NSImage::initWithData(NSImage::alloc(), &data);
                    match img {
                        Some(i) => Some(i),
                        None => {
                            return Err(AppError::config(
                                "NSImage could not decode workspace-folder ICNS bytes",
                            ));
                        }
                    }
                }
                None => None,
            };

            let workspace = unsafe { NSWorkspace::sharedWorkspace() };
            // setIcon:forFile:options: returns BOOL. We honor `false` as a
            // soft failure so callers see a clear error.
            let ok = unsafe {
                workspace.setIcon_forFile_options(
                    image.as_deref(),
                    &ns_path,
                    NSWorkspaceIconCreationOptions(0),
                )
            };
            if !ok {
                return Err(AppError::config(format!(
                    "NSWorkspace.setIcon returned false for {}",
                    path.display()
                )));
            }
            Ok(())
        })
    }
}

#[cfg(all(test, target_os = "macos"))]
#[path = "folder_icon_tests.rs"]
mod tests;
