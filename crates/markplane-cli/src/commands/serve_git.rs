//! Git helpers for Dispatcher lock keys (common dir + worktree-relative Project path).

use std::path::{Path, PathBuf};
use std::process::Command;

/// Resolved identity of a Project inside a git repository for Dispatcher locking.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DispatcherLockKey {
    /// Absolute path to the repository's git common directory.
    pub common_dir: PathBuf,
    /// Project directory relative to the worktree top (`"."` when at the top).
    pub project_rel: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GitResolveError {
    NotAGitRepository,
    CommandFailed(String),
}

impl std::fmt::Display for GitResolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotAGitRepository => write!(f, "project is not inside a git repository"),
            Self::CommandFailed(msg) => write!(f, "{msg}"),
        }
    }
}

/// Resolve the Dispatcher lock key for a Project directory (parent of `.markplane/`).
pub fn resolve_lock_key(project_dir: &Path) -> Result<DispatcherLockKey, GitResolveError> {
    let common_dir = rev_parse(project_dir, &["--path-format=absolute", "--git-common-dir"])?;
    let toplevel = rev_parse(project_dir, &["--show-toplevel"])?;

    let common_dir = canonicalize_path(&common_dir).unwrap_or(common_dir);
    let toplevel = canonicalize_path(&toplevel).unwrap_or(toplevel);
    let project_dir = canonicalize_path(project_dir).unwrap_or_else(|| project_dir.to_path_buf());

    let project_rel = project_dir
        .strip_prefix(&toplevel)
        .map(|p| {
            if p.as_os_str().is_empty() {
                PathBuf::from(".")
            } else {
                p.to_path_buf()
            }
        })
        .unwrap_or_else(|_| PathBuf::from("."));

    Ok(DispatcherLockKey {
        common_dir,
        project_rel,
    })
}

/// Whether `project_dir` is inside a git work tree.
#[cfg_attr(not(test), allow(dead_code))]
pub fn is_git_repository(project_dir: &Path) -> bool {
    resolve_lock_key(project_dir).is_ok()
}

/// Stable lock file stem under `<common_dir>/markplane/dispatcher/`.
///
/// Only the relative Project path is encoded: the directory already scopes by common dir.
pub fn lock_stem(project_rel: &Path) -> String {
    let s = if project_rel.as_os_str().is_empty() || project_rel == Path::new(".") {
        ".".to_string()
    } else {
        project_rel.to_string_lossy().replace('\\', "/")
    };
    s.bytes().map(|b| format!("{b:02x}")).collect()
}

pub fn lock_paths(key: &DispatcherLockKey) -> (PathBuf, PathBuf) {
    let dir = key.common_dir.join("markplane").join("dispatcher");
    let stem = lock_stem(&key.project_rel);
    (dir.join(format!("{stem}.lock")), dir.join(format!("{stem}.json")))
}

fn rev_parse(cwd: &Path, args: &[&str]) -> Result<PathBuf, GitResolveError> {
    let mut cmd = Command::new("git");
    cmd.arg("-C").arg(cwd).arg("rev-parse");
    for a in args {
        cmd.arg(a);
    }
    let output = cmd.output().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            GitResolveError::CommandFailed("git is not available".into())
        } else {
            GitResolveError::CommandFailed(format!("failed to run git: {e}"))
        }
    })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("not a git repository") {
            return Err(GitResolveError::NotAGitRepository);
        }
        return Err(GitResolveError::NotAGitRepository);
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let trimmed = stdout.trim();
    if trimmed.is_empty() {
        return Err(GitResolveError::NotAGitRepository);
    }
    Ok(PathBuf::from(trimmed))
}

fn canonicalize_path(path: &Path) -> Option<PathBuf> {
    path.canonicalize().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process::Command;
    use tempfile::TempDir;

    fn init_repo(dir: &Path) {
        let status = Command::new("git")
            .args(["init"])
            .current_dir(dir)
            .status()
            .unwrap();
        assert!(status.success());
        // Identity for any later commits in other tests.
        let _ = Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(dir)
            .status();
        let _ = Command::new("git")
            .args(["config", "user.name", "Test"])
            .current_dir(dir)
            .status();
    }

    #[test]
    fn resolve_lock_key_at_repo_root() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join("repo");
        fs::create_dir_all(&root).unwrap();
        init_repo(&root);

        let key = resolve_lock_key(&root).unwrap();
        assert_eq!(key.project_rel, PathBuf::from("."));
        assert!(key.common_dir.ends_with(".git") || key.common_dir.join("HEAD").exists());
        assert!(is_git_repository(&root));
    }

    #[test]
    fn resolve_lock_key_nested_project() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join("repo");
        let nested = root.join("apps").join("api");
        fs::create_dir_all(&nested).unwrap();
        init_repo(&root);

        let key = resolve_lock_key(&nested).unwrap();
        assert_eq!(key.project_rel, PathBuf::from("apps/api"));
        let root_key = resolve_lock_key(&root).unwrap();
        assert_eq!(key.common_dir, root_key.common_dir);
        assert_ne!(key.project_rel, root_key.project_rel);
    }

    #[test]
    fn not_a_git_repo_errors() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("plain");
        fs::create_dir_all(&dir).unwrap();
        assert_eq!(
            resolve_lock_key(&dir).unwrap_err(),
            GitResolveError::NotAGitRepository
        );
        assert!(!is_git_repository(&dir));
    }

    #[test]
    fn lock_stem_encodes_relative_path() {
        assert_eq!(lock_stem(Path::new(".")), "2e");
        assert_eq!(
            lock_stem(Path::new("apps/api")),
            "617070732f617069"
        );
    }

    #[test]
    fn worktree_shares_common_dir_and_rel_path() {
        let tmp = TempDir::new().unwrap();
        let main = tmp.path().join("main");
        fs::create_dir_all(&main).unwrap();
        init_repo(&main);
        fs::write(main.join("README"), "x").unwrap();
        let _ = Command::new("git")
            .args(["add", "README"])
            .current_dir(&main)
            .status();
        let _ = Command::new("git")
            .args(["commit", "-m", "init"])
            .current_dir(&main)
            .status();

        let wt = tmp.path().join("wt");
        let status = Command::new("git")
            .args(["worktree", "add", wt.to_str().unwrap(), "-b", "side"])
            .current_dir(&main)
            .status()
            .unwrap();
        assert!(status.success());

        let main_key = resolve_lock_key(&main).unwrap();
        let wt_key = resolve_lock_key(&wt).unwrap();
        assert_eq!(main_key.common_dir, wt_key.common_dir);
        assert_eq!(main_key.project_rel, wt_key.project_rel);
        assert_eq!(main_key, wt_key);
    }
}
