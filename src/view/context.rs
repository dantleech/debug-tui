use crossterm::terminal::EnableLineWrap;
use ratatui::{layout::Rect, text::Line, widgets::{Paragraph, Wrap}, Frame};

use crate::dbgp::client::{ContextGetResponse, Property};

pub fn draw(context: &ContextGetResponse, frame: &mut Frame, area: Rect) {
    let mut lines: Vec<Line> = vec![];
    draw_properties(&context.properties, &mut lines);

    frame.render_widget(Paragraph::new(lines).wrap(Wrap{trim: true}), area);
}

pub fn draw_properties(properties: &Vec<Property>, lines: &mut Vec<Line>) {
    for property in properties {
        lines.push(Line::from(format!(
            "{} {} = {}",
            property.name,
            property.property_type,
            property.clone().value.unwrap_or("".to_string())
        )));
    }
}
