use crate::app::App;
use crate::dbgp::client::Property;
use crate::event::input::AppEvent;
use ratatui::layout::Constraint;
use ratatui::layout::Layout;
use ratatui::layout::Position;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use super::View;

pub struct SourceComponent {
}

impl View for SourceComponent {
    fn handle(_: &App, event: AppEvent) -> Option<AppEvent> {
        match event {
            AppEvent::ScrollDown(amount) => Some(AppEvent::ScrollSource(amount)),
            AppEvent::ScrollUp(amount) => Some(AppEvent::ScrollSource(-amount)),
            _ => None,
        }
    }

    fn draw(app: &App, frame: &mut Frame, area: Rect) {
        let history_entry = match app.history.current() {
            Some(s) => s,
            None => return,
        };

        let constraints = vec![Constraint::Min(1)];
        let rows = Layout::default()
            .margin(0)
            .constraints(constraints)
            .split(area);

        let mut lines: Vec<Line> = Vec::new();
        let mut line_no = 1;
        let analysis = app.analyzed_files.get(&history_entry.source.filename.to_string());

        let mut annotations = vec![];
        for line in history_entry.source.source.lines() {
            lines.push(Line::from(vec![
                Span::styled(
                    format!("{:<6}", line_no),
                    Style::default().fg(Color::Yellow),
                ),
                match history_entry.source.line_no == line_no {
                    true => Span::styled(line.to_string(), Style::default().bg(Color::Blue)),
                    false => Span::styled(line.to_string(), Style::default().fg(Color::White)),
                },
            ]));
            if let Some(analysis) = analysis {
                let mut labels = vec![
                    Span::raw("// ").style(Style::default().fg(Color::DarkGray)),
                ];
                for (_, var) in analysis.row(line_no as usize - 1) {
                    let property = history_entry.get_property(var.name.as_str());
                    if property.is_none() {
                        continue;
                    }
                    match render_label(property.unwrap()) {
                        Some(label) => labels.push(label),
                        None => continue,
                    };
                    labels.push(Span::raw(",").style(Style::default().fg(Color::DarkGray)));
                }
                if labels.len() > 1 {
                    labels.pop();
                    annotations.push((line_no, line.len() + 8, Line::from(labels)));
                }
            };
            line_no += 1;
        }

        let mut offset = 0;
        if history_entry.source.line_no as u16 > area.height {
            offset = (history_entry.source.line_no as u16).saturating_sub(area.height.div_ceil(2));
            lines = lines[offset as usize..].to_vec();
        }
        frame.render_widget(Paragraph::new(lines.clone()).scroll((app.session_view.source_scroll, 0)), rows[0]);

        for (line_no, line_length, line) in annotations {
            let position = Position{ x: line_length as u16, y: line_no.saturating_sub(offset as u32) as u16 + 1 };
            if !rows[0].contains(position) {
                break
            }

            frame.buffer_mut().set_line(position.x, position.y, &line, rows[0].width);
        }
    }
}

fn render_label(property: &Property) -> Option<Span> {
    property.value.as_ref().map(|value| Span::default().content(value.clone()).style(Style::default().fg(Color::DarkGray)))
}
