use super::View;
use crate::app::App;
use crate::dbgp::client::Property;
use crate::dbgp::client::PropertyType;
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

pub struct SourceComponent {}

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

        let mut annotations = vec![];
        let mut lines: Vec<Line> = Vec::new();

        let analysis = app
            .analyzed_files
            .get(&history_entry.source.filename.to_string());

        for (line_no, line) in history_entry.source.source.lines().enumerate() {
            let is_current_line = history_entry.source.line_no == line_no as u32+ 1;

            lines.push(Line::from(vec![
                Span::styled(
                    format!("{:<6}", line_no),
                    Style::default().fg(Color::Yellow),
                ),
                match is_current_line {
                    // highlight the current line
                    true => Span::styled(line.to_string(), Style::default().bg(Color::Blue)),
                    false => Span::styled(line.to_string(), Style::default().fg(Color::White)),
                },
            ]));

            // record annotations to add at the end of the line
            let mut labels = vec![Span::raw("// ").style(Style::default().fg(Color::DarkGray))];

            if is_current_line {
                if let Some(analysis) = analysis {
                    for (_, var) in analysis.row(line_no) {
                        let property = history_entry.get_property(var.name.as_str());
                        if property.is_none() {
                            continue;
                        }
                        match render_label(property.unwrap()) {
                            Some(label) => labels.push(Span::raw(label)),
                            None => continue,
                        };
                        labels.push(Span::raw(",").style(Style::default().fg(Color::DarkGray)));
                    }
                    if labels.len() > 1 {
                        labels.pop();
                        annotations.push((line_no + 1, line.len() + 8, Line::from(labels).style(Style::default().fg(Color::DarkGray))));
                    }
                }
            }
        }


        let scroll:u16 = if history_entry.source.line_no as u16 > area.height {
            let center = (history_entry.source.line_no as u16).saturating_sub(area.height.div_ceil(2)) as i16;
            center.saturating_add(app.session_view.source_scroll.unwrap_or(0)).max(0) as u16
        } else {
            app.session_view.source_scroll.unwrap_or(0).max(0) as u16
        };

        frame.render_widget(
            Paragraph::new(lines.clone()).scroll((scroll, 0)),
            rows[0],
        );

        for (line_no, line_length, line) in annotations {
            let position = Position {
                x: line_length as u16,
                y: (line_no as u32).saturating_sub(scroll as u32) as u16 + 1,
            };
            if !rows[0].contains(position) {
                continue;
            }

            frame
                .buffer_mut()
                .set_line(position.x, position.y, &line, rows[0].width);
        }
    }
}

fn render_label(property: &Property) -> Option<String> {
    Some(match property.property_type {
        PropertyType::Object|PropertyType::Array|PropertyType::Hash => format!("{}{{{}}}", property.type_name(), {
            let mut labels: Vec<String> = Vec::new();
            for child in &property.children {
                let label = render_label(child);
                if label.is_none() {
                    continue;
                }

                labels.push(format!("{}:{}", child.name, label.unwrap()));
            }
            labels.join(",")
        }),
        PropertyType::Bool => {
            if property.value_is("1") {
                String::from("true")
            } else {
                String::from("false")
            }
        }
        PropertyType::Int => property.value.clone().unwrap_or("".to_string()),
        PropertyType::Float => property.value.clone().unwrap_or("".to_string()),
        PropertyType::String => format!("\"{}\"", property.value.clone().unwrap_or("".to_string())),
        PropertyType::Null => String::from("null"),
        PropertyType::Resource => todo!(),
        PropertyType::Undefined => String::from("undefined"),
    })
}

#[cfg(test)]
mod test {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_render_label() {
        assert_eq!(
            Some(String::from("Foo{true:true,bar:\"foo\"}")),
            render_label(&create_object())
        );
        assert_eq!(
            Some(String::from("true")),
            render_label(&create_simple_property(PropertyType::Bool, "1"))
        );
        assert_eq!(
            Some(String::from("false")),
            render_label(&create_simple_property(PropertyType::Bool, "0"))
        );
        assert_eq!(
            Some(String::from("12")),
            render_label(&create_simple_property(PropertyType::Int, "12"))
        );
        assert_eq!(
            Some(String::from("\"12\"")),
            render_label(&create_simple_property(PropertyType::String, "12"))
        );
        assert_eq!(
            Some(String::from("null")),
            render_label(&create_simple_property(PropertyType::Null, ""))
        );
        assert_eq!(
            Some(String::from("undefined")),
            render_label(&create_simple_property(PropertyType::Undefined, ""))
        );
    }

    fn create_simple_property(property_type: PropertyType, value: &str) -> Property {
        Property {
            name: "test".to_string(),
            fullname: "test".to_string(),
            classname: None,
            page: None,
            pagesize: None,
            property_type,
            facet: None,
            size: None,
            children: Vec::new(),
            key: None,
            address: None,
            encoding: None,
            value: Some(value.to_string()),
        }
    }

    fn create_object() -> Property {
        Property {
            name: "$this".to_string(),
            fullname: "$this".to_string(),
            classname: Some("Foo".to_string()),
            page: Some(0),
            pagesize: Some(32),
            property_type: PropertyType::Object,
            facet: None,
            size: None,
            children: vec![
                Property {
                    name: "true".to_string(),
                    fullname: "true".to_string(),
                    classname: None,
                    page: None,
                    pagesize: None,
                    property_type: PropertyType::Bool,
                    facet: Some("public".to_string()),
                    size: None,
                    children: vec![],
                    key: None,
                    address: None,
                    encoding: None,
                    value: Some("1".to_string()),
                },
                Property {
                    name: "bar".to_string(),
                    fullname: "bar".to_string(),
                    classname: None,
                    page: None,
                    pagesize: None,
                    property_type: PropertyType::String,
                    facet: Some("public".to_string()),
                    size: Some(3),
                    children: vec![],
                    key: None,
                    address: None,
                    encoding: Some("base64".to_string()),
                    value: Some("foo".to_string()),
                },
            ],
            key: None,
            address: None,
            encoding: None,
            value: None,
        }
    }
}
