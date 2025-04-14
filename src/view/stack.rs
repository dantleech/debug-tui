use crate::app::App;
use crate::event::input::AppEvent;
use crate::event::input::AppEvents;
use ratatui::layout::Alignment;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use super::View;

pub struct StackComponent {
}

impl View for StackComponent {
    fn handle(_: &App, event: AppEvent) -> AppEvents {
        match event {
            AppEvent::ScrollDown(amount) => AppEvents::one(AppEvent::ScrollStack(amount)),
            AppEvent::ScrollUp(amount) => AppEvents::one(AppEvent::ScrollStack(-amount)),
            _ => AppEvents::none(),
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
                entry_string[entry_string.len().saturating_sub(area.width as usize)..entry_string.len()].to_string()
            ));
        }
        frame.render_widget(
            Paragraph::new(lines)
                .alignment(if app.session_view.full_screen { Alignment::Left } else { Alignment::Right} )
                .style(Style::default().fg(Color::White))
                .scroll((app.session_view.stack_scroll, 0)),
            area
        );
    }
}

