
use ratatui::{layout::{Constraint, Layout}, style::{Color, Style}, text::{Line, Span}, widgets::{Paragraph, Widget}, Frame};

use crate::app::{App, SourceContext};

pub fn render(app: &mut App, frame: &mut Frame) {
    let rows = Layout::default()
        .margin(0)
        .constraints(
            [
                Constraint::Length(3),
                Constraint::Length(1),
                Constraint::Min(4),
            ]
            .as_ref(),
        )
        .split(frame.area());

    frame.render_widget(Paragraph::new(
        format!(
            "Client: {:<10} Server: {}",
            app.state.to_string(),
            app.server_status.to_string()
        )),
        rows[0]
    );

    match &app.source {
        Some(c) => {
            frame.render_widget(Paragraph::new(
                Line::from(vec![
                    Span::styled(c.filename.clone(), Style::default().fg(Color::Green))
                ]),
            ), rows[1]);
            frame.render_widget(source_widget(&c), rows[2]);
        },
        None => {
        },
    }
}

fn source_widget(context: &SourceContext) -> Paragraph {
    let mut lines: Vec<Line> = Vec::new();
    let mut line_no = 1;

    for line in context.source.lines() {
        lines.push(Line::from(vec![
            Span::styled(format!("{:<6}", line_no), Style::default().fg(Color::Yellow)),
            match context.line_no == line_no {
                true => Span::styled(line.to_string(), Style::default().bg(Color::Blue)),
                false => Span::raw(line.to_string()),
            }
        ]));

        line_no += 1;
    }
    Paragraph::new(lines)
}
