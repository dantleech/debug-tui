pub mod debug;
pub mod layout;
pub mod listen;
pub mod session;

pub mod source;
pub mod context;

use crate::app::App;
use crate::event::input::AppEvent;
use ratatui::layout::{Constraint, Rect};
use ratatui::Frame;

pub trait View {
    fn handle(app: &App, event: AppEvent) -> Option<AppEvent>;
    fn draw(app: &mut App, f: &mut Frame, area: Rect);
}

pub enum ComponentType {
    Source,
    Context,
}

pub struct Pane {
    pub component_type: ComponentType,
    pub constraint: Constraint,
}
