use ratatui::prelude::*;
use ratatui::widgets::*;
use crate::tui::theme;

pub struct HelpView;

impl HelpView {
    pub fn render(frame: &mut Frame, area: Rect) {
        let popup_area = centered_rect(60, 70, area);

        frame.render_widget(Clear, popup_area);

        let block = Block::bordered()
            .title(" aidash — Help ")
            .title_style(theme::header_style())
            .border_style(theme::border_style())
            .style(Style::default().bg(theme::bg_dark()));

        let help_text = vec![
            Line::from(vec![
                Span::styled("Navigation", Style::default().fg(theme::gold()).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("  ↑/↓ or j/k  ", Style::default().fg(theme::amber())),
                Span::styled("Navigate rows", Style::default().fg(theme::text())),
            ]),
            Line::from(vec![
                Span::styled("  Enter        ", Style::default().fg(theme::amber())),
                Span::styled("Drill into selected session", Style::default().fg(theme::text())),
            ]),
            Line::from(vec![
                Span::styled("  Esc / Bksp   ", Style::default().fg(theme::amber())),
                Span::styled("Go back to previous view", Style::default().fg(theme::text())),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Dashboard", Style::default().fg(theme::gold()).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("  Tab          ", Style::default().fg(theme::amber())),
                Span::styled("Switch source: All → Copilot → Claude", Style::default().fg(theme::text())),
            ]),
            Line::from(vec![
                Span::styled("  s            ", Style::default().fg(theme::amber())),
                Span::styled("Cycle sort column", Style::default().fg(theme::text())),
            ]),
            Line::from(vec![
                Span::styled("  S            ", Style::default().fg(theme::amber())),
                Span::styled("Reverse sort direction", Style::default().fg(theme::text())),
            ]),
            Line::from(vec![
                Span::styled("  T            ", Style::default().fg(theme::amber())),
                Span::styled("View trends & analytics", Style::default().fg(theme::text())),
            ]),
            Line::from(vec![
                Span::styled("  L            ", Style::default().fg(theme::amber())),
                Span::styled("Live dashboard for active session", Style::default().fg(theme::text())),
            ]),
            Line::from(vec![
                Span::styled("  r            ", Style::default().fg(theme::amber())),
                Span::styled("Refresh data from disk", Style::default().fg(theme::text())),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Session Detail", Style::default().fg(theme::gold()).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("  a            ", Style::default().fg(theme::amber())),
                Span::styled("View sub-agents breakdown", Style::default().fg(theme::text())),
            ]),
            Line::from(vec![
                Span::styled("  t            ", Style::default().fg(theme::amber())),
                Span::styled("View tool usage breakdown", Style::default().fg(theme::text())),
            ]),
            Line::from(vec![
                Span::styled("  m            ", Style::default().fg(theme::amber())),
                Span::styled("View models breakdown", Style::default().fg(theme::text())),
            ]),
            Line::from(vec![
                Span::styled("  l            ", Style::default().fg(theme::amber())),
                Span::styled("Live dashboard (active sessions only)", Style::default().fg(theme::text())),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("General", Style::default().fg(theme::gold()).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("  d            ", Style::default().fg(theme::amber())),
                Span::styled("Toggle dark/light theme", Style::default().fg(theme::text())),
            ]),
            Line::from(vec![
                Span::styled("  ?            ", Style::default().fg(theme::amber())),
                Span::styled("Toggle this help", Style::default().fg(theme::text())),
            ]),
            Line::from(vec![
                Span::styled("  q            ", Style::default().fg(theme::amber())),
                Span::styled("Quit", Style::default().fg(theme::text())),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "  Press ? or Esc to close",
                Style::default().fg(theme::dim()),
            )),
        ];

        let paragraph = Paragraph::new(help_text).block(block);
        frame.render_widget(paragraph, popup_area);
    }
}

/// Create a centered rect using percentage of the given area.
fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
