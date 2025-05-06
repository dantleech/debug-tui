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
    fn handle(_: &App, event: AppEvent) -> Option<AppEvent> {
        match event {
            AppEvent::Scroll(amount) => Some(AppEvent::ScrollStack(amount)),
            _ => None,
        }
    }

    fn draw(app: &App, frame: &mut Frame, area: Rect) {
        let stack = match app.history.current() {
            Some(s) => &s.stack,
            None => return,
        };

        let mut lines: Vec<Line> = Vec::new();

        for entry in &stack.entries {
            let entry_string = format!("{}:{}", entry.filename, entry.line);

            lines.push(Line::from(
                entry_string
                    [entry_string.len().saturating_sub(area.width as usize)..entry_string.len()]
                    .to_string(),
            ));
        }
        frame.render_widget(
            Paragraph::new(lines)
                .alignment(if app.session_view.full_screen {
                    Alignment::Left
                } else {
                    Alignment::Right
                })
                .style(app.theme.scheme().stack_line)
                .scroll(app.session_view.stack_scroll),
            area,
        );
    }
}
