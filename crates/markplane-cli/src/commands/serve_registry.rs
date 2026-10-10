//! User-level Project registry (`projects.json` under the markplane config dir).

use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use fs2::FileExt;
use rand::RngExt;
use serde::{Deserialize, Serialize};

/// A registered Project entry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RegistryEntry {
    pub id: String,
    pub name: String,
    pub path: String,
    pub added_at: DateTime<Utc>,
    pub last_opened: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct RegistryFile {
    projects: Vec<RegistryEntry>,
}

/// Resolve `~/.config/markplane` (honours `XDG_CONFIG_HOME`).
pub fn config_dir() -> PathBuf {
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        if !xdg.is_empty() {
            return PathBuf::from(xdg).join("markplane");
        }
    }
    let home = std::env::var_os("HOME").expect("HOME must be set");
    PathBuf::from(home).join(".config/markplane")
}

fn registry_path(config_dir: &Path) -> PathBuf {
    config_dir.join("projects.json")
}

fn lock_path(config_dir: &Path) -> PathBuf {
    config_dir.join("projects.lock")
}

fn with_lock<T>(config_dir: &Path, f: impl FnOnce() -> anyhow::Result<T>) -> anyhow::Result<T> {
    fs::create_dir_all(config_dir)?;
    let lock_file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(lock_path(config_dir))?;
    lock_file.lock_exclusive()?;
    let result = f();
    let _ = lock_file.unlock();
    result
}

fn read_unlocked(config_dir: &Path) -> anyhow::Result<RegistryFile> {
    let path = registry_path(config_dir);
    if !path.exists() {
        return Ok(RegistryFile::default());
    }
    let mut file = File::open(&path)?;
    let mut buf = String::new();
    file.read_to_string(&mut buf)?;
    if buf.trim().is_empty() {
        return Ok(RegistryFile::default());
    }
    Ok(serde_json::from_str(&buf)?)
}

fn write_unlocked(config_dir: &Path, registry: &RegistryFile) -> anyhow::Result<()> {
    fs::create_dir_all(config_dir)?;
    let path = registry_path(config_dir);
    let tmp = config_dir.join(format!(
        "projects.json.tmp.{}",
        std::process::id()
    ));
    {
        let mut file = File::create(&tmp)?;
        let json = serde_json::to_string_pretty(registry)?;
        file.write_all(json.as_bytes())?;
        file.write_all(b"\n")?;
        file.sync_all()?;
    }
    fs::rename(&tmp, &path)?;
    Ok(())
}

/// List all registered Projects (locked read).
pub fn list(config_dir: &Path) -> anyhow::Result<Vec<RegistryEntry>> {
    with_lock(config_dir, || Ok(read_unlocked(config_dir)?.projects))
}

/// Find an entry by id (locked read).
pub fn find(config_dir: &Path, id: &str) -> anyhow::Result<Option<RegistryEntry>> {
    with_lock(config_dir, || {
        Ok(read_unlocked(config_dir)?
            .projects
            .into_iter()
            .find(|p| p.id == id))
    })
}

/// Re-read the registry without holding a long-lived cache (locked).
pub fn find_reread(config_dir: &Path, id: &str) -> anyhow::Result<Option<RegistryEntry>> {
    find(config_dir, id)
}

fn slugify(name: &str) -> String {
    let mut out = String::new();
    let mut prev_dash = false;
    for ch in name.chars() {
        let c = ch.to_ascii_lowercase();
        if c.is_ascii_alphanumeric() {
            out.push(c);
            prev_dash = false;
        } else if !prev_dash && !out.is_empty() {
            out.push('-');
            prev_dash = true;
        }
    }
    let trimmed = out.trim_matches('-').to_string();
    if trimmed.is_empty() {
        "project".to_string()
    } else {
        trimmed
    }
}

fn random_suffix() -> String {
    const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
    let mut rng = rand::rng();
    (0..5)
        .map(|_| CHARSET[rng.random_range(0..CHARSET.len())] as char)
        .collect()
}

fn allocate_id(existing: &[RegistryEntry], dir_name: &str) -> String {
    let base = slugify(dir_name);
    if !existing.iter().any(|e| e.id == base) {
        return base;
    }
    loop {
        let candidate = format!("{base}-{}", random_suffix());
        if !existing.iter().any(|e| e.id == candidate) {
            return candidate;
        }
    }
}

