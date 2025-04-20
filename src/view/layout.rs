use super::help::HelpView;
use super::listen::ListenView;
use super::session::SessionView;
use super::session::SessionViewMode;
use super::View;
use crate::app::App;
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
        ];

        let rows = Layout::default()
            .margin(0)
            .constraints(constraints)
            .split(area);

        f.render_widget(status_widget(app), rows[0]);

        match app.view_current {
            CurrentView::Listen => ListenView::draw(app, f, rows[1]),
            CurrentView::Session => SessionView::draw(app, f, rows[1]),
            CurrentView::Help => HelpView::draw(app, f, rows[1]),
        }
    }
}

fn status_widget(app: &App) -> Paragraph {
    Paragraph::new(vec![Line::from(vec![
        Span::styled(
            format!(
                " 󱘖 {} ",
                if app.is_connected { "".to_string() } else { format!("{}", app.config.listen) }
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
            format!("   {:<3} ", app.history.current().map_or("n/a".to_string(), |entry| {
                entry.stack.depth().to_string()
            })),
            Style::default()
                .bg(Color::Magenta)
                .bold()
                .fg(Color::White),

        ),
        Span::styled(
            (match app.session_view.mode {
                    SessionViewMode::Current => match app.is_connected {
                        true => format!("   {} / ∞", app.history.offset + 1),
                        false => "   0 / 0".to_string(),
                    },
                    SessionViewMode::History => format!(
                    "   {} / {} history [p] to go back [n] to go forwards [b] to return",
                    app.history.offset + 1,
                    app.history.len()
                ),
                }).to_string(),
            Style::default().bg(
                match app.session_view.mode {
                    SessionViewMode::Current => Color::Blue,
                    SessionViewMode::History => Color::Red,
                }
            ),
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
