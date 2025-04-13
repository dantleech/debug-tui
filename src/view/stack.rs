use crate::app::App;
use crate::event::input::AppEvent;
use ratatui::layout::Alignment;
use ratatui::layout::Constraint;
use ratatui::layout::Layout;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Wrap;
use ratatui::Frame;

use super::View;

pub struct StackComponent {
}

impl View for StackComponent {
    fn handle(_: &App, event: AppEvent) -> Option<AppEvent> {
        match event {
            AppEvent::ScrollDown(amount) => Some(AppEvent::ScrollStack(amount)),
            AppEvent::ScrollUp(amount) => Some(AppEvent::ScrollStack(-amount)),
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
            let entry_string = format!("{}:{}", entry.filename.to_string(), entry.line.to_string());

            lines.push(Line::from(
                entry_string[entry_string.len().saturating_sub(area.width as usize)..entry_string.len()].to_string()
            ));
        }
        frame.render_widget(
            Paragraph::new(lines)
                .alignment(if app.session_view.full_screen == true { Alignment::Left } else { Alignment::Right} )
                .style(Style::default().fg(Color::White))
                .scroll((app.session_view.stack_scroll, 0)),
            area
        );
    }
}

