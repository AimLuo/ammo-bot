//! Background `markplane serve` process management (macOS / Linux).
//!
//! Re-execs the current binary with `--daemon-child`, locks `serve.pid` for the
//! server lifetime, and exposes start / stop / status / logs.

use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{anyhow, bail, Context};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs::TryLockError;
use std::os::unix::process::CommandExt;

use super::serve_registry;

const LOG_ROTATE_BYTES: u64 = 10 * 1024 * 1024;
const READY_TIMEOUT: Duration = Duration::from_secs(10);
const STOP_GRACE: Duration = Duration::from_secs(10);
const POLL: Duration = Duration::from_millis(100);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PidInfo {
    pub pid: u32,
    pub port: u16,
    pub url: String,
    pub version: String,
    pub started_at: DateTime<Utc>,
}

/// Exclusive lock on `serve.pid`, held for the server process lifetime.
pub struct PidLock {
    file: File,
}

impl PidLock {
    /// Open `serve.pid` and take an exclusive lock. Returns `Ok(None)` if another
    /// markplane serve process already holds the lock.
    pub fn try_acquire(config_dir: &Path) -> anyhow::Result<Option<Self>> {
        fs::create_dir_all(config_dir)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            let _ = fs::DirBuilder::new()
                .mode(0o700)
                .recursive(true)
                .create(config_dir);
        }

        let path = pid_path(config_dir);
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)
            .with_context(|| format!("open {}", path.display()))?;

        match file.try_lock() {
            Ok(()) => Ok(Some(Self { file })),
            Err(TryLockError::WouldBlock) => Ok(None),
            Err(TryLockError::Error(e)) => Err(e).context("lock serve.pid"),
        }
    }

    pub fn write_info(&mut self, info: &PidInfo) -> anyhow::Result<()> {
        let json = serde_json::to_string_pretty(info)?;
        self.file.set_len(0)?;
        self.file.seek(SeekFrom::Start(0))?;
        self.file.write_all(json.as_bytes())?;
        self.file.write_all(b"\n")?;
        self.file.sync_all()?;
        Ok(())
    }

    pub fn clear(&mut self) -> anyhow::Result<()> {
        self.file.set_len(0)?;
        self.file.sync_all()?;
        Ok(())
    }
}

fn pid_path(config_dir: &Path) -> PathBuf {
    config_dir.join("serve.pid")
}

fn log_path(config_dir: &Path) -> PathBuf {
    config_dir.join("serve.log")
}

/// Returns `Some(info)` when the exclusive lock is held and the pid file has content.
pub fn read_running_info(config_dir: &Path) -> anyhow::Result<Option<PidInfo>> {
    if !lock_held(config_dir)? {
        return Ok(None);
    }
    read_pid_info(config_dir)
}

fn lock_held(config_dir: &Path) -> anyhow::Result<bool> {
    let path = pid_path(config_dir);
    if !path.exists() {
        return Ok(false);
    }
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&path)?;
    match file.try_lock_shared() {
        Ok(()) => {
            let _ = file.unlock();
            Ok(false)
        }
        Err(TryLockError::WouldBlock) => Ok(true),
        Err(TryLockError::Error(e)) => Err(e.into()),
    }
}

fn read_pid_info(config_dir: &Path) -> anyhow::Result<Option<PidInfo>> {
    let path = pid_path(config_dir);
    if !path.exists() {
        return Ok(None);
    }
    let mut buf = String::new();
    File::open(&path)?.read_to_string(&mut buf)?;
    let trimmed = buf.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    Ok(Some(serde_json::from_str(trimmed)?))
}

fn rotate_log_if_needed(config_dir: &Path) -> anyhow::Result<()> {
    let path = log_path(config_dir);
    if !path.exists() {
        return Ok(());
    }
    let meta = fs::metadata(&path)?;
    if meta.len() <= LOG_ROTATE_BYTES {
        return Ok(());
    }
    let rotated = config_dir.join("serve.log.1");
    let _ = fs::remove_file(&rotated);
    fs::rename(&path, &rotated)?;
    Ok(())
}

