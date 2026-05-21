//! LLM-enhanced docs via a locally-running Ollama instance.
//!
//! Results are cached under `~/.cache/inspect-cli/ollama/` so repeated calls
//! for the same command/model/prompt input are instant.

use std::fs;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::PathBuf;
use std::time::Duration;

use crate::sources::{CommandDocs, Example, Flag};

const DEFAULT_BASE_URL: &str = "http://127.0.0.1:11434";
const DEFAULT_MODEL: &str = "llama3.2";
const TIMEOUT_SECS: u64 = 15;
const PROMPT_VERSION: &str = "ollama-json-v2";

/// Ollama connection settings.
#[derive(Debug, Clone)]
pub struct Config {
    pub base_url: String,
    pub model: String,
}

impl Config {
    pub fn from_options(model: Option<&str>, base_url: Option<&str>) -> Self {
        Self {
            base_url: base_url
                .map(ToOwned::to_owned)
                .or_else(|| std::env::var("INSPECT_OLLAMA_URL").ok())
                .unwrap_or_else(|| DEFAULT_BASE_URL.to_string()),
            model: model
                .map(ToOwned::to_owned)
                .or_else(|| std::env::var("INSPECT_OLLAMA_MODEL").ok())
                .unwrap_or_else(|| DEFAULT_MODEL.to_string()),
        }
    }

    pub fn generate_endpoint(&self) -> String {
        let trimmed = self.base_url.trim_end_matches('/');
        if trimmed.ends_with("/api/generate") {
            trimmed.to_string()
        } else {
            format!("{trimmed}/api/generate")
        }
    }

    fn generate_endpoints(&self) -> Vec<String> {
        let primary = self.generate_endpoint();
        let mut endpoints = vec![primary.clone()];

        if primary.contains("://localhost") {
            endpoints.push(primary.replace("://localhost", "://127.0.0.1"));
        } else if primary.contains("://127.0.0.1") {
            endpoints.push(primary.replace("://127.0.0.1", "://localhost"));
        }

        endpoints
    }
}

/// Try to get LLM-enhanced docs from Ollama for `cmd`.
///
/// Returns an error when Ollama is unavailable, times out, or returns invalid
/// data. Callers decide whether to surface the warning.
pub fn fetch(
    cmd: &str,
    man_summary: Option<&str>,
    raw_flags: &str,
    config: &Config,
) -> Result<Option<CommandDocs>, String> {
    // Check cache first.
    if let Some(cached) = load_cache(cmd, man_summary, raw_flags, config) {
        return Ok(Some(cached));
    }

    let docs = call_ollama(cmd, man_summary, raw_flags, config)?;

    // Best-effort cache write — ignore errors.
    if let Some(docs) = &docs {
        let _ = save_cache(cmd, man_summary, raw_flags, config, docs);
    }

    Ok(docs)
}

// ── Cache helpers ────────────────────────────────────────────────────────────

fn cache_path(
    cmd: &str,
    man_summary: Option<&str>,
    raw_flags: &str,
    config: &Config,
) -> Option<PathBuf> {
    let base = dirs::cache_dir()?;
    let key = cache_key(cmd, man_summary, raw_flags, config);
    let safe_cmd = sanitize_filename(cmd);
    Some(
        base.join("inspect-cli")
            .join("ollama")
            .join(config.model.replace('/', "_"))
            .join(format!("{safe_cmd}-{key:016x}.json")),
    )
}

fn cache_key(cmd: &str, man_summary: Option<&str>, raw_flags: &str, config: &Config) -> u64 {
    let mut hasher = DefaultHasher::new();
    PROMPT_VERSION.hash(&mut hasher);
    config.model.hash(&mut hasher);
    cmd.hash(&mut hasher);
    man_summary.unwrap_or("").hash(&mut hasher);
    raw_flags.hash(&mut hasher);
    hasher.finish()
}

