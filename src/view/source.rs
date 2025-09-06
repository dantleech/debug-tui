use super::View;
use crate::app::App;
use crate::dbgp::client::Property;
use crate::dbgp::client::PropertyType;
use crate::event::input::AppEvent;
use ratatui::layout::Constraint;
use ratatui::layout::Layout;
use ratatui::layout::Position;
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Paragraph;
use ratatui::Frame;

pub struct SourceComponent {}

impl View for SourceComponent {
    fn handle(_: &mut App, event: AppEvent) -> Option<AppEvent> {
        match event {
            AppEvent::Scroll(amount) => Some(AppEvent::ScrollSource(amount)),
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

        // make the app aware of the source area so we can
        // center the source code on the releant line when
        // setpping into code
        app.session_view.source_area.set(rows[0]);

        let mut annotations = vec![];
        let mut lines: Vec<Line> = Vec::new();

        let stack = match history_entry.stack(app.session_view.stack_depth()) {
            None => return,
            Some(stack) => stack
        };

        let analysis = app
            .analyzed_files
            .get(&stack.source.filename.to_string());


        // trunacte the hidden lines
        let truncate_until = app.session_view.source_scroll.0 as u32 + 1;

        for (line_offset, line) in stack.source.source.lines().enumerate() {
            let line_no = line_offset + 1;
            if (line_no as u32) < truncate_until {
                continue
            }
            let is_current_line = stack.source.line_no == line_no as u32;

            lines.push(Line::from(vec![
                Span::styled(format!("{:<6}", line_no), app.theme().source_line_no),
                match is_current_line {
                    // highlight the current line
                    true => Span::styled(line.to_string(), app.theme().source_line_highlight),
                    false => Span::styled(line.to_string(), app.theme().source_line),
                },
            ]));

            // record annotations to add at the end of the line
            let mut labels = vec![Span::raw("// ")];

            if is_current_line {
                if let Some(analysis) = analysis {
                    for (_, var) in analysis.row(line_offset) {
                        let property = stack.get_property(var.name.as_str());
                        if property.is_none() {
                            continue;
                        }
                        match render_label(property.unwrap()) {
                            Some(label) => labels.push(Span::raw(label)),
                            None => continue,
                        };
                        labels.push(Span::raw(","));
                    }
                    if labels.len() > 1 {
                        labels.pop();
                        annotations.push((
                            line_offset,
                            line.len() + 8,
                            Line::from(labels).style(app.theme().source_annotation),
                        ));
                    }
                }
            }
        }

        frame.render_widget(
            Paragraph::new(lines.clone()),
            rows[0],
        );

        for (line_offset, line_length, line) in annotations {
            let x_offset =  rows[0].x + (line_length as u16).saturating_sub(app.session_view.source_scroll.1);
            let area = Rect {
                x: x_offset,
                y: rows[0].y + ((line_offset as u32).saturating_sub(app.session_view.source_scroll.0 as u32) as u16),
                width: rows[0].width.saturating_sub(x_offset),
                height: 1,
            };

            if !frame.buffer_mut().area().contains(Position{x: area.x, y: area.y}) {
                continue
            }
            if !rows[0].contains(Position{x: area.x, y: area.y}) {
                continue
            }

            frame.render_widget(
                Paragraph::new(line.clone()).scroll((
                    0,
                    app.session_view.source_scroll.1.saturating_sub(line_length as u16))
                ),
                area
            );
        }
    }
}

fn render_label(property: &Property) -> Option<String> {
    Some(match property.property_type {
        PropertyType::Object | PropertyType::Array | PropertyType::Hash => {
            format!("{}{{{}}}", property.type_name(), {
                let mut labels: Vec<String> = Vec::new();
                for child in &property.children {
                    let label = render_label(child);
                    if label.is_none() {
                        continue;
                    }

                    labels.push(format!("{}:{}", child.name, label.unwrap()));
                }
                labels.join(",")
            })
        }
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
        PropertyType::Resource => property.value.clone().unwrap_or("".to_string()),
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
        assert_eq!(
            Some(String::from("resource id='18' type='stream'")),
            render_label(&create_resource())
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

    fn create_resource() -> Property {
        Property {
            name: "handle".to_string(),
            fullname: "handle".to_string(),
            classname: None,
            page: None,
            pagesize: None,
            property_type: PropertyType::Resource,
            facet: Some("private".to_string()),
            size: None,
            children: vec![],
            key: None,
            address: None,
            encoding: None,
            value: Some("resource id='18' type='stream'".to_string()),
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
