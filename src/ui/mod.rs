use crate::app::InputMode;
use ratatui::layout::Constraint;
use ratatui::layout::Layout;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Widget;
use ratatui::Frame;
use std::ops::Div;

pub fn render(app: &mut App, frame: &mut Frame) {
    let constraints = vec![
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(4),
        Constraint::Length(match app.input_mode {
            InputMode::Normal => match app.command_response {
                Some(ref response) => response.lines().count() as u16 + 1,
                None => 1,
            },
            _ => 1,
        }),
    ];

    let rows = Layout::default()
        .margin(0)
        .constraints(constraints)
        .split(frame.area());

    frame.render_widget(status_widget(&app), rows[0]);

    match app.views.selected {}
    match &app.source {
        Some(c) => {
            frame.render_widget(
                Paragraph::new(Line::from(vec![Span::styled(
                    c.filename.clone(),
                    Style::default().fg(Color::Green),
                )])),
                rows[1],
            );
            frame.render_widget(source_widget(&c, rows[2].clone()), rows[2]);
        }
        None => {}
    }

    match app.input_mode {
        InputMode::Normal => {
            frame.render_widget(
                Paragraph::new(app.command_response.clone().unwrap_or("".to_string())),
                rows[3],
            );
        }
        InputMode::Command => {
            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::raw(":"),
                    Span::raw(app.command_input.value()),
                ])),
                rows[3],
            );
        }
    }
}

fn status_widget(app: &App) -> Paragraph {
    Paragraph::new(vec![Line::from(vec![
        Span::styled(
            " 🐛 ",
            Style::default()
                .fg(Color::Cyan)
                .bg(Color::Rgb(20, 20, 20))
                .fg(Color::Black),
        ),
        Span::styled(
            format!("  {} ", app.input_mode.to_string()),
            Style::default().bg(match app.input_mode {
                InputMode::Normal => Color::Blue,
                InputMode::Command => Color::Red,
            }),
        ),
        Span::styled(
            format!(" 󱘖 {} ", app.state.to_string()),
            Style::default()
                .add_modifier(Modifier::BOLD)
                .bg(match app.state {
                    AppState::Listening => Color::Yellow,
                    AppState::Connected => Color::Green,
                })
                .fg(match app.state {
                    AppState::Listening => Color::Black,
                    AppState::Connected => Color::Black,
                }),
        ),
        Span::styled(
            match app.notification.is_visible() {
                true => format!(" {} ", app.notification.message.clone()),
                false => "".to_string(),
            },
            Style::default()
                .fg(match app.notification.level {
                    NotificationLevel::Error => Color::White,
                    _ => Color::White,
                })
                .bg(match app.notification.level {
                    NotificationLevel::Error => Color::Red,
                    _ => Color::Black,
                }),
        ),
    ])])
}

fn source_widget(context: &SourceContext, area: Rect) -> Paragraph {
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
    if context.line_no as u16 > area.height {
        let offset = (context.line_no as u16).saturating_sub(area.height.div(2));
        lines = lines[offset as usize..].to_vec();
    }
    Paragraph::new(lines)
}
