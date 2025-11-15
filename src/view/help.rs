use super::View;
use crate::app::{App, ListenStatus, SelectedView};
use crate::event::input::AppEvent;
use ratatui::layout::Rect;
use ratatui::widgets::Paragraph;
use ratatui::Frame;

pub struct HelpView {}

impl View for HelpView {
    fn handle(app: &mut App, event: AppEvent) -> Option<AppEvent> {
        match event {
            AppEvent::Input(_) => {
                if app.listening_status == ListenStatus::Connected{
                    Some(AppEvent::ChangeView(SelectedView::Session))
                } else {
                    Some(AppEvent::Listen)
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

[r]     run
[R]     restart php script (if provided)
[n]     next / step into
[N]     step over
[p]     previous (switches to history mode if in current mode)
[o]     step out
[d]     disconnect
[e]     enter an expression
[j]     scroll down
[J]     scroll down 10
[k]     scroll up
[K]     scroll up 10
[h]     scroll left
[H]     scroll left 10
[l]     scroll right
[L]     scroll right 10
[+]     increase context depth
[-]     decrease context depth
[t]     rotate the theme
[f]     Filter (context pane) - use dot notation to filter on multiple levels.
[enter] toggle pane focus (full screen)

Legend:

󱘖 : Connection status
 : Stack depth
 : Iteration number
".to_string()
}


