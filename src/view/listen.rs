use super::View;
use crate::app::App;
use crate::event::input::{AppEvent, AppEvents};
use ratatui::Frame;

pub struct ListenView {}

impl View for ListenView {
    fn handle(_app: &App, _key: AppEvent) -> AppEvents {
        AppEvents::none()
    }

    fn draw(_app: &App, _f: &mut Frame, _area: ratatui::prelude::Rect) {}
}
