use crate::app::SourceContext;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

pub fn source_widget(context: &SourceContext, area: Rect) -> Paragraph {
    let mut lines: Vec<Line> = Vec::new();
    let mut line_no = 1;

    for line in context.source.lines() {
        lines.push(Line::from(vec![
            Span::styled(
                format!("{:<6}", line_no),
                Style::default().fg(Color::Yellow),
            ),
            match context.line_no == line_no {
                true => Span::styled(line.to_string(), Style::default().bg(Color::Blue)),
                false => Span::raw(line.to_string()),
            },
        ]));

        line_no += 1;
    }
    if context.line_no as u16 > area.height {
        let offset = (context.line_no as u16).saturating_sub(area.height.div_ceil(2));
        lines = lines[offset as usize..].to_vec();
    }
    Paragraph::new(lines)
}
