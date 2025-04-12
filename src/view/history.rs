use super::context;
use super::source;
use super::View;
use crate::app::CurrentView;
use crate::app::InputMode;
use crate::event::input::AppEvent;
use crossterm::event::KeyCode;
use ratatui::layout::Constraint;
use ratatui::layout::Layout;
use ratatui::style::Color;
use ratatui::style::Style;
use ratatui::widgets::Paragraph;

pub struct HistoryView {}

impl View for HistoryView {
    fn handle(
        app: &crate::app::App,
        event: crate::event::input::AppEvent,
    ) -> Option<crate::event::input::AppEvent> {
        if let AppEvent::Input(e) = event {
            if app.input_mode == InputMode::Normal {
                if e.code == KeyCode::Esc {
                    return Some(AppEvent::ChangeView(CurrentView::Session));
                }
                if let KeyCode::Char(char) = e.code {
                    return match char {
                        'n' => Some(AppEvent::HistoryNext),
                        'p' => Some(AppEvent::HistoryPrevious),
                        'b' => Some(AppEvent::ChangeView(CurrentView::Session)),
                        _ => None,
                    };
                }
            }
        }
        None
    }

    fn draw(app: &mut crate::app::App, frame: &mut ratatui::Frame, area: ratatui::prelude::Rect) {
        let constraints = vec![Constraint::Length(1), Constraint::Min(1)];
        let rows = Layout::default()
            .margin(0)
            .constraints(constraints)
            .split(area);

        let cols = Layout::horizontal(vec![
            Constraint::Percentage(70),
            Constraint::Percentage(30),
        ]).split(rows[1]);


        match app.history.get(app.history_offset) {
            Some(entry) => {
                source::draw(&entry.source, frame, cols[0]);
                context::draw(&entry.context, frame, cols[1]);
            },
            None => (),
        }

        frame.render_widget(
            Paragraph::new(
                format!(
                    "{} / {} History [p] to go back [n] to go forwards",
                    app.history_offset + 1,
                    app.history.len()
                )
            ).style(Style::default().bg(Color::Red)), rows[0]
        );
    }
}
