
use ratatui::{layout::{Constraint, Layout}, widgets::Paragraph, Frame};

use crate::app::App;

pub fn render(app: &mut App, frame: &mut Frame) {
    let rows = Layout::default()
        .margin(0)
        .constraints(
            [
                Constraint::Length(3),
                Constraint::Min(4),
            ]
            .as_ref(),
        )
        .split(frame.area());

    frame.render_widget(Paragraph::new(app.state.to_string()), rows[0]);
}
