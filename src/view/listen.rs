use anyhow::Result;

use super::View;

pub struct ListenView {
}

impl View for ListenView {
    fn handle(&mut self, app: &mut crate::app::App, key: crate::event::input::AppEvent) -> Result<()>{
        todo!()
    }

    fn draw(&mut self, app: &mut crate::app::App, f: &mut ratatui::prelude::Buffer, area: ratatui::prelude::Rect) {
        todo!()
    }
}
