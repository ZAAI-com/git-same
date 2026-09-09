//! Where a helper may be installed from, and who owns it.
//!
//! Classification looks only at the fully resolved path of the invoking
//! executable. Nothing here searches `PATH` or guesses install locations:
//! whatever gets recorded is later executed by launchd at every login.

use super::record::{InstallRecord, OwnerKind};
use crate::errors::MonitorAgentError;
use std::path::{Component, Path, PathBuf};

/// Bundle identifier of `Git-Same.app`.
pub const APP_BUNDLE_ID: &str = "com.zaai.git-same";
/// `CFBundleExecutable` of `Git-Same.app`. The LaunchAgent of an app-owned
/// installation execs this file in place: macOS TCC attributes a
/// launchd-spawned process to the bundle only when the executable is the
/// bundle's main one, so a copy elsewhere would be a separate identity that
/// a Full Disk Access grant for "Git-Same" never reaches.
pub const APP_MAIN_EXECUTABLE: &str = "git-same-app";
const FORMULA_NAME: &str = "git-same-cli";

/// A candidate helper source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HelperSource {
    pub owner_kind: OwnerKind,
    /// Stable app bundle or CLI installation path.
    pub owner_path: PathBuf,
    /// Stable path recorded for future updates. For an app-owned source this
    /// is also the program launchd runs; see [`HelperSource::in_place`].
    pub source_binary: PathBuf,
    /// File to verify (and, for a CLI owner, copy) right now. Differs from
    /// `source_binary` only while a cask installer runs from its staging
    /// directory.
    pub copy_from: PathBuf,
}

impl HelperSource {
    /// Whether the installation runs the source where it already lives
    /// instead of copying it into the managed root.
    ///
    /// True for every app bundle: only the bundle's own main executable
    /// carries the bundle's TCC identity. False for CLI installs, whose
    /// binary may be upgraded or removed underneath the service, so the
    /// managed copy is what makes the service survive `brew upgrade`.
    pub fn in_place(&self) -> bool {
        self.owner_kind.is_app()
    }

    /// The program the LaunchAgent execs for this source.
    pub fn program(&self, managed_helper: &Path) -> PathBuf {
        if self.in_place() {
            self.source_binary.clone()
        } else {
            managed_helper.to_path_buf()
        }
    }
}

/// The program a LaunchAgent execs for an installation owned by `owner_kind`
/// at `owner_path`. Derived, never persisted, so an agent installed by an
/// older build that still points at a copied helper is re-rendered onto the
/// bundle executable the next time anything inspects or repairs it.
pub fn program_for(owner_kind: OwnerKind, owner_path: &Path, managed_helper: &Path) -> PathBuf {
    if owner_kind.is_app() {
        app_main_executable(owner_path)
    } else {
        managed_helper.to_path_buf()
    }
}

/// `<bundle>/Contents/MacOS/git-same-app`.
pub fn app_main_executable(bundle: &Path) -> PathBuf {
    bundle
        .join("Contents")
        .join("MacOS")
        .join(APP_MAIN_EXECUTABLE)
}

/// Resolves and classifies the running executable.
pub fn invoking_source() -> Result<HelperSource, MonitorAgentError> {
    let invoked = std::env::current_exe()
        .map_err(|e| MonitorAgentError::MissingSource(format!("current executable: {e}")))?;
    // `current_exe` may be a symlink (Homebrew links, ~/.cargo/bin aliases).
    let real = std::fs::canonicalize(&invoked).unwrap_or(invoked);
    Ok(classify(&real))
}

/// Classifies an already canonicalized executable path.
pub fn classify(real_path: &Path) -> HelperSource {
    if let Some(bundle) = enclosing_app_bundle(real_path) {
        let executable = app_main_executable(&bundle);
        return HelperSource {
            owner_kind: OwnerKind::App,
            owner_path: bundle,
            source_binary: executable.clone(),
            copy_from: executable,
        };
    }
    let stable = homebrew_opt_path(real_path).unwrap_or_else(|| real_path.to_path_buf());
    HelperSource {
        owner_kind: OwnerKind::Cli,
        owner_path: stable.clone(),
        source_binary: stable,
        copy_from: real_path.to_path_buf(),
    }
}

/// Source for a cask installation.
///
/// `staged_cli` is the installer itself: `Contents/Helpers/git-same` inside
/// the bundle Homebrew has staged but not yet moved. What the agent runs is
/// the bundle's main executable, so the file verified now is that executable
/// in the same staged bundle, and the path recorded is where Homebrew is
/// about to put it.
pub fn cask_source(staged_cli: &Path, final_app_path: &Path) -> HelperSource {
    let staged_bundle = enclosing_bundle_dir(staged_cli);
    HelperSource {
        owner_kind: OwnerKind::HomebrewCask,
        owner_path: final_app_path.to_path_buf(),
        source_binary: app_main_executable(final_app_path),
        copy_from: staged_bundle
            .map(|bundle| app_main_executable(&bundle))
            .unwrap_or_else(|| staged_cli.to_path_buf()),
    }
}

/// The `<name>.app` directory two levels above `Contents/<dir>/<file>`.
/// Unlike [`enclosing_app_bundle`] this does not read `Info.plist`: the
/// cask installer already knows which bundle it is running from, and the
/// staged bundle may not be fully assembled.
fn enclosing_bundle_dir(executable: &Path) -> Option<PathBuf> {
    let bundle = executable.parent()?.parent()?.parent()?;
    (bundle.extension().is_some_and(|ext| ext == "app")).then(|| bundle.to_path_buf())
}

