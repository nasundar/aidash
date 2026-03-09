use std::collections::HashMap;

use chrono::NaiveDate;
use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::data::models::Session;
use crate::tui::theme;
use crate::util;

pub struct TrendsView;

impl TrendsView {
    pub fn render(sessions: &[Session], frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // header
                Constraint::Length(12), // daily usage line chart section
                Constraint::Length(14), // model distribution + top tools side-by-side (with spacing)
                Constraint::Min(5),    // session timeline
                Constraint::Length(1), // footer
            ])
            .split(area);

        render_header(sessions, frame, chunks[0]);
        render_daily_sparklines(sessions, frame, chunks[1]);
        render_model_and_tools(sessions, frame, chunks[2]);
        render_session_timeline(sessions, frame, chunks[3]);
        render_footer(frame, chunks[4]);
    }
}

// ---------------------------------------------------------------------------
// Header
// ---------------------------------------------------------------------------

fn render_header(sessions: &[Session], frame: &mut Frame, area: Rect) {
    let total_cost: f64 = sessions.iter().map(|s| s.metrics.estimated_cost_usd).sum();
    let date_range = date_range_label(sessions);

    let header_block = Block::bordered()
        .title(" Trends & Analytics ")
        .title_style(theme::header_style())
        .border_style(theme::border_style());
    let inner = header_block.inner(area);
    frame.render_widget(header_block, area);

    let stats = Line::from(vec![
        Span::styled(" Sessions: ", theme::dim_style()),
        Span::styled(
            format!("{}", sessions.len()),
            Style::default().fg(theme::text()).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" │ Total Cost: ", theme::dim_style()),
        Span::styled(
            util::format_cost(total_cost),
            Style::default().fg(theme::cost_color(total_cost)),
        ),
        Span::styled(" │ ", theme::dim_style()),
        Span::styled(date_range, Style::default().fg(theme::dim())),
    ]);
    frame.render_widget(Paragraph::new(stats), inner);
}

fn date_range_label(sessions: &[Session]) -> String {
    let dates: Vec<_> = sessions.iter().filter_map(|s| s.started_at).collect();
    if dates.is_empty() {
        return "No sessions".to_string();
    }
    let min = dates.iter().min().unwrap();
    let max = dates.iter().max().unwrap();
    format!(
        "{} — {}",
        min.format("%b %d, %Y"),
        max.format("%b %d, %Y")
    )
}

// ---------------------------------------------------------------------------
// Daily sparklines
// ---------------------------------------------------------------------------

