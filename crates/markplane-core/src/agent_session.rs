//! Agent session storage paths (`<configDir>/chats/<md5(workspace)>/<id>/`).

use std::path::{Path, PathBuf};

use md5::{Digest, Md5};

/// Resolve agent config dir: `$CURSOR_CONFIG_DIR` → `$XDG_CONFIG_HOME/cursor` → `~/.cursor`.
pub fn agent_config_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("CURSOR_CONFIG_DIR") {
        let p = PathBuf::from(dir);
        if !p.as_os_str().is_empty() {
            return p;
        }
    }
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME")
        && !xdg.is_empty()
    {
        return PathBuf::from(xdg).join("cursor");
    }
    dirs_home()
        .map(|h| h.join(".cursor"))
        .unwrap_or_else(|| PathBuf::from(".cursor"))
}

fn dirs_home() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

/// MD5 hex of the workspace path string (same keying Cursor uses for chats/).
pub fn workspace_chat_slug(workspace: &str) -> String {
    let digest = Md5::digest(workspace.as_bytes());
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

/// Path to `meta.json` for a session under `config_dir`.
pub fn session_meta_path(config_dir: &Path, workspace: &str, session_id: &str) -> PathBuf {
    config_dir
        .join("chats")
        .join(workspace_chat_slug(workspace))
        .join(session_id)
        .join("meta.json")
}

/// Whether `<configDir>/chats/<md5(workspace)>/<id>/meta.json` exists.
pub fn session_meta_exists(config_dir: &Path, workspace: &str, session_id: &str) -> bool {
    session_meta_path(config_dir, workspace, session_id).is_file()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn workspace_slug_is_stable_md5_hex() {
        assert_eq!(
            workspace_chat_slug("/tmp/wt"),
            "2831bf7f754cc7272556897b18d5a829"
        );
    }

    #[test]
    fn session_meta_exists_checks_file() {
        let tmp = TempDir::new().unwrap();
        let workspace = "/repo/.worktrees/TASK-abc12";
        let sid = "sess-1";
        let path = session_meta_path(tmp.path(), workspace, sid);
        assert!(!session_meta_exists(tmp.path(), workspace, sid));
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "{}").unwrap();
        assert!(session_meta_exists(tmp.path(), workspace, sid));
    }
}
