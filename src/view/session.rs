use super::source::source_widget;
use super::View;
use crate::app::App;
use crate::app::AppState;
use crate::app::InputMode;
use crate::dbgp::client::ContinuationResponse;
use crate::event::input::AppEvent;
use crate::event::input::ServerStatus;
use anyhow::Result;
use crossterm::event::KeyCode;
use ratatui::layout::Constraint;
use ratatui::layout::Layout;
use ratatui::style::Color;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Widget;
use ratatui::Frame;
use tokio::sync::mpsc::Sender;

pub struct SessionView {}

impl View for SessionView {
    fn handle(app: &App, event: AppEvent) -> Option<AppEvent> {
        match event {
            AppEvent::Input(e) => {
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
            _ => (),
        }
        None
    }

    fn draw(app: &mut App, f: &mut Frame, area: ratatui::prelude::Rect) {
        let constraints = vec![Constraint::Length(1), Constraint::Min(1)];
        let rows = Layout::default()
            .margin(0)
            .constraints(constraints)
            .split(area);

        match &app.source {
            Some(source_context) => {
                f.render_widget(
                    Paragraph::new(Line::from(vec![Span::styled(
                        source_context.filename.clone(),
                        Style::default().fg(Color::Green),
                    )])),
                    rows[0],
                );
                f.render_widget(source_widget(&source_context, rows[1]), rows[1]);
            }
            None => (),
        };
    }
}

impl SessionView {
    pub(crate) fn new() -> Self {
        Self {}
    }
}
