//! OS advisory Dispatcher lock under `<git common dir>/markplane/dispatcher/`.

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs::TryLockError;

use super::serve_git::{self, DispatcherLockKey};

/// Holder metadata written beside the empty lock file (display only).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LockHolder {
    pub pid: u32,
    pub hostname: String,
    pub serve_url: String,
    pub config_dir: String,
    pub project_path: String,
    pub acquired_at: DateTime<Utc>,
}

/// Result of probing lock status without changing it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LockProbe {
    Free,
    HeldByUs,
    HeldByOther(LockHolder),
}

/// An exclusive Dispatcher lock held by this process.
pub struct HeldDispatcherLock {
    #[allow(dead_code)] // held for Drop / OS unlock via File
    file: File,
    pub key: DispatcherLockKey,
    pub lock_path: PathBuf,
    pub holder: LockHolder,
}

impl std::fmt::Debug for HeldDispatcherLock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HeldDispatcherLock")
            .field("key", &self.key)
            .field("lock_path", &self.lock_path)
            .field("holder", &self.holder)
            .finish()
    }
}

#[derive(Debug)]
pub enum AcquireError {
    /// Another process holds the OS lock.
    Contended { holder: Option<LockHolder> },
    Io(String),
}

impl std::fmt::Display for AcquireError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Contended { .. } => write!(f, "dispatcher lock is held elsewhere"),
            Self::Io(msg) => write!(f, "{msg}"),
        }
    }
}

/// Try to acquire the Dispatcher lock for `key` (non-blocking).
///
/// The lock file stays empty and is never deleted. Holder info is written
/// atomically to the sibling `.json`. Rust's `File` opens with `O_CLOEXEC` on
/// Unix so the fd is not inherited by child processes.
pub fn try_acquire(
    key: &DispatcherLockKey,
    holder: LockHolder,
) -> Result<HeldDispatcherLock, AcquireError> {
    let (lock_path, json_path) = serve_git::lock_paths(key);
    let dir = lock_path.parent().unwrap();
    fs::create_dir_all(dir).map_err(|e| AcquireError::Io(format!("create lock dir: {e}")))?;

    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&lock_path)
        .map_err(|e| AcquireError::Io(format!("open {}: {e}", lock_path.display())))?;

    match file.try_lock() {
        Ok(()) => {
            // Keep the lock file empty.
            let _ = file.set_len(0);
            write_holder_atomic(&json_path, &holder)
                .map_err(|e| AcquireError::Io(format!("write holder: {e}")))?;
            Ok(HeldDispatcherLock {
                file,
                key: key.clone(),
                lock_path,
                holder,
            })
        }
        Err(TryLockError::WouldBlock) => {
            let holder = read_holder(&json_path).ok().flatten();
            Err(AcquireError::Contended { holder })
        }
        Err(TryLockError::Error(e)) => Err(AcquireError::Io(format!("lock: {e}"))),
    }
}

/// Probe whether the lock is free / held by us / held by another process.
///
/// Uses a shared lock attempt so the exclusive holder's state is unchanged.
pub fn probe(key: &DispatcherLockKey, held_by_us: bool) -> Result<LockProbe, String> {
    if held_by_us {
        return Ok(LockProbe::HeldByUs);
    }
    let (lock_path, json_path) = serve_git::lock_paths(key);
    if !lock_path.exists() {
        return Ok(LockProbe::Free);
    }
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&lock_path)
        .map_err(|e| format!("open {}: {e}", lock_path.display()))?;
    match file.try_lock_shared() {
        Ok(()) => {
            let _ = file.unlock();
            Ok(LockProbe::Free)
        }
        Err(TryLockError::WouldBlock) => {
            let holder = read_holder(&json_path)
                .ok()
                .flatten()
                .unwrap_or_else(|| LockHolder {
                    pid: 0,
                    hostname: "unknown".into(),
                    serve_url: String::new(),
                    config_dir: String::new(),
                    project_path: String::new(),
                    acquired_at: Utc::now(),
                });
            Ok(LockProbe::HeldByOther(holder))
        }
        Err(TryLockError::Error(e)) => Err(format!("probe lock: {e}")),
    }
}

fn write_holder_atomic(path: &Path, holder: &LockHolder) -> std::io::Result<()> {
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    let tmp = dir.join(format!(
        ".{}.tmp",
        path.file_name().and_then(|s| s.to_str()).unwrap_or("holder")
    ));
    {
        let mut f = File::create(&tmp)?;
        let json = serde_json::to_string_pretty(holder)
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        f.write_all(json.as_bytes())?;
        f.write_all(b"\n")?;
        f.sync_all()?;
    }
    fs::rename(&tmp, path)?;
    Ok(())
}

fn read_holder(path: &Path) -> std::io::Result<Option<LockHolder>> {
    if !path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(path)?;
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    Ok(Some(serde_json::from_str(trimmed)?))
}

/// Whether `agent_bin` resolves to an executable (absolute/relative path or PATH entry).
pub fn agent_available(agent_bin: &Path) -> bool {
    // Path with a directory component (absolute or relative) — check that path.
    if agent_bin.is_absolute() || agent_bin.components().count() > 1 {
        return is_executable(agent_bin);
    }
    // Bare command name: search PATH.
    let name = agent_bin.as_os_str();
    let Ok(path_var) = std::env::var("PATH") else {
        return false;
    };
    for dir in std::env::split_paths(&path_var) {
        let candidate = dir.join(name);
        if is_executable(&candidate) {
            return true;
        }
    }
    false
}

