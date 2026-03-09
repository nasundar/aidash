use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::path::Path;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::OpenFlags;
use serde::Deserialize;

use super::models::*;

// ---------------------------------------------------------------------------
// Local serde structs
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct WorkspaceMetadata {
    id: Option<String>,
    summary: Option<String>,
    cwd: Option<String>,
    branch: Option<String>,
    created_at: Option<String>,
    updated_at: Option<String>,
}

#[derive(Debug)]
#[allow(dead_code)]
struct SessionStoreInfo {
    summary: Option<String>,
    cwd: Option<String>,
    branch: Option<String>,
    created_at: Option<String>,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Load all Copilot sessions from ~/.copilot/
pub fn load_sessions() -> Result<Vec<Session>> {
    let home = dirs::home_dir().context("Could not determine home directory")?;
    let session_state_dir = home.join(".copilot").join("session-state");

    if !session_state_dir.exists() {
        return Ok(Vec::new());
    }

    let mut sessions = Vec::new();

    for entry in std::fs::read_dir(&session_state_dir)? {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        match parse_session(&path) {
            Ok(session) => sessions.push(session),
            Err(_) => continue, // skip sessions that fail to parse
        }
    }

    // Sort newest first
    sessions.sort_by(|a, b| b.started_at.cmp(&a.started_at));

    Ok(sessions)
}

// ---------------------------------------------------------------------------
// Session parsing
// ---------------------------------------------------------------------------

/// Parse a single session from its session-state directory
fn parse_session(session_dir: &Path) -> Result<Session> {
    let dir_name = session_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    // Read workspace.yaml (optional)
    let workspace_path = session_dir.join("workspace.yaml");
    let workspace = if workspace_path.exists() {
        read_workspace_yaml(&workspace_path).ok()
    } else {
        None
    };

    let session_id = workspace
        .as_ref()
        .and_then(|w| w.id.clone())
        .unwrap_or_else(|| dir_name.clone());

    // Parse events.jsonl
    let events_path = session_dir.join("events.jsonl");
    let (mut metrics, turns, sub_agents, tool_calls, model, ev_started, ev_ended) =
        if events_path.exists() {
            parse_events(&events_path).unwrap_or_else(|_| default_parse_events_result())
        } else {
            Default::default()
        };

    // Determine is_active: no shutdown event means still active
    let is_active = ev_ended.is_none() && ev_started.is_some();

    // Resolve started_at / ended_at from events or workspace
    let started_at = ev_started.or_else(|| {
        workspace
            .as_ref()
            .and_then(|w| w.created_at.as_deref())
            .and_then(|s| s.parse::<DateTime<Utc>>().ok())
    });
    // For ended_at: only use workspace's updated_at if the session is NOT active
    // Active sessions should have ended_at = None so duration shows as "growing"
    let ended_at = if is_active {
        None
    } else {
        ev_ended.or_else(|| {
            workspace
                .as_ref()
                .and_then(|w| w.updated_at.as_deref())
                .and_then(|s| s.parse::<DateTime<Utc>>().ok())
        })
    };

    // Resolve summary, cwd, branch — prefer workspace, fallback to session-store.db
    let mut summary = workspace.as_ref().and_then(|w| w.summary.clone());
    let mut cwd = workspace.as_ref().and_then(|w| w.cwd.clone());
    let mut branch = workspace.as_ref().and_then(|w| w.branch.clone());

    if summary.is_none() || cwd.is_none() || branch.is_none() {
        if let Ok(Some(store)) = read_session_store(&session_id) {
            if summary.is_none() {
                summary = store.summary;
            }
            if cwd.is_none() {
                cwd = store.cwd;
            }
            if branch.is_none() {
                branch = store.branch;
            }
            // Also use store's created_at as fallback for started_at
            if started_at.is_none() {
                // handled above already
            }
        }
    }

    // Resolve model — fallback to config.json if no model_change event
    let mut model = model;
    if model.is_none() {
        model = read_config_model();
    }

    // If we have a model but models_used is empty, attribute all messages to it
    if let Some(ref m) = model {
        if metrics.models_used.is_empty() && metrics.total_output_tokens > 0 {
            metrics.models_used.insert(m.clone(), turns.len().max(1) as u32);
        }
    }

    // Compute derived metrics
    metrics.total_turns = turns.len() as u32;
    metrics.total_messages = turns.len() as u32;
    metrics.total_tool_calls = tool_calls.len() as u32;
    metrics.total_sub_agents = sub_agents.len() as u32;

    Ok(Session {
        id: session_id,
        source: Source::Copilot,
        summary,
        model,
        cwd,
        branch,
        started_at,
        ended_at,
        is_active,
        metrics,
        turns,
        sub_agents,
        tool_calls,
    })
}

// ---------------------------------------------------------------------------
// Events parsing
// ---------------------------------------------------------------------------

type ParseEventsResult = (
    SessionMetrics,
    Vec<Turn>,
    Vec<SubAgent>,
    Vec<ToolCall>,
    Option<String>,        // last model
    Option<DateTime<Utc>>, // started_at
    Option<DateTime<Utc>>, // ended_at (from shutdown)
);

fn default_parse_events_result() -> ParseEventsResult {
    (
        SessionMetrics::default(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        None,
        None,
        None,
    )
}

/// Parse events.jsonl from a session directory
fn parse_events(events_path: &Path) -> Result<ParseEventsResult> {
    let file = std::fs::File::open(events_path)?;
    let reader = BufReader::new(file);

    let mut metrics = SessionMetrics::default();
    let mut turns: Vec<Turn> = Vec::new();
    let mut sub_agents_map: HashMap<String, SubAgent> = HashMap::new();
    let mut tool_calls_map: HashMap<String, ToolCall> = HashMap::new();

    let mut current_model: Option<String> = None;
    let mut started_at: Option<DateTime<Utc>> = None;
    let mut ended_at: Option<DateTime<Utc>> = None;

    // Track current turn being built
    let mut current_turn_id: Option<String> = None;
    let mut current_turn_tokens: u64 = 0;
    let mut current_turn_tool_requests: u32 = 0;
    let mut current_turn_has_reasoning: bool = false;
    let mut current_turn_timestamp: Option<DateTime<Utc>> = None;

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let event: CopilotEvent = match serde_json::from_str(trimmed) {
            Ok(e) => e,
            Err(_) => continue, // graceful degradation
        };

        let ts = event.timestamp.parse::<DateTime<Utc>>().ok();

        match event.event_type.as_str() {
            "session.start" => {
                if let Some(start_time) = event.data.get("startTime").and_then(|v| v.as_str()) {
                    started_at = start_time.parse::<DateTime<Utc>>().ok();
                }
                if started_at.is_none() {
                    started_at = ts;
                }
            }

            "session.resume" => {
                // Can also provide context info; we already have workspace.yaml for that
            }

            "session.model_change" => {
                if let Some(new_model) = event.data.get("newModel").and_then(|v| v.as_str()) {
                    current_model = Some(new_model.to_string());
                }
            }

            "assistant.turn_start" => {
                if let Some(turn_id) = event.data.get("turnId").and_then(|v| v.as_str()) {
                    current_turn_id = Some(turn_id.to_string());
                    current_turn_tokens = 0;
                    current_turn_tool_requests = 0;
                    current_turn_has_reasoning = false;
                    current_turn_timestamp = ts;
                }
            }

            "assistant.turn_end" => {
                if let Some(turn_id) = current_turn_id.take() {
                    turns.push(Turn {
                        turn_id,
                        timestamp: current_turn_timestamp.take(),
                        output_tokens: current_turn_tokens,
                        tool_request_count: current_turn_tool_requests,
                        has_reasoning: current_turn_has_reasoning,
                    });
                }
            }

            "assistant.message" => {
                let output_tokens = event
                    .data
                    .get("outputTokens")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);

                metrics.total_output_tokens += output_tokens;
                current_turn_tokens += output_tokens;

                // Reasoning estimation from reasoningText (~4 chars/token)
                let has_reasoning_text = event
                    .data
                    .get("reasoningText")
                    .and_then(|v| v.as_str())
                    .map(|s| !s.is_empty())
                    .unwrap_or(false);
                let has_reasoning_opaque = event.data.get("reasoningOpaque").is_some()
                    && !event.data["reasoningOpaque"].is_null();

                if has_reasoning_text || has_reasoning_opaque {
                    current_turn_has_reasoning = true;
                    if let Some(text) = event.data.get("reasoningText").and_then(|v| v.as_str()) {
                        let estimated = (text.len() as u64) / 4;
                        metrics.estimated_reasoning_tokens += estimated;
                    }
                }

                // Tool requests
                if let Some(tool_requests) = event.data.get("toolRequests").and_then(|v| v.as_array()) {
                    let count = tool_requests.len() as u32;
                    current_turn_tool_requests += count;
                    for req in tool_requests {
                        if let Some(name) = req.get("name").and_then(|v| v.as_str()) {
                            *metrics.tool_usage.entry(name.to_string()).or_insert(0) += 1;
                        }
                    }
                }

                // Track model usage — only count non-subagent messages
                let is_subagent = event.data.get("parentToolCallId").is_some()
                    && !event.data["parentToolCallId"].is_null();
                if !is_subagent {
                    if let Some(ref model) = current_model {
                        *metrics.models_used.entry(model.clone()).or_insert(0) += 1;
                    }
                }
            }

            "subagent.started" => {
                let tool_call_id = event
                    .data
                    .get("toolCallId")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let agent_name = event
                    .data
                    .get("agentName")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let display_name = event
                    .data
                    .get("agentDisplayName")
                    .and_then(|v| v.as_str())
                    .unwrap_or(&agent_name)
                    .to_string();

                sub_agents_map.insert(
                    tool_call_id.clone(),
                    SubAgent {
                        tool_call_id,
                        agent_type: agent_name,
                        display_name,
                        started_at: ts,
                        completed_at: None,
                        duration_secs: None,
                    },
                );
            }

            "subagent.completed" => {
                if let Some(tool_call_id) = event.data.get("toolCallId").and_then(|v| v.as_str()) {
                    if let Some(agent) = sub_agents_map.get_mut(tool_call_id) {
                        agent.completed_at = ts;
                        if let (Some(start), Some(end)) = (agent.started_at, ts) {
                            agent.duration_secs =
                                Some((end - start).num_milliseconds() as f64 / 1000.0);
                        }
                    }
                }
            }

            "tool.execution_start" => {
                let tool_call_id = event
                    .data
                    .get("toolCallId")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let tool_name = event
                    .data
                    .get("toolName")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                tool_calls_map.insert(
                    tool_call_id.clone(),
                    ToolCall {
                        tool_call_id,
                        tool_name,
                        started_at: ts,
                        completed_at: None,
                    },
                );
            }

            "tool.execution_complete" => {
                if let Some(tool_call_id) = event.data.get("toolCallId").and_then(|v| v.as_str()) {
                    if let Some(tc) = tool_calls_map.get_mut(tool_call_id) {
                        tc.completed_at = ts;
                    }
                }
            }

            "session.shutdown" => {
                ended_at = ts;

                if let Some(v) = event.data.get("totalPremiumRequests").and_then(|v| v.as_u64()) {
                    metrics.total_premium_requests = v as u32;
                }
                if let Some(v) = event.data.get("totalApiDurationMs").and_then(|v| v.as_u64()) {
                    metrics.total_api_duration_ms = v;
                }
                if let Some(changes) = event.data.get("codeChanges") {
                    if let Some(v) = changes.get("linesAdded").and_then(|v| v.as_u64()) {
                        metrics.lines_added = v as u32;
                    }
                    if let Some(v) = changes.get("linesRemoved").and_then(|v| v.as_u64()) {
                        metrics.lines_removed = v as u32;
                    }
                    if let Some(files) = changes.get("filesModified").and_then(|v| v.as_array()) {
                        metrics.files_modified = files.len() as u32;
                    }
                }
            }

            _ => {} // ignore unknown event types
        }
    }

