#![allow(dead_code)]
use std::sync::atomic::{AtomicBool, Ordering};
use ratatui::style::{Color, Modifier, Style};

// Global theme toggle
static IS_LIGHT: AtomicBool = AtomicBool::new(false);

pub fn toggle_theme() {
    IS_LIGHT.fetch_xor(true, Ordering::Relaxed);
}

pub fn is_light_theme() -> bool {
    IS_LIGHT.load(Ordering::Relaxed)
}

// ── Dark theme (default) ─ toned-down warm palette ──────────────────────

mod dark {
    use ratatui::style::Color;

    pub const GOLD: Color = Color::Rgb(217, 180, 42);
    pub const AMBER: Color = Color::Rgb(210, 140, 25);
    pub const LIME: Color = Color::Rgb(115, 175, 42);
    pub const EMERALD: Color = Color::Rgb(62, 178, 135);
    pub const ORANGE: Color = Color::Rgb(215, 108, 35);

    pub const GREEN_BRIGHT: Color = Color::Rgb(82, 190, 115);
    pub const RED_SOFT: Color = Color::Rgb(210, 100, 100);
    pub const TEXT: Color = Color::Rgb(220, 218, 215);
    pub const DIM: Color = Color::Rgb(110, 104, 98);
    pub const MUTED: Color = Color::Rgb(78, 74, 70);

    pub const BG_SELECTED: Color = Color::Rgb(55, 48, 25);
    pub const BG_DARK: Color = Color::Rgb(28, 25, 23);
    pub const BG_PANEL: Color = Color::Rgb(38, 35, 33);

    pub const MODEL_PREMIUM: Color = Color::Rgb(217, 180, 42);
    pub const MODEL_STANDARD: Color = Color::Rgb(210, 140, 25);
    pub const MODEL_FAST: Color = Color::Rgb(115, 175, 42);

    pub const BAR_1: Color = Color::Rgb(105, 165, 38);
    pub const BAR_2: Color = Color::Rgb(140, 172, 48);
    pub const BAR_3: Color = Color::Rgb(170, 175, 38);
    pub const BAR_4: Color = Color::Rgb(195, 155, 22);
    pub const BAR_5: Color = Color::Rgb(210, 140, 25);
    pub const BAR_6: Color = Color::Rgb(215, 108, 35);
}

// ── Light theme ─ dark text on light backgrounds ────────────────────────

mod light {
    use ratatui::style::Color;

    pub const GOLD: Color = Color::Rgb(160, 120, 0);
    pub const AMBER: Color = Color::Rgb(170, 100, 0);
    pub const LIME: Color = Color::Rgb(60, 120, 15);
    pub const EMERALD: Color = Color::Rgb(20, 130, 90);
    pub const ORANGE: Color = Color::Rgb(185, 80, 10);

    pub const GREEN_BRIGHT: Color = Color::Rgb(30, 140, 60);
    pub const RED_SOFT: Color = Color::Rgb(190, 50, 50);
    pub const TEXT: Color = Color::Rgb(35, 33, 30);
    pub const DIM: Color = Color::Rgb(120, 115, 108);
    pub const MUTED: Color = Color::Rgb(165, 160, 152);

    pub const BG_SELECTED: Color = Color::Rgb(240, 228, 180);
    pub const BG_DARK: Color = Color::Rgb(248, 245, 240);
    pub const BG_PANEL: Color = Color::Rgb(238, 235, 228);

    pub const MODEL_PREMIUM: Color = Color::Rgb(160, 120, 0);
    pub const MODEL_STANDARD: Color = Color::Rgb(170, 100, 0);
    pub const MODEL_FAST: Color = Color::Rgb(60, 120, 15);

    pub const BAR_1: Color = Color::Rgb(60, 120, 15);
    pub const BAR_2: Color = Color::Rgb(95, 130, 20);
    pub const BAR_3: Color = Color::Rgb(130, 135, 15);
    pub const BAR_4: Color = Color::Rgb(155, 120, 5);
    pub const BAR_5: Color = Color::Rgb(170, 100, 0);
    pub const BAR_6: Color = Color::Rgb(185, 80, 10);
}

// ── Public color accessors (theme-aware) ────────────────────────────────

pub fn gold() -> Color { if is_light_theme() { light::GOLD } else { dark::GOLD } }
pub fn amber() -> Color { if is_light_theme() { light::AMBER } else { dark::AMBER } }
pub fn lime() -> Color { if is_light_theme() { light::LIME } else { dark::LIME } }
pub fn emerald() -> Color { if is_light_theme() { light::EMERALD } else { dark::EMERALD } }
pub fn orange() -> Color { if is_light_theme() { light::ORANGE } else { dark::ORANGE } }
pub fn green_bright() -> Color { if is_light_theme() { light::GREEN_BRIGHT } else { dark::GREEN_BRIGHT } }
pub fn red_soft() -> Color { if is_light_theme() { light::RED_SOFT } else { dark::RED_SOFT } }
pub fn text() -> Color { if is_light_theme() { light::TEXT } else { dark::TEXT } }
pub fn dim() -> Color { if is_light_theme() { light::DIM } else { dark::DIM } }
pub fn muted() -> Color { if is_light_theme() { light::MUTED } else { dark::MUTED } }
pub fn bg_selected() -> Color { if is_light_theme() { light::BG_SELECTED } else { dark::BG_SELECTED } }
pub fn bg_dark() -> Color { if is_light_theme() { light::BG_DARK } else { dark::BG_DARK } }
pub fn bg_panel() -> Color { if is_light_theme() { light::BG_PANEL } else { dark::BG_PANEL } }
pub fn model_premium() -> Color { if is_light_theme() { light::MODEL_PREMIUM } else { dark::MODEL_PREMIUM } }
pub fn model_standard() -> Color { if is_light_theme() { light::MODEL_STANDARD } else { dark::MODEL_STANDARD } }
pub fn model_fast() -> Color { if is_light_theme() { light::MODEL_FAST } else { dark::MODEL_FAST } }

