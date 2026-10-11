//! Machine-local Dispatcher settings (`serve.json`) and `agent models` cache.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use markplane_core::{AgentModel, DEFAULT_CONCURRENCY, parse_agent_models, validate_concurrency};
use serde::{Deserialize, Serialize};

/// How long to cache `agent models` output.
pub const MODELS_CACHE_TTL: Duration = Duration::from_secs(10 * 60);

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ServeFile {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dispatcher: Option<ServeDispatcherFile>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ServeDispatcherFile {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub concurrency: Option<u32>,
}

pub fn serve_json_path(config_dir: &Path) -> PathBuf {
    config_dir.join("serve.json")
}

/// Why reading concurrency from `serve.json` failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConcurrencyReadError {
    /// File missing is OK (defaults); this is IO / JSON parse failure.
    Unreadable(String),
    /// JSON parsed but `concurrency` is out of range.
    Invalid(String),
}

impl std::fmt::Display for ConcurrencyReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unreadable(msg) | Self::Invalid(msg) => write!(f, "{msg}"),
        }
    }
}

/// Read concurrency from `serve.json`.
///
/// - Missing file → default 20 (no warning).
/// - Unreadable / invalid JSON → [`ConcurrencyReadError::Unreadable`].
/// - Out-of-range → [`ConcurrencyReadError::Invalid`].
pub fn read_concurrency(config_dir: &Path) -> Result<u32, ConcurrencyReadError> {
    let path = serve_json_path(config_dir);
    if !path.exists() {
        return Ok(DEFAULT_CONCURRENCY);
    }
    let content = std::fs::read_to_string(&path)
        .map_err(|e| ConcurrencyReadError::Unreadable(format!("cannot read serve.json: {e}")))?;
    let file: ServeFile = serde_json::from_str(&content)
        .map_err(|e| ConcurrencyReadError::Unreadable(format!("cannot parse serve.json: {e}")))?;
    let Some(concurrency) = file.dispatcher.as_ref().and_then(|d| d.concurrency) else {
        return Ok(DEFAULT_CONCURRENCY);
    };
    validate_concurrency(concurrency).map_err(|e| ConcurrencyReadError::Invalid(e.message))?;
    Ok(concurrency)
}

/// Write concurrency into `serve.json`, preserving unknown fields when possible.
///
/// Only writes the concurrency key (plus any pre-existing JSON object keys we
/// can round-trip through [`ServeFile`]).
pub fn write_concurrency(config_dir: &Path, concurrency: u32) -> Result<(), String> {
    validate_concurrency(concurrency).map_err(|e| e.message)?;
    std::fs::create_dir_all(config_dir).map_err(|e| format!("cannot create config dir: {e}"))?;
    let path = serve_json_path(config_dir);

    let mut file = if path.exists() {
        let content =
            std::fs::read_to_string(&path).map_err(|e| format!("cannot read serve.json: {e}"))?;
        serde_json::from_str::<ServeFile>(&content).unwrap_or_default()
    } else {
        ServeFile::default()
    };

    let mut dispatcher = file.dispatcher.unwrap_or_default();
    dispatcher.concurrency = Some(concurrency);
    file.dispatcher = Some(dispatcher);

    let json = serde_json::to_string_pretty(&file)
        .map_err(|e| format!("cannot serialize serve.json: {e}"))?;
    std::fs::write(&path, format!("{json}\n"))
        .map_err(|e| format!("cannot write serve.json: {e}"))?;
    Ok(())
}

/// In-memory models cache shared across requests.
#[derive(Debug, Default)]
pub struct ModelsCache {
    models: Vec<AgentModel>,
    fetched_at: Option<Instant>,
}

impl ModelsCache {
    pub fn get_if_fresh(&self) -> Option<Vec<AgentModel>> {
        let fetched_at = self.fetched_at?;
        if fetched_at.elapsed() < MODELS_CACHE_TTL {
            Some(self.models.clone())
        } else {
            None
        }
    }

    pub fn store(&mut self, models: Vec<AgentModel>) {
        self.models = models;
        self.fetched_at = Some(Instant::now());
    }
}

/// Run `agent models` (or an override binary) and parse the text output.
///
/// Sets `NO_COLOR=1`. On any failure returns an empty list.
pub fn fetch_agent_models(agent_bin: &Path) -> Vec<AgentModel> {
    let output = Command::new(agent_bin)
        .arg("models")
        .env("NO_COLOR", "1")
        .output();
    match output {
        Ok(out) if out.status.success() => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            parse_agent_models(&stdout)
        }
        Ok(out) => {
            eprintln!(
                "warning: `{} models` exited with {}: {}",
                agent_bin.display(),
                out.status,
                String::from_utf8_lossy(&out.stderr).trim()
            );
            Vec::new()
        }
        Err(e) => {
            eprintln!(
                "warning: failed to run `{} models`: {e}",
                agent_bin.display()
            );
            Vec::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn missing_serve_json_returns_default_concurrency() {
        let tmp = TempDir::new().unwrap();
        assert_eq!(read_concurrency(tmp.path()).unwrap(), DEFAULT_CONCURRENCY);
    }

    #[test]
    fn write_and_read_concurrency_roundtrip() {
        let tmp = TempDir::new().unwrap();
        write_concurrency(tmp.path(), 42).unwrap();
        assert_eq!(read_concurrency(tmp.path()).unwrap(), 42);
        let content = std::fs::read_to_string(serve_json_path(tmp.path())).unwrap();
        assert!(content.contains("\"concurrency\": 42"));
        // Default-equivalent omit: we still write explicit PATCH values.
        assert!(!content.contains("push_after_merge"));
    }

    #[test]
    fn unreadable_serve_json_errors() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(serve_json_path(tmp.path()), "not-json{{{").unwrap();
        assert!(matches!(
            read_concurrency(tmp.path()),
            Err(ConcurrencyReadError::Unreadable(_))
        ));
    }

    #[test]
    fn out_of_range_concurrency_errors() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(
            serve_json_path(tmp.path()),
            r#"{ "dispatcher": { "concurrency": 0 } }"#,
        )
        .unwrap();
        assert!(matches!(
            read_concurrency(tmp.path()),
            Err(ConcurrencyReadError::Invalid(_))
        ));
    }
}