fn render_daily_sparklines(sessions: &[Session], frame: &mut Frame, area: Rect) {
    let today = chrono::Utc::now().date_naive();
    let days = 30usize;
    let (token_per_day, cost_per_day) = daily_aggregates(sessions, today, days);

    let max_tok = token_per_day
        .iter()
        .cloned()
        .fold(0.0_f64, f64::max)
        .max(1.0);
    let max_cost = cost_per_day
        .iter()
        .cloned()
        .fold(0.0_f64, f64::max)
        .max(0.01);

    // Normalize cost to token scale for overlay
    let tok_data: Vec<(f64, f64)> = token_per_day
        .iter()
        .enumerate()
        .map(|(i, &v)| (i as f64, v))
        .collect();
    let cost_data: Vec<(f64, f64)> = cost_per_day
        .iter()
        .enumerate()
        .map(|(i, &v)| (i as f64, v / max_cost * max_tok))
        .collect();

    let datasets = vec![
        Dataset::default()
            .name(format!(
                "Tokens (peak: {})",
                util::format_tokens_compact(max_tok as u64)
            ))
            .marker(ratatui::symbols::Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(theme::amber()))
            .data(&tok_data),
        Dataset::default()
            .name(format!("Cost (peak: {})", util::format_cost(max_cost)))
            .marker(ratatui::symbols::Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(theme::gold()))
            .data(&cost_data),
    ];

    let x_labels = vec![
        Span::styled(
            "30d ago",
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::raw("15d"),
        Span::styled("Today", Style::default().add_modifier(Modifier::BOLD)),
    ];
    let y_labels = vec![
        Span::styled("0", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(util::format_tokens_compact((max_tok / 2.0) as u64)),
        Span::styled(
            util::format_tokens_compact(max_tok as u64),
            Style::default().add_modifier(Modifier::BOLD),
        ),
    ];

    let chart = Chart::new(datasets)
        .block(
            Block::bordered()
                .title(Span::styled(
                    " Daily Activity (30d) ",
                    Style::default()
                        .fg(theme::gold())
                        .add_modifier(Modifier::BOLD),
                ))
                .border_style(theme::border_style()),
        )
        .x_axis(
            Axis::default()
                .style(Style::default().fg(theme::dim()))
                .bounds([0.0, (days - 1) as f64])
                .labels(x_labels),
        )
        .y_axis(
            Axis::default()
                .style(Style::default().fg(theme::dim()))
                .bounds([0.0, max_tok * 1.1])
                .labels(y_labels),
        )
        .legend_position(Some(LegendPosition::TopRight))
        .hidden_legend_constraints((Constraint::Ratio(1, 3), Constraint::Ratio(1, 4)));

    frame.render_widget(chart, area);
}

fn daily_aggregates(
    sessions: &[Session],
    today: NaiveDate,
    days: usize,
) -> (Vec<f64>, Vec<f64>) {
    let mut token_map: HashMap<NaiveDate, f64> = HashMap::new();
    let mut cost_map: HashMap<NaiveDate, f64> = HashMap::new();

    for s in sessions {
        if let Some(dt) = s.started_at {
            let day = dt.date_naive();
            *token_map.entry(day).or_default() += s.metrics.total_output_tokens as f64;
            *cost_map.entry(day).or_default() += s.metrics.estimated_cost_usd;
        }
    }

    let mut tokens = Vec::with_capacity(days);
    let mut costs = Vec::with_capacity(days);
    for i in (0..days).rev() {
        let d = today - chrono::Duration::days(i as i64);
        tokens.push(*token_map.get(&d).unwrap_or(&0.0));
        costs.push(*cost_map.get(&d).unwrap_or(&0.0));
    }
    (tokens, costs)
}

// ---------------------------------------------------------------------------
// Model distribution + Top tools
// ---------------------------------------------------------------------------

fn render_model_and_tools(sessions: &[Session], frame: &mut Frame, area: Rect) {
    let halves = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    render_model_distribution(sessions, frame, halves[0]);
    render_top_tools(sessions, frame, halves[1]);
}

fn render_model_distribution(sessions: &[Session], frame: &mut Frame, area: Rect) {
    let block = Block::bordered()
        .title(" Model Distribution ")
        .title_style(Style::default().fg(theme::gold()).add_modifier(Modifier::BOLD))
        .border_style(theme::border_style());

    let mut aggregate: HashMap<String, u32> = HashMap::new();
    for s in sessions {
        for (model, count) in &s.metrics.models_used {
            *aggregate.entry(model.clone()).or_default() += count;
        }
    }

    let mut models: Vec<(String, u32)> = aggregate.into_iter().collect();
    models.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

    if models.is_empty() {
        let inner = block.inner(area);
        frame.render_widget(block, area);
        frame.render_widget(
            Paragraph::new(Span::styled("  No model data", theme::dim_style())),
            inner,
        );
        return;
    }

    let inner_height = block.inner(area).height as usize;
    let max_bars = inner_height / 2; // bar_width(1) + bar_gap(1)

    let bar_data: Vec<Bar> = models
        .iter()
        .take(max_bars)
        .map(|(model, count)| {
            Bar::default()
                .value(*count as u64)
                .label(Line::from(theme::short_model_name(model)))
                .style(Style::default().fg(theme::model_color(model)))
                .value_style(
                    Style::default()
                        .fg(theme::text())
                        .add_modifier(Modifier::BOLD),
                )
                .text_value(format!("{} msgs", count))
        })
        .collect();

    let barchart = BarChart::default()
        .block(block)
        .data(BarGroup::default().bars(&bar_data))
        .bar_width(1)
        .bar_gap(1)
        .direction(Direction::Horizontal)
        .value_style(Style::default().fg(theme::text()));

    frame.render_widget(barchart, area);
}

fn render_top_tools(sessions: &[Session], frame: &mut Frame, area: Rect) {
    let block = Block::bordered()
        .title(" Top Tools (All Sessions) ")
        .title_style(Style::default().fg(theme::gold()).add_modifier(Modifier::BOLD))
        .border_style(theme::border_style());

    let mut aggregate: HashMap<String, u32> = HashMap::new();
    for s in sessions {
        for (tool, count) in &s.metrics.tool_usage {
            *aggregate.entry(tool.clone()).or_default() += count;
        }
    }

    let mut tools: Vec<(String, u32)> = aggregate.into_iter().collect();
    tools.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

    if tools.is_empty() {
        let inner = block.inner(area);
        frame.render_widget(block, area);
        frame.render_widget(
            Paragraph::new(Span::styled("  No tool data", theme::dim_style())),
            inner,
        );
        return;
    }

    let inner_height = block.inner(area).height as usize;
    let max_bars = (inner_height / 2).min(10);

    let bar_data: Vec<Bar> = tools
        .iter()
        .take(max_bars)
        .map(|(tool, count)| {
            Bar::default()
                .value(*count as u64)
                .label(Line::from(tool.as_str()))
                .style(Style::default().fg(theme::amber()))
                .value_style(
                    Style::default()
                        .fg(theme::text())
                        .add_modifier(Modifier::BOLD),
                )
                .text_value(count.to_string())
        })
        .collect();

    let barchart = BarChart::default()
        .block(block)
        .data(BarGroup::default().bars(&bar_data))
        .bar_width(1)
        .bar_gap(1)
        .direction(Direction::Horizontal)
        .value_style(
            Style::default()
                .fg(theme::gold())
                .add_modifier(Modifier::BOLD),
        );

    frame.render_widget(barchart, area);
}

// ---------------------------------------------------------------------------
// Session timeline
// ---------------------------------------------------------------------------

fn render_session_timeline(sessions: &[Session], frame: &mut Frame, area: Rect) {
    let block = Block::bordered()
        .title(" Session Timeline ")
        .title_style(Style::default().fg(theme::gold()).add_modifier(Modifier::BOLD))
        .border_style(theme::border_style());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Sort sessions by started_at descending (most recent first)
    let mut sorted: Vec<&Session> = sessions.iter().collect();
    sorted.sort_by(|a, b| b.started_at.cmp(&a.started_at));

    let max_duration = sorted
        .iter()
        .map(|s| session_duration_secs(s))
        .fold(0.0_f64, f64::max)
        .max(1.0);

    // How much space for the timeline bar
    let date_col = 8u16;
    let meta_col = 50u16; // name + model + cost + duration
    let bar_budget = inner.width.saturating_sub(date_col + meta_col + 6) as usize;

    let max_timeline_rows = (inner.height as usize) / 2;
    let mut lines: Vec<Line> = Vec::new();
    for session in sorted.iter().take(max_timeline_rows) {
        let date_str = session
            .started_at
            .map(|dt| dt.format("%b %d").to_string())
            .unwrap_or_else(|| "  -   ".to_string());

        let dur_secs = session_duration_secs(session);
        let bar_len = if max_duration > 0.0 {
            ((dur_secs / max_duration) * bar_budget as f64).ceil() as usize
        } else {
            0
        }
        .max(1)
        .min(bar_budget);

        let bar = format!("●{}●", "━".repeat(bar_len.saturating_sub(2).max(1)));
        let model_name = session.model.as_deref().unwrap_or("-");
        let model_short = theme::short_model_name(model_name);
        let cost = session.metrics.estimated_cost_usd;
        let fallback = util::short_id(&session.id);
        let name = session
            .summary
            .as_deref()
            .unwrap_or(&fallback);
        let dur_str = util::format_duration(session.started_at, session.ended_at);

        lines.push(Line::from(vec![
            Span::styled(
                format!(" {:<7}", date_str),
                Style::default().fg(theme::dim()),
            ),
            Span::styled(
                format!("{:<width$}", bar, width = bar_budget + 2),
                Style::default().fg(theme::model_color(model_name)),
            ),
            Span::styled(
                format!(" {}", util::truncate(name, 22)),
                Style::default().fg(theme::text()),
            ),
            Span::styled(
                format!("  {:<14}", model_short),
                Style::default().fg(theme::model_color(model_name)),
            ),
            Span::styled(
                format!(" {:>7}", util::format_cost(cost)),
                Style::default().fg(theme::cost_color(cost)),
            ),
            Span::styled(
                format!(" {:>5}", dur_str),
                Style::default().fg(theme::dim()),
            ),
        ]));
        lines.push(Line::from("")); // breathing room
    }

    if lines.is_empty() {
        frame.render_widget(
            Paragraph::new(Span::styled("  No sessions", theme::dim_style())),
            inner,
        );
    } else {
        frame.render_widget(Paragraph::new(lines), inner);
    }
}

fn session_duration_secs(session: &Session) -> f64 {
    match (session.started_at, session.ended_at) {
        (Some(s), Some(e)) => e.signed_duration_since(s).num_seconds() as f64,
        (Some(s), None) => chrono::Utc::now().signed_duration_since(s).num_seconds() as f64,
        _ => 0.0,
    }
}

// ---------------------------------------------------------------------------
// Footer
// ---------------------------------------------------------------------------

fn render_footer(frame: &mut Frame, area: Rect) {
    let footer = Paragraph::new(Line::from(vec![
        Span::styled("Esc", Style::default().fg(theme::gold())),
        Span::styled(": back │ ", theme::dim_style()),
        Span::styled("q", Style::default().fg(theme::gold())),
        Span::styled(": quit", theme::dim_style()),
    ]));
    frame.render_widget(footer, area);
}
