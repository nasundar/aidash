use ratatui::prelude::*;
use ratatui::symbols;
use ratatui::widgets::*;
use crate::data::models::Session;
use crate::tui::theme;
use crate::util;

pub struct LiveView;

impl LiveView {
    pub fn render(session: &Session, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(4),  // header with live stats
                Constraint::Min(10),   // body (4 quadrants)
                Constraint::Length(1), // footer
            ])
            .split(area);

        render_live_header(session, frame, chunks[0]);
        render_quadrants(session, frame, chunks[1]);
        render_footer(frame, chunks[2]);
    }
}

fn render_live_header(session: &Session, frame: &mut Frame, area: Rect) {
    let summary = session.summary.as_deref().unwrap_or("Active Session");
    let model = session.model.as_deref().unwrap_or("-");
    let m = &session.metrics;
    let duration = util::format_duration(session.started_at, session.ended_at);

    // Calculate tokens per minute rate
    let elapsed_mins = session
        .started_at
        .map(|s| chrono::Utc::now().signed_duration_since(s).num_seconds() as f64 / 60.0)
        .unwrap_or(1.0)
        .max(1.0);
    let tok_per_min = m.total_output_tokens as f64 / elapsed_mins;

    let title = Line::from(vec![
        Span::styled(
            " LIVE ",
            Style::default()
                .fg(theme::lime())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("● ", Style::default().fg(theme::lime())),
        Span::styled(util::truncate(summary, 40), Style::default().fg(theme::text())),
        Span::styled(" ─ ", theme::dim_style()),
        Span::styled(theme::short_model_name(model), theme::model_style(model)),
    ]);

    let stats = Line::from(vec![
        Span::styled(" Tokens: ", theme::dim_style()),
        Span::styled(
            util::format_tokens(m.total_output_tokens),
            Style::default()
                .fg(theme::text())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(" (+{:.0}/min)", tok_per_min),
            Style::default().fg(theme::lime()),
        ),
        Span::styled("  │  Cost: ", theme::dim_style()),
        Span::styled(
            util::format_cost(m.estimated_cost_usd),
            Style::default()
                .fg(theme::cost_color(m.estimated_cost_usd))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("  │  Turns: ", theme::dim_style()),
        Span::styled(
            format!("{}", m.total_turns),
            Style::default().fg(theme::text()),
        ),
        Span::styled("  │  Agents: ", theme::dim_style()),
        Span::styled(
            format!("{}", m.total_sub_agents),
            Style::default().fg(theme::amber()),
        ),
        Span::styled("  │  ", theme::dim_style()),
        Span::styled(duration, Style::default().fg(theme::text())),
    ]);

    let header = Paragraph::new(vec![stats]).block(
        Block::bordered()
            .title(title)
            .border_style(Style::default().fg(theme::lime())),
    );
    frame.render_widget(header, area);
}

fn render_quadrants(session: &Session, frame: &mut Frame, area: Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let top = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(rows[0]);

    let bottom = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(rows[1]);

    render_token_chart(session, frame, top[0]);
    render_tool_chart(session, frame, top[1]);
    render_model_chart(session, frame, bottom[0]);
    render_agent_panel(session, frame, bottom[1]);
}

fn render_token_chart(session: &Session, frame: &mut Frame, area: Rect) {
    let turn_tokens: Vec<u64> = session.turns.iter().map(|t| t.output_tokens).collect();
    if turn_tokens.is_empty() {
        let block = Block::bordered()
            .title(Span::styled(" Token Accumulation ", theme::header_style()))
            .border_style(theme::border_style());
        frame.render_widget(block, area);
        return;
    }

    let max_tok = turn_tokens.iter().copied().max().unwrap_or(1) as f64;
    let num_turns = turn_tokens.len() as f64;

    // Per-turn tokens as (turn_index, tokens) pairs
    let per_turn_data: Vec<(f64, f64)> = turn_tokens
        .iter()
        .enumerate()
        .map(|(i, &t)| (i as f64, t as f64))
        .collect();

    // Cumulative tokens (normalized to same Y scale)
    let mut cumulative = 0u64;
    let total_tokens: u64 = turn_tokens.iter().sum();
    let cum_data: Vec<(f64, f64)> = turn_tokens
        .iter()
        .enumerate()
        .map(|(i, &t)| {
            cumulative += t;
            (i as f64, cumulative as f64 / total_tokens.max(1) as f64 * max_tok)
        })
        .collect();

    let datasets = vec![
        Dataset::default()
            .name("Per Turn")
            .marker(symbols::Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(theme::amber()))
            .data(&per_turn_data),
        Dataset::default()
            .name("Cumulative")
            .marker(symbols::Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(theme::gold()))
            .data(&cum_data),
    ];

    // X-axis labels
    let mid = (num_turns / 2.0).round();
    let x_labels = vec![
        Span::styled("T1", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(format!("T{:.0}", mid)),
        Span::styled(
            format!("T{:.0}", num_turns),
            Style::default().add_modifier(Modifier::BOLD),
        ),
    ];

    // Y-axis labels
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
                .title(Span::styled(" Token Accumulation ", theme::header_style()))
                .border_style(theme::border_style()),
        )
        .x_axis(
            Axis::default()
                .title("Turn")
                .style(Style::default().fg(theme::dim()))
                .bounds([0.0, (num_turns - 1.0).max(1.0)])
                .labels(x_labels),
        )
        .y_axis(
            Axis::default()
                .title("Tokens")
                .style(Style::default().fg(theme::dim()))
                .bounds([0.0, max_tok * 1.1])
                .labels(y_labels),
        )
        .legend_position(Some(LegendPosition::TopRight))
        .hidden_legend_constraints((Constraint::Ratio(1, 3), Constraint::Ratio(1, 4)));

    frame.render_widget(chart, area);
}

fn render_tool_chart(session: &Session, frame: &mut Frame, area: Rect) {
    let mut tools: Vec<(&String, &u32)> = session.metrics.tool_usage.iter().collect();
    tools.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));
    let available = area.height.saturating_sub(2) as usize;

    let bar_data: Vec<Bar> = tools
        .iter()
        .take(available)
        .map(|(name, count)| {
            Bar::default()
                .value(**count as u64)
                .label(Line::from(util::truncate(name, 16)))
                .style(Style::default().fg(theme::amber()))
                .value_style(Style::default().fg(theme::text()))
                .text_value(format!("{}", count))
        })
        .collect();

    let chart = BarChart::default()
        .block(
            Block::bordered()
                .title(Span::styled(" Tool Activity ", theme::header_style()))
                .border_style(theme::border_style()),
        )
        .data(BarGroup::default().bars(&bar_data))
        .bar_width(1)
        .bar_gap(0)
        .direction(Direction::Horizontal);
    frame.render_widget(chart, area);
}

