use crate::app::SourceContext;
use crate::event::input::AppEvent;
use ratatui::layout::Constraint;
use ratatui::layout::Layout;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Paragraph;
use ratatui::Frame;

pub fn draw(source_context: &SourceContext, frame: &mut Frame, area: Rect) {
    let constraints = vec![Constraint::Min(1)];
    let rows = Layout::default()
        .margin(0)
        .constraints(constraints)
        .split(area);

    let mut lines: Vec<Line> = Vec::new();
    let mut line_no = 1;

    for line in source_context.source.lines() {
        lines.push(Line::from(vec![
            Span::styled(
                format!("{:<6}", line_no),
                Style::default().fg(Color::Yellow),
            ),
            match source_context.line_no == line_no {
                true => Span::styled(line.to_string(), Style::default().bg(Color::Blue)),
                false => Span::styled(line.to_string(), Style::default().fg(Color::White)),
            },
        ]));

        line_no += 1;
    }
    if source_context.line_no as u16 > area.height {
        let offset = (source_context.line_no as u16).saturating_sub(area.height.div_ceil(2));
        lines = lines[offset as usize..].to_vec();
    }

    frame.render_widget(Paragraph::new(lines), rows[0]);
}

pub(crate) fn handle(event: AppEvent) -> Option<crate::event::input::AppEvent> {
    todo!()
}
