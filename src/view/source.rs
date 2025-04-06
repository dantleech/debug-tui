use crate::app::SourceContext;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

pub fn draw(source_context: &SourceContext, frame: &mut Frame, area: Rect) {
    let constraints = vec![Constraint::Length(1), Constraint::Min(1)];
    let rows = Layout::default()
        .margin(0)
        .constraints(constraints)
        .split(area);

    frame.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(
            source_context.filename.clone(),
            Style::default().fg(Color::Green),
        )])),
        rows[0],
    );


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
                false => Span::raw(line.to_string()),
            },
        ]));

        line_no += 1;
    }
    if source_context.line_no as u16 > area.height {
        let offset = (source_context.line_no as u16).saturating_sub(area.height.div_ceil(2));
        lines = lines[offset as usize..].to_vec();
    }
    
    frame.render_widget(Paragraph::new(lines), rows[1]);
}
