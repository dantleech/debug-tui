pub mod debug;
pub mod layout;
pub mod listen;
pub mod session;

use crate::app::App;
use crate::event::input::AppEvent;
use anyhow::Result;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use std::future::Future;

pub trait View {
    fn handle(&mut self, app: &mut App, event: AppEvent) -> Option<AppEvent>;
    fn draw(&mut self, app: &mut App, f: &mut Buffer, area: Rect);
}
