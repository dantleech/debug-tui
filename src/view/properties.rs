
use crate::dbgp::client::Property;
use crate::dbgp::client::PropertyType;
use crate::theme::Scheme;
use ratatui::text::Line;
use ratatui::text::Span;

pub fn draw_properties(
    theme: &Scheme,
    properties: &Vec<Property>,
    lines: &mut Vec<Line>,
    level: usize,
    filter_path: &mut Vec<&str>,
    truncate_until: &u32,
    line_no: &mut u32,
) {
    let filter = filter_path.pop();

    for property in properties {
        if let Some(filter) = filter {
            if !property.name.starts_with(filter) {
                continue;
            }
        }
        let mut spans = vec![
            Span::raw("  ".repeat(level)),
            Span::styled(property.name.to_string(), theme.syntax_label),
            Span::raw(" ".to_string()),
            Span::styled(
                property.type_name(),
                match property.property_type {
                    PropertyType::Object => theme.syntax_type_object,
                    _ => theme.syntax_type,
                },
            ),
            Span::raw(" = ".to_string()),
            render_value(theme, property),
        ];

        let delimiters = match property.property_type {
            PropertyType::Array => ("[", "]"),
            _ => ("{", "}"),
        };

        if !property.children.is_empty() {
            spans.push(Span::raw(delimiters.0).style(theme.syntax_brace));
        }

        // only "render" lines that we need to
        if *line_no >= *truncate_until {
            lines.push(Line::from(spans));
        }
        *line_no += 1;

        if !property.children.is_empty() {
            draw_properties(theme, &property.children, lines, level + 1, filter_path, truncate_until,  line_no);
            if *line_no >= *truncate_until {
                lines.push(Line::from(vec![Span::raw(delimiters.1)]).style(theme.syntax_brace));
            }
            *line_no += 1;
        }
    }
}

pub fn render_value<'a>(theme: &Scheme, property: &Property) -> Span<'a> {
    let value = property.value.clone().unwrap_or("".to_string());
    match property.property_type {
        PropertyType::Bool => Span::styled(value, theme.syntax_literal),
        PropertyType::Int => Span::styled(value, theme.syntax_literal),
        PropertyType::Float => Span::styled(value, theme.syntax_literal),
        PropertyType::String => Span::styled(format!("\"{}\"", value), theme.syntax_literal_string),
        PropertyType::Array => Span::styled(value, theme.syntax_literal),
        PropertyType::Hash => Span::styled(value, theme.syntax_literal),
        PropertyType::Object => Span::styled(value, theme.syntax_literal),
        PropertyType::Resource => Span::styled(value, theme.syntax_literal),
        PropertyType::Undefined => Span::styled(value, theme.syntax_literal),
        _ => Span::styled(value, theme.syntax_literal),
    }
}

#[cfg(test)]
mod test {
    use crate::theme::Theme;
    use anyhow::Result;

    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_draw_properties_empty() -> Result<()> {
        let mut lines = vec![];
        draw_properties(
            &Theme::SolarizedDark.scheme(),
            &Vec::new(),
            &mut lines,
            0,
            &mut Vec::new(),
            &0,
            &mut 0,
        );
        assert_eq!(0, lines.len());
        Ok(())
    }

    #[test]
    fn test_draw_properties_two_levels() -> Result<()> {
        let mut lines = vec![];
        let mut prop1 = Property::default();
        let mut prop2 = Property::default();
        prop2.name = "bar".to_string();
        prop1.children = vec![
            prop2
        ];
        prop1.name = "foo".to_string();

        draw_properties(
            &Theme::SolarizedDark.scheme(),
            &vec![
                prop1
            ],
            &mut lines,
            0,
            &mut Vec::new(),
            &0,
            &mut 0,
        );
        assert_eq!(vec![
            "foo string = \"\"{",
            "  bar string = \"\"",
            "}",
        ], lines.iter().map(
            |l| { l.to_string()}
        ).collect::<Vec<String>>());
        Ok(())
    }

    #[test]
    fn test_filter_property_multiple_level() -> Result<()> {
        let mut lines = vec![];
        let mut prop1 = Property::default();
        let mut prop2 = Property::default();
        let prop3 = Property::default();

        prop2.name = "bar".to_string();
        prop1.children = vec![
            prop2
        ];
        prop1.name = "foo".to_string();

        // segments are reversed
        let filter = &mut vec![
            "bar",
            "foo",
        ];

        draw_properties(
            &Theme::SolarizedDark.scheme(),
            &vec![
                prop1,
                prop3
            ],
            &mut lines,
            0,
            filter,
            &0,
            &mut 0,
        );

        assert_eq!(vec![
            "foo string = \"\"{",
            "  bar string = \"\"",
            "}",
        ], lines.iter().map(
            |l| { l.to_string()}
        ).collect::<Vec<String>>());

        Ok(())
    }
}
