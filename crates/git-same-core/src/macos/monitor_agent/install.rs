//! Transactional installation of the managed helper and its LaunchAgent.
//!
//! Nothing live is touched until the new helper is staged and verified.
//! Before active files are replaced, rollback copies and a small transaction
//! record are written; if the process dies in between, the next operation
//! finds the record and restores the previous installation first.

use super::context::MonitorAgentPaths;
use super::control_lock::create_private_dir;
use super::record::{file_stamp, sha256_file, InstallRecord};
use super::source::HelperSource;
use super::system::System;
use crate::errors::MonitorAgentError;
use crate::fsutil::atomic_write;
use crate::ipc::APP_GROUP_ID;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Duration;

const CODESIGN: &str = "/usr/bin/codesign";
const CODESIGN_TIMEOUT: Duration = Duration::from_secs(30);

/// Written before active files are replaced; removed on commit or rollback.
#[derive(Debug, Serialize, Deserialize)]
struct Transaction {
    had_helper: bool,
    had_plist: bool,
    had_record: bool,
    /// The activation ran the source in place and never touched the managed
    /// helper, so rollback must leave that file exactly as it found it.
    /// Absent in records written before in-place installs existed, where
    /// every activation replaced the helper.
    #[serde(default)]
    in_place: bool,
}

/// A verified source waiting to be activated.
#[derive(Debug)]
pub struct Staged {
    pub source: HelperSource,
    pub sha256: String,
    /// The staged copy to move into place, or `None` for an in-place
    /// installation, which runs the source where it already lives.
    temp_helper: Option<PathBuf>,
}

pub struct Installer<'a> {
    system: &'a dyn System,
    paths: &'a MonitorAgentPaths,
}

impl<'a> Installer<'a> {
    pub fn new(system: &'a dyn System, paths: &'a MonitorAgentPaths) -> Self {
        Self { system, paths }
    }

    fn staged_helper(&self) -> PathBuf {
        self.paths.managed_root.join("git-same.new")
    }
    fn helper_backup(&self) -> PathBuf {
        self.paths.managed_root.join("git-same.rollback")
    }
    fn plist_backup(&self) -> PathBuf {
        self.paths.managed_root.join("launch-agent.rollback.plist")
    }
    fn record_backup(&self) -> PathBuf {
        self.paths.managed_root.join("install.rollback.json")
    }

    /// Verifies the source, and copies it unless the installation runs it in
    /// place. Nothing live is touched either way.
    pub fn stage(&self, source: HelperSource) -> Result<Staged, MonitorAgentError> {
        let from = &source.copy_from;
        if !from.is_file() {
            return Err(MonitorAgentError::MissingSource(format!(
                "'{}' does not exist",
                from.display()
            )));
        }
        if !is_executable(from) {
            return Err(invalid(from, "it is not executable"));
        }
        create_private_dir(&self.paths.managed_root)?;

        // An in-place installation has nothing to copy or verify a copy of:
        // launchd execs the source itself. Its hash still goes into the
        // record so a later app upgrade is detected as a changed source.
        if source.in_place() {
            let sha256 = sha256_file(from)
                .map_err(|e| MonitorAgentError::io("Failed to hash the monitor program", e))?;
            return Ok(Staged {
                source,
                sha256,
                temp_helper: None,
            });
        }

        let temp_helper = self.staged_helper();
        let _ = std::fs::remove_file(&temp_helper);
        let staged = (|| {
            self.system
                .copy_executable(from, &temp_helper)
                .map_err(|e| MonitorAgentError::io("Failed to copy the monitor helper", e))?;
            let expected = sha256_file(from)
                .map_err(|e| MonitorAgentError::io("Failed to hash the helper source", e))?;
            let actual = sha256_file(&temp_helper)
                .map_err(|e| MonitorAgentError::io("Failed to hash the copied helper", e))?;
            if expected != actual {
                return Err(invalid(&temp_helper, "the copy does not match its source"));
            }
            self.verify_signature(from, &temp_helper)?;
            Ok(expected)
        })();
        match staged {
            Ok(sha256) => Ok(Staged {
                source,
                sha256,
                temp_helper: Some(temp_helper),
            }),
            Err(e) => {
                let _ = std::fs::remove_file(&temp_helper);
                Err(e)
            }
        }
    }