fn is_executable(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        path.metadata()
            .map(|m| m.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
    }
    #[cfg(not(unix))]
    {
        true
    }
}

pub fn local_hostname() -> String {
    std::env::var("HOSTNAME")
        .or_else(|_| std::env::var("COMPUTERNAME"))
        .ok()
        .filter(|s| !s.is_empty())
        .or_else(|| {
            std::process::Command::new("hostname")
                .output()
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        })
        .unwrap_or_else(|| "unknown".into())
}

/// Human-readable conflict message including optional "on another machine" note.
pub fn locked_message(holder: &LockHolder, local_hostname: &str) -> String {
    let other_machine = holder.hostname != local_hostname
        && !holder.hostname.is_empty()
        && holder.hostname != "unknown";
    let machine_note = if other_machine {
        " (on another machine)"
    } else {
        ""
    };
    format!(
        "This repository's Dispatcher is held by {}{} pid {} ({} , Project {}), acquired at {}. Turn off Scan there or stop that serve first.",
        holder.hostname,
        machine_note,
        holder.pid,
        holder.serve_url,
        holder.project_path,
        holder.acquired_at.to_rfc3339()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;
    use tempfile::TempDir;

    fn sample_key(tmp: &TempDir) -> DispatcherLockKey {
        let common = tmp.path().join("git-common");
        fs::create_dir_all(&common).unwrap();
        DispatcherLockKey {
            common_dir: common,
            project_rel: PathBuf::from("."),
        }
    }

    fn sample_holder(path: &str) -> LockHolder {
        LockHolder {
            pid: 12345,
            hostname: "host-a".into(),
            serve_url: "http://localhost:4200".into(),
            config_dir: "/tmp/cfg".into(),
            project_path: path.into(),
            acquired_at: Utc::now(),
        }
    }

    #[test]
    fn try_acquire_and_probe_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let key = sample_key(&tmp);
        let holder = sample_holder("/proj");

        assert_eq!(probe(&key, false).unwrap(), LockProbe::Free);

        let held = try_acquire(&key, holder.clone()).unwrap();
        assert_eq!(probe(&key, true).unwrap(), LockProbe::HeldByUs);
        match probe(&key, false).unwrap() {
            LockProbe::HeldByOther(h) => {
                assert_eq!(h.pid, 12345);
                assert_eq!(h.hostname, "host-a");
            }
            other => panic!("expected HeldByOther, got {other:?}"),
        }

        let (lock_path, json_path) = serve_git::lock_paths(&key);
        assert_eq!(fs::read_to_string(&lock_path).unwrap(), "");
        let disk: LockHolder =
            serde_json::from_str(&fs::read_to_string(&json_path).unwrap()).unwrap();
        assert_eq!(disk.pid, holder.pid);

        drop(held);
        assert_eq!(probe(&key, false).unwrap(), LockProbe::Free);
        // Lock file is not reclaimed.
        assert!(lock_path.exists());
    }

    #[test]
    fn second_acquire_is_contended() {
        let tmp = TempDir::new().unwrap();
        let key = sample_key(&tmp);
        let _held = try_acquire(&key, sample_holder("/a")).unwrap();
        match try_acquire(&key, sample_holder("/b")).unwrap_err() {
            AcquireError::Contended { holder } => {
                assert_eq!(holder.unwrap().project_path, "/a");
            }
            other => panic!("unexpected {other}"),
        }
    }

    #[test]
    fn agent_available_checks_injected_path() {
        let tmp = TempDir::new().unwrap();
        let bin = tmp.path().join("fake-agent");
        fs::write(&bin, "#!/bin/sh\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&bin, fs::Permissions::from_mode(0o755)).unwrap();
        }
        assert!(agent_available(&bin));
        assert!(!agent_available(&tmp.path().join("missing-agent")));
    }

    #[test]
    fn locked_message_notes_other_machine() {
        let holder = LockHolder {
            pid: 1,
            hostname: "remote".into(),
            serve_url: "http://x:1".into(),
            config_dir: "/c".into(),
            project_path: "/p".into(),
            acquired_at: Utc::now(),
        };
        let msg = locked_message(&holder, "local");
        assert!(msg.contains("on another machine"));
        assert!(msg.contains("remote"));
    }

    #[test]
    fn cloexec_is_set_on_unix() {
        #[cfg(unix)]
        {
            use std::os::unix::io::AsRawFd;
            let tmp = TempDir::new().unwrap();
            let key = sample_key(&tmp);
            let held = try_acquire(&key, sample_holder("/p")).unwrap();
            let flags = unsafe { libc::fcntl(held.file.as_raw_fd(), libc::F_GETFD) };
            assert!(flags >= 0);
            assert_ne!(flags & libc::FD_CLOEXEC, 0);
        }
    }

    #[test]
    fn git_is_available_for_lock_key_tests() {
        // Sanity: git exists in CI/dev so other modules' tests can run.
        assert!(Command::new("git").arg("--version").status().unwrap().success());
    }
}
