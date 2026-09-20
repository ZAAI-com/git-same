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
}

/// A verified helper copy waiting to be activated.
#[derive(Debug)]
pub struct Staged {
    pub source: HelperSource,
    pub sha256: String,
    temp_helper: PathBuf,
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

    /// Copies and verifies the source without touching the live installation.
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
                temp_helper,
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

    fn has_app_group(&self, path: &Path) -> Result<bool, MonitorAgentError> {
        let arg = path.display().to_string();
        let output = self.system.run(
            CODESIGN,
            &["-d", "--entitlements", "-", "--xml", &arg],
            CODESIGN_TIMEOUT,
        )?;
        Ok(output.stdout.contains(APP_GROUP_ID))
    }

    /// Saves rollback copies and the transaction record, then replaces the
    /// helper and the plist atomically.
    pub fn activate(&self, staged: &Staged, plist: &str) -> Result<(), MonitorAgentError> {
        let transaction = Transaction {
            had_helper: self.paths.helper.exists(),
            had_plist: self.paths.launch_agent.exists(),
            had_record: self.paths.install_record.exists(),
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

        std::fs::rename(&staged.temp_helper, &self.paths.helper)
            .map_err(|e| io("Failed to activate the new helper", e))?;
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
        self.cleanup();
        Ok(())
    }

    /// Restores the files that were live before [`Self::activate`].
    pub fn rollback(&self) -> Result<(), MonitorAgentError> {
        let Some(transaction) = self.read_transaction() else {
            self.cleanup();
            return Ok(());
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
        restore(
            transaction.had_helper,
            self.helper_backup(),
            &self.paths.helper,
        );
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
            self.cleanup();
            Ok(())
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
        self.rollback()?;
        Ok(true)
    }

    fn read_transaction(&self) -> Option<Transaction> {
        let content = std::fs::read(&self.paths.transaction_record).ok()?;
        // An unreadable record means the crash happened while writing it,
        // i.e. before any live file was replaced.
        serde_json::from_slice(&content).ok()
    }

    fn cleanup(&self) {
        for path in [
            self.paths.transaction_record.clone(),
            self.staged_helper(),
            self.helper_backup(),
            self.plist_backup(),
            self.record_backup(),
        ] {
            let _ = std::fs::remove_file(path);
        }
    }

    /// Removes the managed payload. Locks, logs, and configuration stay.
    pub fn remove_installation(&self) -> Result<(), MonitorAgentError> {
        self.cleanup();
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
