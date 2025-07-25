pub mod layout;
pub mod listen;
pub mod session;
pub mod stack;
pub mod help;

pub mod source;
pub mod context;

use crate::app::App;
use crate::event::input::AppEvent;
use ratatui::layout::{Constraint, Rect};
use ratatui::Frame;

pub trait View {
    fn handle(app: &mut App, event: AppEvent) -> Option<AppEvent>;
    fn draw(app: &App, frame: &mut Frame, area: Rect);
}

#[derive(Debug)]
pub enum ComponentType {
    Source,
    Context,
    Stack,
}

#[derive(Debug, PartialEq)]
pub enum Col {
    Left,
    Right,
}

#[derive(Debug)]
pub struct Pane {
    pub component_type: ComponentType,
    pub constraint: Constraint,
    pub col: Col,
}

pub type Scroll = (i16,i16);
