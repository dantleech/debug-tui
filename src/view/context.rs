use ratatui::{layout::Rect, style::{Color, Style}, text::{Line, Span}, widgets::{Paragraph, Wrap}, Frame};

use crate::dbgp::client::{ContextGetResponse, Property};

pub fn draw(context: &ContextGetResponse, frame: &mut Frame, area: Rect) {
    let mut lines: Vec<Line> = vec![];
    draw_properties(&context.properties, &mut lines, 0);

    frame.render_widget(Paragraph::new(lines).wrap(Wrap{trim: false}), area);
}

pub fn draw_properties(properties: &Vec<Property>, lines: &mut Vec<Line>, level: usize) {
    for property in properties {
        let value = property.value.clone().unwrap_or("".to_string());
        lines.push(Line::from(vec![
            Span::raw("  ".repeat(level)),
            Span::styled(property.name.to_string(), Style::default().fg(Color::White)),
            Span::raw(" ".to_string()),
            Span::styled(property.property_type.to_string(), Style::default().fg(Color::Blue)),
            Span::raw(" = ".to_string()),
            match property.property_type.as_str() {
                "bool" => Span::styled(value, Style::default().fg(Color::LightRed)),
                "int" => Span::styled(value, Style::default().fg(Color::LightBlue)),
                "float" => Span::styled(value, Style::default().fg(Color::LightBlue)),
                "string" => Span::styled(value, Style::default().fg(Color::LightGreen)),
                "array" => Span::styled(value, Style::default().fg(Color::Cyan)),
                "hash" => Span::styled(value, Style::default().fg(Color::Cyan)),
                "object" => match &property.classname {
                    Some(name) => Span::styled(name.to_string(), Style::default().fg(Color::Red)),
                    None => Span::styled(value, Style::default().fg(Color::Red)),
                },
                "resource" => Span::styled(value, Style::default().fg(Color::Red)),
                "undefined" => Span::styled(value, Style::default().fg(Color::White)),
                _ => Span::raw(value),
            },
        ]));

        if property.children.len() > 0 {
            draw_properties(&property.children, lines, level + 1);
        }
    }
}
