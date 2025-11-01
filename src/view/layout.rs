use super::eval::EvalDialog;
use super::help::HelpView;
use super::listen::ListenView;
use super::session::SessionView;
use super::session::SessionViewMode;
use super::View;
use crate::app::ActiveDialog;
use crate::app::App;
use crate::app::ListenStatus;
use crate::app::SelectedView;
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
    fn handle(_app: &mut App, _key: AppEvent) -> Option<AppEvent> {
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
        f.render_widget(notification_widget(app), rows[0]);

        match app.view_current {
            SelectedView::Listen => ListenView::draw(app, f, rows[1]),
            SelectedView::Session => SessionView::draw(app, f, rows[1]),
            SelectedView::Help => HelpView::draw(app, f, rows[1]),
        }

        if let Some(dialog) = &app.active_dialog {
            match &dialog {
                ActiveDialog::Eval => EvalDialog::draw(app, f, area),
            }
        }
    }
}

fn notification_widget(app: &App) -> Paragraph<'_> {
    Paragraph::new(vec![Line::from(Span::styled(
        match app.notification.is_visible() {
            true => format!(
                "{} {}",
                app.notification.message.clone(),
                app.notification.countdown_char()
            ),
            false => "".to_string(),
        },
        match app.notification.level {
            NotificationLevel::Error => app.theme().notification_error,
            NotificationLevel::Warning => app.theme().notification_warning,
            NotificationLevel::Info => app.theme().notification_info,
            NotificationLevel::None => Style::default(),
        },
    ))
    .alignment(ratatui::layout::Alignment::Right)])
}

fn status_widget<'a>(app: &'a App) -> Paragraph<'a> {
    Paragraph::new(vec![Line::from(vec![
        Span::styled(
            format!(
                " 󱘖 {} ",
                match app.listening_status {
                    ListenStatus::Connected => "connected".to_string(),

                    ListenStatus::Listening => app.config.listen.to_string(),
                    ListenStatus::Refusing => "refusing".to_string(),
                },
            ),
            match app.listening_status.is_connected() {
                false => app.theme().widget_inactive,
                true => app.theme().widget_active,
            },
        ),
        Span::styled(
            format!(
                "   {:<3} ",
                app.history.current().map_or("n/a".to_string(), |entry| {
                    entry.stacks.len().to_string()
                })
            ),
            app.theme().widget_inactive,
        ),
        Span::styled(
            (match app.session_view.mode {
                SessionViewMode::Current => match app.listening_status.is_connected() {
                    true => format!("   {} / ∞", app.history.offset + 1),
                    false => "   0 / 0".to_string(),
                },
                SessionViewMode::History => match app.listening_status {
                    ListenStatus::Connected => format!(
                        "   {} / {} history [p] to go back [n] to go forwards [b] to return",
                        app.history.offset + 1,
                        app.history.len()
                    ),
                    ListenStatus::Refusing => format!(
                        "   {} / {} disconnected [p] to go back [n] to go forwards [b] to listen",
                        app.history.offset + 1,
                        app.history.len()
                    ),
                    ListenStatus::Listening => String::new(),
                },
            })
            .to_string(),
            match app.session_view.mode {
                SessionViewMode::Current => app.theme().widget_mode_debug,
                SessionViewMode::History => app.theme().widget_mode_history,
            },
        ),
    ])])
}
