use std::collections::HashMap;
use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::data::models::{ModelPricing, Session};
use crate::tui::theme;
use crate::util;

pub struct ModelsView;

impl ModelsView {
    pub fn render(
        session: &Session,
        pricing: &HashMap<String, ModelPricing>,
        frame: &mut Frame,
        area: Rect,
    ) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(4),  // header
                Constraint::Length(12), // message distribution
                Constraint::Length(12), // cost attribution
                Constraint::Min(5),    // details table
                Constraint::Length(1), // footer
            ])
            .split(area);

        render_header(session, frame, chunks[0]);
        render_message_chart(session, frame, chunks[1]);
        render_cost_chart(session, pricing, frame, chunks[2]);
        render_details_table(session, pricing, frame, chunks[3]);
        render_footer(frame, chunks[4]);
    }
}

fn render_header(session: &Session, frame: &mut Frame, area: Rect) {
    let summary = session.summary.as_deref().unwrap_or("Untitled session");
    let model_name = session.model.as_deref().unwrap_or("-");

    let title = Line::from(vec![
        Span::styled(" Models — Session: ", theme::header_style()),
        Span::styled(
            util::truncate(summary, 40),
            Style::default().fg(theme::text()),
        ),
        Span::raw(" "),
    ]);

    let content = vec![Line::from(vec![
        Span::styled(" Primary: ", theme::dim_style()),
        Span::styled(
            theme::short_model_name(model_name),
            theme::model_style(model_name),
        ),
    ])];

    let block = Paragraph::new(content).block(
        Block::bordered()
            .title(title)
            .border_style(theme::border_style()),
    );
    frame.render_widget(block, area);
}

fn sorted_models(session: &Session) -> Vec<(String, u32)> {
    let mut models: Vec<(String, u32)> = session
        .metrics
        .models_used
        .iter()
        .map(|(k, v)| (k.clone(), *v))
        .collect();
    models.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    models
}

fn render_message_chart(session: &Session, frame: &mut Frame, area: Rect) {
    let models = sorted_models(session);
    if models.is_empty() {
        let empty = Paragraph::new(" No model data available").block(
            Block::bordered()
                .title(Span::styled(
                    " Model Distribution (messages) ",
                    theme::header_style(),
                ))
                .border_style(theme::border_style()),
        );
        frame.render_widget(empty, area);
        return;
    }

    let max_msgs = models.iter().map(|(_, c)| *c as u64).max().unwrap_or(1);

    let bar_data: Vec<Bar> = models
        .iter()
        .map(|(name, count)| {
            let ratio = *count as f64 / max_msgs.max(1) as f64;
            Bar::default()
                .value(*count as u64)
                .label(Line::from(theme::short_model_name(name)))
                .style(Style::default().fg(theme::bar_gradient(ratio)))
                .value_style(Style::default().fg(theme::text()))
                .text_value(format!("{} msgs", count))
        })
        .collect();

    let chart = BarChart::default()
        .block(
            Block::bordered()
                .title(Span::styled(
                    " Model Distribution (messages) ",
                    theme::header_style(),
                ))
                .border_style(theme::border_style()),
        )
        .data(BarGroup::default().bars(&bar_data))
        .bar_width(1)
        .bar_gap(1)
        .direction(Direction::Horizontal)
        .bar_style(Style::default().fg(theme::amber()));

    frame.render_widget(chart, area);
}

fn estimate_model_cost(
    model: &str,
    msg_count: u32,
    total_messages: u32,
    session: &Session,
    pricing: &HashMap<String, ModelPricing>,
) -> f64 {
    let fraction = msg_count as f64 / total_messages.max(1) as f64;
    let output_tokens = session.metrics.total_output_tokens as f64 * fraction;
    let input_tokens = session.metrics.estimated_input_tokens as f64 * fraction;

    if let Some(price) = pricing.get(model).or_else(|| {
        pricing
            .iter()
            .find(|(k, _)| model.starts_with(k.as_str()) || k.starts_with(model))
            .map(|(_, v)| v)
    }) {
        output_tokens * price.output_per_million / 1_000_000.0
            + input_tokens * price.input_per_million / 1_000_000.0
    } else {
        0.0
    }
}

