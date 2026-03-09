use std::collections::HashMap;
use std::path::PathBuf;
use anyhow::{Context, Result};
use crate::data::models::ModelPricing;

/// Get the aidash config directory path
pub fn config_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".aidash"))
}

/// Get the pricing file path
#[allow(dead_code)]
pub fn pricing_path() -> Option<PathBuf> {
    config_dir().map(|d| d.join("pricing.json"))
}

/// Default pricing table — used as seed for pricing.json
fn default_pricing() -> HashMap<String, ModelPricing> {
    let mut m = HashMap::new();
    // Anthropic Claude models
    m.insert("claude-opus-4.6-1m".into(), ModelPricing { input_per_million: 15.0, output_per_million: 75.0, is_premium: true });
    m.insert("claude-opus-4.6".into(), ModelPricing { input_per_million: 15.0, output_per_million: 75.0, is_premium: true });
    m.insert("claude-opus-4.5".into(), ModelPricing { input_per_million: 15.0, output_per_million: 75.0, is_premium: true });
    m.insert("claude-sonnet-4.6".into(), ModelPricing { input_per_million: 3.0, output_per_million: 15.0, is_premium: true });
    m.insert("claude-sonnet-4.5".into(), ModelPricing { input_per_million: 3.0, output_per_million: 15.0, is_premium: true });
    m.insert("claude-sonnet-4".into(), ModelPricing { input_per_million: 3.0, output_per_million: 15.0, is_premium: true });
    m.insert("claude-haiku-4.5".into(), ModelPricing { input_per_million: 0.8, output_per_million: 4.0, is_premium: false });
    // OpenAI GPT models
    m.insert("gpt-5.4".into(), ModelPricing { input_per_million: 5.0, output_per_million: 25.0, is_premium: true });
    m.insert("gpt-5.3-codex".into(), ModelPricing { input_per_million: 5.0, output_per_million: 25.0, is_premium: true });
    m.insert("gpt-5.2-codex".into(), ModelPricing { input_per_million: 2.5, output_per_million: 10.0, is_premium: true });
    m.insert("gpt-5.2".into(), ModelPricing { input_per_million: 2.5, output_per_million: 10.0, is_premium: true });
    m.insert("gpt-5.1-codex-max".into(), ModelPricing { input_per_million: 5.0, output_per_million: 25.0, is_premium: true });
    m.insert("gpt-5.1-codex".into(), ModelPricing { input_per_million: 2.5, output_per_million: 10.0, is_premium: true });
    m.insert("gpt-5.1".into(), ModelPricing { input_per_million: 2.5, output_per_million: 10.0, is_premium: true });
    m.insert("gpt-5.1-codex-mini".into(), ModelPricing { input_per_million: 0.5, output_per_million: 2.0, is_premium: false });
    m.insert("gpt-5-mini".into(), ModelPricing { input_per_million: 0.3, output_per_million: 1.2, is_premium: false });
    m.insert("gpt-4.1".into(), ModelPricing { input_per_million: 2.0, output_per_million: 8.0, is_premium: false });
    // Google Gemini models
    m.insert("gemini-3-pro-preview".into(), ModelPricing { input_per_million: 1.25, output_per_million: 5.0, is_premium: true });
    m
}

/// Initialize the pricing file if it doesn't exist
pub fn init_pricing() -> Result<PathBuf> {
    let dir = config_dir().context("Could not determine home directory")?;
    std::fs::create_dir_all(&dir)?;
    let path = dir.join("pricing.json");
    if !path.exists() {
        let pricing = default_pricing();
        let json = serde_json::to_string_pretty(&pricing)?;
        std::fs::write(&path, json)?;
    }
    Ok(path)
}

/// Load pricing from ~/.aidash/pricing.json, creating it if needed
pub fn load_pricing() -> HashMap<String, ModelPricing> {
    // Ensure pricing file exists
    if let Ok(path) = init_pricing() {
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(pricing) = serde_json::from_str::<HashMap<String, ModelPricing>>(&content) {
                return pricing;
            }
        }
    }
    // Absolute fallback
    default_pricing()
}

/// Update pricing by merging new entries into existing file.
/// Starts with defaults, then overlays existing user customizations so user edits are preserved.
pub fn update_pricing() -> Result<String> {
    let dir = config_dir().context("Could not determine home directory")?;
    std::fs::create_dir_all(&dir)?;
    let path = dir.join("pricing.json");

    // Load existing custom pricing
    let existing: HashMap<String, ModelPricing> = if path.exists() {
        let content = std::fs::read_to_string(&path)?;
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        HashMap::new()
    };

    // Start with defaults, overlay existing (user customizations win)
    let mut merged = default_pricing();
    merged.extend(existing);

    let json = serde_json::to_string_pretty(&merged)?;
    std::fs::write(&path, &json)?;

    Ok(format!("Updated {} with {} models at {}", path.display(), merged.len(), chrono::Utc::now().format("%Y-%m-%d %H:%M UTC")))
}
