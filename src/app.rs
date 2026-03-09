use std::io;
use std::time::{Duration, Instant};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::prelude::*;
use crate::data::models::Session;
use crate::tui::{
    dashboard::DashboardView,
    session::SessionDetailView,
    agents::AgentsView,
    tools::ToolsView,
    trends::TrendsView,
    help::HelpView,
    models::ModelsView,
    live::LiveView,
};

pub enum View {
    Dashboard,
    SessionDetail(String),
    Agents(String),
    Tools(String),
    Models(String),
    Live(String),
    Trends,
    Help,
}

pub struct App {
    pub view: View,
    pub dashboard: DashboardView,
    pub agents_view: AgentsView,
    pub sessions: Vec<Session>,
    pub should_quit: bool,
    pub last_refresh: Instant,
    pub has_active_sessions: bool,
}

impl App {
    pub fn new(sessions: Vec<Session>) -> Self {
        let has_active = sessions.iter().any(|s| s.is_active);
        let dashboard = DashboardView::new(sessions.clone());
        Self {
            view: View::Dashboard,
            dashboard,
            agents_view: AgentsView::new(),
            sessions,
            should_quit: false,
            last_refresh: Instant::now(),
            has_active_sessions: has_active,
        }
    }

    pub fn run(&mut self, terminal: &mut Terminal<impl Backend>) -> io::Result<()> {
        let live_interval = Duration::from_secs(2);

        loop {
            terminal.draw(|frame| self.render(frame))?;

            // Only auto-refresh in the Live view; all other views are manual (r key)
            let timeout = match &self.view {
                View::Live(_) => live_interval,
                _ => Duration::from_secs(120), // just needs to be responsive to key input
            };

            if crossterm::event::poll(timeout)? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        self.handle_key(key.code);
                    }
                }
            }

            // Auto-refresh only in Live view
            if matches!(&self.view, View::Live(_)) && self.last_refresh.elapsed() >= live_interval {
                self.refresh_data();
            }

            if self.should_quit {
                break;
            }
        }
        Ok(())
    }

    fn find_session(&self, id: &str) -> Option<&Session> {
        self.sessions.iter().find(|s| s.id == id)
    }

    fn render(&mut self, frame: &mut Frame) {
        let area = frame.area();
        match &self.view {
            View::Dashboard => {
                self.dashboard.render(frame, area);
            }
            View::SessionDetail(ref id) => {
                if let Some(session) = self.find_session(id) {
                    SessionDetailView::render(session, frame, area);
                }
            }
            View::Agents(ref id) => {
                if let Some(session) = self.sessions.iter().find(|s| s.id == *id) {
                    self.agents_view.render(session, frame, area);
                }
            }
            View::Tools(ref id) => {
                if let Some(session) = self.find_session(id) {
                    ToolsView::render(session, frame, area);
                }
            }
            View::Models(ref id) => {
                if let Some(session) = self.find_session(id) {
                    let pricing = crate::config::load_pricing();
                    ModelsView::render(session, &pricing, frame, area);
                }
            }
            View::Live(ref id) => {
                if let Some(session) = self.find_session(id) {
                    LiveView::render(session, frame, area);
                }
            }
            View::Trends => {
                TrendsView::render(&self.sessions, frame, area);
            }
            View::Help => {
                // Render current view behind the help overlay
                self.dashboard.render(frame, area);
                HelpView::render(frame, area);
            }
        }
    }

    fn handle_key(&mut self, key: KeyCode) {
        match &self.view {
            View::Dashboard => self.handle_dashboard_key(key),
            View::SessionDetail(id) => {
                let id = id.clone();
                self.handle_session_detail_key(key, id);
            }
            View::Agents(id) => {
                let id = id.clone();
                self.handle_agents_key(key, id);
            }
            View::Tools(id) => {
                let id = id.clone();
                self.handle_tools_key(key, id);
            }
            View::Models(id) => {
                let id = id.clone();
                self.handle_models_key(key, id);
            }
            View::Live(id) => {
                let id = id.clone();
                self.handle_live_key(key, id);
            }
            View::Trends => self.handle_trends_key(key),
            View::Help => self.handle_help_key(key),
        }
    }

    fn handle_dashboard_key(&mut self, key: KeyCode) {
        use crate::tui::dashboard::SortColumn;
        match key {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char('d') => crate::tui::theme::toggle_theme(),
            KeyCode::Down | KeyCode::Char('j') => self.dashboard.next(),
            KeyCode::Up | KeyCode::Char('k') => self.dashboard.previous(),
            KeyCode::Enter => {
                if let Some(id) = self.dashboard.selected_session_id() {
                    self.view = View::SessionDetail(id);
                }
            }
            KeyCode::Char('s') => self.dashboard.toggle_sort(),
            KeyCode::Char('S') => self.dashboard.reverse_sort(),
            // Number keys for direct column sort (matches column order)
            KeyCode::Char('1') => self.dashboard.sort_by(SortColumn::Status),
            KeyCode::Char('2') => self.dashboard.sort_by(SortColumn::Name),
            KeyCode::Char('3') => self.dashboard.sort_by(SortColumn::Model),
            KeyCode::Char('4') => self.dashboard.sort_by(SortColumn::Tokens),
            KeyCode::Char('5') => self.dashboard.sort_by(SortColumn::Cost),
            KeyCode::Char('6') => self.dashboard.sort_by(SortColumn::Turns),
            KeyCode::Char('7') => self.dashboard.sort_by(SortColumn::Agents),
            KeyCode::Char('8') => self.dashboard.sort_by(SortColumn::Duration),
            KeyCode::Char('9') => self.dashboard.sort_by(SortColumn::Source),
            KeyCode::Char('r') => self.refresh_data(),
            KeyCode::Tab => self.dashboard.toggle_source(),
            KeyCode::Char('L') => {
                if let Some(id) = self.dashboard.selected_session_id() {
                    if self.find_session(&id).map_or(false, |s| s.is_active) {
                        self.view = View::Live(id);
                    }
                }
            }
            KeyCode::Char('T') => self.view = View::Trends,
            KeyCode::Char('?') => self.view = View::Help,
            _ => {}
        }
    }

    fn handle_trends_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char('d') => crate::tui::theme::toggle_theme(),
            KeyCode::Esc | KeyCode::Backspace => self.view = View::Dashboard,
            _ => {}
        }
    }

    fn handle_session_detail_key(&mut self, key: KeyCode, id: String) {
        match key {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char('d') => crate::tui::theme::toggle_theme(),
            KeyCode::Esc | KeyCode::Backspace => self.view = View::Dashboard,
            KeyCode::Char('a') => {
                self.agents_view = AgentsView::new();
                self.view = View::Agents(id);
            }
            KeyCode::Char('t') => self.view = View::Tools(id),
            KeyCode::Char('m') => self.view = View::Models(id),
            KeyCode::Char('l') => {
                if self.find_session(&id).map_or(false, |s| s.is_active) {
                    self.view = View::Live(id);
                }
            }
            KeyCode::Char('?') => self.view = View::Help,
            _ => {}
        }
    }

    fn handle_agents_key(&mut self, key: KeyCode, id: String) {
        match key {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char('d') => crate::tui::theme::toggle_theme(),
            KeyCode::Esc | KeyCode::Backspace => self.view = View::SessionDetail(id),
            KeyCode::Down | KeyCode::Char('j') => {
                let len = self.find_session(&id).map_or(0, |s| s.sub_agents.len());
                self.agents_view.next(len);
            }
            KeyCode::Up | KeyCode::Char('k') => self.agents_view.previous(),
            _ => {}
        }
    }

    fn handle_tools_key(&mut self, key: KeyCode, id: String) {
        match key {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char('d') => crate::tui::theme::toggle_theme(),
            KeyCode::Esc | KeyCode::Backspace => self.view = View::SessionDetail(id),
            _ => {}
        }
    }

    fn handle_models_key(&mut self, key: KeyCode, id: String) {
        match key {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char('d') => crate::tui::theme::toggle_theme(),
            KeyCode::Esc | KeyCode::Backspace => self.view = View::SessionDetail(id),
            _ => {}
        }
    }

    fn handle_live_key(&mut self, key: KeyCode, id: String) {
        match key {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char('d') => crate::tui::theme::toggle_theme(),
            KeyCode::Char('r') => self.refresh_data(),
            KeyCode::Esc | KeyCode::Backspace => self.view = View::SessionDetail(id),
            _ => {}
        }
    }

    fn handle_help_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char('d') => crate::tui::theme::toggle_theme(),
            KeyCode::Esc | KeyCode::Char('?') => self.view = View::Dashboard,
            _ => {}
        }
    }

    fn refresh_data(&mut self) {
        // Reload sessions from disk
        let mut sessions = Vec::new();
        sessions.extend(crate::data::copilot::load_sessions().unwrap_or_default());
        sessions.extend(crate::data::claude::load_sessions().unwrap_or_default());

        let pricing = crate::config::load_pricing();
        crate::cost::estimator::estimate_all_costs(&mut sessions, &pricing);
        sessions.sort_by(|a, b| b.started_at.cmp(&a.started_at));

        self.has_active_sessions = sessions.iter().any(|s| s.is_active);
        self.sessions = sessions.clone();

        // Preserve current selection and view state
        let selected = self.dashboard.table_state.selected();
        let sort = self.dashboard.sort_column;
        let ascending = self.dashboard.sort_ascending;
        let filter = self.dashboard.source_filter;

        self.dashboard.sessions = sessions;
        self.dashboard.sort_column = sort;
        self.dashboard.sort_ascending = ascending;
        self.dashboard.source_filter = filter;
        self.dashboard.has_active_sessions = self.has_active_sessions;
        self.dashboard.sort_sessions();

        if let Some(sel) = selected {
            let len = self.dashboard.filtered_sessions().len();
            if sel < len {
                self.dashboard.table_state.select(Some(sel));
            } else if len > 0 {
                self.dashboard.table_state.select(Some(0));
            }
        }

        self.last_refresh = Instant::now();
    }
}
