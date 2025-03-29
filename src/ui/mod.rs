
use ratatui::{layout::{Constraint, Layout}, style::Style, text::Line, widgets::{Paragraph, Widget}, Frame};

use crate::app::{App, SourceContext};

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

    match &app.source {
        Some(c) => {
            frame.render_widget(source_widget(&c), rows[1])
        },
        None => {
        },
    }
}

fn source_widget(context: &SourceContext) -> Paragraph {
    let mut lines: Vec<Line> = Vec::new();
    let mut line_no = 1;

    for line in context.source.lines() {
        if line_no == context.line_no {
            lines.push(Line::styled(line.to_string(), Style::default().bg(ratatui::style::Color::Blue)));
        } else {
            lines.push(Line::raw(line.to_string()));
        }
        line_no += 1;
    }
    Paragraph::new(lines)
}
