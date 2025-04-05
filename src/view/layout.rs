use super::listen::ListenView;
use super::session::SessionView;
use super::View;
use crate::app::App;
use crate::app::AppState;
use crate::app::InputMode;
use crate::app::SelectedView;
use crate::event::input::AppEvent;
use crate::notification::NotificationLevel;
use ratatui::buffer::Buffer;
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

pub struct LayoutView {}

impl View for LayoutView {
    fn handle(app: &App, key: AppEvent) -> Option<AppEvent> {
        None
    }

    fn draw(app: &mut App, f: &mut Frame, area: Rect) {
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
            .split(area);

        f.render_widget(status_widget(&app), rows[1]);

        match app.view_current {
            SelectedView::Listen => ListenView::draw(app, f, rows[1]),
            SelectedView::Session => SessionView::draw(app, f, rows[1]),
        }

        match app.input_mode {
            InputMode::Normal => {
                f.render_widget(
                    Paragraph::new(app.command_response.clone().unwrap_or("".to_string())),
                    rows[3],
                );
            }
            InputMode::Command => {
                f.render_widget(
                    Paragraph::new(Line::from(vec![
                        Span::raw(":"),
                        Span::raw(app.command_input.value()),
                    ])),
                    rows[3],
                );
            }
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
            format!(
                " 󱘖 {} ",
                app.client
                    .is_connected()
                    .then(|| "connected".to_string())
                    .unwrap_or_else(|| "listening".to_string())
            ),
            Style::default()
                .add_modifier(Modifier::BOLD)
                .bg(match app.client.is_connected() {
                    false => Color::Yellow,
                    true => Color::Green,
                })
                .fg(match app.client.is_connected() {
                    false => Color::Black,
                    true => Color::Black,
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