    /// When the source carries a real (team) signature, the copy must verify
    /// and keep the same team and the app-group entitlement. Unsigned and
    /// ad-hoc builds (cargo) are accepted as they are.
    fn verify_signature(&self, source: &Path, copy: &Path) -> Result<(), MonitorAgentError> {
        let Some(team) = self.signing_team(source)? else {
            return Ok(());
        };
        let copy_arg = copy.display().to_string();
        let verify = self.system.run(
            CODESIGN,
            &["--verify", "--strict", &copy_arg],
            CODESIGN_TIMEOUT,
        )?;
        if !verify.success() {
            return Err(invalid(copy, "its code signature did not survive the copy"));
        }
        if self.signing_team(copy)?.as_deref() != Some(team.as_str()) {
            return Err(invalid(
                copy,
                "its signing identity changed during the copy",
            ));
        }
        if self.has_app_group(source)? && !self.has_app_group(copy)? {
            return Err(invalid(copy, "it lost the app-group entitlement"));
        }
        Ok(())
    }

    fn signing_team(&self, path: &Path) -> Result<Option<String>, MonitorAgentError> {
        let arg = path.display().to_string();
        let output = self
            .system
            .run(CODESIGN, &["-dv", "--verbose=2", &arg], CODESIGN_TIMEOUT)?;
        if !output.success() {
            return Ok(None);
        }
        // codesign prints its description on stderr.
        Ok(output
            .stderr
            .lines()
            .filter_map(|line| line.strip_prefix("TeamIdentifier="))
            .map(str::trim)
            .find(|team| !team.is_empty() && *team != "not set")
            .map(str::to_string))
    }

    /// Only ever asked about a binary already known to carry a team
    /// signature, so a failed `codesign` is a real problem and not "no
    /// entitlements". Reporting `false` there would silently skip the
    /// app-group check and ship a helper that cannot write the group
    /// container. Which stream carries the entitlements depends on the
    /// `codesign` version, so both are searched.
    fn has_app_group(&self, path: &Path) -> Result<bool, MonitorAgentError> {
        let arg = path.display().to_string();
        let output = self.system.run(
            CODESIGN,
            &["-d", "--entitlements", "-", "--xml", &arg],
            CODESIGN_TIMEOUT,
        )?;
        if !output.success() {
            return Err(invalid(
                path,
                "its entitlements could not be read by codesign",
            ));
        }
        Ok(output.stdout.contains(APP_GROUP_ID) || output.stderr.contains(APP_GROUP_ID))
    }

    /// Saves rollback copies and the transaction record, then replaces the
    /// helper and the plist atomically.
    pub fn activate(&self, staged: &Staged, plist: &str) -> Result<(), MonitorAgentError> {
        let in_place = staged.temp_helper.is_none();
        let transaction = Transaction {
            had_helper: !in_place && self.paths.helper.exists(),
            had_plist: self.paths.launch_agent.exists(),
            had_record: self.paths.install_record.exists(),
            in_place,
        };
        let io = |context: &str, e: std::io::Error| MonitorAgentError::io(context.to_string(), e);

        if transaction.had_helper {
            self.system
                .copy_executable(&self.paths.helper, &self.helper_backup())
                .map_err(|e| io("Failed to back up the current helper", e))?;
        }
        if transaction.had_plist {
            std::fs::copy(&self.paths.launch_agent, self.plist_backup())
                .map_err(|e| io("Failed to back up the LaunchAgent", e))?;
        }
        if transaction.had_record {
            std::fs::copy(&self.paths.install_record, self.record_backup())
                .map_err(|e| io("Failed to back up the install record", e))?;
        }
        let json = serde_json::to_vec(&transaction)
            .map_err(|e| io("Failed to encode the transaction record", e.into()))?;
        atomic_write(&self.paths.transaction_record, &json, Some(0o600))
            .map_err(|e| io("Failed to write the transaction record", e))?;

        if let Some(temp_helper) = &staged.temp_helper {
            std::fs::rename(temp_helper, &self.paths.helper)
                .map_err(|e| io("Failed to activate the new helper", e))?;
        }
        atomic_write(&self.paths.launch_agent, plist.as_bytes(), Some(0o644))
            .map_err(|e| io("Failed to write the LaunchAgent", e))?;
        Ok(())
    }

    /// Records the new installation and removes transaction leftovers.
    pub fn commit(&self, staged: &Staged, version: &str) -> Result<(), MonitorAgentError> {
        InstallRecord {
            schema_version: InstallRecord::SCHEMA_VERSION,
            owner_kind: staged.source.owner_kind,
            owner_path: staged.source.owner_path.clone(),
            source_binary: staged.source.source_binary.clone(),
            binary_version: version.to_string(),
            binary_sha256: staged.sha256.clone(),
            installed_at: chrono::Utc::now().to_rfc3339(),
            source_stamp: file_stamp(&staged.source.copy_from),
        }
        .save(&self.paths.install_record)?;
        self.cleanup()
    }

