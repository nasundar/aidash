mod app;
mod config;
mod util;
mod data;
mod cost;
mod tui;

use clap::{Parser, Subcommand};
use crate::app::App;

#[derive(Parser)]
#[command(name = "aidash", about = "AI Coding Assistant Token & Cost Dashboard")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Data source filter (all, copilot, claude)
    #[arg(long, default_value = "all")]
    source: String,

    /// Use light theme
    #[arg(long)]
    light: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// List sessions as a table (non-interactive)
    List {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Show details for a specific session
    Session {
        /// Session ID (or prefix)
        id: String,
    },
    /// Show cost summary
    Cost {
        /// Only include sessions since this date (YYYY-MM-DD)
        #[arg(long)]
        since: Option<String>,
    },
    /// Initialize or reset pricing configuration
    Init,
    /// Update pricing with latest known model prices
    UpdatePricing,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Handle config-only commands before loading sessions
    match &cli.command {
        Some(Commands::Init) => {
            match crate::config::init_pricing() {
                Ok(path) => println!("Pricing initialized at: {}", path.display()),
                Err(e) => eprintln!("Error: {}", e),
            }
            return Ok(());
        }
        Some(Commands::UpdatePricing) => {
            match crate::config::update_pricing() {
                Ok(msg) => println!("{}", msg),
                Err(e) => eprintln!("Error updating pricing: {}", e),
            }
            return Ok(());
        }
        _ => {}
    }

    let mut sessions = Vec::new();
    if cli.source != "claude" {
        sessions.extend(crate::data::copilot::load_sessions().unwrap_or_default());
    }
    if cli.source != "copilot" {
        sessions.extend(crate::data::claude::load_sessions().unwrap_or_default());
    }

    let pricing = crate::config::load_pricing();
    crate::cost::estimator::estimate_all_costs(&mut sessions, &pricing);

    // Sort by start time, newest first
    sessions.sort_by(|a, b| b.started_at.cmp(&a.started_at));

    match cli.command {
        None => {
            if cli.light {
                crate::tui::theme::toggle_theme();
            }
            run_tui(sessions)?;
        }
        Some(Commands::List { json }) => list_sessions(&sessions, json),
        Some(Commands::Cost { since }) => show_cost(&sessions, since),
        Some(Commands::Session { id }) => show_session(&sessions, &id),
        Some(Commands::Init) | Some(Commands::UpdatePricing) => unreachable!(),
    }

    Ok(())
}

fn run_tui(sessions: Vec<data::models::Session>) -> anyhow::Result<()> {
    crossterm::terminal::enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    crossterm::execute!(stdout, crossterm::terminal::EnterAlternateScreen)?;
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = ratatui::Terminal::new(backend)?;

    let mut app = App::new(sessions);
    let result = app.run(&mut terminal);

    // Restore terminal even if app errored
    crossterm::terminal::disable_raw_mode()?;
    crossterm::execute!(std::io::stdout(), crossterm::terminal::LeaveAlternateScreen)?;

    result.map_err(Into::into)
}

