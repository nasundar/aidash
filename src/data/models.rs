use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Which AI assistant produced the data
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Source {
    Copilot,
    Claude,
}

/// A unified session from either Copilot or Claude
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Session {
    pub id: String,
    pub source: Source,
    pub summary: Option<String>,
    pub model: Option<String>,
    pub cwd: Option<String>,
    pub branch: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub ended_at: Option<DateTime<Utc>>,
    pub is_active: bool,
    pub metrics: SessionMetrics,
    pub turns: Vec<Turn>,
    pub sub_agents: Vec<SubAgent>,
    pub tool_calls: Vec<ToolCall>,
}

/// Aggregated metrics for a session
#[derive(Debug, Clone, Default)]
pub struct SessionMetrics {
    pub total_output_tokens: u64,
    pub estimated_input_tokens: u64,
    pub estimated_reasoning_tokens: u64,
    pub total_turns: u32,
    pub total_messages: u32,
    pub total_tool_calls: u32,
    pub total_sub_agents: u32,
    pub total_premium_requests: u32,
    pub total_api_duration_ms: u64,
    pub lines_added: u32,
    pub lines_removed: u32,
    pub files_modified: u32,
    pub estimated_cost_usd: f64,
    pub models_used: HashMap<String, u32>,
    pub tool_usage: HashMap<String, u32>,
}

/// A single conversation turn
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Turn {
    pub turn_id: String,
    pub timestamp: Option<DateTime<Utc>>,
    pub output_tokens: u64,
    pub tool_request_count: u32,
    pub has_reasoning: bool,
}

/// A sub-agent task (explore, general-purpose, code-review, etc.)
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct SubAgent {
    pub tool_call_id: String,
    pub agent_type: String,
    pub display_name: String,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration_secs: Option<f64>,
}

/// A tool execution record
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ToolCall {
    pub tool_call_id: String,
    pub tool_name: String,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// Raw event from Copilot events.jsonl
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct CopilotEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub data: serde_json::Value,
    pub id: String,
    pub timestamp: String,
    #[serde(rename = "parentId")]
    pub parent_id: Option<String>,
}

/// Raw event from Claude Code session JSONL
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct ClaudeEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    #[serde(rename = "sessionId")]
    pub session_id: Option<String>,
    pub timestamp: Option<String>,
    pub uuid: Option<String>,
    #[serde(rename = "parentUuid")]
    pub parent_uuid: Option<String>,
    pub message: Option<serde_json::Value>,
}

/// Model pricing info
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ModelPricing {
    pub input_per_million: f64,
    pub output_per_million: f64,
    pub is_premium: bool,
}
