use std::fs;
use std::path::PathBuf;

use anyhow::Result;
use chrono::{DateTime, Utc};

use super::models::*;

/// Load all Claude Code sessions. Returns Ok(vec![]) if ~/.claude doesn't exist.
pub fn load_sessions() -> Result<Vec<Session>> {
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => return Ok(vec![]),
    };

    let claude_dir = home.join(".claude");
    if !claude_dir.exists() {
        return Ok(vec![]);
    }

    let projects_dir = claude_dir.join("projects");
    if !projects_dir.exists() {
        return Ok(vec![]);
    }

    let mut sessions = Vec::new();

    let project_dirs = match fs::read_dir(&projects_dir) {
        Ok(rd) => rd,
        Err(_) => return Ok(vec![]),
    };

    for project_entry in project_dirs.flatten() {
        let project_path = project_entry.path();
        if !project_path.is_dir() {
            continue;
        }

        let cwd = decode_project_dir(project_entry.file_name().to_string_lossy().as_ref());

        let files = match fs::read_dir(&project_path) {
            Ok(rd) => rd,
            Err(_) => continue,
        };

        for file_entry in files.flatten() {
            let file_path = file_entry.path();
            if file_path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
                continue;
            }

            if let Some(session) = parse_session_file(&file_path, &cwd) {
                sessions.push(session);
            }
        }
    }

    Ok(sessions)
}

/// Decode a hyphen-encoded project directory name back to a filesystem path.
///
/// On Unix: `-home-alice-dev-myapp-` → `/home/alice/dev/myapp`
/// On Windows: `C--Users-JohnDoe-dev-proj` → `C:\Users\JohnDoe\dev\proj`
fn decode_project_dir(encoded: &str) -> String {
    let trimmed = encoded.trim_matches('-');

    // Detect Windows-style: starts with a drive letter followed by `-`
    // e.g. "C--Users-JohnDoe-dev-proj" → trimmed = "C--Users-JohnDoe-dev-proj"
    if trimmed.len() >= 2 && trimmed.as_bytes()[0].is_ascii_alphabetic() && trimmed[1..].starts_with('-') {
        // Windows: "C--Users-JohnDoe-dev-proj"
        let drive = &trimmed[..1];
        let rest = &trimmed[1..]; // "--Users-JohnDoe-dev-proj"
        // The double-hyphen after drive letter represents `:` + `\`
        // Replace leading `--` with `:\` then remaining `-` with `\`
        if let Some(stripped) = rest.strip_prefix("--") {
            let path_part = stripped.replace('-', "\\");
            return format!("{}:\\{}", drive, path_part);
        }
        // Single hyphen after drive letter: just path separators
        let rest = rest.trim_start_matches('-');
        let path_part = rest.replace('-', "\\");
        return format!("{}:\\{}", drive, path_part);
    }

    // Unix-style: leading hyphen was the root `/`
    let path_part = trimmed.replace('-', "/");
    format!("/{}", path_part)
}

