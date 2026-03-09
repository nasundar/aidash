use ratatui::prelude::*;
use ratatui::widgets::*;
use crate::data::models::{Session, Source};
use crate::tui::theme;
use crate::util;

#[derive(Clone, Copy, PartialEq)]
pub enum SortColumn {
    Name,
    Model,
    Tokens,
    Cost,
    Turns,
    Agents,
    Duration,
    Source,
    Status,
}

#[derive(Clone, Copy, PartialEq)]
pub enum SourceFilter {
    All,
    Copilot,
    Claude,
}

pub struct DashboardView {
    pub table_state: TableState,
    pub sessions: Vec<Session>,
    pub sort_column: SortColumn,
    pub sort_ascending: bool,
    pub source_filter: SourceFilter,
    pub has_active_sessions: bool,
}

impl DashboardView {
    pub fn new(sessions: Vec<Session>) -> Self {
        let mut state = TableState::default();
        if !sessions.is_empty() {
            state.select(Some(0));
        }
        let has_active = sessions.iter().any(|s| s.is_active);
        let mut view = Self {
            table_state: state,
            sessions,
            sort_column: SortColumn::Status,
            sort_ascending: false, // active first (true > false descending)
            source_filter: SourceFilter::All,
            has_active_sessions: has_active,
        };
        view.sort_sessions();
        view
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2), // header + tabs
                Constraint::Length(1), // summary stats
                Constraint::Min(5),   // table
                Constraint::Length(1), // footer
            ])
            .split(area);

        self.render_header(frame, chunks[0]);
        self.render_summary(frame, chunks[1]);
        self.render_table(frame, chunks[2]);
        self.render_footer(frame, chunks[3]);
    }

    fn render_header(&self, frame: &mut Frame, area: Rect) {
        let header_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(30), Constraint::Length(30)])
            .split(area);

        let mut title_spans = vec![
            Span::styled("aidash", theme::header_style()),
            Span::styled(" — AI Usage Dashboard", Style::default().fg(theme::dim())),
        ];
        if self.has_active_sessions {
            title_spans.push(Span::styled("  ● LIVE", Style::default().fg(theme::lime()).add_modifier(Modifier::BOLD)));
        }
        let title = Paragraph::new(Line::from(title_spans));
        frame.render_widget(title, header_chunks[0]);

        let all_style = if self.source_filter == SourceFilter::All { theme::active_tab_style() } else { theme::inactive_tab_style() };
        let cop_style = if self.source_filter == SourceFilter::Copilot { theme::active_tab_style() } else { theme::inactive_tab_style() };
        let cla_style = if self.source_filter == SourceFilter::Claude { theme::active_tab_style() } else { theme::inactive_tab_style() };

        let tabs = Paragraph::new(Line::from(vec![
            Span::styled("All", all_style),
            Span::styled(" │ ", Style::default().fg(theme::dim())),
            Span::styled("Copilot", cop_style),
            Span::styled(" │ ", Style::default().fg(theme::dim())),
            Span::styled("Claude", cla_style),
        ]))
        .alignment(Alignment::Right);
        frame.render_widget(tabs, header_chunks[1]);
    }

    fn render_summary(&self, frame: &mut Frame, area: Rect) {
        let filtered = self.filtered_sessions();
        let session_count = filtered.len();
        let total_tokens: u64 = filtered.iter().map(|s| s.metrics.total_output_tokens).sum();
        let total_cost: f64 = filtered.iter().map(|s| s.metrics.estimated_cost_usd).sum();
        let premium: u32 = filtered.iter().map(|s| s.metrics.total_premium_requests).sum();

        let summary = Paragraph::new(Line::from(vec![
            Span::styled("Sessions: ", Style::default().fg(theme::dim())),
            Span::styled(format!("{}", session_count), Style::default().fg(theme::text())),
            Span::styled(" │ Total Tokens: ", Style::default().fg(theme::dim())),
            Span::styled(util::format_tokens(total_tokens), Style::default().fg(theme::text())),
            Span::styled(" │ Est. Cost: ", Style::default().fg(theme::dim())),
            Span::styled(util::format_cost(total_cost), Style::default().fg(theme::cost_color(total_cost))),
            Span::styled(" │ Premium: ", Style::default().fg(theme::dim())),
            Span::styled(format!("{}", premium), Style::default().fg(theme::gold())),
        ]));
        frame.render_widget(summary, area);
    }

    fn render_table(&mut self, frame: &mut Frame, area: Rect) {
        let filtered = self.filtered_sessions();

        let sort_indicator = |col: SortColumn| -> &str {
            if self.sort_column == col {
                if self.sort_ascending { " ▲" } else { " ▼" }
            } else {
                ""
            }
        };

        let header = Row::new(vec![
            Cell::from(format!("St{}", sort_indicator(SortColumn::Status))),
            Cell::from("#"),
            Cell::from(format!("Session Name{}", sort_indicator(SortColumn::Name))),
            Cell::from(format!("Model{}", sort_indicator(SortColumn::Model))),
            Cell::from(format!("Tokens Out{}", sort_indicator(SortColumn::Tokens))),
            Cell::from(format!("Est Cost{}", sort_indicator(SortColumn::Cost))),
            Cell::from(format!("Turns{}", sort_indicator(SortColumn::Turns))),
            Cell::from(format!("Agents{}", sort_indicator(SortColumn::Agents))),
            Cell::from(format!("Duration{}", sort_indicator(SortColumn::Duration))),
            Cell::from(format!("Source{}", sort_indicator(SortColumn::Source))),
        ])
        .style(theme::table_header_style())
        .height(1);

        let rows: Vec<Row> = filtered
            .iter()
            .enumerate()
            .map(|(i, session)| {
                let model_name = session.model.as_deref().unwrap_or("-");
                let short_model = theme::short_model_name(model_name);
                let cost = session.metrics.estimated_cost_usd;

                let fallback = util::short_id(&session.id);
                let name = session
                    .summary
                    .as_deref()
                    .unwrap_or(&fallback);

                let status_cell = if session.is_active {
                    Cell::from(Span::styled("● ON", Style::default().fg(theme::lime()).add_modifier(Modifier::BOLD)))
                } else {
                    Cell::from(Span::styled("done", theme::dim_style()))
                };

                let source_label = match session.source {
                    Source::Copilot => "Copilot",
                    Source::Claude => "Claude",
                };

                Row::new(vec![
                    status_cell,
                    Cell::from(format!("{}", i + 1)),
                    Cell::from(util::truncate(name, 38)),
                    Cell::from(short_model.clone()).style(theme::model_style(model_name)),
                    Cell::from(util::format_tokens_compact(session.metrics.total_output_tokens)),
                    Cell::from(util::format_cost(cost)).style(Style::default().fg(theme::cost_color(cost))),
                    Cell::from(format!("{}", session.metrics.total_turns)),
                    Cell::from(format!("{}", session.metrics.total_sub_agents)),
                    Cell::from(util::format_duration(session.started_at, session.ended_at)),
                    Cell::from(source_label),
                ])
            })
            .collect();

        let widths = [
            Constraint::Length(5),   // Status
            Constraint::Length(4),   // #
            Constraint::Min(16),     // Session Name
            Constraint::Length(16),  // Model
            Constraint::Length(10),  // Tokens Out
            Constraint::Length(10),  // Est Cost
            Constraint::Length(6),   // Turns
            Constraint::Length(7),   // Agents
            Constraint::Length(10),  // Duration
            Constraint::Length(8),   // Source
        ];

        let table = Table::new(rows, widths)
            .header(header)
            .row_highlight_style(theme::selected_style())
            .highlight_symbol("▶ ")
            .highlight_spacing(HighlightSpacing::Always)
            .block(
                Block::default()
                    .borders(Borders::TOP)
                    .border_style(theme::border_style()),
            );

        frame.render_stateful_widget(table, area, &mut self.table_state);
    }

    fn render_footer(&self, frame: &mut Frame, area: Rect) {
        let footer = Paragraph::new(Line::from(vec![
            Span::styled("Enter", Style::default().fg(theme::gold()).add_modifier(Modifier::BOLD)),
            Span::styled(": detail │ ", Style::default().fg(theme::dim())),
            Span::styled("s", Style::default().fg(theme::gold()).add_modifier(Modifier::BOLD)),
            Span::styled("/", Style::default().fg(theme::dim())),
            Span::styled("1-9", Style::default().fg(theme::gold()).add_modifier(Modifier::BOLD)),
            Span::styled(": sort │ ", Style::default().fg(theme::dim())),
            Span::styled("S", Style::default().fg(theme::gold()).add_modifier(Modifier::BOLD)),
            Span::styled(": reverse │ ", Style::default().fg(theme::dim())),
            Span::styled("r", Style::default().fg(theme::gold()).add_modifier(Modifier::BOLD)),
            Span::styled(": refresh │ ", Style::default().fg(theme::dim())),
            Span::styled("L", Style::default().fg(theme::gold()).add_modifier(Modifier::BOLD)),
            Span::styled(": live │ ", Style::default().fg(theme::dim())),
            Span::styled("?", Style::default().fg(theme::gold()).add_modifier(Modifier::BOLD)),
            Span::styled(": help │ ", Style::default().fg(theme::dim())),
            Span::styled("q", Style::default().fg(theme::gold()).add_modifier(Modifier::BOLD)),
            Span::styled(": quit", Style::default().fg(theme::dim())),
        ]));
        frame.render_widget(footer, area);
    }

    pub fn next(&mut self) {
        let filtered_len = self.filtered_sessions().len();
        if filtered_len == 0 {
            return;
        }
        let i = match self.table_state.selected() {
            Some(i) => {
                if i >= filtered_len - 1 { 0 } else { i + 1 }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    pub fn previous(&mut self) {
        let filtered_len = self.filtered_sessions().len();
        if filtered_len == 0 {
            return;
        }
        let i = match self.table_state.selected() {
            Some(i) => {
                if i == 0 { filtered_len - 1 } else { i - 1 }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    pub fn toggle_sort(&mut self) {
        self.sort_column = match self.sort_column {
            SortColumn::Status => SortColumn::Name,
            SortColumn::Name => SortColumn::Model,
            SortColumn::Model => SortColumn::Tokens,
            SortColumn::Tokens => SortColumn::Cost,
            SortColumn::Cost => SortColumn::Turns,
            SortColumn::Turns => SortColumn::Agents,
            SortColumn::Agents => SortColumn::Duration,
            SortColumn::Duration => SortColumn::Source,
            SortColumn::Source => SortColumn::Status,
        };
        self.sort_sessions();
    }

    pub fn reverse_sort(&mut self) {
        self.sort_ascending = !self.sort_ascending;
        self.sort_sessions();
    }

    /// Sort by a specific column — if already sorting by it, reverse direction
    pub fn sort_by(&mut self, col: SortColumn) {
        if self.sort_column == col {
            self.sort_ascending = !self.sort_ascending;
        } else {
            self.sort_column = col;
            self.sort_ascending = false;
        }
        self.sort_sessions();
    }

    pub fn toggle_source(&mut self) {
        self.source_filter = match self.source_filter {
            SourceFilter::All => SourceFilter::Copilot,
            SourceFilter::Copilot => SourceFilter::Claude,
            SourceFilter::Claude => SourceFilter::All,
        };
        // Reset selection when filter changes
        let filtered_len = self.filtered_sessions().len();
        if filtered_len > 0 {
            self.table_state.select(Some(0));
        } else {
            self.table_state.select(None);
        }
    }

    #[allow(dead_code)]
    pub fn selected_session(&self) -> Option<&Session> {
        let filtered = self.filtered_sessions();
        self.table_state
            .selected()
            .and_then(|i| filtered.into_iter().nth(i))
    }

    pub fn filtered_sessions(&self) -> Vec<&Session> {
        self.sessions
            .iter()
            .filter(|s| match self.source_filter {
                SourceFilter::All => true,
                SourceFilter::Copilot => s.source == Source::Copilot,
                SourceFilter::Claude => s.source == Source::Claude,
            })
            .collect()
    }

    pub fn sort_sessions(&mut self) {
        let ascending = self.sort_ascending;
        let column = self.sort_column;
        self.sessions.sort_by(|a, b| {
            let ord = match column {
                SortColumn::Name => {
                    let a_name = a.summary.as_deref().unwrap_or(&a.id);
                    let b_name = b.summary.as_deref().unwrap_or(&b.id);
                    a_name.to_lowercase().cmp(&b_name.to_lowercase())
                }
                SortColumn::Source => {
                    let a_src = format!("{:?}", a.source);
                    let b_src = format!("{:?}", b.source);
                    a_src.cmp(&b_src)
                }
                SortColumn::Model => {
                    let a_model = a.model.as_deref().unwrap_or("");
                    let b_model = b.model.as_deref().unwrap_or("");
                    a_model.cmp(b_model)
                }
                SortColumn::Tokens => a.metrics.total_output_tokens.cmp(&b.metrics.total_output_tokens),
                SortColumn::Cost => a.metrics.estimated_cost_usd.partial_cmp(&b.metrics.estimated_cost_usd).unwrap_or(std::cmp::Ordering::Equal),
                SortColumn::Turns => a.metrics.total_turns.cmp(&b.metrics.total_turns),
                SortColumn::Agents => a.metrics.total_sub_agents.cmp(&b.metrics.total_sub_agents),
                SortColumn::Duration => {
                    let a_dur = Self::session_duration_secs(a);
                    let b_dur = Self::session_duration_secs(b);
                    a_dur.partial_cmp(&b_dur).unwrap_or(std::cmp::Ordering::Equal)
                }
                SortColumn::Status => a.is_active.cmp(&b.is_active),
            };
            // Secondary sort: always by tokens descending as tiebreaker
            let ord = if ascending { ord } else { ord.reverse() };
            ord.then_with(|| b.metrics.total_output_tokens.cmp(&a.metrics.total_output_tokens))
        });
        // Preserve selection at top after re-sort
        if !self.sessions.is_empty() {
            let sel = self.table_state.selected().unwrap_or(0);
            let filtered_len = self.filtered_sessions().len();
            if sel >= filtered_len {
                self.table_state.select(Some(0));
            }
        }
    }

    fn session_duration_secs(session: &Session) -> f64 {
        match (session.started_at, session.ended_at) {
            (Some(s), Some(e)) => e.signed_duration_since(s).num_seconds() as f64,
            (Some(s), None) => chrono::Utc::now().signed_duration_since(s).num_seconds() as f64,
            _ => 0.0,
        }
    }

    /// Get the session ID of the currently selected session
    pub fn selected_session_id(&self) -> Option<String> {
        let filtered = self.filtered_sessions();
        let selected = self.table_state.selected()?;
        let session = filtered.get(selected)?;
        Some(session.id.clone())
    }

    #[allow(dead_code)]
    /// Get the index of the selected session in the original sessions vec
    pub fn selected_session_index(&self) -> Option<usize> {
        let filtered = self.filtered_sessions();
        let selected = self.table_state.selected()?;
        let session = filtered.get(selected)?;
        let session_id = &session.id;
        self.sessions.iter().position(|s| s.id == *session_id)
    }
}