fn sanitize_filename(input: &str) -> String {
    let safe: String = input
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
                ch
            } else {
                '_'
            }
        })
        .collect();

    if safe.is_empty() {
        "command".to_string()
    } else {
        safe
    }
}

fn load_cache(
    cmd: &str,
    man_summary: Option<&str>,
    raw_flags: &str,
    config: &Config,
) -> Option<CommandDocs> {
    let path = cache_path(cmd, man_summary, raw_flags, config)?;
    let bytes = fs::read(&path).ok()?;
    serde_json::from_slice(&bytes).ok()
}

fn save_cache(
    cmd: &str,
    man_summary: Option<&str>,
    raw_flags: &str,
    config: &Config,
    docs: &CommandDocs,
) -> std::io::Result<()> {
    let path = cache_path(cmd, man_summary, raw_flags, config)
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "no cache dir"))?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_vec_pretty(docs).map_err(std::io::Error::other)?;
    fs::write(path, json)
}

// ── Ollama HTTP call ─────────────────────────────────────────────────────────

fn call_ollama(
    cmd: &str,
    man_summary: Option<&str>,
    raw_flags: &str,
    config: &Config,
) -> Result<Option<CommandDocs>, String> {
    let summary_text = man_summary.unwrap_or("");
    let flags_snippet: String = raw_flags.chars().take(500).collect();

    let prompt = format!(
        r#"You are a concise CLI reference tool. Given the command "{cmd}" with this raw documentation:

SUMMARY: {summary_text}
FLAGS (raw): {flags_snippet}

Output a JSON object with this exact schema:
{{
  "summary": "one short sentence describing what this command does",
  "flags": [
    {{"names": "-f, --flag", "description": "one clear line"}}
  ],
  "examples": [
    {{"command": "cmd --flag arg", "description": "what this does"}}
  ]
}}

Rules:
- summary: max 10 words, plain English, no jargon
- flags: only the 6 most important/common ones
- examples: exactly 3 practical examples a developer would actually use
- descriptions: one short plain-English sentence each, no man-page prose
- output ONLY the JSON object, no markdown fences, no commentary"#
    );

    let body = serde_json::json!({
        "model": config.model,
        "prompt": prompt,
        "format": "json",
        "stream": false,
        "options": {
            "temperature": 0
        }
    });

    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(TIMEOUT_SECS))
        .build();

    let mut errors = Vec::new();
    let mut response = None;
    for endpoint in config.generate_endpoints() {
        match agent
            .post(&endpoint)
            .set("Content-Type", "application/json")
            .send_json(&body)
        {
            Ok(value) => {
                response = Some(value);
                break;
            }
            Err(error) => {
                errors.push(format!("{endpoint}: {error}"));
            }
        }
    }

    let response = response.ok_or_else(|| {
        if errors.is_empty() {
            "request failed: no endpoint attempted".to_string()
        } else {
            format!("request failed: {}", errors.join(" | "))
        }
    })?;

    let response_json: serde_json::Value = response
        .into_json()
        .map_err(|e| format!("invalid Ollama response JSON: {e}"))?;
    let response_text = response_json["response"]
        .as_str()
        .ok_or_else(|| "Ollama response did not contain a `response` string".to_string())?;

    parse_llm_response(response_text)
        .map(Some)
        .ok_or_else(|| "Ollama returned invalid inspect JSON".to_string())
}

// ── Response parsing ─────────────────────────────────────────────────────────