/// Parse a single .jsonl session file into a Session.
fn parse_session_file(path: &PathBuf, cwd: &str) -> Option<Session> {
    let content = fs::read_to_string(path).ok()?;

    let session_id = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    let mut events: Vec<ClaudeEvent> = Vec::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Ok(event) = serde_json::from_str::<ClaudeEvent>(line) {
            events.push(event);
        }
    }

    if events.is_empty() {
        return None;
    }

    let mut metrics = SessionMetrics::default();
    let mut turns = Vec::new();
    let mut first_timestamp: Option<DateTime<Utc>> = None;
    let mut last_timestamp: Option<DateTime<Utc>> = None;
    let mut summary: Option<String> = None;
    let mut last_model: Option<String> = None;
    let mut turn_count: u32 = 0;

    for event in &events {
        let ts = event
            .timestamp
            .as_deref()
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc));

        if let Some(t) = ts {
            if first_timestamp.is_none() || t < first_timestamp.unwrap() {
                first_timestamp = Some(t);
            }
            if last_timestamp.is_none() || t > last_timestamp.unwrap() {
                last_timestamp = Some(t);
            }
        }

        match event.event_type.as_str() {
            "user" => {
                turn_count += 1;
                metrics.total_turns += 1;
                metrics.total_messages += 1;

                // Extract first user message as summary
                if summary.is_none() {
                    if let Some(msg) = &event.message {
                        let text = extract_user_text(msg);
                        if !text.is_empty() {
                            summary = Some(truncate(&text, 60));
                        }
                    }
                }

                // Estimate input tokens from content length (~4 chars per token)
                if let Some(msg) = &event.message {
                    let text = extract_user_text(msg);
                    let estimated = (text.len() as u64) / 4;
                    metrics.estimated_input_tokens += estimated;
                }

                let turn = Turn {
                    turn_id: event
                        .uuid
                        .clone()
                        .unwrap_or_else(|| format!("turn-{}", turn_count)),
                    timestamp: ts,
                    output_tokens: 0,
                    tool_request_count: 0,
                    has_reasoning: false,
                };
                turns.push(turn);
            }
            "assistant" => {
                metrics.total_messages += 1;

                if let Some(msg) = &event.message {
                    // Extract usage info
                    if let Some(usage) = msg.get("usage") {
                        let input_tokens = usage
                            .get("input_tokens")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0);
                        let output_tokens = usage
                            .get("output_tokens")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0);

                        metrics.estimated_input_tokens += input_tokens;
                        metrics.total_output_tokens += output_tokens;
                    }

                    // Extract model
                    if let Some(model) = msg.get("model").and_then(|v| v.as_str()) {
                        let model_str = model.to_string();
                        *metrics.models_used.entry(model_str.clone()).or_insert(0) += 1;
                        last_model = Some(model_str);
                    }

                    // Count tool use blocks in content array
                    if let Some(content) = msg.get("content").and_then(|v| v.as_array()) {
                        for block in content {
                            if block.get("type").and_then(|v| v.as_str()) == Some("tool_use") {
                                metrics.total_tool_calls += 1;
                            }
                        }
                    }
                }
            }
            "result" => {
                if let Some(msg) = &event.message {
                    // Result events carry cost/duration at the message level,
                    // but the fields may also be at the event top level via serde_json::Value.
                    // We check the message object first.
                    let obj = msg;

                    if let Some(cost) = obj.get("costUSD").and_then(|v| v.as_f64()) {
                        metrics.estimated_cost_usd += cost;
                    }
                    if let Some(dur) = obj.get("duration_ms").and_then(|v| v.as_u64()) {
                        metrics.total_api_duration_ms += dur;
                    }
                    if let Some(input_tokens) = obj.get("input_tokens").and_then(|v| v.as_u64()) {
                        metrics.estimated_input_tokens += input_tokens;
                    }
                    if let Some(output_tokens) = obj.get("output_tokens").and_then(|v| v.as_u64())
                    {
                        metrics.total_output_tokens += output_tokens;
                    }
                }

                // Also check top-level fields (result events sometimes have them at root)
                // We can parse the raw line again but ClaudeEvent already has limited fields.
                // The message field should cover it.
            }
            _ => {
                // system and other types — skip
            }
        }
    }

    let is_active = last_timestamp
        .map(|t| Utc::now().signed_duration_since(t).num_minutes() < 5)
        .unwrap_or(false);

    Some(Session {
        id: session_id,
        source: Source::Claude,
        summary,
        model: last_model,
        cwd: Some(cwd.to_string()),
        branch: None,
        started_at: first_timestamp,
        ended_at: last_timestamp,
        is_active,
        metrics,
        turns,
        sub_agents: vec![],
        tool_calls: vec![],
    })
}

/// Extract text content from a user message value.
/// The content can be a string or an array of content blocks.
fn extract_user_text(msg: &serde_json::Value) -> String {
    // Try message.content first
    if let Some(content) = msg.get("content") {
        if let Some(s) = content.as_str() {
            return s.to_string();
        }
        if let Some(arr) = content.as_array() {
            let mut parts = Vec::new();
            for block in arr {
                if let Some(text) = block.get("text").and_then(|v| v.as_str()) {
                    parts.push(text.to_string());
                } else if let Some(s) = block.as_str() {
                    parts.push(s.to_string());
                }
            }
            return parts.join(" ");
        }
    }
    // Fallback: try the value itself as a string
    if let Some(s) = msg.as_str() {
        return s.to_string();
    }
    String::new()
}

/// Truncate a string to `max_len` characters, appending "…" if truncated.
fn truncate(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_len.saturating_sub(1)).collect();
        format!("{}…", truncated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_project_dir_unix() {
        assert_eq!(
            decode_project_dir("-home-alice-dev-myapp-"),
            "/home/alice/dev/myapp"
        );
    }

    #[test]
    fn test_decode_project_dir_windows() {
        assert_eq!(
            decode_project_dir("C--Users-JohnDoe-dev-proj"),
            "C:\\Users\\JohnDoe\\dev\\proj"
        );
    }

    #[test]
    fn test_truncate_short() {
        assert_eq!(truncate("hello", 60), "hello");
    }

    #[test]
    fn test_truncate_long() {
        let long = "a".repeat(100);
        let result = truncate(&long, 60);
        assert_eq!(result.chars().count(), 60);
        assert!(result.ends_with('…'));
    }

    #[test]
    fn test_load_sessions_no_claude_dir() {
        // Should not error even if ~/.claude doesn't exist
        let result = load_sessions();
        assert!(result.is_ok());
    }
}
