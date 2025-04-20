use super::listen::ListenView;
use super::session::SessionView;
use super::session::SessionViewMode;
use super::View;
use crate::app::App;
use crate::app::InputMode;
use crate::app::CurrentView;
use crate::event::input::AppEvent;
use crate::notification::NotificationLevel;
use ratatui::layout::Constraint;
use ratatui::layout::Layout;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Paragraph;
use ratatui::Frame;

pub struct LayoutView {}

impl View for LayoutView {
    fn handle(_app: &App, _key: AppEvent) -> Option<AppEvent> {
        None
    }

    fn draw(app: &App, f: &mut Frame, area: Rect) {
        let constraints = vec![
            Constraint::Length(1),
            Constraint::Min(4),
            Constraint::Length(match app.input_mode {
                InputMode::Normal => match app.command_response {
                    Some(ref response) => response.lines().count() as u16 + 1,
                    None => 0,
                },
                _ => 1,
            }),
        ];

        let rows = Layout::default()
            .margin(0)
            .constraints(constraints)
            .split(area);

        f.render_widget(status_widget(app), rows[0]);

        match app.view_current {
            CurrentView::Listen => ListenView::draw(app, f, rows[1]),
            CurrentView::Session => SessionView::draw(app, f, rows[1]),
        }

        match app.input_mode {
            InputMode::Normal => {
                f.render_widget(
                    Paragraph::new(app.command_response.clone().unwrap_or("".to_string())),
                    rows[2],
                );
            }
            InputMode::Command => {
                f.render_widget(
                    Paragraph::new(Line::from(vec![
                        Span::raw(":"),
                        Span::raw(app.command_input.value()),
                    ])),
                    rows[2],
                );
            }
        }
    }
}

fn status_widget(app: &App) -> Paragraph {
    Paragraph::new(vec![Line::from(vec![
        Span::styled(
            format!("{:<3}°🐛", app.history.current().map_or("".to_string(), |entry| {
                entry.stack.depth().to_string()
            })),
            Style::default()
                .bg(Color::Magenta)
                .bold()
                .fg(Color::White),

        ),
        Span::styled(
            format!("  {} ", app.input_mode),
            Style::default().bg(match app.input_mode {
                InputMode::Normal => Color::Blue,
                InputMode::Command => Color::Red,
            }),
        ),
        Span::styled(
            format!(
                " 󱘖 {} ",
                if app.is_connected { "connected".to_string() } else { format!("listening {}", app.config.listen) }
            ),
            Style::default()
                .add_modifier(Modifier::BOLD)
                .bg(match app.is_connected {
                    false => Color::Yellow,
                    true => Color::Green,
                })
                .fg(match app.is_connected {
                    false => Color::Black,
                    true => Color::Black,
                }),
        ),
        Span::styled(
            (match app.session_view.mode {
                    SessionViewMode::Current => "".to_string(),
                    SessionViewMode::History => format!(
                    " {} / {} history [p] to go back [n] to go forwards [b] to return",
                    app.history.offset + 1,
                    app.history.len()
                ),
                }).to_string(),
            Style::default().bg(Color::Red),
        ),
        Span::styled(
            match app.notification.is_visible() {
                true => format!(" {} ", app.notification.message.clone()),
                false => "".to_string(),
            },
            Style::default()
                .fg(match app.notification.level {
                    NotificationLevel::Info => Color::Green,
                    _ => Color::White,
                })
                .bg(match app.notification.level {
                    NotificationLevel::Error => Color::Red,
                    _ => Color::Black,
                }),
        ),
    ])])
}
