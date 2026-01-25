use super::View;
use crate::app::App;
use crate::event::input::AppEvent;
use crate::view::eval::ChannelsComponent;
use crossterm::event::KeyCode;
use ratatui::layout::Rect;
use ratatui::widgets::{Block, Borders, Clear};
use ratatui::Frame;

pub struct ListenView {}

impl View for ListenView {
    fn handle(app: &mut App, event: AppEvent) -> Option<AppEvent> {
        let input_event = match event {
            AppEvent::Input(key_event) => key_event,
            _ => return ChannelsComponent::handle(app, event)
        };

        match input_event.code {
            KeyCode::Char('R') => Some(AppEvent::RestartProcess),
            _ => None,
        }
    }

    fn draw(app: &App, frame: &mut Frame, area: Rect, _outer_area: Rect) {

        let block = Block::default()
            .borders(Borders::all())
            .style(app.theme().pane_border_active);

        frame.render_widget(Clear, area);
        frame.render_widget(&block, area);
        ChannelsComponent::draw(app, frame, block.inner(area), area);
    }
}
