use std::collections::HashMap;

use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::data::models::Session;
use crate::tui::theme;
use crate::util;

pub struct AgentsView {
    pub table_state: TableState,
}

impl AgentsView {
    pub fn new() -> Self {
        let mut state = TableState::default();
        state.select(Some(0));
        Self { table_state: state }
    }

    pub fn render(&mut self, session: &Session, frame: &mut Frame, area: Rect) {
        let summary_text = session
            .summary
            .as_deref()
            .unwrap_or("Unknown session");

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // header
                Constraint::Length(1), // summary line
                Constraint::Min(5),   // table
                Constraint::Length(1), // footer
            ])
            .split(area);

        // Header
        let header_block = Block::bordered()
            .title(format!(" Sub-Agents — {} ", util::truncate(summary_text, 60)))
            .title_style(theme::header_style())
            .border_style(theme::border_style());
        frame.render_widget(header_block, chunks[0]);

        // Summary line: count by agent type
        let agents = &session.sub_agents;
        let mut type_counts: HashMap<&str, usize> = HashMap::new();
        for agent in agents {
            *type_counts.entry(agent.agent_type.as_str()).or_default() += 1;
        }
        let mut type_parts: Vec<String> = type_counts
            .iter()
            .map(|(k, v)| format!("{}: {}", k, v))
            .collect();
        type_parts.sort();
        let summary = Line::from(vec![
            Span::styled(
                format!(" Total: {} agents", agents.len()),
                Style::default().fg(theme::text()).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" │ {}", type_parts.join(" │ ")),
                theme::dim_style(),
            ),
        ]);
        frame.render_widget(Paragraph::new(summary), chunks[1]);

        // Table
        let header = Row::new(vec!["#", "Type", "Display Name", "Started", "Duration", "Status"])
            .style(theme::table_header_style())
            .height(1);

        let rows: Vec<Row> = agents
            .iter()
            .enumerate()
            .map(|(i, agent)| {
                let type_color = match agent.agent_type.as_str() {
                    "explore" => theme::lime(),
                    "general-purpose" => theme::amber(),
                    "code-review" => theme::gold(),
                    "task" => theme::emerald(),
                    _ => theme::text(),
                };

                let status = if agent.completed_at.is_some() {
                    Span::styled("✓ Done", Style::default().fg(theme::lime()))
                } else {
                    Span::styled("● Running", Style::default().fg(theme::gold()))
                };

                let duration = match agent.duration_secs {
                    Some(d) => util::format_duration_secs(d),
                    None => "-".to_string(),
                };

                Row::new(vec![
                    Cell::from(format!("{}", i + 1)),
                    Cell::from(Span::styled(
                        agent.agent_type.clone(),
                        Style::default().fg(type_color),
                    )),
                    Cell::from(util::truncate(&agent.display_name, 40)),
                    Cell::from(util::format_time_short(agent.started_at)),
                    Cell::from(duration),
                    Cell::from(status),
                ])
            })
            .collect();

        let table = Table::new(
            rows,
            [
                Constraint::Length(4),  // #
                Constraint::Length(18), // Type
                Constraint::Min(20),   // Display Name
                Constraint::Length(14), // Started
                Constraint::Length(10), // Duration
                Constraint::Length(12), // Status
            ],
        )
        .header(header)
        .row_highlight_style(theme::selected_style())
        .block(
            Block::bordered()
                .border_style(theme::border_style()),
        );

        frame.render_stateful_widget(table, chunks[2], &mut self.table_state);

        // Footer
        let footer = Paragraph::new(Line::from(vec![
            Span::styled("↑/↓", Style::default().fg(theme::gold())),
            Span::styled(": navigate │ ", theme::dim_style()),
            Span::styled("Esc", Style::default().fg(theme::gold())),
            Span::styled(": back │ ", theme::dim_style()),
            Span::styled("q", Style::default().fg(theme::gold())),
            Span::styled(": quit", theme::dim_style()),
        ]));
        frame.render_widget(footer, chunks[3]);
    }

    pub fn next(&mut self, len: usize) {
        if len == 0 {
            return;
        }
        let i = match self.table_state.selected() {
            Some(i) => (i + 1) % len,
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    pub fn previous(&mut self) {
        let i = match self.table_state.selected() {
            Some(0) | None => 0,
            Some(i) => i - 1,
        };
        self.table_state.select(Some(i));
    }
}
