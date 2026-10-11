//! Dispatcher settings helpers: shared `config.yaml` shape, validation,
//! effective defaults, and `agent models` text parsing.

use serde::{Deserialize, Serialize};

/// Default model slug for every Dispatcher Stage.
pub const DEFAULT_DISPATCHER_MODEL: &str = "cursor-grok-4.5-high";

/// Default Stage timeout in minutes.
pub const DEFAULT_TIMEOUT_MINUTES: u32 = 60;

/// Default global concurrency limit (serve.json).
pub const DEFAULT_CONCURRENCY: u32 = 20;

/// Maximum model slug length.
pub const MAX_MODEL_LEN: usize = 200;

/// Inclusive timeout range (minutes).
pub const TIMEOUT_MIN: u32 = 1;
pub const TIMEOUT_MAX: u32 = 1440;

/// Inclusive concurrency range.
pub const CONCURRENCY_MIN: u32 = 1;
pub const CONCURRENCY_MAX: u32 = 100;

/// One model entry from `agent models` output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentModel {
    pub slug: String,
    pub label: String,
}

/// Per-Stage settings as stored in `config.yaml` (all fields optional).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DispatcherStageConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_minutes: Option<u32>,
}

impl DispatcherStageConfig {
    fn is_empty(&self) -> bool {
        self.model.is_none() && self.timeout_minutes.is_none()
    }
}

/// Stages block in `config.yaml`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DispatcherStagesConfig {
    #[serde(default, skip_serializing_if = "DispatcherStageConfig::is_empty")]
    pub implement: DispatcherStageConfig,
    #[serde(default, skip_serializing_if = "DispatcherStageConfig::is_empty")]
    pub fix: DispatcherStageConfig,
    #[serde(default, skip_serializing_if = "DispatcherStageConfig::is_empty")]
    pub merge: DispatcherStageConfig,
}

impl DispatcherStagesConfig {
    fn is_empty(&self) -> bool {
        self.implement.is_empty() && self.fix.is_empty() && self.merge.is_empty()
    }
}

/// Shared Dispatcher settings stored under `config.yaml` → `dispatcher`.
///
/// Unset fields are omitted on write-back so future default changes take effect.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DispatcherConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub push_after_merge: Option<bool>,
    #[serde(default, skip_serializing_if = "DispatcherStagesConfig::is_empty")]
    pub stages: DispatcherStagesConfig,
}

impl DispatcherConfig {
    /// True when nothing is explicitly set (section can be omitted entirely).
    pub fn is_empty(&self) -> bool {
        self.push_after_merge.is_none() && self.stages.is_empty()
    }
}

/// Effective per-Stage settings after applying defaults.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EffectiveStageSettings {
    pub model: String,
    pub timeout_minutes: u32,
}

/// Effective Dispatcher settings snapshot for Stage start / API responses.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EffectiveDispatcherSettings {
    pub push_after_merge: bool,
    pub stages: EffectiveStagesSettings,
}

/// Effective settings for all three Stages.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EffectiveStagesSettings {
    pub implement: EffectiveStageSettings,
    pub fix: EffectiveStageSettings,
    pub merge: EffectiveStageSettings,
}

/// Validation failure for a Dispatcher setting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DispatcherValidationError {
    pub message: String,
}

impl std::fmt::Display for DispatcherValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

/// Validate a model slug: trim, non-empty, no whitespace, ≤200 chars.
pub fn validate_model(raw: &str) -> Result<String, DispatcherValidationError> {
    let model = raw.trim();
    if model.is_empty() {
        return Err(DispatcherValidationError {
            message: "Model must be non-empty".into(),
        });
    }
    if model.chars().any(|c| c.is_whitespace()) {
        return Err(DispatcherValidationError {
            message: "Model must not contain whitespace".into(),
        });
    }
    if model.chars().count() > MAX_MODEL_LEN {
        return Err(DispatcherValidationError {
            message: format!("Model must be {MAX_MODEL_LEN} characters or fewer"),
        });
    }
    Ok(model.to_string())
}

