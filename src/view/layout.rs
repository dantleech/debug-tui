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
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Block;
use ratatui::widgets::Paragraph;
use ratatui::Frame;

pub struct LayoutView {}

impl View for LayoutView {
    fn handle(_app: &App, _key: AppEvent) -> Option<AppEvent> {
        None
    }

    fn draw(app: &App, f: &mut Frame, area: Rect) {
        let constraints = vec![Constraint::Length(1), Constraint::Min(4)];

        let rows = Layout::default()
            .margin(0)
            .constraints(constraints)
            .split(area);

        f.render_widget(Block::default().style(app.theme().background), area);
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
                if app.is_connected {
                    "connected".to_string()
                } else {
                    app.config.listen.to_string()
                }
            ),
            match app.is_connected {
                false => app.theme().widget_inactive,
                true => app.theme().widget_active,
            },
        ),
        Span::styled(
            format!(
                "   {:<3} ",
                app.history.current().map_or("n/a".to_string(), |entry| {
                    entry.stack.depth().to_string()
                })
            ),
            app.theme().widget_inactive,
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
            })
            .to_string(),
            match app.session_view.mode {
                SessionViewMode::Current => app.theme().widget_mode_debug,
                SessionViewMode::History => app.theme().widget_mode_history,
            },
        ),
        Span::styled(
            match app.notification.is_visible() {
                true => format!(" {} ", app.notification.message.clone()),
                false => "".to_string(),
            },
            match app.notification.level {
                NotificationLevel::Error => app.theme().notification_error,
                NotificationLevel::Info => app.theme().notification_info,
                NotificationLevel::None => Style::default(),
            },
        ),
    ])])
}
