use super::View;
use crate::app::App;
use crate::event::input::AppEvent;
use ratatui::layout::Alignment;
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::widgets::Paragraph;
use ratatui::Frame;

pub struct StackComponent {}

impl View for StackComponent {
    fn handle(_: &mut App, event: AppEvent) -> Option<AppEvent> {
        match event {
            AppEvent::Scroll(amount) => Some(AppEvent::ScrollStack(amount)),
            _ => None,
        }
    }

    fn draw(app: &App, frame: &mut Frame, _inner_area: Rect, area: Rect) {
        let entry = match app.history.current() {
            Some(s) => s,
            None => return,
        };

        let mut lines: Vec<Line> = Vec::new();

        for stack in &entry.stacks {
            let entry_string = format!("{}:{}", stack.source.filename, stack.source.line_no);

            lines.push(Line::from(
                entry_string
                    [entry_string.len().saturating_sub(area.width as usize)..entry_string.len()]
                    .to_string(),
            ).style(match stack.level == app.session_view.stack_depth() {
                true => app.theme().source_line_highlight,
                false => app.theme().source_line,
            }));
        }
        let y_scroll = match (app.session_view.stack_depth() + 1) > area.height {
            true => (app.session_view.stack_depth() + 1) - area.height,
            false => 0,
        };
        frame.render_widget(
            Paragraph::new(lines)
                .alignment(if app.session_view.full_screen {
                    Alignment::Left
                } else {
                    Alignment::Right
                })
                .style(app.theme.scheme().stack_line)
                .scroll((y_scroll, app.session_view.stack_scroll.1)),
            area,
        );
    }
}
