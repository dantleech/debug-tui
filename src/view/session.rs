use super::source::source_widget;
use super::View;
use crate::app::App;
use crate::app::InputMode;
use crate::event::input::AppEvent;
use crossterm::event::KeyCode;
use ratatui::layout::Constraint;
use ratatui::layout::Layout;
use ratatui::style::Color;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Paragraph;
use ratatui::Frame;

pub struct SessionView {}

impl View for SessionView {
    fn handle(app: &App, event: AppEvent) -> Option<AppEvent> {
        if let AppEvent::Input(e) = event {
            if app.input_mode != InputMode::Command {
                if let KeyCode::Char(char) = e.code {
                    return match char {
                        'r' => Some(AppEvent::Run),
                        'n' => Some(AppEvent::StepInto),
                        'o' => Some(AppEvent::StepOver),
                        _ => None,
                    };
                }
            }
        }
        None
    }

    fn draw(app: &mut App, f: &mut Frame, area: ratatui::prelude::Rect) {
        let constraints = vec![Constraint::Length(1), Constraint::Min(1)];
        let rows = Layout::default()
            .margin(0)
            .constraints(constraints)
            .split(area);

        if let Some(source_context) = &app.source {
            f.render_widget(
                Paragraph::new(Line::from(vec![Span::styled(
                    source_context.filename.clone(),
                    Style::default().fg(Color::Green),
                )])),
                rows[0],
            );
            f.render_widget(source_widget(source_context, rows[1]), rows[1]);
        };
    }
}

impl SessionView {
    pub(crate) fn new() -> Self {
        Self {}
    }
}
