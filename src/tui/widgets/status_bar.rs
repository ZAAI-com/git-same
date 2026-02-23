//! Bottom status bar showing context-sensitive keybindings.

use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

/// Render a status bar at the given area.
pub fn render(frame: &mut Frame, area: Rect, hint: &str) {
    let bar = Paragraph::new(Line::from(vec![Span::styled(
        format!(" {} ", hint),
        Style::default().fg(Color::DarkGray),
    )]));
    frame.render_widget(bar, area);
}