fn render_model_chart(session: &Session, frame: &mut Frame, area: Rect) {
    let mut models: Vec<(&String, &u32)> = session.metrics.models_used.iter().collect();
    models.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));

    let bar_data: Vec<Bar> = models
        .iter()
        .map(|(model, count)| {
            Bar::default()
                .value(**count as u64)
                .label(Line::from(theme::short_model_name(model)))
                .style(Style::default().fg(theme::model_color(model)))
                .value_style(Style::default().fg(theme::text()))
                .text_value(format!("{} msgs", count))
        })
        .collect();

    let chart = BarChart::default()
        .block(
            Block::bordered()
                .title(Span::styled(" Models ", theme::header_style()))
                .border_style(theme::border_style()),
        )
        .data(BarGroup::default().bars(&bar_data))
        .bar_width(1)
        .bar_gap(1)
        .direction(Direction::Horizontal);
    frame.render_widget(chart, area);
}

fn render_agent_panel(session: &Session, frame: &mut Frame, area: Rect) {
    let block = Block::bordered()
        .title(Span::styled(
            format!(" Sub-Agents ({}) ", session.sub_agents.len()),
            theme::header_style(),
        ))
        .border_style(theme::border_style());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Group by type
    let mut groups: std::collections::HashMap<&str, (u32, f64)> =
        std::collections::HashMap::new();
    for a in &session.sub_agents {
        let e = groups.entry(a.agent_type.as_str()).or_insert((0, 0.0));
        e.0 += 1;
        if let Some(d) = a.duration_secs {
            e.1 += d;
        }
    }
    let mut sorted: Vec<_> = groups.iter().collect();
    sorted.sort_by(|a, b| b.1 .0.cmp(&a.1 .0).then_with(|| a.0.cmp(b.0)));

    let mut lines: Vec<Line> = Vec::new();
    for (name, (count, dur)) in &sorted {
        lines.push(Line::from(vec![
            Span::styled(
                format!(" {} ", name),
                Style::default().fg(theme::agent_type_color(name)),
            ),
            Span::styled(format!("({}) ", count), Style::default().fg(theme::text())),
            Span::styled(
                format!("~{}", util::format_duration_secs(*dur)),
                theme::dim_style(),
            ),
        ]));
    }

    // Show any currently running agents
    let active: Vec<_> = session
        .sub_agents
        .iter()
        .filter(|a| a.completed_at.is_none())
        .collect();
    if !active.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            " ● Running:",
            Style::default()
                .fg(theme::lime())
                .add_modifier(Modifier::BOLD),
        )));
        for a in active.iter().take(3) {
            let elapsed = a
                .started_at
                .map(|s| {
                    let d = chrono::Utc::now().signed_duration_since(s).num_seconds();
                    format!(" {}s", d)
                })
                .unwrap_or_default();
            lines.push(Line::from(vec![
                Span::styled(
                    format!("   {} ", a.agent_type),
                    Style::default().fg(theme::agent_type_color(&a.agent_type)),
                ),
                Span::styled(elapsed, Style::default().fg(theme::lime())),
            ]));
        }
    }

    frame.render_widget(Paragraph::new(lines), inner);
}

fn render_footer(frame: &mut Frame, area: Rect) {
    let footer = Paragraph::new(Line::from(vec![
        Span::styled(
            " LIVE ",
            Style::default()
                .fg(theme::lime())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("auto-refreshing", theme::dim_style()),
        Span::styled("  │  ", theme::dim_style()),
        Span::styled("r", Style::default().fg(theme::gold())),
        Span::styled(": force refresh │ ", theme::dim_style()),
        Span::styled("Esc", Style::default().fg(theme::gold())),
        Span::styled(": back │ ", theme::dim_style()),
        Span::styled("q", Style::default().fg(theme::gold())),
        Span::styled(": quit", theme::dim_style()),
    ]));
    frame.render_widget(footer, area);
}