/// Register a Project directory. Path is realpath'd; duplicates reuse the entry.
///
/// Returns the registry entry (existing or newly created). Does not update
/// `lastOpened` — that happens when a Project runtime is established.
pub fn register(config_dir: &Path, project_path: &Path) -> anyhow::Result<RegistryEntry> {
    let canonical = fs::canonicalize(project_path).map_err(|e| {
        anyhow::anyhow!(
            "cannot resolve project path {}: {e}",
            project_path.display()
        )
    })?;
    let markplane = canonical.join(".markplane");
    if !markplane.is_dir() {
        anyhow::bail!(
            "not a Markplane project (missing .markplane/): {}",
            canonical.display()
        );
    }

    with_lock(config_dir, || {
        let mut registry = read_unlocked(config_dir)?;
        let path_str = canonical.to_string_lossy().to_string();
        if let Some(existing) = registry.projects.iter().find(|e| e.path == path_str) {
            return Ok(existing.clone());
        }

        let dir_name = canonical
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("project");
        let now = Utc::now();
        let entry = RegistryEntry {
            id: allocate_id(&registry.projects, dir_name),
            name: dir_name.to_string(),
            path: path_str,
            added_at: now,
            last_opened: now,
        };
        registry.projects.push(entry.clone());
        write_unlocked(config_dir, &registry)?;
        Ok(entry)
    })
}

/// Update `lastOpened` for a Project id (called when its runtime is established).
pub fn touch_last_opened(config_dir: &Path, id: &str) -> anyhow::Result<()> {
    with_lock(config_dir, || {
        let mut registry = read_unlocked(config_dir)?;
        let Some(entry) = registry.projects.iter_mut().find(|e| e.id == id) else {
            anyhow::bail!("project id not in registry: {id}");
        };
        entry.last_opened = Utc::now();
        write_unlocked(config_dir, &registry)?;
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use markplane_core::Project;
    use tempfile::TempDir;

    fn init_project(dir: &Path, name: &str) -> PathBuf {
        let root = dir.join(".markplane");
        Project::init(root, name, "registry test").unwrap();
        dir.to_path_buf()
    }

    #[test]
    fn register_writes_projects_json_under_config_dir() {
        let tmp = TempDir::new().unwrap();
        let config = tmp.path().join("config");
        let project_dir = tmp.path().join("my-app");
        fs::create_dir_all(&project_dir).unwrap();
        init_project(&project_dir, "My App");

        let entry = register(&config, &project_dir).unwrap();
        assert_eq!(entry.id, "my-app");
        assert_eq!(entry.name, "my-app");

        let listed = list(&config).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, "my-app");
        assert!(registry_path(&config).is_file());
    }

    #[test]
    fn register_same_path_via_symlink_keeps_single_entry() {
        let tmp = TempDir::new().unwrap();
        let config = tmp.path().join("config");
        let project_dir = tmp.path().join("shared");
        fs::create_dir_all(&project_dir).unwrap();
        init_project(&project_dir, "Shared");

        let first = register(&config, &project_dir).unwrap();
        let link = tmp.path().join("alias");
        std::os::unix::fs::symlink(&project_dir, &link).unwrap();

        let second = register(&config, &link).unwrap();
        assert_eq!(first.id, second.id);
        assert_eq!(list(&config).unwrap().len(), 1);
    }

    #[test]
    fn register_collision_gets_random_suffix() {
        let tmp = TempDir::new().unwrap();
        let config = tmp.path().join("config");

        let a = tmp.path().join("dup");
        fs::create_dir_all(&a).unwrap();
        init_project(&a, "A");
        let first = register(&config, &a).unwrap();
        assert_eq!(first.id, "dup");

        // Second directory with the same basename lives elsewhere.
        let other_parent = tmp.path().join("other");
        fs::create_dir_all(&other_parent).unwrap();
        let b = other_parent.join("dup");
        fs::create_dir_all(&b).unwrap();
        init_project(&b, "B");
        let second = register(&config, &b).unwrap();
        assert!(second.id.starts_with("dup-"));
        assert_ne!(second.id, first.id);
        assert_eq!(list(&config).unwrap().len(), 2);
    }

    #[test]
    fn touch_last_opened_updates_timestamp() {
        let tmp = TempDir::new().unwrap();
        let config = tmp.path().join("config");
        let project_dir = tmp.path().join("touch-me");
        fs::create_dir_all(&project_dir).unwrap();
        init_project(&project_dir, "Touch");

        let entry = register(&config, &project_dir).unwrap();
        let before = entry.last_opened;
        std::thread::sleep(std::time::Duration::from_millis(5));
        touch_last_opened(&config, &entry.id).unwrap();
        let after = find(&config, &entry.id).unwrap().unwrap().last_opened;
        assert!(after > before);
    }

    #[test]
    fn config_dir_respects_xdg_config_home() {
        let tmp = TempDir::new().unwrap();
        let xdg = tmp.path().join("xdg");
        // Safety: tests run serially within this module for env mutation.
        // SAFETY: confined to this test process; restored below.
        let prev = std::env::var_os("XDG_CONFIG_HOME");
        unsafe { std::env::set_var("XDG_CONFIG_HOME", &xdg) };
        let dir = config_dir();
        match prev {
            Some(v) => unsafe { std::env::set_var("XDG_CONFIG_HOME", v) },
            None => unsafe { std::env::remove_var("XDG_CONFIG_HOME") },
        }
        assert_eq!(dir, xdg.join("markplane"));
    }
}