fn tail_log(config_dir: &Path, lines: usize) -> String {
    let path = log_path(config_dir);
    if !path.exists() {
        return String::new();
    }
    let output = Command::new("tail")
        .args(["-n", &lines.to_string()])
        .arg(&path)
        .output();
    match output {
        Ok(o) => String::from_utf8_lossy(&o.stdout).into_owned(),
        Err(_) => fs::read_to_string(&path).unwrap_or_default(),
    }
}

/// Start the server in the background (no tokio runtime in this process).
pub fn start(port: u16, dev: bool) -> anyhow::Result<()> {
    let config_dir = serve_registry::config_dir();
    fs::create_dir_all(&config_dir)?;

    if let Some(info) = read_running_info(&config_dir)? {
        println!(
            "Already running (pid {}, port {}, {})",
            info.pid, info.port, info.url
        );
        return Ok(());
    }
    // Lock held by a starting process with empty pid file — wait briefly.
    if lock_held(&config_dir)? {
        let deadline = Instant::now() + READY_TIMEOUT;
        while Instant::now() < deadline {
            if let Some(info) = read_running_info(&config_dir)? {
                println!(
                    "Already running (pid {}, port {}, {})",
                    info.pid, info.port, info.url
                );
                return Ok(());
            }
            thread::sleep(POLL);
        }
        bail!("serve.pid is locked but server did not become ready");
    }

    rotate_log_if_needed(&config_dir)?;

    let exe = std::env::current_exe().context("current_exe")?;
    let log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path(&config_dir))
        .context("open serve.log")?;
    let log_err = log_file.try_clone()?;

    let home = std::env::var_os("HOME").context("HOME must be set")?;

    let mut command = Command::new(&exe);
    command
        .args(["serve", "--daemon-child", "--port", &port.to_string()])
        .stdin(Stdio::null())
        .stdout(Stdio::from(log_file))
        .stderr(Stdio::from(log_err))
        .current_dir(home)
        .process_group(0);

    if dev {
        command.arg("--dev");
    }

    // Detach from the controlling terminal when possible.
    unsafe {
        command.pre_exec(|| {
            if libc::setsid() == -1 {
                // process_group(0) already isolates from the shell's foreground group.
                let err = std::io::Error::last_os_error();
                if err.raw_os_error() != Some(libc::EPERM) {
                    return Err(err);
                }
            }
            Ok(())
        });
    }

    let mut child = command.spawn().context("spawn daemon child")?;
    let child_pid = child.id();

    let deadline = Instant::now() + READY_TIMEOUT;
    loop {
        if let Some(status) = child.try_wait().context("wait daemon child")? {
            let tail = tail_log(&config_dir, 20);
            eprint!("{tail}");
            if !tail.ends_with('\n') && !tail.is_empty() {
                eprintln!();
            }
            eprintln!(
                "markplane serve failed to start (exit {}). \
                 Try --port <other> or: lsof -iTCP:{port} -sTCP:LISTEN",
                status.code().unwrap_or(-1)
            );
            std::process::exit(status.code().unwrap_or(1));
        }

        if let Some(info) = read_pid_info(&config_dir)?
            && info.pid == child_pid
        {
            println!("Started markplane serve on {}", info.url);
            println!("pid {}  port {}", info.pid, info.port);
            return Ok(());
        }

        if Instant::now() >= deadline {
            let _ = child.kill();
            let tail = tail_log(&config_dir, 20);
            eprint!("{tail}");
            bail!("timed out waiting for serve to become ready");
        }
        thread::sleep(POLL);
    }
}