fn list_sessions(sessions: &[data::models::Session], json: bool) {
    if json {
        // Simple JSON array output
        let entries: Vec<serde_json::Value> = sessions
            .iter()
            .map(|s| {
                serde_json::json!({
                    "id": s.id,
                    "source": format!("{:?}", s.source),
                    "summary": s.summary,
                    "model": s.model,
                    "tokens_out": s.metrics.total_output_tokens,
                    "cost_usd": s.metrics.estimated_cost_usd,
                    "turns": s.metrics.total_turns,
                    "agents": s.metrics.total_sub_agents,
                    "started_at": s.started_at.map(|d| d.to_rfc3339()),
                    "is_active": s.is_active,
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&entries).unwrap_or_default());
        return;
    }

    // Formatted table
    println!(
        "{:<5} {:<40} {:<18} {:>10} {:>10} {:>6} {:>7} {:>10}",
        "#", "Session Name", "Model", "Tokens", "Cost", "Turns", "Agents", "Duration"
    );
    println!("{}", "-".repeat(110));
    for (i, s) in sessions.iter().enumerate() {
        let fallback = util::short_id(&s.id);
        let name = s
            .summary
            .as_deref()
            .unwrap_or(&fallback);
        let model = s.model.as_deref().map(tui::theme::short_model_name).unwrap_or_else(|| "-".to_string());
        println!(
            "{:<5} {:<40} {:<18} {:>10} {:>10} {:>6} {:>7} {:>10}",
            i + 1,
            util::truncate(name, 38),
            model,
            util::format_tokens_compact(s.metrics.total_output_tokens),
            util::format_cost(s.metrics.estimated_cost_usd),
            s.metrics.total_turns,
            s.metrics.total_sub_agents,
            util::format_duration(s.started_at, s.ended_at),
        );
    }
}

fn show_cost(sessions: &[data::models::Session], since: Option<String>) {
    let filtered: Vec<&data::models::Session> = if let Some(ref since_str) = since {
        if let Ok(date) = chrono::NaiveDate::parse_from_str(since_str, "%Y-%m-%d") {
            let since_dt = date.and_hms_opt(0, 0, 0)
                .unwrap()
                .and_utc();
            sessions
                .iter()
                .filter(|s| s.started_at.map_or(false, |d| d >= since_dt))
                .collect()
        } else {
            eprintln!("Invalid date format. Use YYYY-MM-DD.");
            return;
        }
    } else {
        sessions.iter().collect()
    };

    let total_tokens: u64 = filtered.iter().map(|s| s.metrics.total_output_tokens).sum();
    let total_cost: f64 = filtered.iter().map(|s| s.metrics.estimated_cost_usd).sum();
    let premium: u32 = filtered.iter().map(|s| s.metrics.total_premium_requests).sum();

    println!("aidash — Cost Summary");
    println!("=====================");
    if let Some(ref s) = since {
        println!("Since: {}", s);
    }
    println!("Sessions:          {}", filtered.len());
    println!("Total Tokens Out:  {}", util::format_tokens(total_tokens));
    println!("Estimated Cost:    {}", util::format_cost(total_cost));
    println!("Premium Requests:  {}", premium);
}

fn show_session(sessions: &[data::models::Session], id: &str) {
    let session = sessions.iter().find(|s| s.id == id || s.id.starts_with(id));
    match session {
        Some(s) => {
            let name = s.summary.as_deref().unwrap_or("(unnamed)");
            let model = s.model.as_deref().unwrap_or("-");
            println!("Session: {}", s.id);
            println!("Name:    {}", name);
            println!("Source:  {:?}", s.source);
            println!("Model:   {}", model);
            println!("Started: {}", util::format_time_short(s.started_at));
            println!("Ended:   {}", util::format_time_short(s.ended_at));
            println!("Active:  {}", if s.is_active { "yes" } else { "no" });
            println!();
            println!("Metrics:");
            println!("  Tokens Out:   {}", util::format_tokens(s.metrics.total_output_tokens));
            println!("  Est. Cost:    {}", util::format_cost(s.metrics.estimated_cost_usd));
            println!("  Turns:        {}", s.metrics.total_turns);
            println!("  Sub-agents:   {}", s.metrics.total_sub_agents);
            println!("  Tool Calls:   {}", s.metrics.total_tool_calls);
            println!("  Premium Reqs: {}", s.metrics.total_premium_requests);
            println!("  Duration:     {}", util::format_duration(s.started_at, s.ended_at));
            if let Some(ref cwd) = s.cwd {
                println!("  CWD:          {}", cwd);
            }
            if let Some(ref branch) = s.branch {
                println!("  Branch:       {}", branch);
            }
        }
        None => {
            eprintln!("Session not found: {}", id);
        }
    }
}