pub fn bar_gradient(ratio: f64) -> Color {
    let l = is_light_theme();
    match ratio {
        r if r > 0.85 => if l { light::BAR_6 } else { dark::BAR_6 },
        r if r > 0.70 => if l { light::BAR_5 } else { dark::BAR_5 },
        r if r > 0.55 => if l { light::BAR_4 } else { dark::BAR_4 },
        r if r > 0.40 => if l { light::BAR_3 } else { dark::BAR_3 },
        r if r > 0.20 => if l { light::BAR_2 } else { dark::BAR_2 },
        _ => if l { light::BAR_1 } else { dark::BAR_1 },
    }
}

// Backward-compat constant aliases (dark theme only, used by WARM_GRADIENT etc.)
pub const GOLD: Color = dark::GOLD;
pub const AMBER: Color = dark::AMBER;
pub const LIME: Color = dark::LIME;
pub const EMERALD: Color = dark::EMERALD;
pub const ORANGE: Color = dark::ORANGE;
pub const GREEN_BRIGHT: Color = dark::GREEN_BRIGHT;
pub const RED_SOFT: Color = dark::RED_SOFT;
pub const WHITE: Color = dark::TEXT;
pub const DIM: Color = dark::DIM;
pub const MUTED: Color = dark::MUTED;
pub const BG_SELECTED: Color = dark::BG_SELECTED;
pub const BG_DARK: Color = dark::BG_DARK;
pub const BG_PANEL: Color = dark::BG_PANEL;
pub const MODEL_PREMIUM: Color = dark::MODEL_PREMIUM;
pub const MODEL_STANDARD: Color = dark::MODEL_STANDARD;
pub const MODEL_FAST: Color = dark::MODEL_FAST;
pub const BAR_1: Color = dark::BAR_1;
pub const BAR_2: Color = dark::BAR_2;
pub const BAR_3: Color = dark::BAR_3;
pub const BAR_4: Color = dark::BAR_4;
pub const BAR_5: Color = dark::BAR_5;
pub const BAR_6: Color = dark::BAR_6;

// ── Style helpers (use dynamic color accessors) ──────────────────────────

pub fn header_style() -> Style {
    Style::default().fg(gold()).add_modifier(Modifier::BOLD)
}

pub fn table_header_style() -> Style {
    Style::default().fg(amber()).add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
}

pub fn selected_style() -> Style {
    Style::default().bg(bg_selected()).fg(gold()).add_modifier(Modifier::BOLD)
}

pub fn cost_style() -> Style { Style::default().fg(gold()) }
pub fn token_style() -> Style { Style::default().fg(text()) }
pub fn warning_style() -> Style { Style::default().fg(orange()).add_modifier(Modifier::BOLD) }
pub fn dim_style() -> Style { Style::default().fg(dim()) }
pub fn active_style() -> Style { Style::default().fg(lime()) }
pub fn border_style() -> Style { Style::default().fg(muted()) }

pub fn model_color(model_name: &str) -> Color {
    let name = model_name.to_lowercase();
    if name.contains("opus") || (name.contains("gpt-5.") && name.contains("max")) {
        model_premium()
    } else if name.contains("sonnet") || name.contains("codex") || (name.contains("gpt-5.") && !name.contains("mini")) {
        model_standard()
    } else if name.contains("haiku") || name.contains("mini") || name.contains("gpt-4") {
        model_fast()
    } else {
        emerald()
    }
}

pub fn model_style(model_name: &str) -> Style { Style::default().fg(model_color(model_name)) }
pub fn active_tab_style() -> Style { Style::default().fg(gold()).add_modifier(Modifier::BOLD | Modifier::UNDERLINED) }
pub fn inactive_tab_style() -> Style { Style::default().fg(dim()) }

pub fn cost_color(cost: f64) -> Color {
    if cost > 10.0 { orange() }
    else if cost > 5.0 { amber() }
    else if cost > 1.0 { gold() }
    else { green_bright() }
}

pub fn short_model_name(model: &str) -> String {
    model.replace("claude-", "").replace("gpt-", "gpt")
}

pub fn agent_type_color(agent_type: &str) -> Color {
    match agent_type {
        "explore" => lime(),
        "general-purpose" => amber(),
        "code-review" => gold(),
        "task" => emerald(),
        _ => dim(),
    }
}
