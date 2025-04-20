use super::View;
use crate::app::{App, CurrentView};
use crate::event::input::AppEvent;
use ratatui::layout::Rect;
use ratatui::widgets::Paragraph;
use ratatui::Frame;

pub struct HelpView {}

impl View for HelpView {
    fn handle(app: &App, event: AppEvent) -> Option<AppEvent> {
        match event {
            AppEvent::Input(_) => {
                if app.is_connected {
                    Some(AppEvent::ChangeView(CurrentView::Session))
                } else {
                    Some(AppEvent::ChangeView(CurrentView::Listen))
                }
            },
            _ => None
        }
    }

    fn draw(_app: &App, frame: &mut Frame, area: Rect) {

        frame.render_widget(Paragraph::new(help()), area);

    }
}

fn help() -> String {
"
Help for you - press any key to return.

Key mappings (prefix with number to repeat):

[n]     next / step into
[N]     step over
[p]     previous (switches to history mode if in current mode)
[o]     step out
[j]     scroll down (with shift to scroll down 10)
[k]     scroll up (with shift to scroll up 10)
[tab]   switch pane
[enter] toggle pane focus (full screen)

Legend:

󱘖 : Connection status
 : Stack depth
 : Iteration number
".to_string()
}