fn parse_llm_response(text: &str) -> Option<CommandDocs> {
    // Strip markdown fences if present.
    let stripped = strip_fences(text.trim());

    let value: serde_json::Value = serde_json::from_str(stripped).ok()?;

    let summary = value["summary"].as_str().map(|s| s.to_string());

    let flags: Vec<Flag> = value["flags"].as_array().map_or_else(Vec::new, |items| {
        items
            .iter()
            .filter_map(|f| {
                let names = f["names"].as_str()?.to_string();
                let description = f["description"].as_str().unwrap_or("").to_string();
                Some(Flag { names, description })
            })
            .collect()
    });

    let examples: Vec<Example> = value["examples"].as_array().map_or_else(Vec::new, |items| {
        items
            .iter()
            .filter_map(|e| {
                let command = e["command"].as_str()?.to_string();
                let description = e["description"].as_str().map(|s| s.to_string());
                Some(Example {
                    command,
                    description,
                })
            })
            .collect()
    });

    // Only return a result if at least the summary parsed.
    if summary.is_none() && flags.is_empty() && examples.is_empty() {
        return None;
    }

    Some(CommandDocs {
        summary,
        usage: None,
        flags,
        examples,
        risks: Vec::new(),
        sources: vec!["ollama".to_string()],
    })
}

fn strip_fences(text: &str) -> &str {
    // Strip ```json ... ``` or ``` ... ``` wrappers.
    let text = if let Some(rest) = text.strip_prefix("```json") {
        rest
    } else if let Some(rest) = text.strip_prefix("```") {
        rest
    } else {
        return text;
    };
    // Strip trailing fence.
    if let Some(pos) = text.rfind("```") {
        text[..pos].trim()
    } else {
        text.trim()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_markdown_fences() {
        let input = "```json\n{\"summary\": \"test\"}\n```";
        let stripped = strip_fences(input);
        assert_eq!(stripped, "{\"summary\": \"test\"}");
    }

    #[test]
    fn strips_plain_fences() {
        let input = "```\n{\"summary\": \"test\"}\n```";
        let stripped = strip_fences(input);
        assert_eq!(stripped, "{\"summary\": \"test\"}");
    }

    #[test]
    fn parse_valid_response() {
        let json = r#"{
            "summary": "copy files and directories",
            "flags": [
                {"names": "-r, --recursive", "description": "copy directories recursively"}
            ],
            "examples": [
                {"command": "cp -r src/ dest/", "description": "copy a directory"}
            ]
        }"#;
        let docs = parse_llm_response(json).expect("should parse");
        assert_eq!(docs.summary.as_deref(), Some("copy files and directories"));
        assert_eq!(docs.flags.len(), 1);
        assert_eq!(docs.examples.len(), 1);
        assert!(docs.sources.contains(&"ollama".to_string()));
    }

    #[test]
    fn parse_invalid_json_returns_none() {
        assert!(parse_llm_response("not json at all").is_none());
    }

    #[test]
    fn parse_empty_object_returns_none() {
        // An empty object has no summary/flags/examples so should return None.
        assert!(parse_llm_response("{}").is_none());
    }

    #[test]
    fn config_uses_cli_values_before_env_defaults() {
        let config = Config::from_options(Some("qwen2.5"), Some("http://example.test"));
        assert_eq!(config.model, "qwen2.5");
        assert_eq!(
            config.generate_endpoint(),
            "http://example.test/api/generate"
        );
    }

    #[test]
    fn default_endpoint_uses_ipv4_loopback() {
        let config = Config::from_options(None, None);
        assert_eq!(
            config.generate_endpoint(),
            "http://127.0.0.1:11434/api/generate"
        );
    }

    #[test]
    fn localhost_endpoint_adds_ipv4_fallback() {
        let config = Config::from_options(None, Some("http://localhost:11434"));
        assert_eq!(
            config.generate_endpoints(),
            vec![
                "http://localhost:11434/api/generate".to_string(),
                "http://127.0.0.1:11434/api/generate".to_string(),
            ]
        );
    }

    #[test]
    fn cache_key_changes_when_model_changes() {
        let a = Config {
            base_url: "http://localhost:11434".to_string(),
            model: "llama3.2".to_string(),
        };
        let b = Config {
            base_url: "http://localhost:11434".to_string(),
            model: "qwen2.5".to_string(),
        };

        assert_ne!(
            cache_key("cp", Some("copy"), "-r", &a),
            cache_key("cp", Some("copy"), "-r", &b)
        );
    }
}
