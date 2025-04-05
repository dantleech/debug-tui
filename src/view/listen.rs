use super::View;
use crate::app::App;
use crate::event::input::AppEvent;
use ratatui::Frame;

pub struct ListenView {}

impl View for ListenView {
    fn handle(app: &App, key: AppEvent) -> Option<AppEvent> {
        None
    }

    fn draw(app: &mut App, f: &mut Frame, area: ratatui::prelude::Rect) {}
}
