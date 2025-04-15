use super::View;
use crate::app::App;
use crate::event::input::AppEvent;
use ratatui::Frame;

pub struct ListenView {}

impl View for ListenView {
    fn handle(_app: &App, _key: AppEvent) -> Option<AppEvent> {
        None
    }

    fn draw(_app: &App, _f: &mut Frame, _area: ratatui::prelude::Rect) {}
}
