use super::View;
use crate::event::input::AppEvent;
use anyhow::Result;

pub struct ListenView {}

impl View for ListenView {
    fn handle(
        &mut self,
        app: &mut crate::app::App,
        key: crate::event::input::AppEvent,
    ) -> Option<AppEvent> {
        todo!()
    }

    fn draw(
        &mut self,
        app: &mut crate::app::App,
        f: &mut ratatui::prelude::Buffer,
        area: ratatui::prelude::Rect,
    ) {
        todo!()
    }
}
