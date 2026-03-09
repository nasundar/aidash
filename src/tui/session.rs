use ratatui::prelude::*;
use ratatui::symbols;
use ratatui::widgets::*;

use crate::data::models::Session;
use crate::tui::theme;
use crate::util;

/// Build a ratio bar like `████████████░░░░░░` of the given width.
fn ratio_bar(filled_ratio: f64, width: usize) -> (String, String) {
    let filled = (filled_ratio * width as f64).round() as usize;
    let empty = width.saturating_sub(filled);
    ("█".repeat(filled), "░".repeat(empty))
}

/// Render a mini proportional bar for tool counts (session detail right column).
fn mini_tool_bar(count: u32, max_count: u32, bar_width: usize) -> String {
    if max_count == 0 {
        return " ".repeat(bar_width);
    }
    let ratio = count as f64 / max_count as f64;
    let chars = (ratio * bar_width as f64).max(0.5).round() as usize;
    let bar: String = "█".repeat(chars);
    let pad: String = " ".repeat(bar_width.saturating_sub(chars));
    format!("{}{}", bar, pad)
}

/// Format milliseconds as a human-readable duration string.
fn format_api_duration(ms: u64) -> String {
    let secs = ms as f64 / 1000.0;
    util::format_duration_secs(secs)
}

/// Source label for the session.
fn source_label(session: &Session) -> &'static str {
    match session.source {
        crate::data::models::Source::Copilot => "Copilot",
        crate::data::models::Source::Claude => "Claude",
    }
}

pub struct SessionDetailView;

impl SessionDetailView {
    pub fn render(session: &Session, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(6),
                Constraint::Min(10),
                Constraint::Length(1),
            ])
            .split(area);

        render_header(session, frame, chunks[0]);

        let body = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[1]);

        render_left_column(session, frame, body[0]);
        render_right_column(session, frame, body[1]);
        render_footer(frame, chunks[2]);
    }
}

fn render_header(session: &Session, frame: &mut Frame, area: Rect) {
    let summary = session
        .summary
        .as_deref()
        .unwrap_or("Untitled session");

    let title = Line::from(vec![
        Span::styled(" Session: ", theme::header_style()),
        Span::styled(util::truncate(summary, 50), Style::default().fg(theme::text())),
        Span::raw(" "),
    ]);

    let model_name = session.model.as_deref().unwrap_or("-");
    let branch = session.branch.as_deref().unwrap_or("-");
    let cwd = session.cwd.as_deref().unwrap_or("-");
    let started = util::format_time_short(session.started_at);
    let duration = util::format_duration(session.started_at, session.ended_at);
    let api_dur = format_api_duration(session.metrics.total_api_duration_ms);
    let status = if session.is_active {
        Span::styled("● Active", theme::active_style())
    } else {
        Span::styled("● Done", theme::dim_style())
    };

    let row1 = Line::from(vec![
        Span::styled(" ID: ", theme::dim_style()),
        Span::styled(&session.id, Style::default().fg(theme::dim())),
        Span::styled("  │  Source: ", theme::dim_style()),
        Span::styled(source_label(session), Style::default().fg(theme::emerald())),
    ]);

    let row2 = Line::from(vec![
        Span::styled(" Model: ", theme::dim_style()),
        Span::styled(
            theme::short_model_name(model_name),
            theme::model_style(model_name),
        ),
        Span::styled("  │  Branch: ", theme::dim_style()),
        Span::styled(
            util::truncate(branch, 20),
            Style::default().fg(theme::text()),
        ),
        Span::styled("  │  CWD: ", theme::dim_style()),
        Span::styled(
            util::truncate(cwd, 30),
            Style::default().fg(theme::text()),
        ),
    ]);

    let row3 = Line::from(vec![
        Span::styled(" Started: ", theme::dim_style()),
        Span::styled(started, Style::default().fg(theme::text())),
        Span::styled("  │  Duration: ", theme::dim_style()),
        Span::styled(duration, Style::default().fg(theme::text())),
        Span::styled("  │  API Time: ", theme::dim_style()),
        Span::styled(api_dur, Style::default().fg(theme::amber())),
        Span::styled("  │  Status: ", theme::dim_style()),
        status,
    ]);

    let header = Paragraph::new(vec![row1, row2, row3]).block(
        Block::bordered()
            .title(title)
            .border_style(theme::border_style()),
    );

    frame.render_widget(header, area);
}