/// The `Git-Same.app` containing `path`, verified through its `Info.plist`.
fn enclosing_app_bundle(path: &Path) -> Option<PathBuf> {
    path.ancestors()
        .filter(|ancestor| ancestor.extension().is_some_and(|ext| ext == "app"))
        .find(|bundle| {
            std::fs::read_to_string(bundle.join("Contents").join("Info.plist")).is_ok_and(|plist| {
                plist_bundle_identifier(&plist).as_deref() == Some(APP_BUNDLE_ID)
            })
        })
        .map(Path::to_path_buf)
}

/// Minimal scan for `CFBundleIdentifier`. The bundle's `Info.plist` is
/// generated by this repository and is always XML.
fn plist_bundle_identifier(plist: &str) -> Option<String> {
    let after_key = plist.split("<key>CFBundleIdentifier</key>").nth(1)?;
    let start = after_key.find("<string>")? + "<string>".len();
    let end = after_key[start..].find("</string>")? + start;
    Some(after_key[start..end].trim().to_string())
}

/// Maps `<prefix>/Cellar/git-same-cli/<version>/bin/git-same` to the stable
/// `<prefix>/opt/git-same-cli/bin/git-same`, which survives `brew upgrade`.
/// Returned only when it really resolves to `real_path`.
fn homebrew_opt_path(real_path: &Path) -> Option<PathBuf> {
    let components: Vec<Component> = real_path.components().collect();
    let cellar = components.iter().rposition(|c| c.as_os_str() == "Cellar")?;
    if components.get(cellar + 1)?.as_os_str() != FORMULA_NAME {
        return None;
    }
    let prefix: PathBuf = components[..cellar].iter().collect();
    // Skip "Cellar/<formula>/<version>".
    let inside_keg: PathBuf = components.get(cellar + 3..)?.iter().collect();
    let candidate = prefix.join("opt").join(FORMULA_NAME).join(inside_keg);
    (std::fs::canonicalize(&candidate).ok()? == real_path).then_some(candidate)
}

/// Returns `true` for executables inside a cargo target directory.
///
/// Dev builds live in disposable worktrees and change on every build; they
/// must never be installed automatically over a user's real helper.
pub fn is_dev_build(real_path: &Path) -> bool {
    real_path.ancestors().any(|dir| {
        dir.file_name().is_some_and(|name| name == "target")
            && (dir.join("CACHEDIR.TAG").exists() || dir.join(".rustc_info.json").exists())
    })
}

/// What to do with the helper, given who is asking.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Selection {
    /// The installation is intact and current.
    Keep,
    /// Install or update from this source.
    Install(HelperSource),
    /// Nothing usable is installed and the caller cannot provide a source.
    Unavailable(String),
}

/// Applies the source selection rules for a non-cask caller.
///
/// * `existing`: the current record; `helper_intact`: the helper file exists.
/// * `caller`: the invoking app or CLI, when it could be resolved.
/// * `allow_dev_source`: explicit commands may install a dev build; automatic
///   recovery may not.
pub fn select(
    existing: Option<&InstallRecord>,
    helper_intact: bool,
    caller: Option<&HelperSource>,
    allow_dev_source: bool,
    source_changed: impl Fn(&InstallRecord) -> bool,
) -> Selection {
    let usable = |source: &HelperSource| {
        super::install::is_executable(&source.copy_from)
            && (allow_dev_source || !is_dev_build(&source.copy_from))
    };
    let caller_usable = caller.is_some_and(usable);

    if let Some(record) = existing {
        let recorded_source = recorded_as_source(record);
        // The recorded source gets the same dev-build test as the caller. It
        // was written by an explicit Start, which may point at a cargo
        // `target/` build; automatic recovery must not then reinstall and
        // restart the monitor after every rebuild.
        let recorded_usable = usable(&recorded_source);

        // A standalone owner yields to the app's signed bundled helper.
        if let Some(caller) = caller {
            if record.owner_kind == OwnerKind::Cli && caller.owner_kind.is_app() && caller_usable {
                return Selection::Install(caller.clone());
            }
        }
        if helper_intact {
            // Update only from the owner's own source; a different caller
            // never replaces a healthy helper.
            if source_changed(record) && recorded_usable {
                return Selection::Install(recorded_source);
            }
            return Selection::Keep;
        }
        // Helper missing: repair from the recorded owner when possible.
        if recorded_usable {
            return Selection::Install(recorded_source);
        }
    }

    let Some(caller) = caller else {
        return Selection::Unavailable("the invoking executable is unknown".to_string());
    };
    if caller_usable {
        let mut source = caller.clone();
        // The same bundle the cask installed keeps its cask ownership.
        if let Some(record) = existing {
            if record.owner_kind == OwnerKind::HomebrewCask
                && record.owner_path == caller.owner_path
            {
                source.owner_kind = OwnerKind::HomebrewCask;
            }
        }
        Selection::Install(source)
    } else {
        Selection::Unavailable(dev_build_reason())
    }
}

fn recorded_as_source(record: &InstallRecord) -> HelperSource {
    HelperSource {
        owner_kind: record.owner_kind,
        owner_path: record.owner_path.clone(),
        source_binary: record.source_binary.clone(),
        copy_from: record.source_binary.clone(),
    }
}

fn dev_build_reason() -> String {
    "this is a development build; run 'gisa monitor --start' to install it deliberately".to_string()
}

#[cfg(test)]
#[path = "source_tests.rs"]
mod tests;