/// Validate timeout_minutes ∈ [1, 1440].
pub fn validate_timeout_minutes(value: u32) -> Result<u32, DispatcherValidationError> {
    if !(TIMEOUT_MIN..=TIMEOUT_MAX).contains(&value) {
        return Err(DispatcherValidationError {
            message: format!(
                "timeout_minutes must be between {TIMEOUT_MIN} and {TIMEOUT_MAX}"
            ),
        });
    }
    Ok(value)
}

/// Validate concurrency ∈ [1, 100].
pub fn validate_concurrency(value: u32) -> Result<u32, DispatcherValidationError> {
    if !(CONCURRENCY_MIN..=CONCURRENCY_MAX).contains(&value) {
        return Err(DispatcherValidationError {
            message: format!(
                "concurrency must be between {CONCURRENCY_MIN} and {CONCURRENCY_MAX}"
            ),
        });
    }
    Ok(value)
}

/// Validate every explicitly set field in a stored Dispatcher config.
pub fn validate_dispatcher_config(
    config: &DispatcherConfig,
) -> Result<(), DispatcherValidationError> {
    for (name, stage) in [
        ("implement", &config.stages.implement),
        ("fix", &config.stages.fix),
        ("merge", &config.stages.merge),
    ] {
        if let Some(ref model) = stage.model {
            validate_model(model).map_err(|e| DispatcherValidationError {
                message: format!("stages.{name}.model: {}", e.message),
            })?;
        }
        if let Some(timeout) = stage.timeout_minutes {
            validate_timeout_minutes(timeout).map_err(|e| DispatcherValidationError {
                message: format!("stages.{name}.timeout_minutes: {}", e.message),
            })?;
        }
    }
    Ok(())
}

fn effective_stage(stage: &DispatcherStageConfig) -> EffectiveStageSettings {
    EffectiveStageSettings {
        model: stage
            .model
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or(DEFAULT_DISPATCHER_MODEL)
            .to_string(),
        timeout_minutes: stage.timeout_minutes.unwrap_or(DEFAULT_TIMEOUT_MINUTES),
    }
}

/// Resolve stored optional fields to effective defaults.
pub fn effective_dispatcher_settings(config: &DispatcherConfig) -> EffectiveDispatcherSettings {
    EffectiveDispatcherSettings {
        push_after_merge: config.push_after_merge.unwrap_or(true),
        stages: EffectiveStagesSettings {
            implement: effective_stage(&config.stages.implement),
            fix: effective_stage(&config.stages.fix),
            merge: effective_stage(&config.stages.merge),
        },
    }
}

/// Strip CSI / OSC ANSI escape sequences from text.
pub fn strip_ansi(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\u{1b}' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('[') => {
                // CSI: ESC [ ... final byte in 0x40..=0x7E
                for ch in chars.by_ref() {
                    if ('\u{40}'..='\u{7e}').contains(&ch) {
                        break;
                    }
                }
            }
            Some(']') => {
                // OSC: ESC ] ... BEL or ST (ESC \)
                while let Some(ch) = chars.next() {
                    if ch == '\u{07}' {
                        break;
                    }
                    if ch == '\u{1b}' && chars.peek() == Some(&'\\') {
                        let _ = chars.next();
                        break;
                    }
                }
            }
            Some(_) | None => {}
        }
    }
    out
}

fn strip_model_label_suffix(label: &str) -> String {
    let mut out = label.to_string();
    loop {
        let before = out.len();
        for suffix in [" (default)", " (current)"] {
            if let Some(stripped) = out.strip_suffix(suffix) {
                out = stripped.to_string();
            }
        }
        if out.len() == before {
            break;
        }
    }
    out
}