fn render_left_column(session: &Session, frame: &mut Frame, area: Rect) {
    let m = &session.metrics;

    let model_count = m.models_used.len().max(1).min(6) as u16;
    let model_count = model_count * 2; // each model row + spacer

    let has_turns = !session.turns.is_empty();
    // Input/output ratio bar: 4 lines
    let io_height: u16 = 5;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(9),                    // token breakdown
            Constraint::Length(model_count + 2),      // models used
            Constraint::Length(io_height),            // input vs output
            Constraint::Min(if has_turns { 8 } else { 0 }),  // bar chart
        ])
        .split(area);

    // Token Breakdown
    let total_est =
        m.total_output_tokens + m.estimated_input_tokens + m.estimated_reasoning_tokens;
    let token_lines = vec![
        Line::from(vec![
            Span::styled(" Output:      ", theme::dim_style()),
            Span::styled(
                util::format_tokens(m.total_output_tokens),
                Style::default().fg(theme::text()),
            ),
        ]),
        Line::from(vec![
            Span::styled(" Est Input:  ~", theme::dim_style()),
            Span::styled(
                util::format_tokens(m.estimated_input_tokens),
                Style::default().fg(theme::text()),
            ),
        ]),
        Line::from(vec![
            Span::styled(" Reasoning:  ~", theme::dim_style()),
            Span::styled(
                util::format_tokens(m.estimated_reasoning_tokens),
                Style::default().fg(theme::text()),
            ),
        ]),
        Line::from(vec![
            Span::styled(" Total Est:  ~", theme::dim_style()),
            Span::styled(
                util::format_tokens(total_est),
                Style::default()
                    .fg(theme::gold())
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled(" Est Cost:    ", theme::dim_style()),
            Span::styled(
                util::format_cost(m.estimated_cost_usd),
                Style::default()
                    .fg(theme::cost_color(m.estimated_cost_usd))
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled(" Premium Req: ", theme::dim_style()),
            Span::styled(
                m.total_premium_requests.to_string(),
                Style::default().fg(theme::gold()),
            ),
        ]),
    ];

    let token_block = Paragraph::new(token_lines).block(
        Block::bordered()
            .title(Span::styled(" Token Breakdown ", theme::header_style()))
            .border_style(theme::border_style()),
    );
    frame.render_widget(token_block, chunks[0]);

    // Models Used
    let mut models: Vec<(&String, &u32)> = m.models_used.iter().collect();
    models.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));

    let mut model_lines: Vec<Line> = Vec::new();
    for (name, count) in models.iter().take(6) {
        let short = theme::short_model_name(name);
        let padding = 18usize.saturating_sub(short.len());
        model_lines.push(Line::from(vec![
            Span::raw(" "),
            Span::styled(short, theme::model_style(name)),
            Span::raw(" ".repeat(padding)),
            Span::styled(format!("{} msgs", count), theme::dim_style()),
        ]));
        model_lines.push(Line::from("")); // breathing room
    }

    let models_block = Paragraph::new(model_lines).block(
        Block::bordered()
            .title(Span::styled(" Models Used ", theme::header_style()))
            .border_style(theme::border_style()),
    );
    frame.render_widget(models_block, chunks[1]);

    // Input vs Output ratio bar
    let total_io = m.total_output_tokens + m.estimated_input_tokens;
    let (out_ratio, out_pct, in_pct) = if total_io > 0 {
        let r = m.total_output_tokens as f64 / total_io as f64;
        (r, r * 100.0, (1.0 - r) * 100.0)
    } else {
        (0.5, 50.0, 50.0)
    };

    let bar_w = (chunks[2].width.saturating_sub(4) as usize).saturating_sub(14); // label + pct
    let (out_filled, out_empty) = ratio_bar(out_ratio, bar_w);
    let (in_filled, in_empty) = ratio_bar(1.0 - out_ratio, bar_w);

    let io_lines = vec![
        Line::from(vec![
            Span::styled(" Output ", theme::dim_style()),
            Span::styled(out_filled, Style::default().fg(theme::gold())),
            Span::styled(out_empty, Style::default().fg(theme::muted())),
            Span::styled(format!(" {:>4.0}%", out_pct), Style::default().fg(theme::gold())),
        ]),
        Line::from(vec![
            Span::styled(" Input  ", theme::dim_style()),
            Span::styled(in_empty, Style::default().fg(theme::muted())),
            Span::styled(in_filled, Style::default().fg(theme::emerald())),
            Span::styled(
                format!(" {:>4.0}%", in_pct),
                Style::default().fg(theme::emerald()),
            ),
        ]),
    ];

    let io_block = Paragraph::new(io_lines).block(
        Block::bordered()
            .title(Span::styled(" Output vs Input ", theme::header_style()))
            .border_style(theme::border_style()),
    );
    frame.render_widget(io_block, chunks[2]);

    // Per-turn token line chart
    if has_turns {
        let turn_tokens: Vec<u64> = session.turns.iter().map(|t| t.output_tokens).collect();
        let max_tok = turn_tokens.iter().copied().max().unwrap_or(1) as f64;
        let num_turns = turn_tokens.len() as f64;

        let data: Vec<(f64, f64)> = turn_tokens
            .iter()
            .enumerate()
            .map(|(i, &t)| (i as f64, t as f64))
            .collect();

        let dataset = Dataset::default()
            .name("Tokens/Turn")
            .marker(symbols::Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(theme::amber()))
            .data(&data);

        let x_labels = vec![
            Span::styled("T1", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(
                format!("T{:.0}", num_turns),
                Style::default().add_modifier(Modifier::BOLD),
            ),
        ];
        let y_labels = vec![
            Span::styled("0", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(
                util::format_tokens_compact(max_tok as u64),
                Style::default().add_modifier(Modifier::BOLD),
            ),
        ];

        let chart = Chart::new(vec![dataset])
            .block(
                Block::bordered()
                    .title(Span::styled(
                        format!(
                            " Tokens/Turn ({} turns, peak: {}) ",
                            turn_tokens.len(),
                            util::format_tokens_compact(max_tok as u64),
                        ),
                        theme::header_style(),
                    ))
                    .border_style(theme::border_style()),
            )
            .x_axis(
                Axis::default()
                    .style(Style::default().fg(theme::dim()))
                    .bounds([0.0, (num_turns - 1.0).max(1.0)])
                    .labels(x_labels),
            )
            .y_axis(
                Axis::default()
                    .style(Style::default().fg(theme::dim()))
                    .bounds([0.0, max_tok * 1.1])
                    .labels(y_labels),
            );

        frame.render_widget(chart, chunks[3]);
    }
}

fn render_right_column(session: &Session, frame: &mut Frame, area: Rect) {
    let m = &session.metrics;

    // Group sub-agents by type
    let mut agent_groups: std::collections::HashMap<&str, (u32, f64)> =
        std::collections::HashMap::new();
    for agent in &session.sub_agents {
        let entry = agent_groups
            .entry(agent.agent_type.as_str())
            .or_insert((0, 0.0));
        entry.0 += 1;
        if let Some(dur) = agent.duration_secs {
            entry.1 += dur;
        }
    }
    let mut agents_sorted: Vec<(&&str, &(u32, f64))> = agent_groups.iter().collect();
    agents_sorted.sort_by(|a, b| b.1 .0.cmp(&a.1 .0).then_with(|| a.0.cmp(b.0)));

    let agent_count = agents_sorted.len().max(1).min(6) as u16;
    let agent_count = agent_count * 2; // each agent row + spacer

    // Top tools — up to 10
    let mut tools: Vec<(&String, &u32)> = m.tool_usage.iter().collect();
    tools.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));
    let max_tool_rows = 10usize;
    let top_tools: Vec<(&String, &u32)> = tools.into_iter().take(max_tool_rows).collect();
    let tool_rows = (top_tools.len().max(1) * 2) as u16; // each tool row + spacer

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(agent_count + 2), // sub-agents
            Constraint::Length(tool_rows + 2),   // top tools
            Constraint::Length(4),               // code changes
            Constraint::Min(0),                  // spacer
        ])
        .split(area);

    // Sub-Agents
    let total_agents: u32 = agents_sorted.iter().map(|(_, (c, _))| c).sum();
    let mut agent_lines: Vec<Line> = Vec::new();
    for (name, (count, total_dur)) in agents_sorted.iter().take(6) {
        agent_lines.push(Line::from(vec![
            Span::raw(" "),
            Span::styled(
                format!("{} ({})", name, count),
                Style::default().fg(theme::agent_type_color(name)),
            ),
            Span::styled(
                format!("  ~{}", util::format_duration_secs(*total_dur)),
                theme::dim_style(),
            ),
        ]));
        agent_lines.push(Line::from("")); // breathing room
    }

    let agents_block = Paragraph::new(agent_lines).block(
        Block::bordered()
            .title(Span::styled(
                format!(" Sub-Agents ({}) ", total_agents),
                theme::header_style(),
            ))
            .border_style(theme::border_style()),
    );
    frame.render_widget(agents_block, chunks[0]);

    // Top Tools — each with a mini proportional bar
    let tool_max = top_tools.first().map(|(_, c)| **c).unwrap_or(1);
    let name_w = top_tools
        .iter()
        .map(|(n, _)| n.len())
        .max()
        .unwrap_or(8)
        .min(18);
    let count_w = format!("{}", tool_max).len().max(3);
    // Remaining width for the bar (account for borders, padding, count)
    let bar_w = (chunks[1].width.saturating_sub(4) as usize)
        .saturating_sub(name_w + count_w + 3);

    let mut tool_lines: Vec<Line> = Vec::new();
    for (name, count) in &top_tools {
        let display_name = if name.len() > name_w {
            format!("{:.width$}", name, width = name_w)
        } else {
            format!("{:<width$}", name, width = name_w)
        };
        let bar = mini_tool_bar(**count, tool_max, bar_w);
        let ratio = **count as f64 / tool_max.max(1) as f64;
        let bar_color = theme::bar_gradient(ratio);

        tool_lines.push(Line::from(vec![
            Span::styled(format!(" {}", display_name), Style::default().fg(theme::emerald())),
            Span::styled(format!(" {}", bar), Style::default().fg(bar_color)),
            Span::styled(
                format!(" {:>width$}", count, width = count_w),
                Style::default().fg(theme::text()),
            ),
        ]));
        tool_lines.push(Line::from("")); // breathing room
    }

    let tools_block = Paragraph::new(tool_lines).block(
        Block::bordered()
            .title(Span::styled(" Top Tools ", theme::header_style()))
            .border_style(theme::border_style()),
    );
    frame.render_widget(tools_block, chunks[1]);

    // Code Changes
    let code_lines = vec![Line::from(vec![
        Span::raw(" "),
        Span::styled(
            format!("+{}", m.lines_added),
            Style::default().fg(theme::green_bright()),
        ),
        Span::styled(" / ", theme::dim_style()),
        Span::styled(
            format!("-{}", m.lines_removed),
            Style::default().fg(theme::red_soft()),
        ),
        Span::styled("  │  ", theme::dim_style()),
        Span::styled(
            format!("{} files", m.files_modified),
            Style::default().fg(theme::text()),
        ),
    ])];

    let code_block = Paragraph::new(code_lines).block(
        Block::bordered()
            .title(Span::styled(" Code Changes ", theme::header_style()))
            .border_style(theme::border_style()),
    );
    frame.render_widget(code_block, chunks[2]);
}

fn render_footer(frame: &mut Frame, area: Rect) {
    let footer = Paragraph::new(Line::from(vec![
        Span::styled(" ► ", Style::default().fg(theme::gold())),
        Span::styled(
            "a",
            Style::default()
                .fg(theme::gold())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": agents detail", theme::dim_style()),
        Span::styled("  │  ", theme::dim_style()),
        Span::styled(
            "t",
            Style::default()
                .fg(theme::gold())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": tools detail", theme::dim_style()),
        Span::styled("  │  ", theme::dim_style()),
        Span::styled(
            "m",
            Style::default()
                .fg(theme::gold())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": models", theme::dim_style()),
        Span::styled("  │  ", theme::dim_style()),
        Span::styled(
            "l",
            Style::default()
                .fg(theme::gold())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": live", theme::dim_style()),
        Span::styled("  │  ", theme::dim_style()),
        Span::styled(
            "Esc",
            Style::default()
                .fg(theme::gold())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": back", theme::dim_style()),
        Span::styled("  │  ", theme::dim_style()),
        Span::styled(
            "q",
            Style::default()
                .fg(theme::gold())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": quit", theme::dim_style()),
    ]));
    frame.render_widget(footer, area);
}
