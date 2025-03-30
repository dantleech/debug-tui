use ratatui::{
    layout::{Constraint, Layout},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Paragraph, Widget},
    Frame,
};

use crate::app::{App, InputMode, SourceContext};

pub fn render(app: &mut App, frame: &mut Frame) {
    let mut constraints = vec![
        Constraint::Length(3),
        Constraint::Length(1),
        Constraint::Min(4),
    ];

    if app.input_mode == InputMode::Command {
        constraints.push(Constraint::Length(1));
    }

    let rows = Layout::default()
        .margin(0)
        .constraints(constraints)
        .split(frame.area());

    frame.render_widget(
        Paragraph::new(format!(
            "Client: {:<10} Server: {}",
            app.state.to_string(),
            app.server_status.to_string()
        )),
        rows[0],
    );

    match &app.source {
        Some(c) => {
            frame.render_widget(
                Paragraph::new(Line::from(vec![Span::styled(
                    c.filename.clone(),
                    Style::default().fg(Color::Green),
                )])),
                rows[1],
            );
            frame.render_widget(source_widget(&c), rows[2]);
        }
        None => {}
    }

    if app.input_mode == InputMode::Command {
        frame.render_widget(Paragraph::new("Command line!"), rows[3]);
    }
}

fn source_widget(context: &SourceContext) -> Paragraph {
    let mut lines: Vec<Line> = Vec::new();
    let mut line_no = 1;

    for line in context.source.lines() {
        lines.push(Line::from(vec![
            Span::styled(
                format!("{:<6}", line_no),
                Style::default().fg(Color::Yellow),
            ),
            match context.line_no == line_no {
                true => Span::styled(line.to_string(), Style::default().bg(Color::Blue)),
                false => Span::raw(line.to_string()),
            },
        ]));

        line_no += 1;
    }
    Paragraph::new(lines)
}