    /// Restores the files that were live before [`Self::activate`].
    pub fn rollback(&self) -> Result<(), MonitorAgentError> {
        let Some(transaction) = self.read_transaction() else {
            return self.cleanup();
        };
        let mut failures = Vec::new();
        let mut restore = |had: bool, backup: PathBuf, target: &Path| {
            let result = if had {
                if backup.exists() {
                    std::fs::rename(&backup, target)
                } else {
                    // The backup was already restored by an earlier attempt.
                    Ok(())
                }
            } else {
                match std::fs::remove_file(target) {
                    Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e),
                    _ => Ok(()),
                }
            };
            if let Err(e) = result {
                failures.push(format!("{}: {e}", target.display()));
            }
        };
        if !transaction.in_place {
            restore(
                transaction.had_helper,
                self.helper_backup(),
                &self.paths.helper,
            );
        }
        restore(
            transaction.had_plist,
            self.plist_backup(),
            &self.paths.launch_agent,
        );
        restore(
            transaction.had_record,
            self.record_backup(),
            &self.paths.install_record,
        );
        if failures.is_empty() {
            self.cleanup()
        } else {
            Err(MonitorAgentError::Transaction {
                original: "could not restore the previous installation".to_string(),
                rollback: Some(failures.join("; ")),
            })
        }
    }

    /// Finishes a transaction that a crashed operation left behind.
    /// Returns `true` when something was recovered.
    pub fn recover_interrupted(&self) -> Result<bool, MonitorAgentError> {
        if !self.paths.transaction_record.exists() {
            let _ = std::fs::remove_file(self.staged_helper());
            return Ok(false);
        }
        let transaction =
            self.read_transaction()
                .ok_or_else(|| MonitorAgentError::Transaction {
                    original: "transaction marker is unreadable".to_string(),
                    rollback: None,
                })?;
        if self.commit_has_landed(&transaction) {
            self.cleanup()?;
            return Ok(true);
        }
        self.rollback()?;
        Ok(true)
    }

    fn read_transaction(&self) -> Option<Transaction> {
        let content = std::fs::read(&self.paths.transaction_record).ok()?;
        // An unreadable record means the crash happened while writing it,
        // i.e. before any live file was replaced.
        serde_json::from_slice(&content).ok()
    }

    /// A changed/new install record is the commit point. This makes recovery
    /// safe when cleanup removed some backups but could not remove the marker.
    fn commit_has_landed(&self, transaction: &Transaction) -> bool {
        let Ok(live) = std::fs::read(&self.paths.install_record) else {
            return false;
        };
        if !transaction.had_record {
            return true;
        }
        match std::fs::read(self.record_backup()) {
            Ok(previous) => live != previous,
            // Backups are removed only during post-commit cleanup. If it is
            // already gone while the marker remains, the commit landed.
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => true,
            Err(_) => false,
        }
    }

    pub fn commit_landed(&self) -> bool {
        self.read_transaction()
            .is_some_and(|transaction| self.commit_has_landed(&transaction))
    }

    fn cleanup(&self) -> Result<(), MonitorAgentError> {
        let mut failures = Vec::new();
        for path in [
            self.staged_helper(),
            self.helper_backup(),
            self.plist_backup(),
            self.record_backup(),
        ] {
            if let Err(e) = std::fs::remove_file(&path) {
                if e.kind() != std::io::ErrorKind::NotFound {
                    failures.push(format!("{}: {e}", path.display()));
                }
            }
        }
        // The marker is removed last and only if all other cleanup succeeded.
        // As long as it exists, recovery can distinguish commit completion
        // from interrupted activation using the live-vs-backup record.
        if failures.is_empty() {
            if let Err(e) = std::fs::remove_file(&self.paths.transaction_record) {
                if e.kind() != std::io::ErrorKind::NotFound {
                    failures.push(format!("{}: {e}", self.paths.transaction_record.display()));
                }
            }
        }
        if failures.is_empty() {
            Ok(())
        } else {
            Err(MonitorAgentError::Lifecycle {
                operation: "transaction cleanup",
                failures,
            })
        }
    }

    /// Removes the managed payload. Locks, logs, and configuration stay.
    pub fn remove_installation(&self) -> Result<(), MonitorAgentError> {
        self.cleanup()?;
        for path in [
            &self.paths.helper,
            &self.paths.install_record,
            &self.paths.launch_agent,
        ] {
            match std::fs::remove_file(path) {
                Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
                    return Err(MonitorAgentError::io(
                        format!("Failed to remove '{}'", path.display()),
                        e,
                    ))
                }
                _ => {}
            }
        }
        Ok(())
    }
}

fn invalid(path: &Path, reason: &str) -> MonitorAgentError {
    MonitorAgentError::InvalidExecutable {
        path: path.to_path_buf(),
        reason: reason.to_string(),
    }
}

pub fn is_executable(path: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::metadata(path)
            .map(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
    }
    #[cfg(not(unix))]
    {
        path.is_file()
    }
}

#[cfg(test)]
#[path = "install_tests.rs"]
mod tests;
