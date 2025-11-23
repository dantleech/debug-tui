pub mod layout;
pub mod listen;
pub mod session;
pub mod stack;
pub mod help;
pub mod eval;
pub mod common;
pub mod properties;

pub mod source;
pub mod context;

use crate::app::App;
use crate::event::input::AppEvent;
use ratatui::layout::{Constraint, Rect};
use ratatui::Frame;

pub trait View {
    fn handle(app: &mut App, event: AppEvent) -> Option<AppEvent>;
    fn draw(app: &App, frame: &mut Frame, area: Rect, outer_area: Rect);
}

#[derive(Debug)]
pub enum ComponentType {
    Source,
    Context,
    Eval,
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

pub fn centered_rect_absolute(width: u16, height: u16, r: Rect) -> Rect {
    Rect::new(
        (r.width.saturating_sub(width)) / 2,
        (r.height.saturating_sub(height)) / 2,
        width.min(r.width),
        height.min(r.height),
    )
}