pub fn stop() -> anyhow::Result<()> {
    let config_dir = serve_registry::config_dir();
    if !lock_held(&config_dir)? {
        // Clear stale content if present.
        if read_pid_info(&config_dir)?.is_some()
            && let Ok(Some(mut lock)) = PidLock::try_acquire(&config_dir)
        {
            let _ = lock.clear();
        }
        println!("Not running");
        return Ok(());
    }

    let info = read_pid_info(&config_dir)?
        .ok_or_else(|| anyhow!("serve is starting; try again in a moment"))?;

    let pid = info.pid as libc::pid_t;
    let term = unsafe { libc::kill(pid, libc::SIGTERM) };
    if term != 0 {
        let err = std::io::Error::last_os_error();
        // ESRCH: already gone
        if err.raw_os_error() != Some(libc::ESRCH) {
            return Err(err).context("SIGTERM");
        }
    }

    let deadline = Instant::now() + STOP_GRACE;
    while Instant::now() < deadline {
        if !lock_held(&config_dir)? {
            println!("Stopped (pid {pid})");
            return Ok(());
        }
        thread::sleep(POLL);
    }

    let _ = unsafe { libc::kill(pid, libc::SIGKILL) };
    let kill_deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < kill_deadline {
        if !lock_held(&config_dir)? {
            println!("Stopped (pid {pid}, forced)");
            return Ok(());
        }
        thread::sleep(POLL);
    }
    bail!("failed to stop serve (pid {pid})");
}

pub fn status() -> anyhow::Result<()> {
    let config_dir = serve_registry::config_dir();
    if !lock_held(&config_dir)? {
        println!("Not running");
        return Ok(());
    }

    let Some(info) = read_pid_info(&config_dir)? else {
        println!("Starting…");
        return Ok(());
    };

    let addr = format!("127.0.0.1:{}", info.port);
    let tcp_ok = TcpStream::connect_timeout(
        &addr
            .to_socket_addrs()?
            .next()
            .unwrap_or(SocketAddr::from(([127, 0, 0, 1], info.port))),
        Duration::from_millis(500),
    )
    .is_ok();

    if !tcp_ok {
        println!("Not running");
        println!(
            "(pid file present for pid {} but port {} is not accepting connections)",
            info.pid, info.port
        );
        return Ok(());
    }

    println!("Running");
    println!("pid: {}", info.pid);
    println!("port: {}", info.port);
    println!("url: {}", info.url);
    println!("version: {}", info.version);
    println!("started_at: {}", info.started_at.to_rfc3339());
    Ok(())
}

pub fn logs(lines: usize, follow: bool) -> anyhow::Result<()> {
    let config_dir = serve_registry::config_dir();
    let path = log_path(&config_dir);
    if !path.exists() {
        bail!("No serve.log yet — start the server with `markplane serve start`");
    }

    let mut args = vec!["-n".to_string(), lines.to_string()];
    if follow {
        args.push("-F".to_string());
    }
    args.push(path.display().to_string());

    let err = Command::new("tail").args(&args).exec();
    Err(anyhow!("failed to exec tail: {err}"))
}

/// Register the current Project with a running server and return the URL to open.
pub fn reuse_running_server(config_dir: &Path) -> anyhow::Result<Option<(PidInfo, String)>> {
    if !lock_held(config_dir)? {
        return Ok(None);
    }

    let deadline = Instant::now() + READY_TIMEOUT;
    let info = loop {
        if let Some(info) = read_pid_info(config_dir)? {
            break info;
        }
        if Instant::now() >= deadline {
            bail!("serve is starting but pid file is still empty");
        }
        thread::sleep(POLL);
    };

    let open_path = match markplane_core::Project::from_current_dir() {
        Ok(project) => {
            let project_dir = project
                .root()
                .parent()
                .unwrap_or(project.root())
                .to_path_buf();
            let entry = serve_registry::register(config_dir, &project_dir)?;
            println!(
                "Registered project '{}' ({}) with running server",
                entry.name,
                project_dir.display()
            );
            format!("/p/{}/", entry.id)
        }
        Err(_) => {
            println!("No Project in current directory — opening landing page on running server");
            "/".to_string()
        }
    };

    let url = format!(
        "{}{}",
        info.url.trim_end_matches('/'),
        open_path
    );
    Ok(Some((info, url)))
}
