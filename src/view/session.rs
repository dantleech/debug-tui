use super::context;
use super::source;
use super::View;
use crate::app::App;
use crate::app::CurrentView;
use crate::app::InputMode;
use crate::event::input::AppEvent;
use crossterm::event::KeyCode;
use ratatui::layout::Constraint;
use ratatui::layout::Layout;
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
                        'p' => Some(AppEvent::ChangeView(CurrentView::History)),
                        _ => None,
                    };
                }
            }
        }
        None
    }

    fn draw(app: &mut App, frame: &mut Frame, area: ratatui::prelude::Rect) {
        let layout = Layout::horizontal(vec![
            Constraint::Percentage(75),
            Constraint::Percentage(25),
        ]).split(area);

        if let Some(source_context) = &app.source {
            source::draw(&source_context, frame, layout[0]);
        }
        if let Some(context_get) = &app.context {
            context::draw(&context_get, frame, layout[1]);
        }
    }
}

impl SessionView {
    pub(crate) fn new() -> Self {
        Self {}
    }
}