/// Parse `agent models` human-readable output into slug/label pairs.
///
/// Lines matching `<slug> - <label>` are kept. Trailing ` (default)` /
/// ` (current)` markers are stripped from labels. ANSI codes are removed first.
///
/// Callers should set `NO_COLOR=1` when invoking `agent models`.
pub fn parse_agent_models(text: &str) -> Vec<AgentModel> {
    let cleaned = strip_ansi(text);
    let mut models = Vec::new();
    for line in cleaned.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Some((slug, rest)) = line.split_once(" - ") else {
            continue;
        };
        if slug.is_empty() || slug.chars().any(|c| c.is_whitespace()) {
            continue;
        }
        let label = strip_model_label_suffix(rest.trim());
        if label.is_empty() {
            continue;
        }
        models.push(AgentModel {
            slug: slug.to_string(),
            label,
        });
    }
    models
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_agent_models_basic_lines() {
        let text = "\
Available models

claude-opus-4.5 - Claude Opus 4.5
cursor-grok-4.5-high - Grok 4.5 High

Tip: use --model to select
";
        let models = parse_agent_models(text);
        assert_eq!(
            models,
            vec![
                AgentModel {
                    slug: "claude-opus-4.5".into(),
                    label: "Claude Opus 4.5".into(),
                },
                AgentModel {
                    slug: "cursor-grok-4.5-high".into(),
                    label: "Grok 4.5 High".into(),
                },
            ]
        );
    }

    #[test]
    fn parse_agent_models_strips_default_and_current_suffixes() {
        let text = "\
gpt-5.3-codex - GPT-5.3 Codex (default)
cursor-grok-4.5-high - Grok 4.5 High (current)
other-model - Other Model (default) (current)
";
        let models = parse_agent_models(text);
        assert_eq!(models[0].label, "GPT-5.3 Codex");
        assert_eq!(models[1].label, "Grok 4.5 High");
        assert_eq!(models[2].label, "Other Model");
    }

    #[test]
    fn parse_agent_models_strips_ansi_codes() {
        let text = "\
\x1b[1mAvailable models\x1b[0m

\x1b[32mclaude-opus-4.5\x1b[0m - \x1b[36mClaude Opus 4.5\x1b[0m (default)
\x1b[90mcursor-grok-4.5-high\x1b[0m - Grok 4.5 High (current)
";
        let models = parse_agent_models(text);
        assert_eq!(
            models,
            vec![
                AgentModel {
                    slug: "claude-opus-4.5".into(),
                    label: "Claude Opus 4.5".into(),
                },
                AgentModel {
                    slug: "cursor-grok-4.5-high".into(),
                    label: "Grok 4.5 High".into(),
                },
            ]
        );
    }

    #[test]
    fn parse_agent_models_ignores_non_matching_lines() {
        let text = "\
not a model line
- missing slug
slug-only -
has spaces in slug - Label
";
        assert!(parse_agent_models(text).is_empty());
    }

    #[test]
    fn effective_settings_apply_defaults() {
        let effective = effective_dispatcher_settings(&DispatcherConfig::default());
        assert!(effective.push_after_merge);
        assert_eq!(effective.stages.implement.model, DEFAULT_DISPATCHER_MODEL);
        assert_eq!(
            effective.stages.implement.timeout_minutes,
            DEFAULT_TIMEOUT_MINUTES
        );
        assert_eq!(effective.stages.fix.model, DEFAULT_DISPATCHER_MODEL);
        assert_eq!(effective.stages.merge.timeout_minutes, DEFAULT_TIMEOUT_MINUTES);
    }

    #[test]
    fn validate_rejects_out_of_range_timeout_and_concurrency() {
        assert!(validate_timeout_minutes(0).is_err());
        assert!(validate_timeout_minutes(1441).is_err());
        assert!(validate_concurrency(0).is_err());
        assert!(validate_concurrency(101).is_err());
        assert!(validate_model("").is_err());
        assert!(validate_model("has space").is_err());
    }

    #[test]
    fn dispatcher_config_omits_unset_fields_on_serialize() {
        let cfg = DispatcherConfig {
            push_after_merge: Some(false),
            ..Default::default()
        };
        let yaml = serde_yaml::to_string(&cfg).unwrap();
        assert!(yaml.contains("push_after_merge: false"));
        assert!(!yaml.contains("stages"));
        assert!(!yaml.contains("model"));
        assert!(!yaml.contains("timeout"));
    }
}
