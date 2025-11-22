use super::View;
use crate::app::App;
use crate::event::input::AppEvent;
use crate::view::eval::ChannelsComponent;
use ratatui::layout::Rect;
use ratatui::widgets::{Block, Borders, Clear};
use ratatui::Frame;

pub struct ListenView {}

impl View for ListenView {
    fn handle(_app: &mut App, _key: AppEvent) -> Option<AppEvent> {
        None
    }

    fn draw(app: &App, frame: &mut Frame, _inner_area: Rect, area: Rect) {

        let block = Block::default()
            .borders(Borders::all())
            .style(app.theme().pane_border_active);

        frame.render_widget(Clear, area);
        frame.render_widget(&block, area);
        ChannelsComponent::draw(app, frame, block.inner(area), area);
    }
}