    // Flush any in-progress turn that never got a turn_end
    if let Some(turn_id) = current_turn_id.take() {
        turns.push(Turn {
            turn_id,
            timestamp: current_turn_timestamp.take(),
            output_tokens: current_turn_tokens,
            tool_request_count: current_turn_tool_requests,
            has_reasoning: current_turn_has_reasoning,
        });
    }

    let sub_agents: Vec<SubAgent> = sub_agents_map.into_values().collect();
    let tool_calls: Vec<ToolCall> = tool_calls_map.into_values().collect();

    Ok((
        metrics,
        turns,
        sub_agents,
        tool_calls,
        current_model,
        started_at,
        ended_at,
    ))
}

// ---------------------------------------------------------------------------
// Workspace YAML
// ---------------------------------------------------------------------------

/// Read session metadata from workspace.yaml
fn read_workspace_yaml(path: &Path) -> Result<WorkspaceMetadata> {
    let contents = std::fs::read_to_string(path)?;
    let meta: WorkspaceMetadata = serde_yaml::from_str(&contents)?;
    Ok(meta)
}

// ---------------------------------------------------------------------------
// Session store SQLite fallback
// ---------------------------------------------------------------------------

/// Fallback: read session info from session-store.db
fn read_session_store(session_id: &str) -> Result<Option<SessionStoreInfo>> {
    let home = dirs::home_dir().context("Could not determine home directory")?;
    let db_path = home.join(".copilot").join("session-store.db");

    if !db_path.exists() {
        return Ok(None);
    }

    let conn = rusqlite::Connection::open_with_flags(&db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;

    let mut stmt =
        conn.prepare("SELECT summary, cwd, branch, created_at FROM sessions WHERE id = ?1")?;

    let result = stmt.query_row(rusqlite::params![session_id], |row| {
        Ok(SessionStoreInfo {
            summary: row.get(0)?,
            cwd: row.get(1)?,
            branch: row.get(2)?,
            created_at: row.get(3)?,
        })
    });

    match result {
        Ok(info) => Ok(Some(info)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

// ---------------------------------------------------------------------------
// Config fallback for model
// ---------------------------------------------------------------------------

/// Read the current model from ~/.copilot/config.json
fn read_config_model() -> Option<String> {
    let home = dirs::home_dir()?;
    let config_path = home.join(".copilot").join("config.json");
    let contents = std::fs::read_to_string(config_path).ok()?;
    let config: serde_json::Value = serde_json::from_str(&contents).ok()?;
    config.get("model").and_then(|v| v.as_str()).map(|s| s.to_string())
}
