// Formatting utilities for numbers, durations, and display strings

use chrono::{DateTime, Utc};

/// Format a token count with comma separators (e.g., 159266 → "159,266")
pub fn format_tokens(count: u64) -> String {
    if count == 0 {
        return "0".to_string();
    }
    let s = count.to_string();
    let mut result = String::with_capacity(s.len() + s.len() / 3);
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result
}

/// Format a token count in compact form (e.g., 159266 → "159K", 2400000 → "2.4M")
pub fn format_tokens_compact(count: u64) -> String {
    if count >= 1_000_000 {
        format!("{:.1}M", count as f64 / 1_000_000.0)
    } else if count >= 1_000 {
        format!("{:.0}K", count as f64 / 1_000.0)
    } else {
        count.to_string()
    }
}

/// Format a cost in USD (e.g., 11.94 → "$11.94", 0.03 → "$0.03")
pub fn format_cost(cost: f64) -> String {
    if cost >= 100.0 {
        format!("${:.0}", cost)
    } else if cost >= 1.0 {
        format!("${:.2}", cost)
    } else if cost >= 0.01 {
        format!("${:.2}", cost)
    } else if cost > 0.0 {
        format!("${:.3}", cost)
    } else {
        "$0.00".to_string()
    }
}

/// Format a duration in seconds to human readable (e.g., 2520 → "42m", 7200 → "2h 0m")
pub fn format_duration_secs(secs: f64) -> String {
    if secs < 60.0 {
        format!("{:.0}s", secs)
    } else if secs < 3600.0 {
        format!("{:.0}m", secs / 60.0)
    } else {
        let hours = (secs / 3600.0).floor() as u64;
        let mins = ((secs % 3600.0) / 60.0).floor() as u64;
        format!("{}h {}m", hours, mins)
    }
}

/// Format a duration between two timestamps
pub fn format_duration(start: Option<DateTime<Utc>>, end: Option<DateTime<Utc>>) -> String {
    match (start, end) {
        (Some(s), Some(e)) => {
            let duration = e.signed_duration_since(s);
            format_duration_secs(duration.num_seconds() as f64)
        }
        (Some(s), None) => {
            let duration = Utc::now().signed_duration_since(s);
            format!("{}~", format_duration_secs(duration.num_seconds() as f64))
        }
        _ => "-".to_string(),
    }
}

/// Truncate a string to max_len, adding "…" if truncated
pub fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else if max_len <= 1 {
        "…".to_string()
    } else {
        format!("{}…", &s[..max_len - 1])
    }
}

/// Format a DateTime as a short string (e.g., "Mar 08 21:44")
pub fn format_time_short(dt: Option<DateTime<Utc>>) -> String {
    match dt {
        Some(dt) => dt.format("%b %d %H:%M").to_string(),
        None => "-".to_string(),
    }
}

/// Shorten a session ID for display (first 8 chars)
pub fn short_id(id: &str) -> String {
    if id.len() > 8 {
        id[..8].to_string()
    } else {
        id.to_string()
    }
}
