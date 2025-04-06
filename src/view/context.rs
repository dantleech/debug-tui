use ratatui::{layout::Rect, text::Line, widgets::Paragraph, Frame};

use crate::dbgp::client::ContextGetResponse;

pub fn draw(context: &ContextGetResponse, frame: &mut Frame, area: Rect) {
    let lines: Vec<Line> = context.properties.iter().map(|property| {
        Line::from(format!(
            "{} {} = {}",
            property.name,
            property.property_type,
            property.clone().value.unwrap_or("".to_string())
        ))
    }).collect();

    frame.render_widget(Paragraph::new(lines), area);
}
