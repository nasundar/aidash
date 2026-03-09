use std::collections::HashMap;
use crate::data::models::{Session, ModelPricing};

/// Estimate cost for a single session based on its metrics and a pricing table
pub fn estimate_session_cost(session: &mut Session, pricing: &HashMap<String, ModelPricing>) {
    let mut total_cost = 0.0;

    // Calculate cost per model used
    for (model_name, &message_count) in &session.metrics.models_used {
        if let Some(price) = find_pricing(model_name, pricing) {
            // Output tokens are tracked per-session, not per-model
            // Approximate: distribute proportionally by message count
            let total_messages = session.metrics.total_messages.max(1) as f64;
            let model_fraction = message_count as f64 / total_messages;

            let output_tokens = session.metrics.total_output_tokens as f64 * model_fraction;
            let input_tokens = session.metrics.estimated_input_tokens as f64 * model_fraction;

            let output_cost = output_tokens * price.output_per_million / 1_000_000.0;
            let input_cost = input_tokens * price.input_per_million / 1_000_000.0;

            total_cost += output_cost + input_cost;
        }
    }

    // If no model info, use the session's primary model
    if session.metrics.models_used.is_empty() {
        if let Some(ref model) = session.model {
            if let Some(price) = find_pricing(model, pricing) {
                let output_cost = session.metrics.total_output_tokens as f64 * price.output_per_million / 1_000_000.0;
                let input_cost = session.metrics.estimated_input_tokens as f64 * price.input_per_million / 1_000_000.0;
                total_cost = output_cost + input_cost;
            }
        }
    }

    session.metrics.estimated_cost_usd = total_cost;
}

/// Find pricing for a model, trying exact match first then prefix matching
fn find_pricing<'a>(model_name: &str, pricing: &'a HashMap<String, ModelPricing>) -> Option<&'a ModelPricing> {
    // Exact match
    if let Some(p) = pricing.get(model_name) {
        return Some(p);
    }
    // Try without version suffixes (e.g. "claude-opus-4.6-1m" -> "claude-opus-4.6")
    for (key, value) in pricing {
        if model_name.starts_with(key) || key.starts_with(model_name) {
            return Some(value);
        }
    }
    None
}

/// Estimate cost for all sessions
pub fn estimate_all_costs(sessions: &mut [Session], pricing: &HashMap<String, ModelPricing>) {
    for session in sessions.iter_mut() {
        estimate_session_cost(session, pricing);
    }
}

/// Check if a model is premium based on pricing table
#[allow(dead_code)]
pub fn is_premium_model(model_name: &str, pricing: &HashMap<String, ModelPricing>) -> bool {
    find_pricing(model_name, pricing).map_or(true, |p| p.is_premium)
}

/// Get total cost across all sessions
#[allow(dead_code)]
pub fn total_cost(sessions: &[Session]) -> f64 {
    sessions.iter().map(|s| s.metrics.estimated_cost_usd).sum()
}

/// Get total output tokens across all sessions
#[allow(dead_code)]
pub fn total_output_tokens(sessions: &[Session]) -> u64 {
    sessions.iter().map(|s| s.metrics.total_output_tokens).sum()
}

/// Get total premium requests across all sessions
#[allow(dead_code)]
pub fn total_premium_requests(sessions: &[Session]) -> u32 {
    sessions.iter().map(|s| s.metrics.total_premium_requests).sum()
}