fn render_cost_chart(
    session: &Session,
    pricing: &HashMap<String, ModelPricing>,
    frame: &mut Frame,
    area: Rect,
) {
    let models = sorted_models(session);
    let total_messages: u32 = models.iter().map(|(_, c)| c).sum();

    if models.is_empty() {
        let empty = Paragraph::new(" No cost data available").block(
            Block::bordered()
                .title(Span::styled(
                    " Cost Attribution (estimated) ",
                    theme::header_style(),
                ))
                .border_style(theme::border_style()),
        );
        frame.render_widget(empty, area);
        return;
    }

    let costs: Vec<(String, f64)> = models
        .iter()
        .map(|(name, count)| {
            let cost = estimate_model_cost(name, *count, total_messages, session, pricing);
            (name.clone(), cost)
        })
        .collect();

    let max_cost = costs
        .iter()
        .map(|(_, c)| (*c * 100.0) as u64)
        .max()
        .unwrap_or(1);

    let bar_data: Vec<Bar> = costs
        .iter()
        .map(|(name, cost)| {
            let val = (*cost * 100.0) as u64; // cents for bar value
            let ratio = val as f64 / max_cost.max(1) as f64;
            Bar::default()
                .value(val)
                .label(Line::from(theme::short_model_name(name)))
                .style(Style::default().fg(theme::bar_gradient(ratio)))
                .value_style(Style::default().fg(theme::text()))
                .text_value(util::format_cost(*cost))
        })
        .collect();

    let chart = BarChart::default()
        .block(
            Block::bordered()
                .title(Span::styled(
                    " Cost Attribution (estimated) ",
                    theme::header_style(),
                ))
                .border_style(theme::border_style()),
        )
        .data(BarGroup::default().bars(&bar_data))
        .bar_width(1)
        .bar_gap(1)
        .direction(Direction::Horizontal)
        .bar_style(Style::default().fg(theme::amber()));

    frame.render_widget(chart, area);
}

fn is_premium_model(model: &str, pricing: &HashMap<String, ModelPricing>) -> bool {
    if let Some(p) = pricing.get(model) {
        return p.is_premium;
    }
    // Fuzzy match
    pricing
        .iter()
        .find(|(k, _)| model.starts_with(k.as_str()) || k.starts_with(model))
        .map(|(_, v)| v.is_premium)
        .unwrap_or(false)
}

fn render_details_table(
    session: &Session,
    pricing: &HashMap<String, ModelPricing>,
    frame: &mut Frame,
    area: Rect,
) {
    let models = sorted_models(session);
    let total_messages: u32 = models.iter().map(|(_, c)| c).sum();

    let header = Row::new(vec![
        Cell::from("Model"),
        Cell::from("Msgs"),
        Cell::from("Est Tokens"),
        Cell::from("Est Cost"),
        Cell::from("Tier"),
    ])
    .style(theme::table_header_style());

    let rows: Vec<Row> = models
        .iter()
        .map(|(name, count)| {
            let fraction = *count as f64 / total_messages.max(1) as f64;
            let est_tokens = (session.metrics.total_output_tokens as f64 * fraction
                + session.metrics.estimated_input_tokens as f64 * fraction)
                as u64;
            let cost = estimate_model_cost(name, *count, total_messages, session, pricing);
            let premium = is_premium_model(name, pricing);

            Row::new(vec![
                Cell::from(theme::short_model_name(name))
                    .style(theme::model_style(name)),
                Cell::from(format!("{}", count))
                    .style(Style::default().fg(theme::text())),
                Cell::from(util::format_tokens(est_tokens))
                    .style(Style::default().fg(theme::text())),
                Cell::from(util::format_cost(cost))
                    .style(Style::default().fg(theme::cost_color(cost))),
                Cell::from(if premium { "★" } else { "" })
                    .style(Style::default().fg(theme::gold())),
            ])
        })
        .collect();

    let widths = [
        Constraint::Min(20),
        Constraint::Length(6),
        Constraint::Length(12),
        Constraint::Length(10),
        Constraint::Length(4),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::bordered()
                .title(Span::styled(" Model Details ", theme::header_style()))
                .border_style(theme::border_style()),
        )
        .row_highlight_style(theme::selected_style());

    frame.render_widget(table, area);
}

fn render_footer(frame: &mut Frame, area: Rect) {
    let footer = Paragraph::new(Line::from(vec![
        Span::styled(" ► ", Style::default().fg(theme::gold())),
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
