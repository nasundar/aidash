use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::data::models::Session;
use crate::tui::theme;

/// Return a color for a tool name based on its category.
fn tool_name_color(name: &str) -> Color {
    match name {
        "view" | "glob" | "grep" => theme::lime(),
        "powershell" | "bash" => theme::amber(),
        "edit" | "create" => theme::gold(),
        "task" | "read_agent" | "list_agents" => theme::emerald(),
        _ => theme::dim(),
    }
}

pub struct ToolsView;

impl ToolsView {
    pub fn render(session: &Session, frame: &mut Frame, area: Rect) {
        let summary_text = session
            .summary
            .as_deref()
            .unwrap_or("Unknown session");

        let tool_usage = &session.metrics.tool_usage;
        let mut tools: Vec<(&String, &u32)> = tool_usage.iter().collect();
        tools.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));

        let total_calls: u32 = tools.iter().map(|(_, c)| **c).sum();
        let unique_tools = tools.len();

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // header
                Constraint::Length(2), // summary line + gap
                Constraint::Min(5),   // bar chart area
                Constraint::Length(1), // footer
            ])
            .split(area);

        // Header
        let header_block = Block::bordered()
            .title(format!(
                " Tool Usage — {} ",
                crate::util::truncate(summary_text, 60)
            ))
            .title_style(theme::header_style())
            .border_style(theme::border_style());
        frame.render_widget(header_block, chunks[0]);

        // Summary line
        let summary = Line::from(vec![
            Span::styled(
                format!(" Total: {} tool calls", total_calls),
                Style::default()
                    .fg(theme::text())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" across {} unique tools", unique_tools),
                theme::dim_style(),
            ),
        ]);
        frame.render_widget(Paragraph::new(summary), chunks[1]);

        // Bar chart using ratatui's native BarChart widget
        if tools.is_empty() {
            let empty = Paragraph::new(Span::styled(
                "  No tool usage data",
                theme::dim_style(),
            ));
            frame.render_widget(empty, chunks[2]);
        } else {
            let max_bars = (chunks[2].height.saturating_sub(2)) as usize;
            let bar_data: Vec<Bar> = tools
                .iter()
                .take(max_bars)
                .map(|(name, count)| {
                    let pct = if total_calls > 0 {
                        format!(" {:.0}%", **count as f64 / total_calls as f64 * 100.0)
                    } else {
                        String::new()
                    };
                    Bar::default()
                        .value(**count as u64)
                        .label(Line::from(name.as_str()))
                        .text_value(format!("{}{}", count, pct))
                        .style(Style::default().fg(tool_name_color(name)))
                        .value_style(
                            Style::default()
                                .fg(theme::text())
                                .add_modifier(Modifier::BOLD),
                        )
                })
                .collect();

            let barchart = BarChart::default()
                .block(
                    Block::bordered()
                        .border_style(theme::border_style()),
                )
                .data(BarGroup::default().bars(&bar_data))
                .bar_width(1)
                .bar_gap(1)
                .direction(Direction::Horizontal)
                .bar_style(Style::default().fg(theme::amber()))
                .value_style(
                    Style::default()
                        .fg(theme::text())
                        .add_modifier(Modifier::BOLD),
                );

            frame.render_widget(barchart, chunks[2]);
        }

        // Footer
        let footer = Paragraph::new(Line::from(vec![
            Span::styled("Esc", Style::default().fg(theme::gold())),
            Span::styled(": back │ ", theme::dim_style()),
            Span::styled("q", Style::default().fg(theme::gold())),
            Span::styled(": quit", theme::dim_style()),
        ]));
        frame.render_widget(footer, chunks[3]);
    }
}
