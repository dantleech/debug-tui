pub mod debug;
pub mod layout;
pub mod listen;
pub mod session;
pub mod stack;

pub mod source;
pub mod context;

use crate::app::App;
use crate::event::input::{AppEvent, AppEvents};
use ratatui::layout::{Constraint, Rect};
use ratatui::Frame;

pub trait View {
    fn handle(app: &App, event: AppEvent) -> AppEvents;
    fn draw(app: &App, frame: &mut Frame, area: Rect);
}

#[derive(Debug)]
pub enum ComponentType {
    Source,
    Context,
    Stack,
}

#[derive(Debug)]
pub struct Pane {
    pub component_type: ComponentType,
    pub constraint: Constraint,
}
