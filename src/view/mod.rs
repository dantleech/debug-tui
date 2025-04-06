pub mod debug;
pub mod layout;
pub mod listen;
pub mod session;
pub mod history;

pub mod source;
pub mod context;

use crate::app::App;
use crate::event::input::AppEvent;
use ratatui::layout::Rect;
use ratatui::Frame;

pub trait View {
    fn handle(app: &App, event: AppEvent) -> Option<AppEvent>;
    fn draw(app: &mut App, f: &mut Frame, area: Rect);
}
