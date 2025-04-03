pub mod debug;
pub mod session;
pub mod listen;

use std::future::Future;

use anyhow::Result;
use ratatui::{buffer::Buffer, layout::Rect};

use crate::{app::App, event::input::AppEvent};

pub trait View {
    fn handle(&mut self, app: &mut App, event: AppEvent) -> Option<AppEvent>;
    fn draw(&mut self, app: &mut App, f: &mut Buffer, area: Rect);
}
